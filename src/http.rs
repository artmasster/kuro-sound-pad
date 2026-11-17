//! The two requests the updater makes: read a small text file, download a large one. Blocking, for a worker thread.
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use ureq::Agent;

const TIMEOUT: Duration = Duration::from_secs(30);
/// For a whole download; connecting and the response's head still get `TIMEOUT`.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const TEXT_LIMIT: u64 = 1024 * 1024;
const USER_AGENT: &str = concat!("kuro-sound-pad/", env!("CARGO_PKG_VERSION"));

fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        Agent::new_with_config(
            Agent::config_builder().http_status_as_error(false).timeout_global(Some(TIMEOUT)).user_agent(USER_AGENT).tls_config(tls).build(),
        )
    })
}

/// The body of `url` as text. Anything but a 2xx answer is an error.
pub fn text(url: &str) -> Result<String, String> {
    let mut resp = agent().get(url).call().map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}"));
    }
    let body = resp.body_mut().with_config().limit(TEXT_LIMIT).read_to_vec().map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&body).into_owned())
}

fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// Streams `url` into `<dest>.part`, then renames it to `dest`, so `dest` only ever holds a whole file.
/// `progress(done, total)` is called as bytes arrive; total is `None` when the server sends no length.
pub fn download(url: &str, dest: &Path, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<(), String> {
    let resp = agent()
        .get(url)
        .config()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .timeout_connect(Some(TIMEOUT))
        .timeout_recv_response(Some(TIMEOUT))
        .build()
        .call()
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}"));
    }
    let total = resp.body().content_length();
    let mut reader = resp.into_body().into_with_config().limit(u64::MAX).reader();

    let part = part_path(dest);
    let result = (|| {
        let mut file = File::create(&part).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; 64 * 1024];
        let mut done = 0u64;
        progress(0, total);
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.to_string()),
            };
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            progress(done, total);
        }
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&part, dest).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_path_appends_suffix() {
        assert_eq!(part_path(Path::new("a/b/setup.exe")), PathBuf::from("a/b/setup.exe.part"));
    }

    #[test]
    fn nothing_listening_is_an_error() {
        // Port 9 on this machine: the connection is refused here, without touching the network.
        assert!(text("http://127.0.0.1:9/").is_err());
    }
}
