//! The bundled sounds as one file, `library.kpk`, that only this app opens.
//!
//! The file holds the list of sounds and every sound, each squeezed (deflate) and then encrypted
//! (XChaCha20-Poly1305). Sounds with the same content are stored once. The app reads one sound at a time straight
//! into memory to play it: the sounds never lie around as ordinary audio files, in the repository or on a user's disk.
//!
//! The key is not in the repository. A build takes it from `library.key` beside `Cargo.toml` (or the
//! `KURO_LIBRARY_KEY` variable) and carries it inside the exe: that keeps the sounds from being copied out of the
//! install folder, not from someone prepared to take the program apart.
//!
//! Layout: `KUROLIB1`, the length of the sealed list (u32, little-endian), the sealed list (JSON), then the sealed
//! sounds; the list says where each one starts, counted from the end of the list, and how long it is. Something
//! sealed is a 24-byte nonce followed by the ciphertext. The nonce is derived from the key and the content, so
//! packing the same sounds again gives the same bytes and git sees no change.
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"KUROLIB1";
const NONCE: usize = 24;
/// More than any sound or list may unpack to: a damaged file cannot make the app ask for endless memory.
const LIMIT: usize = 64 * 1024 * 1024;

pub type Key = [u8; 32];

// `LIBRARY_KEY`, written by build.rs: the key in two halves that only meet here, or `None` for a build without one.
include!(concat!(env!("OUT_DIR"), "/library_key.rs"));

/// The key this build was given, if any.
pub fn built_in_key() -> Option<Key> {
    LIBRARY_KEY.map(|(a, b)| std::array::from_fn(|i| a[i] ^ b[i]))
}

/// A key as written in `library.key`: 64 hexadecimal digits.
#[allow(dead_code)]
pub fn parse_key(text: &str) -> Option<Key> {
    let hex = text.trim();
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(key)
}

/// One sound of the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: u32,
    /// What pads and exported settings call the sound.
    pub file: String,
    pub name: String,
    /// Names the same sound went by in earlier versions; pads saved back then still find it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub was: Vec<String>,
    /// Where its sealed bytes start, counted from the end of the list, and how many there are.
    #[serde(default)]
    at: u64,
    #[serde(default)]
    len: u64,
}

fn seal(key: &Key, plain: &[u8]) -> Vec<u8> {
    let packed = miniz_oxide::deflate::compress_to_vec(plain, 10);
    let digest = Sha512::new().chain_update(b"kuro library nonce").chain_update(key).chain_update(&packed).finalize();
    let nonce = &digest[..NONCE];
    let sealed = XChaCha20Poly1305::new(key.into()).encrypt(XNonce::from_slice(nonce), packed.as_slice()).expect("encrypting in memory");
    [nonce, &sealed].concat()
}

fn unseal(key: &Key, sealed: &[u8]) -> Result<Vec<u8>, String> {
    if sealed.len() < NONCE {
        return Err("ไฟล์คลังเสียงเสียหาย".into());
    }
    let (nonce, body) = sealed.split_at(NONCE);
    let packed = XChaCha20Poly1305::new(key.into())
        .decrypt(XNonce::from_slice(nonce), body)
        .map_err(|_| "เปิดคลังเสียงไม่ได้ (รหัสไม่ตรงหรือไฟล์เสียหาย)".to_string())?;
    miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, LIMIT).map_err(|_| "ไฟล์คลังเสียงเสียหาย".to_string())
}

/// The bytes of a library file holding these sounds, in this order.
#[allow(dead_code)]
pub fn build(key: &Key, sounds: &[(Entry, Vec<u8>)]) -> Vec<u8> {
    let mut data = Vec::new();
    let mut entries = Vec::with_capacity(sounds.len());
    // The same content under two names is stored once.
    let mut stored: HashMap<Vec<u8>, (u64, u64)> = HashMap::new();
    for (entry, bytes) in sounds {
        let (at, len) = *stored.entry(Sha512::digest(bytes).to_vec()).or_insert_with(|| {
            let sealed = seal(key, bytes);
            let place = (data.len() as u64, sealed.len() as u64);
            data.extend_from_slice(&sealed);
            place
        });
        entries.push(Entry { at, len, ..entry.clone() });
    }
    let list = seal(key, &serde_json::to_vec(&entries).expect("a list of sounds"));
    let mut file = Vec::with_capacity(MAGIC.len() + 4 + list.len() + data.len());
    file.extend_from_slice(MAGIC);
    file.extend_from_slice(&(list.len() as u32).to_le_bytes());
    file.extend_from_slice(&list);
    file.extend_from_slice(&data);
    file
}

/// An open library file: the list is in memory, the sounds are read when asked for.
pub struct Pack {
    path: PathBuf,
    key: Key,
    data_at: u64,
    pub entries: Vec<Entry>,
}

impl Pack {
    pub fn open(path: &Path, key: Key) -> Result<Self, String> {
        let mut file = std::fs::File::open(path).map_err(|e| format!("เปิดไฟล์คลังเสียงไม่ได้: {e}"))?;
        let mut head = [0u8; 12];
        file.read_exact(&mut head).map_err(|_| "ไฟล์คลังเสียงเสียหาย".to_string())?;
        if &head[..8] != MAGIC {
            return Err("ไฟล์คลังเสียงไม่ใช่รูปแบบที่รู้จัก".into());
        }
        let list_len = u32::from_le_bytes([head[8], head[9], head[10], head[11]]) as usize;
        if list_len > LIMIT {
            return Err("ไฟล์คลังเสียงเสียหาย".into());
        }
        let mut list = vec![0u8; list_len];
        file.read_exact(&mut list).map_err(|_| "ไฟล์คลังเสียงเสียหาย".to_string())?;
        let entries = serde_json::from_slice(&unseal(&key, &list)?).map_err(|_| "ไฟล์คลังเสียงเสียหาย".to_string())?;
        Ok(Self { path: path.to_path_buf(), key, data_at: 12 + list_len as u64, entries })
    }

    /// One sound, as the audio file it was.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>, String> {
        if entry.len as usize > LIMIT {
            return Err("ไฟล์คลังเสียงเสียหาย".into());
        }
        let mut file = std::fs::File::open(&self.path).map_err(|e| format!("เปิดไฟล์คลังเสียงไม่ได้: {e}"))?;
        let mut sealed = vec![0u8; entry.len as usize];
        file.seek(SeekFrom::Start(self.data_at + entry.at))
            .and_then(|_| file.read_exact(&mut sealed))
            .map_err(|_| "ไฟล์คลังเสียงเสียหาย".to_string())?;
        unseal(&self.key, &sealed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32, file: &str) -> Entry {
        Entry { id, file: file.into(), name: format!("sound {id}"), was: vec![format!("{id}_old.mp3")], at: 0, len: 0 }
    }

    #[test]
    fn what_is_packed_comes_back_and_only_with_the_key() {
        let key: Key = [7; 32];
        let a = b"ID3 first sound ".repeat(400);
        let b = b"another one".to_vec();
        let sounds = vec![(entry(1, "001.mp3"), a.clone()), (entry(2, "002.mp3"), b.clone()), (entry(3, "003.mp3"), a.clone())];
        let bytes = build(&key, &sounds);
        // Packing again gives the very same file.
        assert_eq!(bytes, build(&key, &sounds));
        // Nothing of the sounds or their names is readable in it.
        assert!(!bytes.windows(9).any(|w| w == b"ID3 first") && !bytes.windows(7).any(|w| w == b"001.mp3"));
        let path = std::env::temp_dir().join(format!("kuro-pack-test-{}.kpk", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let pack = Pack::open(&path, key).unwrap();
        assert_eq!(pack.entries.len(), 3);
        assert_eq!((pack.entries[1].id, pack.entries[1].file.as_str(), pack.entries[1].was[0].as_str()), (2, "002.mp3", "2_old.mp3"));
        assert_eq!(pack.read(&pack.entries[0]).unwrap(), a);
        assert_eq!(pack.read(&pack.entries[1]).unwrap(), b);
        // The third has the first one's content: stored once.
        assert_eq!(pack.read(&pack.entries[2]).unwrap(), a);
        assert_eq!((pack.entries[0].at, pack.entries[0].len), (pack.entries[2].at, pack.entries[2].len));
        assert!(Pack::open(&path, [8; 32]).is_err());
        // A changed byte is noticed, not played.
        let mut broken = bytes.clone();
        let last = broken.len() - 1;
        broken[last] ^= 1;
        std::fs::write(&path, &broken).unwrap();
        let pack = Pack::open(&path, key).unwrap();
        assert!(pack.read(&pack.entries[1]).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn keys() {
        assert_eq!(parse_key(&"0f".repeat(32)), Some([15; 32]));
        assert_eq!(parse_key("  0F0f\n"), None);
        assert_eq!(parse_key(&"zz".repeat(32)), None);
    }
}
