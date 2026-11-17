//! What the app remembers: 16 pads, volumes, hotkeys, devices. One JSON file in the data folder, with the sounds the
//! user brought in copied next to it so the pads keep working after the originals move.
use crate::keys::{Hotkey, DEFAULT_PAD_KEYS, DEFAULT_STOP_KEY};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const PADS: usize = 16;
pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
pub const AUDIO_EXTENSIONS: [&str; 6] = ["mp3", "wav", "ogg", "m4a", "aac", "flac"];
const MAX_NAME: usize = 100;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "file", rename_all = "lowercase")]
pub enum Source {
    /// A file of the bundled library, by file name.
    Library(String),
    /// A file the user added, copied into `<data>/sounds`, by file name.
    File(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sound {
    pub name: String,
    #[serde(flatten)]
    pub source: Source,
}

impl Sound {
    /// The audio file's bytes: out of the sealed library, or from the copy in the data folder.
    pub fn load(&self) -> Result<Arc<[u8]>, String> {
        let bytes = match &self.source {
            Source::Library(f) => crate::library::read(f).ok(),
            Source::File(f) => std::fs::read(sounds_dir().join(f)).ok(),
        };
        bytes.map(Arc::from).ok_or_else(|| format!("ไม่พบไฟล์เสียงของ \"{}\"", self.name))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pad {
    pub sound: Option<Sound>,
    pub volume: u8,
    pub key: Option<Hotkey>,
    /// Which of the pad light colours; `None` takes the row's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u8>,
}

impl Default for Pad {
    fn default() -> Self {
        Self { sound: None, volume: 100, key: None, color: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub pads: Vec<Pad>,
    pub master_volume: u8,
    pub stop_key: Option<Hotkey>,
    /// Hotkeys while another app has the focus. `None` until the user has answered the first-run question.
    pub global_hotkeys: Option<bool>,
    /// `None` follows the system default device.
    pub output_device: Option<String>,
    /// A second device that gets the same sound: a virtual cable into the microphone, usually.
    pub monitor_device: Option<String>,
    /// A newer version the user chose not to be offered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_version: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pads: default_pads(),
            master_volume: 100,
            stop_key: Some(Hotkey::plain(DEFAULT_STOP_KEY)),
            global_hotkeys: None,
            output_device: None,
            monitor_device: None,
            skip_version: None,
        }
    }
}

fn default_pads() -> Vec<Pad> {
    DEFAULT_PAD_KEYS.iter().map(|&vk| Pad { key: Some(Hotkey::plain(vk)), ..Pad::default() }).collect()
}

impl Settings {
    pub fn load() -> Self {
        let mut s: Settings = std::fs::read(settings_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        s.normalize();
        s
    }

    /// Whatever the file said, the rest of the app gets 16 pads and volumes in range.
    pub fn normalize(&mut self) {
        self.pads.resize_with(PADS, Pad::default);
        self.master_volume = self.master_volume.min(100);
        for p in &mut self.pads {
            p.volume = p.volume.min(100);
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("สร้างโฟลเดอร์ข้อมูลไม่ได้: {e}"))?;
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp = dir.join("settings.json.tmp");
        std::fs::write(&tmp, json).map_err(|e| format!("บันทึกการตั้งค่าไม่ได้: {e}"))?;
        std::fs::rename(&tmp, settings_path()).map_err(|e| format!("บันทึกการตั้งค่าไม่ได้: {e}"))
    }

    pub fn reset_keys(&mut self) {
        for (p, &vk) in self.pads.iter_mut().zip(DEFAULT_PAD_KEYS.iter()) {
            p.key = Some(Hotkey::plain(vk));
        }
        self.stop_key = Some(Hotkey::plain(DEFAULT_STOP_KEY));
    }

    /// Gives `key` to `target` (a pad, or `None` for stop) and takes it from whatever had it. Returns the pad that
    /// lost it, if any (`Some(None)` when it was the stop key).
    pub fn assign_key(&mut self, target: Option<usize>, key: Hotkey) -> Option<Option<usize>> {
        let mut lost = None;
        for (i, p) in self.pads.iter_mut().enumerate() {
            if p.key == Some(key) && target != Some(i) {
                p.key = None;
                lost = Some(Some(i));
            }
        }
        if self.stop_key == Some(key) && target.is_some() {
            self.stop_key = None;
            lost = Some(None);
        }
        match target {
            Some(i) => self.pads[i].key = Some(key),
            None => self.stop_key = Some(key),
        }
        lost
    }

    /// Two pads trade what they play (sound, volume, colour). The hotkeys belong to the positions and stay.
    pub fn swap_pads(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        let (ka, kb) = (self.pads[a].key, self.pads[b].key);
        self.pads.swap(a, b);
        self.pads[a].key = ka;
        self.pads[b].key = kb;
    }

    /// Deletes copied sounds no pad uses any more.
    pub fn collect_garbage(&self) {
        let used: Vec<&str> = self
            .pads
            .iter()
            .filter_map(|p| match p.sound.as_ref()?.source {
                Source::File(ref f) => Some(f.as_str()),
                Source::Library(_) => None,
            })
            .collect();
        let Ok(dir) = std::fs::read_dir(sounds_dir()) else { return };
        for e in dir.flatten() {
            let name = e.file_name();
            if !used.iter().any(|u| name.to_str() == Some(u)) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

pub fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("KURO_DATA_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Kuro Sound Pad")
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn sounds_dir() -> PathBuf {
    data_dir().join("sounds")
}

/// A name safe to show and to put in a file name: the old app's rule (drop `<>:"/\|?*`, at most 100 characters).
pub fn clean_name(name: &str) -> String {
    let s: String = name.chars().filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') && !c.is_control()).collect();
    s.trim().chars().take(MAX_NAME).collect::<String>().trim().to_string()
}

fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// What kind of audio these bytes are, for the stored file's extension.
pub fn sniff_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 12 {
        return None;
    }
    if &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        Some("wav")
    } else if &bytes[..4] == b"OggS" {
        Some("ogg")
    } else if &bytes[..4] == b"fLaC" {
        Some("flac")
    } else if &bytes[4..8] == b"ftyp" {
        Some("m4a")
    } else if &bytes[..3] == b"ID3" {
        Some("mp3")
    } else if bytes[0] == 0xFF && bytes[1] & 0xF6 == 0xF0 {
        Some("aac")
    } else if bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0 {
        Some("mp3")
    } else {
        None
    }
}

/// Keeps a copy of `bytes` in the data folder (named by content, so the same sound is stored once) and returns the
/// pad's sound for it.
pub fn store_sound(name: &str, bytes: &[u8], extension: &str) -> Result<Sound, String> {
    if bytes.is_empty() {
        return Err("ไฟล์เสียงว่างเปล่า".into());
    }
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err("ขนาดไฟล์เกิน 50MB".into());
    }
    let dir = sounds_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("สร้างโฟลเดอร์เสียงไม่ได้: {e}"))?;
    let file = format!("{:016x}-{:x}.{extension}", fnv64(bytes), bytes.len());
    let path = dir.join(&file);
    if !path.is_file() {
        std::fs::write(&path, bytes).map_err(|e| format!("คัดลอกไฟล์เสียงไม่ได้: {e}"))?;
    }
    let name = clean_name(name);
    Ok(Sound { name: if name.is_empty() { "เสียง".into() } else { name }, source: Source::File(file) })
}

/// Brings a file from disk in: checks the type and the size, copies it.
pub fn import_file(path: &Path) -> Result<Sound, String> {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
    if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        return Err("ประเภทไฟล์ไม่ถูกต้อง ใช้ได้เฉพาะ MP3, WAV, OGG, M4A, AAC, FLAC".into());
    }
    let meta = std::fs::metadata(path).map_err(|e| format!("เปิดไฟล์ไม่ได้: {e}"))?;
    if meta.len() > MAX_FILE_SIZE {
        return Err("ขนาดไฟล์เกิน 50MB".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("อ่านไฟล์ไม่ได้: {e}"))?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("เสียง");
    store_sound(stem, &bytes, &ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(clean_name("  a<b>:c?.mp3 "), "abc.mp3");
        assert_eq!(clean_name(&"ก".repeat(300)).chars().count(), 100);
    }

    #[test]
    fn assign_takes_the_key_from_its_old_owner() {
        let mut s = Settings::default();
        let k = s.pads[3].key.unwrap();
        assert_eq!(s.assign_key(Some(0), k), Some(Some(3)));
        assert_eq!(s.pads[0].key, Some(k));
        assert_eq!(s.pads[3].key, None);
        let stop = s.stop_key.unwrap();
        assert_eq!(s.assign_key(Some(1), stop), Some(None));
        assert_eq!(s.stop_key, None);
        assert_eq!(s.assign_key(None, k), Some(Some(0)));
        assert_eq!(s.stop_key, Some(k));
        assert_eq!(s.assign_key(None, k), None);
    }

    #[test]
    fn swapping_keeps_the_keys_in_place() {
        let mut s = Settings::default();
        s.pads[0].sound = Some(Sound { name: "a".into(), source: Source::Library("a.mp3".into()) });
        s.pads[0].volume = 40;
        s.pads[0].color = Some(3);
        let (k0, k5) = (s.pads[0].key, s.pads[5].key);
        s.swap_pads(0, 5);
        assert!(s.pads[0].sound.is_none() && s.pads[0].volume == 100 && s.pads[0].color.is_none());
        assert_eq!(s.pads[5].sound.as_ref().unwrap().name, "a");
        assert_eq!((s.pads[5].volume, s.pads[5].color), (40, Some(3)));
        assert_eq!((s.pads[0].key, s.pads[5].key), (k0, k5));
    }

    #[test]
    fn round_trip_and_partial_files() {
        let mut s = Settings::default();
        s.pads[2].sound = Some(Sound { name: "Vine Boom".into(), source: Source::Library("2_vine-boom.mp3".into()) });
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
        let mut partial: Settings = serde_json::from_str(r#"{"pads":[{"volume":250}],"master_volume":180}"#).unwrap();
        partial.normalize();
        assert_eq!(partial.pads.len(), PADS);
        assert_eq!(partial.pads[0].volume, 100);
        assert_eq!(partial.master_volume, 100);
    }

    #[test]
    fn sniffing() {
        assert_eq!(sniff_extension(b"RIFF\0\0\0\0WAVEfmt "), Some("wav"));
        assert_eq!(sniff_extension(b"ID3\x04\0\0\0\0\0\0\0\0"), Some("mp3"));
        assert_eq!(sniff_extension(b"\xFF\xFB\x90\0\0\0\0\0\0\0\0\0"), Some("mp3"));
        assert_eq!(sniff_extension(b"\0\0\0\x20ftypM4A \0\0\0\0"), Some("m4a"));
        assert_eq!(sniff_extension(b"hello world!"), None);
    }
}
