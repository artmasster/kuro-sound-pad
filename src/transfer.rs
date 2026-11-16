//! Exporting the pads to one JSON file and reading one back. The format is the old web app's (sounds as data
//! URLs, hotkeys as `ctrl+key65`), so its exports import here; files this app writes add nothing the old one needs.
use crate::keys::Hotkey;
use crate::library::{self, LibSound};
use crate::settings::{self, Pad, Settings, Sound, Source, PADS};
use base64::Engine as _;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// `(year, month, day)` of a Unix time, UTC.
fn civil(secs: u64) -> (i64, u32, u32) {
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Today's date where the user is. (The export's own time stamp is UTC; a file name is read by a person.)
#[cfg(windows)]
fn local_date() -> (i64, u32, u32) {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (t.wYear as i64, t.wMonth as u32, t.wDay as u32)
}

#[cfg(not(windows))]
fn local_date() -> (i64, u32, u32) {
    civil(now())
}

pub fn export_file_name() -> String {
    let (y, m, d) = local_date();
    format!("kuro-sound-pad-settings-{y:04}-{m:02}-{d:02}.json")
}

fn mime(file: &str) -> &'static str {
    match file.rsplit('.').next().unwrap_or("") {
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        _ => "audio/mpeg",
    }
}

pub fn export(s: &Settings, path: &Path) -> Result<(), String> {
    let mut sounds = Vec::new();
    for p in &s.pads {
        sounds.push(match &p.sound {
            None => Value::Null,
            Some(sound) => {
                let url = match &sound.source {
                    Source::Library(f) => format!("./sounds/{f}"),
                    Source::File(f) => {
                        let bytes = std::fs::read(sound.path()).map_err(|e| format!("อ่านเสียง \"{}\" ไม่ได้: {e}", sound.name))?;
                        format!("data:{};base64,{}", mime(f), B64.encode(bytes))
                    }
                };
                json!({ "name": sound.name, "url": url })
            }
        });
    }
    let key = |k: &Option<Hotkey>| k.map(|k| k.to_legacy()).unwrap_or_default();
    let mut shortcuts: Vec<String> = s.pads.iter().map(|p| key(&p.key)).collect();
    shortcuts.push(key(&s.stop_key));
    let secs = now();
    let (y, m, d) = civil(secs);
    let t = secs % 86400;
    let doc = json!({
        "sounds": sounds,
        "masterVolume": s.master_volume,
        "shortcuts": shortcuts,
        "individualVolumes": s.pads.iter().map(|p| p.volume).collect::<Vec<_>>(),
        "padColors": s.pads.iter().map(|p| p.color).collect::<Vec<_>>(),
        "exportDate": format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z", t / 3600, t / 60 % 60, t % 60),
        "version": "2.0",
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("บันทึกไฟล์ไม่ได้: {e}"))
}

#[derive(Clone, Debug)]
pub enum ImportedSound {
    Library(String),
    /// Decoded bytes of a sound that travelled inside the file, and the extension to store them under.
    Data(Vec<u8>, &'static str),
}

#[derive(Clone, Debug, Default)]
pub struct ImportedPad {
    pub sound: Option<(String, ImportedSound)>,
    pub volume: u8,
    pub key: Option<Hotkey>,
    pub color: Option<u8>,
}

/// A settings file, read and checked but not applied yet: what the confirmation dialog shows.
#[derive(Clone, Debug)]
pub struct Imported {
    pub pads: Vec<ImportedPad>,
    pub master_volume: u8,
    pub stop_key: Option<Hotkey>,
    /// Sounds the file named that could not be read (an address this app does not know, broken data).
    pub skipped: usize,
}

fn extension_of(mime: &str, name: &str, bytes: &[u8]) -> &'static str {
    if let Some(e) = settings::sniff_extension(bytes) {
        return e;
    }
    let by_name = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    let m = mime.to_lowercase();
    for e in settings::AUDIO_EXTENSIONS {
        if by_name == e || m.ends_with(e) {
            return e;
        }
    }
    if m.contains("wav") {
        "wav"
    } else if m.contains("mp4") {
        "m4a"
    } else {
        "mp3"
    }
}

fn read_sound(v: &Value, lib: &[LibSound]) -> Result<Option<(String, ImportedSound)>, ()> {
    if v.is_null() {
        return Ok(None);
    }
    let url = v.get("url").and_then(Value::as_str).ok_or(())?;
    let raw_name = v.get("name").and_then(Value::as_str).unwrap_or("");
    if let Some(rest) = url.strip_prefix("data:") {
        let (head, data) = rest.split_once(',').ok_or(())?;
        if !head.contains("base64") {
            return Err(());
        }
        let bytes = B64.decode(data.trim()).map_err(|_| ())?;
        if bytes.is_empty() || bytes.len() as u64 > settings::MAX_FILE_SIZE {
            return Err(());
        }
        let ext = extension_of(head.split(';').next().unwrap_or(""), raw_name, &bytes);
        // The old app kept the file name as the sound's name and hid the extension when showing it.
        let name = match raw_name.rsplit_once('.') {
            Some((stem, e)) if settings::AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()) && !stem.is_empty() => stem,
            _ => raw_name,
        };
        return Ok(Some((settings::clean_name(name), ImportedSound::Data(bytes, ext))));
    }
    // "./sounds/x.mp3" in the desktop app, "/sounds/x.mp3" on the web.
    let file = url.rsplit('/').next().unwrap_or(url);
    let found = library::resolve(lib, file).ok_or(())?;
    let name = settings::clean_name(raw_name);
    Ok(Some((if name.is_empty() { found.name.clone() } else { name }, ImportedSound::Library(found.file.clone()))))
}

pub fn parse(text: &str, lib: &[LibSound]) -> Result<Imported, String> {
    let bad = |what: &str| format!("ไฟล์การตั้งค่าไม่ถูกต้อง ({what})");
    let doc: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|_| bad("ไม่ใช่ JSON"))?;
    let sounds = doc.get("sounds").and_then(Value::as_array).filter(|a| a.len() == PADS).ok_or_else(|| bad("sounds"))?;
    let master = doc
        .get("masterVolume")
        .and_then(Value::as_f64)
        .filter(|v| (0.0..=100.0).contains(v))
        .ok_or_else(|| bad("masterVolume"))?;
    // The old app wrote 17 hotkeys (the last one stops everything) though its own import asked for 16: take both.
    let shortcuts = doc
        .get("shortcuts")
        .and_then(Value::as_array)
        .filter(|a| a.len() == PADS || a.len() == PADS + 1)
        .ok_or_else(|| bad("shortcuts"))?;
    let volumes = doc.get("individualVolumes").and_then(Value::as_array).filter(|a| a.len() == PADS);
    // Only this app writes colours.
    let colors = doc.get("padColors").and_then(Value::as_array).filter(|a| a.len() == PADS);
    let key = |v: &Value| v.as_str().and_then(Hotkey::from_legacy);

    let mut skipped = 0;
    let mut pads = Vec::with_capacity(PADS);
    for i in 0..PADS {
        let sound = read_sound(&sounds[i], lib).unwrap_or_else(|()| {
            skipped += 1;
            None
        });
        let volume = volumes.and_then(|v| v[i].as_f64()).map_or(100, |v| v.clamp(0.0, 100.0).round() as u8);
        let color = colors.and_then(|c| c[i].as_u64()).filter(|c| *c < 8).map(|c| c as u8);
        pads.push(ImportedPad { sound, volume, key: key(&shortcuts[i]), color });
    }
    // The same key twice would leave one of them dead: the first pad keeps it.
    for i in 0..PADS {
        if let Some(k) = pads[i].key {
            for p in pads.iter_mut().skip(i + 1) {
                if p.key == Some(k) {
                    p.key = None;
                }
            }
        }
    }
    let stop_key = shortcuts.get(PADS).map_or(Some(Hotkey::plain(crate::keys::DEFAULT_STOP_KEY)), key);
    let stop_key = stop_key.filter(|k| pads.iter().all(|p| p.key != Some(*k)));
    Ok(Imported { pads, master_volume: master.round() as u8, stop_key, skipped })
}

impl Imported {
    pub fn sound_count(&self) -> usize {
        self.pads.iter().filter(|p| p.sound.is_some()).count()
    }

    pub fn key_count(&self) -> usize {
        self.pads.iter().filter(|p| p.key.is_some()).count()
    }

    pub fn custom_volume_count(&self) -> usize {
        self.pads.iter().filter(|p| p.volume != 100).count()
    }

    /// Replaces pads, volumes and hotkeys; devices and the other preferences stay.
    pub fn apply(self, s: &mut Settings) -> Result<(), String> {
        let mut pads = Vec::with_capacity(PADS);
        for p in self.pads {
            let sound = match p.sound {
                None => None,
                Some((name, ImportedSound::Library(file))) => Some(Sound { name, source: Source::Library(file) }),
                Some((name, ImportedSound::Data(bytes, ext))) => Some(settings::store_sound(&name, &bytes, ext)?),
            };
            pads.push(Pad { sound, volume: p.volume, key: p.key, color: p.color });
        }
        s.pads = pads;
        s.master_volume = self.master_volume;
        s.stop_key = self.stop_key;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(951782400), (2000, 2, 29));
        assert_eq!(civil(1794787200), (2026, 11, 16));
    }

    fn old_export(shortcuts: usize) -> String {
        let mut sounds = vec![Value::Null; PADS];
        sounds[0] = json!({ "name": "Vine Boom", "url": "./sounds/2_vine-boom.mp3" });
        sounds[1] = json!({ "name": "Among", "url": "/sounds/among-us-role-reveal-sound.mp3" });
        sounds[2] = json!({ "name": "my clip.wav", "url": format!("data:audio/*;base64,{}", B64.encode(b"RIFF\0\0\0\0WAVEfmt data")) });
        sounds[3] = json!({ "name": "gone", "url": "./sounds/not-there.mp3" });
        let mut keys: Vec<Value> = ["7", "8", "9", "0", "U", "I", "O", "P", "H", "J", "K", "L", "N", "M", ",", "."]
            .iter()
            .map(|k| json!(k))
            .collect();
        keys[4] = json!("ctrl+key85");
        keys[5] = json!("7");
        if shortcuts == 17 {
            keys.push(json!("Y"));
        }
        let mut volumes = vec![100; PADS];
        volumes[2] = 35;
        json!({ "sounds": sounds, "masterVolume": 80, "shortcuts": keys, "individualVolumes": volumes, "version": "1.0" })
            .to_string()
    }

    #[test]
    fn reads_the_old_apps_exports() {
        let lib = library::load();
        for n in [16, 17] {
            let imp = parse(&old_export(n), &lib).unwrap();
            assert_eq!(imp.master_volume, 80);
            assert_eq!(imp.sound_count(), 3);
            assert_eq!(imp.skipped, 1);
            assert!(matches!(&imp.pads[0].sound, Some((n, ImportedSound::Library(f))) if n == "Vine Boom" && f == "2_vine-boom.mp3"));
            assert!(matches!(&imp.pads[1].sound, Some((_, ImportedSound::Library(f))) if f == "5_among-us-role-reveal-sound.mp3"));
            assert!(matches!(&imp.pads[2].sound, Some((n, ImportedSound::Data(_, "wav"))) if n == "my clip"));
            assert_eq!(imp.pads[2].volume, 35);
            assert_eq!(imp.pads[0].key, Some(Hotkey::plain(0x37)));
            assert_eq!(imp.pads[4].key, Some(Hotkey { vk: 85, ctrl: true, shift: false, alt: false }));
            assert_eq!(imp.pads[5].key, None, "a duplicate key stays with the first pad");
            assert_eq!(imp.stop_key, Some(Hotkey::plain(0x59)));
            assert_eq!(imp.custom_volume_count(), 1);
        }
    }

    #[test]
    fn what_is_exported_imports_again() {
        let lib = library::load();
        let mut s = Settings::default();
        s.master_volume = 65;
        s.pads[0].sound = Some(Sound { name: "Vine Boom".into(), source: Source::Library("2_vine-boom.mp3".into()) });
        s.pads[0].volume = 40;
        s.pads[0].color = Some(6);
        s.pads[3].key = Some(Hotkey { vk: 0x41, ctrl: true, shift: false, alt: true });
        s.stop_key = Some(Hotkey::plain(0x70));
        let dir = std::env::temp_dir().join(format!("kuro-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(export_file_name());
        export(&s, &path).unwrap();
        let back = parse(&std::fs::read_to_string(&path).unwrap(), &lib).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(back.master_volume, 65);
        assert_eq!(back.skipped, 0);
        assert!(matches!(&back.pads[0].sound, Some((n, ImportedSound::Library(f))) if n == "Vine Boom" && f == "2_vine-boom.mp3"));
        assert_eq!((back.pads[0].volume, back.pads[0].color), (40, Some(6)));
        assert_eq!(back.pads[3].key, s.pads[3].key);
        assert_eq!(back.pads[1].key, s.pads[1].key);
        assert_eq!(back.stop_key, s.stop_key);
    }

    #[test]
    fn rejects_other_files() {
        let lib = library::load();
        assert!(parse("not json", &lib).is_err());
        assert!(parse("{}", &lib).is_err());
        assert!(parse(r#"{"sounds":[],"masterVolume":50,"shortcuts":[]}"#, &lib).is_err());
    }
}
