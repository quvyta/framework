//! Input events delivered to widgets.

use crate::keymap::{Key, KeyChord, Modifiers};

/// Whether a key went down, auto-repeated or came up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// The key went down.
    Press,
    /// The key is held and the terminal repeats it (only reported by terminals with the kitty
    /// keyboard protocol).
    Repeat,
    /// The key came up (kitty keyboard protocol only).
    Release,
}

/// A key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key and modifiers, normalised for keymap matching: letters are lowercase with
    /// `shift` set when typed uppercase.
    pub chord: KeyChord,
    /// Down, repeat or up.
    pub kind: KeyKind,
    /// The character to insert when typing, if this key produces one.
    pub text: Option<char>,
}

impl KeyEvent {
    /// A key press from a chord such as `"ctrl+s"`, with the character it types.
    ///
    /// # Panics
    ///
    /// Panics when `chord` is not a valid chord; meant for tests and fixed bindings.
    #[must_use]
    pub fn press(chord: &str) -> Self {
        let chord: KeyChord = chord.parse().unwrap_or_else(|message| panic!("invalid chord `{chord}`: {message}"));
        Self::from_chord(chord)
    }

    /// A press of `chord`; `text` is derived from plain character and space keys.
    #[must_use]
    pub fn from_chord(chord: KeyChord) -> Self {
        let typing = !chord.mods.ctrl && !chord.mods.alt;
        let text = match chord.key {
            Key::Char(c) if typing && chord.mods.shift => Some(c.to_uppercase().next().unwrap_or(c)),
            Key::Char(c) if typing => Some(c),
            Key::Space if typing => Some(' '),
            _ => None,
        };
        Self { chord, kind: KeyKind::Press, text }
    }

    /// Whether this is a press (not repeat or release) of exactly `key` with no modifiers.
    #[must_use]
    pub fn is_plain(&self, key: Key) -> bool {
        self.kind != KeyKind::Release && self.chord.key == key && self.chord.mods == Modifiers::default()
    }
}

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Wheel button.
    Middle,
}

/// What the mouse did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    /// A button went down.
    Down(MouseButton),
    /// A button came up.
    Up(MouseButton),
    /// The mouse moved with a button held.
    Drag(MouseButton),
    /// The mouse moved with no button held. Widgets hear it only when they ask with
    /// [`PaintCx::track_pointer_moves`](crate::widget::PaintCx::track_pointer_moves) and the
    /// pointer is over them or over a child of theirs: most widgets take any mouse event under
    /// them as theirs, and a move alone is no reason to act. Hover looks need no event: they
    /// are painted from [`PaintCx::is_hovered`](crate::widget::PaintCx::is_hovered).
    Moved,
    /// Wheel towards the top of the content.
    ScrollUp,
    /// Wheel towards the bottom of the content.
    ScrollDown,
}

/// A mouse event at a screen cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// What happened.
    pub kind: MouseKind,
    /// Screen column.
    pub x: i32,
    /// Screen row.
    pub y: i32,
    /// Held modifiers.
    pub mods: Modifiers,
}

/// An input event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A key.
    Key(KeyEvent),
    /// The mouse.
    Mouse(MouseEvent),
    /// Text pasted into the terminal.
    Paste(String),
    /// The pointer went down outside the widget that captured the keyboard, e.g. outside an
    /// open dropdown. The widget usually closes.
    PointerOutside,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_derives_typed_text() {
        assert_eq!(KeyEvent::press("a").text, Some('a'));
        assert_eq!(KeyEvent::press("shift+a").text, Some('A'));
        assert_eq!(KeyEvent::press("space").text, Some(' '));
        assert_eq!(KeyEvent::press("ctrl+a").text, None);
        assert_eq!(KeyEvent::press("enter").text, None);
        assert!(KeyEvent::press("enter").is_plain(Key::Enter));
        assert!(!KeyEvent::press("shift+enter").is_plain(Key::Enter));
    }
}
