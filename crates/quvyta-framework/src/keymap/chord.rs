//! Key chords such as `ctrl+shift+p`.

use std::fmt;
use std::str::FromStr;

/// A key without modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// A printable character, stored lowercase for letters.
    Char(char),
    /// Enter / Return.
    Enter,
    /// Escape.
    Esc,
    /// Tab.
    Tab,
    /// Space bar.
    Space,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Insert.
    Insert,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Function key F1–F24.
    F(u8),
    /// The context menu key (reported by terminals with the kitty keyboard protocol).
    Menu,
}

const NAMED: [(&str, Key); 16] = [
    ("enter", Key::Enter),
    ("esc", Key::Esc),
    ("tab", Key::Tab),
    ("space", Key::Space),
    ("backspace", Key::Backspace),
    ("delete", Key::Delete),
    ("insert", Key::Insert),
    ("home", Key::Home),
    ("end", Key::End),
    ("pgup", Key::PageUp),
    ("pgdn", Key::PageDown),
    ("up", Key::Up),
    ("down", Key::Down),
    ("left", Key::Left),
    ("right", Key::Right),
    ("menu", Key::Menu),
];

/// Held modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Modifiers {
    /// Control.
    pub ctrl: bool,
    /// Alt / Option.
    pub alt: bool,
    /// Shift. A letter keeps shift beside its lowercase form: typing `A` is `shift+a`, and so is
    /// `"A"` in a keymap file. Other characters fold shift into the character itself: `?`, not
    /// `shift+/`.
    pub shift: bool,
}

/// A key plus modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyChord {
    /// The key.
    pub key: Key,
    /// The modifiers held with it.
    pub mods: Modifiers,
}

impl KeyChord {
    /// A chord without modifiers.
    #[must_use]
    pub fn plain(key: Key) -> Self {
        Self { key, mods: Modifiers::default() }
    }

    /// Short label for key hint bars: `ctrl p`, `shift tab`, `?`, `f12`.
    #[must_use]
    pub fn label(&self) -> String {
        self.parts().join(" ")
    }

    fn parts(&self) -> Vec<String> {
        let mut parts = Vec::new();
        if self.mods.ctrl {
            parts.push("ctrl".to_owned());
        }
        if self.mods.alt {
            parts.push("alt".to_owned());
        }
        if self.mods.shift {
            parts.push("shift".to_owned());
        }
        parts.push(match self.key {
            Key::Char(c) => c.to_string(),
            Key::F(n) => format!("f{n}"),
            other => {
                NAMED.iter().find(|(_, key)| *key == other).map(|(name, _)| (*name).to_owned()).unwrap_or_default()
            }
        });
        parts
    }
}

impl FromStr for KeyChord {
    type Err = String;

    /// Parses `ctrl+shift+p`, `?`, `f12`, `shift+tab`. Modifier and key names are
    /// case-insensitive, but an uppercase letter means shift plus that letter: `S` is
    /// `shift+s`, as a terminal reports it. `+` alone is the plus key.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("empty key binding".to_owned());
        }
        let (modifier_part, key_part) = if trimmed == "+" {
            ("", "+")
        } else if let Some(prefix) = trimmed.strip_suffix("++") {
            (prefix, "+")
        } else {
            match trimmed.rsplit_once('+') {
                Some((mods, key)) => (mods, key),
                None => ("", trimmed),
            }
        };
        let mut mods = Modifiers::default();
        for modifier in modifier_part.split('+').filter(|m| !m.is_empty()) {
            match modifier.to_lowercase().as_str() {
                "ctrl" | "control" => mods.ctrl = true,
                "alt" | "option" => mods.alt = true,
                "shift" => mods.shift = true,
                other => {
                    return Err(format!("unknown modifier `{other}` in `{text}`; use ctrl, alt or shift"));
                }
            }
        }
        let key = parse_key(key_part, &mut mods).ok_or_else(|| {
            format!("unknown key `{key_part}` in `{text}`; use a character, f1–f24 or a key name such as enter, esc, tab, space, up")
        })?;
        Ok(Self { key, mods })
    }
}

/// The key named `text`. A single uppercase letter turns on `mods.shift` and becomes its
/// lowercase letter, the way the terminal runtime normalises typed letters.
fn parse_key(text: &str, mods: &mut Modifiers) -> Option<Key> {
    let mut chars = text.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_whitespace() || c.is_control() {
            return None;
        }
        if c.is_uppercase() {
            mods.shift = true;
            return Some(Key::Char(c.to_lowercase().next().unwrap_or(c)));
        }
        return Some(Key::Char(c));
    }
    let name = text.to_lowercase();
    if let Some((_, key)) = NAMED.iter().find(|(n, _)| *n == name) {
        return Some(*key);
    }
    let number = name.strip_prefix('f')?.parse::<u8>().ok()?;
    (1..=24).contains(&number).then_some(Key::F(number))
}

impl fmt::Display for KeyChord {
    /// The canonical form used in keymap files: `ctrl+shift+p`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.parts().join("+"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(text: &str) -> KeyChord {
        text.parse().expect("valid chord")
    }

    #[test]
    fn parses_modifiers_and_keys() {
        let c = chord("Ctrl+Shift+P");
        assert_eq!(c.key, Key::Char('p'));
        assert!(c.mods.ctrl && c.mods.shift && !c.mods.alt);
        assert_eq!(chord("?"), KeyChord::plain(Key::Char('?')));
        assert_eq!(chord("f12"), KeyChord::plain(Key::F(12)));
        assert_eq!(chord("shift+tab").key, Key::Tab);
        assert_eq!(chord("+"), KeyChord::plain(Key::Char('+')));
        assert_eq!(chord("ctrl++").key, Key::Char('+'));
    }

    #[test]
    fn an_uppercase_letter_means_shift_plus_that_letter() {
        assert_eq!(chord("S"), chord("shift+s"));
        assert_eq!(chord("ctrl+S"), chord("ctrl+shift+s"));
        assert_eq!(chord("shift+S"), chord("shift+s"));
        assert_eq!(chord("s"), KeyChord::plain(Key::Char('s')));
        assert_eq!(chord("Ş"), chord("shift+ş"));
        assert_eq!(chord("F12"), KeyChord::plain(Key::F(12)), "key names stay case-insensitive");
        assert_eq!(chord("Ctrl+Enter"), chord("ctrl+enter"));
        assert_eq!(chord("?"), KeyChord::plain(Key::Char('?')), "symbols carry no shift");
        assert_eq!(chord("S").to_string(), "shift+s");
    }

    #[test]
    fn rejects_unknown_parts() {
        assert!("hyper+x".parse::<KeyChord>().is_err());
        assert!("ctrl+banana".parse::<KeyChord>().is_err());
        assert!("f25".parse::<KeyChord>().is_err());
        assert!("".parse::<KeyChord>().is_err());
    }

    #[test]
    fn formats_for_files_and_hint_bars() {
        let c = chord("shift+ctrl+pgup");
        assert_eq!(c.to_string(), "ctrl+shift+pgup");
        assert_eq!(c.label(), "ctrl shift pgup");
        assert_eq!(chord("f12").label(), "f12");
    }
}
