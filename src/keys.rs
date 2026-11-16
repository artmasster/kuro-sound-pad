//! Hotkeys: a Windows virtual-key code plus Ctrl / Shift / Alt. The codes are the same numbers the old web app
//! stored (`KeyboardEvent.keyCode`), so its settings import unchanged.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hotkey {
    pub vk: u16,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ctrl: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub shift: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub alt: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;

/// Keys that only modify others: never a hotkey on their own.
pub fn is_modifier(vk: u16) -> bool {
    matches!(vk, VK_SHIFT | VK_CONTROL | VK_MENU | VK_LWIN | VK_RWIN | 0xA0..=0xA5)
}

impl Hotkey {
    pub const fn plain(vk: u16) -> Self {
        Self { vk, ctrl: false, shift: false, alt: false }
    }

    /// The label on a key cap: `Ctrl + Shift + F1`.
    pub fn label(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl + ");
        }
        if self.shift {
            s.push_str("Shift + ");
        }
        if self.alt {
            s.push_str("Alt + ");
        }
        s.push_str(&key_name(self.vk));
        s
    }

    /// The old app's storage form: `ctrl+shift+key112`. Used in exported settings so the old app reads them too.
    pub fn to_legacy(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("ctrl+");
        }
        if self.shift {
            s.push_str("shift+");
        }
        if self.alt {
            s.push_str("alt+");
        }
        s.push_str(&format!("key{}", self.vk));
        s
    }

    /// Reads both forms the old app wrote: a bare key (`"7"`, `"u"`, `","`, `"space"`) and `ctrl+key85`.
    pub fn from_legacy(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();
        if s.is_empty() {
            return None;
        }
        // "+" splits the parts, but a bare "+" is not a key the old app could store, so this is safe.
        let parts: Vec<&str> = s.split('+').collect();
        let key = *parts.last()?;
        let mods = &parts[..parts.len() - 1];
        let vk = if let Some(n) = key.strip_prefix("key").filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) {
            n.parse::<u16>().ok()?
        } else {
            legacy_key(key)?
        };
        if vk == 0 || vk > 255 || is_modifier(vk) {
            return None;
        }
        Some(Self {
            vk,
            ctrl: mods.contains(&"ctrl"),
            shift: mods.contains(&"shift"),
            alt: mods.contains(&"alt"),
        })
    }
}

fn legacy_key(key: &str) -> Option<u16> {
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return match c {
            'a'..='z' => Some(c as u16 - 'a' as u16 + 0x41),
            '0'..='9' => Some(c as u16),
            ';' => Some(186),
            '=' => Some(187),
            ',' => Some(188),
            '-' => Some(189),
            '.' => Some(190),
            '/' => Some(191),
            '`' => Some(192),
            '[' => Some(219),
            '\\' => Some(220),
            ']' => Some(221),
            '\'' => Some(222),
            ' ' => Some(32),
            _ => None,
        };
    }
    Some(match key {
        "space" => 32,
        "enter" => 13,
        "backspace" => 8,
        "tab" => 9,
        "pageup" => 33,
        "pagedown" => 34,
        "end" => 35,
        "home" => 36,
        "left" | "arrowleft" => 37,
        "up" | "arrowup" => 38,
        "right" | "arrowright" => 39,
        "down" | "arrowdown" => 40,
        "insert" => 45,
        "delete" => 46,
        _ => {
            let n = key.strip_prefix('f')?.parse::<u16>().ok()?;
            if (1..=24).contains(&n) {
                0x70 + n - 1
            } else {
                return None;
            }
        }
    })
}

pub fn key_name(vk: u16) -> String {
    let s = match vk {
        0x30..=0x39 | 0x41..=0x5A => return (vk as u8 as char).to_string(),
        0x70..=0x87 => return format!("F{}", vk - 0x6F),
        0x60..=0x69 => return format!("Num {}", vk - 0x60),
        8 => "Backspace",
        9 => "Tab",
        13 => "Enter",
        19 => "Pause",
        20 => "Caps Lock",
        27 => "Esc",
        32 => "Space",
        33 => "Page Up",
        34 => "Page Down",
        35 => "End",
        36 => "Home",
        37 => "←",
        38 => "↑",
        39 => "→",
        40 => "↓",
        44 => "Print Screen",
        45 => "Insert",
        46 => "Delete",
        93 => "Menu",
        106 => "Num *",
        107 => "Num +",
        109 => "Num -",
        110 => "Num .",
        111 => "Num /",
        144 => "Num Lock",
        145 => "Scroll Lock",
        0xA6 => "Browser Back",
        0xA7 => "Browser Forward",
        0xAD => "Mute",
        0xAE => "Volume Down",
        0xAF => "Volume Up",
        0xB0 => "Next Track",
        0xB1 => "Previous Track",
        0xB2 => "Media Stop",
        0xB3 => "Play / Pause",
        186 => ";",
        187 => "=",
        188 => ",",
        189 => "-",
        190 => ".",
        191 => "/",
        192 => "`",
        219 => "[",
        220 => "\\",
        221 => "]",
        222 => "'",
        _ => return format!("Key {vk}"),
    };
    s.to_string()
}

/// The old app's defaults: 7 8 9 0 / U I O P / H J K L / N M , .
pub const DEFAULT_PAD_KEYS: [u16; 16] =
    [0x37, 0x38, 0x39, 0x30, 0x55, 0x49, 0x4F, 0x50, 0x48, 0x4A, 0x4B, 0x4C, 0x4E, 0x4D, 188, 190];
/// Y stops everything.
pub const DEFAULT_STOP_KEY: u16 = 0x59;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_forms() {
        assert_eq!(Hotkey::from_legacy("7"), Some(Hotkey::plain(0x37)));
        assert_eq!(Hotkey::from_legacy("U"), Some(Hotkey::plain(0x55)));
        assert_eq!(Hotkey::from_legacy(","), Some(Hotkey::plain(188)));
        assert_eq!(Hotkey::from_legacy("key85"), Some(Hotkey::plain(85)));
        assert_eq!(
            Hotkey::from_legacy("ctrl+shift+key112"),
            Some(Hotkey { vk: 112, ctrl: true, shift: true, alt: false })
        );
        assert_eq!(Hotkey::from_legacy("alt+space"), Some(Hotkey { vk: 32, ctrl: false, shift: false, alt: true }));
        assert_eq!(Hotkey::from_legacy(""), None);
        assert_eq!(Hotkey::from_legacy("key16"), None);
        assert_eq!(Hotkey::from_legacy("key"), None);
    }

    #[test]
    fn round_trip() {
        let k = Hotkey { vk: 0x41, ctrl: true, shift: false, alt: true };
        assert_eq!(Hotkey::from_legacy(&k.to_legacy()), Some(k));
        assert_eq!(k.label(), "Ctrl + Alt + A");
        assert_eq!(Hotkey::plain(0x70).label(), "F1");
    }
}
