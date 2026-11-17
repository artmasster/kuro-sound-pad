//! Updates from GitHub Releases, the way Ai Quota Overlay does them.
//!
//! The latest release carries a small `latest.yml` beside the installer: the version, the installer's name, size and
//! SHA-512. Nothing downloads on its own: a newer version is offered in the window, and only when the user says so
//! is the installer downloaded, checked (size, SHA-512, and that it is signed by the publisher of this very app),
//! run silently and the app closed; the installer starts the new version. Skip remembers a version not wanted.
//!
//! Only an installed copy updates itself: a copy run from a build folder has nothing to replace.
//!
//! Development aid: `KURO_UPDATE_FEED=http://127.0.0.1:8000` reads `latest.yml` and the installer from there instead.
use crate::trust;
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const REPO: &str = "artmasster/kuro-sound-pad";
const FEED: &str = "latest.yml";
/// Written by the installer next to the exe: its presence is what makes a copy an installed one.
const UNINSTALLER: &str = "Uninstall Kuro Sound Pad.exe";
/// The first look comes a little after start-up, so it never competes with the window opening.
pub const FIRST_CHECK: Duration = Duration::from_secs(5);
pub const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Not an installed copy: no checks at all.
    Unsupported,
    Idle,
    /// Looking, with nothing on offer yet.
    Checking,
    Available,
    Downloading,
    /// The installer is about to take over.
    Installing,
    /// The download (or its checks) failed; the version is still on offer.
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub file: String,
    pub sha512: String,
    pub size: Option<u64>,
}

/// What the worker threads report back.
#[derive(Clone, Debug)]
pub enum UpMsg {
    Checked(Result<Option<Release>, String>),
    Progress(u32),
    Downloaded(Result<PathBuf, String>),
}

/// What a report means for the window.
pub enum Outcome {
    Nothing,
    /// A version turned up that was not on offer before (or the user asked and it still is).
    Found(String),
    /// The user asked, and there is nothing newer.
    UpToDate,
    /// The user asked, and the check failed.
    CheckFailed(String),
    /// The installer is downloaded and checked: run it and close.
    Run(PathBuf),
}

pub struct Updater {
    pub phase: Phase,
    pub release: Option<Release>,
    pub percent: u32,
    /// Why the download failed.
    pub error: Option<String>,
    /// The check under way was asked for by the user, who gets an answer either way.
    asked: bool,
    report: Arc<dyn Fn(UpMsg) + Send + Sync>,
}

/// "2.10.0" is newer than "2.9.2"; anything after a "-" (a pre-release tag) is ignored.
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.split('-').next().unwrap_or("").split('.').map(|n| n.trim().parse().unwrap_or(0)).collect() };
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if p != q {
            return p > q;
        }
    }
    false
}

/// Whether this copy was put here by the installer (and so may be replaced by one).
fn installed() -> bool {
    std::env::current_exe().ok().and_then(|e| Some(e.parent()?.join(UNINSTALLER).exists())).unwrap_or(false)
}

/// Where a downloaded installer waits: in the data folder, which this app can always write to.
fn cache_dir() -> PathBuf {
    crate::settings::data_dir().join("update")
}

impl Updater {
    pub fn new(report: impl Fn(UpMsg) + Send + Sync + 'static) -> Self {
        // Whatever an earlier update left behind has done its work.
        let _ = std::fs::remove_dir_all(cache_dir());
        Self {
            phase: if installed() { Phase::Idle } else { Phase::Unsupported },
            release: None,
            percent: 0,
            error: None,
            asked: false,
            report: Arc::new(report),
        }
    }

    pub fn supported(&self) -> bool {
        self.phase != Phase::Unsupported
    }

    /// Looks for a newer version, `after` a pause. A version already on offer stays on offer meanwhile.
    pub fn check(&mut self, asked: bool, after: Duration) {
        if matches!(self.phase, Phase::Unsupported | Phase::Checking | Phase::Downloading | Phase::Installing) {
            return;
        }
        self.asked = asked;
        if self.release.is_none() {
            self.phase = Phase::Checking;
        }
        let report = self.report.clone();
        std::thread::spawn(move || {
            std::thread::sleep(after);
            let found = latest().map(|r| newer(&r.version, env!("CARGO_PKG_VERSION")).then_some(r));
            report(UpMsg::Checked(found));
        });
    }

    pub fn download(&mut self) {
        let Some(release) = self.release.clone() else { return };
        if !matches!(self.phase, Phase::Available | Phase::Failed) {
            return;
        }
        self.phase = Phase::Downloading;
        self.percent = 0;
        self.error = None;
        let report = self.report.clone();
        std::thread::spawn(move || {
            let progress = report.clone();
            let done = download(&release, &mut |p| progress(UpMsg::Progress(p)));
            report(UpMsg::Downloaded(done));
        });
    }

    /// The installer could not be started.
    pub fn fail(&mut self, why: String) {
        self.phase = Phase::Failed;
        self.error = Some(why);
    }

    pub fn on(&mut self, message: UpMsg) -> Outcome {
        let asked = std::mem::take(&mut self.asked);
        match message {
            UpMsg::Checked(Ok(Some(release))) => {
                let version = release.version.clone();
                let new = self.release.as_ref().map(|r| r.version.as_str()) != Some(version.as_str());
                self.release = Some(release);
                if new || self.phase == Phase::Checking {
                    // A newer one replaces a failed download of the one before.
                    self.phase = Phase::Available;
                    self.error = None;
                }
                if new || asked {
                    Outcome::Found(version)
                } else {
                    Outcome::Nothing
                }
            }
            UpMsg::Checked(Ok(None)) => {
                self.release = None;
                self.phase = Phase::Idle;
                if asked {
                    Outcome::UpToDate
                } else {
                    Outcome::Nothing
                }
            }
            UpMsg::Checked(Err(e)) => {
                if self.phase == Phase::Checking {
                    self.phase = Phase::Idle;
                }
                if asked {
                    Outcome::CheckFailed(e)
                } else {
                    Outcome::Nothing
                }
            }
            UpMsg::Progress(p) => {
                self.asked = asked;
                if self.phase == Phase::Downloading {
                    self.percent = p;
                }
                Outcome::Nothing
            }
            UpMsg::Downloaded(Ok(path)) => {
                self.phase = Phase::Installing;
                self.percent = 100;
                Outcome::Run(path)
            }
            UpMsg::Downloaded(Err(e)) => {
                self.fail(e);
                Outcome::Nothing
            }
        }
    }
}

/// Where the feed and the files come from: the development override, else the latest GitHub release.
fn feed_url() -> String {
    match std::env::var("KURO_UPDATE_FEED").ok().filter(|b| !b.is_empty()) {
        Some(base) => format!("{}/{FEED}", base.trim_end_matches('/')),
        None => format!("https://github.com/{REPO}/releases/latest/download/{FEED}"),
    }
}

fn file_url(release: &Release) -> String {
    if release.file.starts_with("https://") {
        return release.file.clone();
    }
    match std::env::var("KURO_UPDATE_FEED").ok().filter(|b| !b.is_empty()) {
        Some(base) => format!("{}/{}", base.trim_end_matches('/'), release.file),
        None => format!("https://github.com/{REPO}/releases/download/v{}/{}", release.version, release.file),
    }
}

fn latest() -> Result<Release, String> {
    parse_feed(&crate::http::text(&feed_url())?).ok_or_else(|| format!("อ่าน {FEED} ไม่ได้"))
}

/// Reads `version` and the first `files` entry (`url`, `sha512`, `size`) of an electron-builder style update feed.
pub fn parse_feed(yml: &str) -> Option<Release> {
    let mut version = None;
    let (mut file, mut sha, mut size) = (None, None, None);
    let (mut path, mut top_sha) = (None, None);
    let mut in_files = false;
    let mut first_done = false;
    let unquote = |v: &str| v.trim().trim_matches('\'').trim_matches('"').to_string();
    for line in yml.lines() {
        let indented = line.starts_with(' ') || line.starts_with('-');
        let l = line.trim();
        if !indented {
            in_files = l.starts_with("files:");
            if let Some(v) = l.strip_prefix("version:") {
                version = Some(unquote(v));
            } else if let Some(v) = l.strip_prefix("path:") {
                path = Some(unquote(v));
            } else if let Some(v) = l.strip_prefix("sha512:") {
                top_sha = Some(unquote(v));
            }
            continue;
        }
        if !in_files || first_done {
            continue;
        }
        if line.trim_start().starts_with("- ") && file.is_some() {
            first_done = true;
            continue;
        }
        let l = l.trim_start_matches("- ").trim();
        if let Some(v) = l.strip_prefix("url:") {
            file = Some(unquote(v));
        } else if let Some(v) = l.strip_prefix("sha512:") {
            sha = Some(unquote(v));
        } else if let Some(v) = l.strip_prefix("size:") {
            size = unquote(v).parse().ok();
        }
    }
    Some(Release { version: version?, file: file.or(path)?, sha512: sha.or(top_sha)?, size })
}

fn sha512_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(Sha512::digest(bytes))
}

/// Downloads the installer of `release` and checks it: the size and SHA-512 the feed promised, and the signature.
fn download(release: &Release, progress: &mut dyn FnMut(u32)) -> Result<PathBuf, String> {
    let dir = cache_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // The feed names a file; it never gets to choose a folder.
    let name = release.file.rsplit(['/', '\\']).next().unwrap_or_default();
    if name.is_empty() || !name.to_ascii_lowercase().ends_with(".exe") {
        return Err(format!("ชื่อไฟล์ตัวติดตั้งไม่ถูกต้อง ({})", release.file));
    }
    let dest = dir.join(name);
    let mut last = u32::MAX;
    crate::http::download(&file_url(release), &dest, &mut |done, total| {
        let total = total.or(release.size).unwrap_or(0);
        if total > 0 {
            let p = ((done as f64 / total as f64) * 100.0).floor().min(100.0) as u32;
            if p != last {
                last = p;
                progress(p);
            }
        }
    })?;
    let checked = check_file(&dest, release);
    if checked.is_err() {
        let _ = std::fs::remove_file(&dest);
    }
    checked.map(|()| dest)
}

fn check_file(file: &Path, release: &Release) -> Result<(), String> {
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    if release.size.is_some_and(|size| bytes.len() as u64 != size) {
        return Err("ไฟล์ที่ดาวน์โหลดมาขนาดไม่ตรง".into());
    }
    if sha512_base64(&bytes) != release.sha512 {
        return Err("ไฟล์ที่ดาวน์โหลดมาไม่ตรงกับที่ประกาศไว้".into());
    }
    drop(bytes);
    // A signed app only accepts an installer signed by the same publisher. A copy that is not signed itself is a
    // developer's own build, which has no publisher to compare with.
    let own = std::env::current_exe().ok().and_then(|exe| trust::signer(&exe));
    match own {
        Some(publisher) if trust::signer(file).as_deref() != Some(publisher.as_str()) => {
            Err(format!("ตัวติดตั้งไม่ได้ลงนามโดย {publisher}"))
        }
        _ => Ok(()),
    }
}

/// Starts the installer silently, into the folder this copy runs from. It waits for this app to exit, replaces it and
/// starts the new version.
pub fn run(installer: &Path) -> Result<(), String> {
    let dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)).ok_or("ไม่พบโฟลเดอร์ของแอป")?;
    let mut command = std::process::Command::new(installer);
    command.args(["/S", "--updated"]);
    // The installer takes `/D=` as the last thing on its command line, unquoted even with spaces in it.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.raw_arg(format!("/D={}", dir.display()));
    }
    // The new copy starts from the installer and would inherit a development script meant for this one.
    command.env_remove("KURO_AUTOPLAY");
    command.spawn().map(|_| ()).map_err(|e| format!("เปิดตัวติดตั้งไม่ได้: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn updater() -> Updater {
        Updater { phase: Phase::Idle, release: None, percent: 0, error: None, asked: false, report: Arc::new(|_| {}) }
    }

    fn release(version: &str) -> Release {
        Release { version: version.into(), file: "setup.exe".into(), sha512: "x".into(), size: None }
    }

    #[test]
    fn versions() {
        assert!(newer("2.10.0", "2.9.2"));
        assert!(newer("3.0.0", "2.9.9"));
        assert!(!newer("2.1.0", "2.1.0"));
        assert!(!newer("2.1.0-beta", "2.1.0"));
        assert!(!newer("2.0.9", "2.1.0"));
    }

    #[test]
    fn feed() {
        let yml = "version: 2.1.0\nfiles:\n  - url: KuroSoundPad-2.1.0-win-x64-setup.exe\n    sha512: abc==\n    size: 25874768\npath: KuroSoundPad-2.1.0-win-x64-setup.exe\nsha512: abc==\nreleaseDate: '2026-11-17T08:00:00.000Z'\n";
        let r = parse_feed(yml).unwrap();
        assert_eq!(r.version, "2.1.0");
        assert_eq!(r.file, "KuroSoundPad-2.1.0-win-x64-setup.exe");
        assert_eq!(r.sha512, "abc==");
        assert_eq!(r.size, Some(25874768));
        let two = "version: 2.2.0\nfiles:\n  - url: a.exe\n    sha512: one\n    size: 1\n  - url: b.zip\n    sha512: two\n    size: 2\npath: a.exe\nsha512: one\n";
        let r = parse_feed(two).unwrap();
        assert_eq!((r.file.as_str(), r.sha512.as_str(), r.size), ("a.exe", "one", Some(1)));
        assert!(parse_feed("<html>not found</html>").is_none());
    }

    #[test]
    fn a_check_offers_a_new_version_once() {
        let mut u = updater();
        u.phase = Phase::Checking;
        assert!(matches!(u.on(UpMsg::Checked(Ok(Some(release("2.2.0"))))), Outcome::Found(v) if v == "2.2.0"));
        assert_eq!(u.phase, Phase::Available);
        // The hourly look finds the same one again: nothing to say.
        assert!(matches!(u.on(UpMsg::Checked(Ok(Some(release("2.2.0"))))), Outcome::Nothing));
        // A failed look does not take the offer away.
        assert!(matches!(u.on(UpMsg::Checked(Err("offline".into()))), Outcome::Nothing));
        assert_eq!(u.phase, Phase::Available);
        assert!(matches!(u.on(UpMsg::Checked(Ok(Some(release("2.3.0"))))), Outcome::Found(v) if v == "2.3.0"));
    }

    #[test]
    fn a_check_the_user_asked_for_always_answers() {
        let mut u = updater();
        u.phase = Phase::Checking;
        u.asked = true;
        assert!(matches!(u.on(UpMsg::Checked(Ok(None))), Outcome::UpToDate));
        assert_eq!(u.phase, Phase::Idle);
        u.phase = Phase::Checking;
        u.asked = true;
        assert!(matches!(u.on(UpMsg::Checked(Err("offline".into()))), Outcome::CheckFailed(_)));
        assert_eq!(u.phase, Phase::Idle);
        // The answer was given: the next, silent look says nothing.
        assert!(matches!(u.on(UpMsg::Checked(Ok(None))), Outcome::Nothing));
    }

    #[test]
    fn a_failed_download_keeps_the_offer() {
        let mut u = updater();
        u.release = Some(release("2.2.0"));
        u.phase = Phase::Downloading;
        assert!(matches!(u.on(UpMsg::Progress(40)), Outcome::Nothing));
        assert_eq!(u.percent, 40);
        assert!(matches!(u.on(UpMsg::Downloaded(Err("HTTP 404".into()))), Outcome::Nothing));
        assert_eq!((u.phase, u.error.as_deref()), (Phase::Failed, Some("HTTP 404")));
        assert!(u.release.is_some());
    }

    #[test]
    fn a_download_must_match_the_feed() {
        let dir = std::env::temp_dir();
        let file = dir.join(format!("kuro-update-test-{}.exe", std::process::id()));
        std::fs::write(&file, b"installer bytes").unwrap();
        let good = Release { version: "9.9.9".into(), file: "x.exe".into(), sha512: sha512_base64(b"installer bytes"), size: Some(15) };
        // (The test program is not signed, so no publisher is demanded of the file.)
        assert_eq!(check_file(&file, &good), Ok(()));
        assert!(check_file(&file, &Release { size: Some(16), ..good.clone() }).is_err());
        assert!(check_file(&file, &Release { sha512: sha512_base64(b"other bytes"), ..good }).is_err());
        let _ = std::fs::remove_file(&file);
    }
}
