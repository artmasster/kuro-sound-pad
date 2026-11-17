//! Packs and unpacks the bundled sound library (`assets/library.kpk`, see `src/pack.rs`).
//!
//!     cargo run --release --example library -- unpack [folder]   # the sounds and index.json, as ordinary files
//!     cargo run --release --example library -- pack [folder]     # back into assets/library.kpk
//!
//! The folder is `library-src` unless named; git ignores it. To change the library: unpack, add or replace audio
//! files, edit `index.json` (`id`, `file`, `name`, and `was` for names a sound had before), pack, commit the .kpk.
//! Needs the key (`library.key`, or `KURO_LIBRARY_KEY`), like every build that is to open the library.
#[path = "../src/pack.rs"]
mod pack;

use pack::{Entry, Pack};
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let folder = args.get(1).map_or_else(|| root.join("library-src"), PathBuf::from);
    let file = root.join("assets").join("library.kpk");
    let Some(key) = pack::built_in_key() else {
        eprintln!("no key: put the 64-digit key in library.key beside Cargo.toml (or set KURO_LIBRARY_KEY) and run again");
        std::process::exit(1);
    };
    let done = match args.first().map(String::as_str) {
        Some("pack") => pack(&folder, &file, &key),
        Some("unpack") => unpack(&folder, &file, key),
        _ => Err("usage: library pack|unpack [folder]".into()),
    };
    if let Err(e) = done {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn pack(folder: &Path, file: &Path, key: &pack::Key) -> Result<(), String> {
    let list = std::fs::read(folder.join("index.json")).map_err(|e| format!("{}: {e}", folder.join("index.json").display()))?;
    let entries: Vec<Entry> = serde_json::from_slice(&list).map_err(|e| format!("index.json: {e}"))?;
    let mut sounds = Vec::with_capacity(entries.len());
    let mut raw = 0;
    for entry in entries {
        if sounds.iter().any(|(other, _): &(Entry, Vec<u8>)| other.file == entry.file || other.id == entry.id) {
            return Err(format!("index.json lists {} (id {}) twice", entry.file, entry.id));
        }
        let bytes = std::fs::read(folder.join(&entry.file)).map_err(|e| format!("{}: {e}", entry.file))?;
        raw += bytes.len();
        sounds.push((entry, bytes));
    }
    let bytes = pack::build(key, &sounds);
    std::fs::write(file, &bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    println!("{} sounds, {:.2} MB -> {} ({:.2} MB)", sounds.len(), raw as f64 / 1e6, file.display(), bytes.len() as f64 / 1e6);
    Ok(())
}

fn unpack(folder: &Path, file: &Path, key: pack::Key) -> Result<(), String> {
    let pack = Pack::open(file, key)?;
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    for entry in &pack.entries {
        // A name from the list is a plain file name, never a path out of the folder.
        if entry.file.contains(['/', '\\']) || entry.file.starts_with('.') {
            return Err(format!("the list names a file this will not write: {}", entry.file));
        }
        std::fs::write(folder.join(&entry.file), pack.read(entry)?).map_err(|e| format!("{}: {e}", entry.file))?;
    }
    // The list as a person edits it: without where each sound sits in the file.
    let list: Vec<serde_json::Value> = pack
        .entries
        .iter()
        .map(|e| {
            let mut v = serde_json::json!({ "id": e.id, "file": e.file, "name": e.name });
            if !e.was.is_empty() {
                v["was"] = serde_json::json!(e.was);
            }
            v
        })
        .collect();
    let text = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    std::fs::write(folder.join("index.json"), text + "\n").map_err(|e| format!("index.json: {e}"))?;
    println!("{} sounds -> {}", pack.entries.len(), folder.display());
    Ok(())
}
