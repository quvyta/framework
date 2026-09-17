//! Stable widget identities.

use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hasher};

/// Identifies a widget across frames.
///
/// Derived from the parent's id and the widget's key: its explicit name when one was given
/// with [`NodeMut::id`](crate::widget::NodeMut::id), otherwise its position among its siblings
/// and its type. Hashing is deterministic (FNV-1a), so ids are the same on every run.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(u64);

impl WidgetId {
    /// The id of the root of the view.
    pub const ROOT: Self = Self(0xcbf2_9ce4_8422_2325);

    pub(crate) fn child(self, key: &Key, type_name: &str) -> Self {
        let mut hash = Fnv(self.0);
        match key {
            Key::Index(index) => {
                hash.write(b"#");
                hash.write(&index.to_le_bytes());
                hash.write(type_name.as_bytes());
            }
            Key::Named(name) => {
                hash.write(b"@");
                hash.write(name.as_bytes());
            }
        }
        Self(hash.0)
    }
}

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetId({:016x})", self.0)
    }
}

/// How a node is told apart from its siblings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Key {
    Index(usize),
    Named(String),
}

/// A map keyed by widget ids, as painting fills several of them for every widget in every frame.
pub(crate) type IdMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;

/// Hashes keys made of widget ids and small numbers. A widget id is already a well mixed hash,
/// so a multiply-and-rotate step per field is enough; the keys come from the application's own
/// view, not from input an attacker controls, so the collision resistance of the standard hasher
/// buys nothing and costs a noticeable share of each frame.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct IdHasher(u64);

impl IdHasher {
    fn add(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.add(u64::from(*byte));
        }
    }

    fn write_u16(&mut self, value: u16) {
        self.add(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }
}

struct Fnv(u64);

impl Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_depend_on_parent_key_and_type() {
        let a = WidgetId::ROOT.child(&Key::Index(0), "Button");
        assert_eq!(a, WidgetId::ROOT.child(&Key::Index(0), "Button"));
        assert_ne!(a, WidgetId::ROOT.child(&Key::Index(1), "Button"));
        assert_ne!(a, WidgetId::ROOT.child(&Key::Index(0), "Text"));
        let named = WidgetId::ROOT.child(&Key::Named("save".into()), "Button");
        assert_eq!(named, WidgetId::ROOT.child(&Key::Named("save".into()), "Text"));
        assert_ne!(a.child(&Key::Index(0), "Text"), named.child(&Key::Index(0), "Text"));
    }
}
