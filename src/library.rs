//! The bundled sounds: `index.json` in the library folder names each file.
use crate::settings::library_dir;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct LibSound {
    pub id: u32,
    pub file: String,
    pub name: String,
    /// Lower-cased name, for searching.
    #[serde(skip)]
    pub key: String,
}

pub fn load() -> Vec<LibSound> {
    let mut list: Vec<LibSound> = std::fs::read(library_dir().join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    for s in &mut list {
        s.key = s.name.to_lowercase();
    }
    list
}

/// The library file an old `./sounds/<file>` address meant. The old list had wrong file names for some entries
/// (the id prefix was missing), so a miss falls back to the entry's own id, then to the file name without a prefix.
pub fn resolve<'a>(list: &'a [LibSound], file: &str) -> Option<&'a LibSound> {
    list.iter().find(|s| s.file == file).or_else(|| {
        list.iter().find(|s| s.file.split_once('_').is_some_and(|(_, rest)| rest == file))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_index_matches_the_files() {
        let list = load();
        assert_eq!(list.len(), 288);
        for s in &list {
            assert!(library_dir().join(&s.file).is_file(), "{} is missing", s.file);
        }
        assert_eq!(resolve(&list, "among-us-role-reveal-sound.mp3").map(|s| s.id), Some(5));
        assert_eq!(resolve(&list, "2_vine-boom.mp3").map(|s| s.id), Some(2));
    }
}
