//! The sound library's key, and on Windows the exe icon, version info and manifest (Per-Monitor-V2 DPI, common
//! controls v6 for the file dialogs).
use std::path::PathBuf;

/// The key that opens the bundled sound library (`assets/library.kpk`) is not in the repository: it comes from
/// `library.key` beside `Cargo.toml`, or from `KURO_LIBRARY_KEY`, as 64 hexadecimal digits. It goes into the program
/// as two halves that are only joined when the app runs, so it is not one readable string in the exe. A build
/// without the key works, with an empty library.
fn library_key(manifest: &std::path::Path, out: &std::path::Path) {
    println!("cargo:rerun-if-changed=library.key");
    println!("cargo:rerun-if-env-changed=KURO_LIBRARY_KEY");
    let text = std::env::var("KURO_LIBRARY_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| std::fs::read_to_string(manifest.join("library.key")).ok());
    let code = match text {
        None => {
            println!("cargo:warning=no library.key: this build cannot open the bundled sound library");
            "pub const LIBRARY_KEY: Option<([u8; 32], [u8; 32])> = None;\n".to_string()
        }
        Some(text) => {
            let hex = text.trim();
            assert!(hex.len() == 64 && hex.is_ascii(), "the library key must be 64 hexadecimal digits");
            let key: Vec<u8> = (0..32).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("the library key must be hexadecimal")).collect();
            let mask: Vec<u8> = (0..32).map(|i| key[(i * 7 + 3) % 32].rotate_left(3) ^ 0xA5 ^ (i as u8).wrapping_mul(29)).collect();
            let masked: Vec<u8> = key.iter().zip(&mask).map(|(k, m)| k ^ m).collect();
            format!("pub const LIBRARY_KEY: Option<([u8; 32], [u8; 32])> = Some(({masked:?}, {mask:?}));\n")
        }
    };
    std::fs::write(out.join("library_key.rs"), code).expect("write library_key.rs");
}

fn main() {
    library_key(
        &PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()),
        &PathBuf::from(std::env::var("OUT_DIR").unwrap()),
    );
    println!("cargo:rerun-if-changed=build/app.manifest");
    println!("cargo:rerun-if-changed=build/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let v = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let mut n: Vec<u32> = v.split(['.', '-']).take(3).map(|p| p.parse().unwrap_or(0)).collect();
    n.resize(3, 0);
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("build");
    let path = |f: &str| dir.join(f).display().to_string().replace('\\', "/");
    let commas = format!("{},{},{},0", n[0], n[1], n[2]);
    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {commas}
PRODUCTVERSION {commas}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "Pawat Benjaoun"
      VALUE "FileDescription", "Kuro Sound Pad"
      VALUE "FileVersion", "{v}"
      VALUE "InternalName", "Kuro Sound Pad"
      VALUE "OriginalFilename", "Kuro Sound Pad.exe"
      VALUE "ProductName", "Kuro Sound Pad"
      VALUE "ProductVersion", "{v}"
      VALUE "LegalCopyright", "Pawat Benjaoun"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = path("icon.ico"),
        manifest = path("app.manifest"),
    );
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app.rc");
    std::fs::write(&out, rc).expect("write app.rc");
    embed_resource::compile(&out, embed_resource::NONE).manifest_optional().expect("Windows resources");
}
