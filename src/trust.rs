//! Who signed a program file, as Windows itself judges it (Authenticode).
//!
//! The updater runs an installer it has just downloaded. Before it does, the installer must carry a valid signature
//! from the same publisher as the running app: a file that merely has the right name and checksum on the download
//! page is not enough.
use std::path::Path;

/// The name on the certificate a file was signed with, if the file carries a signature Windows accepts: unbroken,
/// from a trusted publisher certificate. `None` for an unsigned, altered or untrusted file.
#[cfg(windows)]
pub fn signer(path: &Path) -> Option<String> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Security::Cryptography::{CertGetNameStringW, CERT_NAME_SIMPLE_DISPLAY_TYPE};
    use windows::Win32::Security::WinTrust::{
        WTHelperGetProvCertFromChain, WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData, WinVerifyTrust,
        WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO, WTD_CACHE_ONLY_URL_RETRIEVAL,
        WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    };

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(wide.as_ptr()),
        ..Default::default()
    };
    // No window and no network: the check is of the file's own bytes against the certificates on this machine.
    let mut data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL,
        ..Default::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let none = HWND(std::ptr::null_mut());
    unsafe {
        let verdict = WinVerifyTrust(none, &mut action, &mut data as *mut _ as *mut c_void);
        let mut name = None;
        if verdict == 0 {
            // The verified state names the certificate at the head of the signer's chain.
            let provider = WTHelperProvDataFromStateData(data.hWVTStateData);
            if !provider.is_null() {
                let signer = WTHelperGetProvSignerFromChain(provider, 0, false, 0);
                if !signer.is_null() {
                    let cert = WTHelperGetProvCertFromChain(signer, 0);
                    if !cert.is_null() && !(*cert).pCert.is_null() {
                        let mut buf = [0u16; 512];
                        let len = CertGetNameStringW((*cert).pCert, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, Some(&mut buf)) as usize;
                        // The length counts the closing zero.
                        if len > 1 {
                            name = Some(String::from_utf16_lossy(&buf[..len - 1]));
                        }
                    }
                }
            }
        }
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(none, &mut action, &mut data as *mut _ as *mut c_void);
        name
    }
}

#[cfg(not(windows))]
pub fn signer(_path: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_is_not_signed_has_no_signer() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kuro-trust-test-{}.exe", std::process::id()));
        std::fs::write(&path, b"MZ not really a program").unwrap();
        assert_eq!(signer(&path), None);
        let _ = std::fs::remove_file(&path);
        assert_eq!(signer(&dir.join("kuro-no-such-file.exe")), None);
    }

    /// With a signed file at hand: `KURO_TEST_SIGNED=<file> KURO_TEST_SIGNER=<name> cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn a_signed_file_names_its_signer() {
        let file = std::env::var("KURO_TEST_SIGNED").expect("KURO_TEST_SIGNED");
        assert_eq!(signer(Path::new(&file)), std::env::var("KURO_TEST_SIGNER").ok());
    }
}
