//! The bundled sounds. They come in one sealed file (`library.kpk`, see `pack.rs`): the list is read once, a sound
//! when it is needed.
use crate::pack::{self, Entry, Pack};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct LibSound {
    pub id: u32,
    /// What pads and exported settings call the sound.
    pub file: String,
    pub name: String,
    /// What it was called in earlier versions.
    pub was: Vec<String>,
    /// Lower-cased name, for searching.
    pub key: String,
}

struct Library {
    pack: Option<Pack>,
    sounds: Vec<LibSound>,
}

/// The library file: beside the exe once installed, the repository's `assets/library.kpk` while developing.
fn pack_path() -> PathBuf {
    if let Some(p) = std::env::var_os("KURO_LIBRARY_PACK").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    let installed = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.join("library.kpk")));
    match installed {
        Some(p) if p.is_file() => p,
        _ => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/library.kpk")),
    }
}

fn library() -> &'static Library {
    static LIBRARY: OnceLock<Library> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        // A build made without the key, or a missing file, leaves the library empty; the window says so.
        let pack = pack::built_in_key().and_then(|key| Pack::open(&pack_path(), key).ok());
        let sounds = pack.iter().flat_map(|p| &p.entries).map(sound).collect();
        Library { pack, sounds }
    })
}

fn sound(e: &Entry) -> LibSound {
    LibSound { id: e.id, file: e.file.clone(), name: e.name.clone(), was: e.was.clone(), key: e.name.to_lowercase() }
}

pub fn load() -> Vec<LibSound> {
    library().sounds.clone()
}

/// The audio file behind a library sound, by its name now or one it had before.
pub fn read(file: &str) -> Result<Vec<u8>, String> {
    let lib = library();
    let missing = || "ไม่พบเสียงนี้ในคลังเสียง".to_string();
    let found = resolve(&lib.sounds, file).ok_or_else(missing)?;
    let pack = lib.pack.as_ref().ok_or_else(missing)?;
    let entry = pack.entries.iter().find(|e| e.file == found.file).ok_or_else(missing)?;
    pack.read(entry)
}

/// The library sound a file name means: its name now, a name it had in an earlier version, or the name the old web
/// app's list used (that list left the id prefix off some entries).
pub fn resolve<'a>(list: &'a [LibSound], file: &str) -> Option<&'a LibSound> {
    list.iter()
        .find(|s| s.file == file)
        .or_else(|| list.iter().find(|s| s.was.iter().any(|w| w == file)))
        .or_else(|| list.iter().find(|s| s.was.iter().any(|w| w.split_once('_').is_some_and(|(_, rest)| rest == file))))
}

/// For tests that need the real library: `None` (with a note) in a build made without the key.
#[cfg(test)]
pub fn for_tests() -> Option<Vec<LibSound>> {
    let list = load();
    if list.is_empty() {
        eprintln!("skipped: this build has no library key (library.key), so the bundled sounds cannot be opened");
        return None;
    }
    Some(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_library_opens_and_old_names_still_work() {
        let Some(list) = for_tests() else { return };
        assert_eq!(list.len(), 288);
        for (n, s) in list.iter().enumerate() {
            assert_eq!(s.file, format!("{:03}.mp3", n + 1));
            assert_eq!(s.id as usize, n + 1);
        }
        assert_eq!(resolve(&list, "002.mp3").map(|s| s.id), Some(2));
        assert_eq!(resolve(&list, "2_vine-boom.mp3").map(|s| s.id), Some(2));
        assert_eq!(resolve(&list, "among-us-role-reveal-sound.mp3").map(|s| s.id), Some(5));
        assert!(resolve(&list, "not-there.mp3").is_none());
        // By either name, the same audio file comes out.
        let now = read("002.mp3").unwrap();
        assert!(now.len() > 1000 && crate::settings::sniff_extension(&now) == Some("mp3"));
        assert_eq!(read("2_vine-boom.mp3").unwrap(), now);
        assert!(read("not-there.mp3").is_err());
    }
}
