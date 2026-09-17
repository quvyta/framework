//! Style properties set by theme rules and typography roles.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::paint::{Expr, Paint};

/// The value of one style property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropValue {
    /// A colour: `fg = "$text"`.
    Paint(Paint),
    /// A switch: `bold = true`.
    Flag(bool),
    /// A cell count: `gap = 1`.
    Cells(u16),
    /// Vertical and horizontal cell counts: `padding = [0, 2]`.
    Pair(u16, u16),
    /// One of a fixed set of words: `style = "thin"`. Only keys listed in [`WORD_PROPS`] hold
    /// words.
    Word(&'static str),
}

/// Style keys whose value is a word from a fixed list, as `(widget, key, allowed words)`.
/// Theme files are checked against this list; any other word is reported with its location.
pub const WORD_PROPS: [(&str, &str, &[&str]); 1] = [("scrollbar", "style", &["block", "half", "thin", "dots"])];

/// The allowed words of `key` in rules for `widget`, when that key holds a word.
pub(crate) fn allowed_words(widget: &str, key: &str) -> Option<&'static [&'static str]> {
    WORD_PROPS.iter().find(|(w, k, _)| *w == widget && *k == key).map(|(_, _, words)| *words)
}

/// Style properties after all matching rules have been layered.
///
/// Clones share their storage, so handing out a theme's remembered style every frame copies
/// nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleProps {
    values: Arc<BTreeMap<String, PropValue>>,
}

impl StyleProps {
    /// The raw value of `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<PropValue> {
        self.values.get(key).copied()
    }

    /// The paint stored under `key`, if `key` holds a colour.
    #[must_use]
    pub fn paint(&self, key: &str) -> Option<Paint> {
        match self.get(key)? {
            PropValue::Paint(paint) => Some(paint),
            _ => None,
        }
    }

    /// The flag stored under `key`; `false` when unset.
    #[must_use]
    pub fn flag(&self, key: &str) -> bool {
        matches!(self.get(key), Some(PropValue::Flag(true)))
    }

    /// The cell count stored under `key`.
    #[must_use]
    pub fn cells(&self, key: &str) -> Option<u16> {
        match self.get(key)? {
            PropValue::Cells(n) => Some(n),
            _ => None,
        }
    }

    /// The `(vertical, horizontal)` pair stored under `key`.
    #[must_use]
    pub fn pair(&self, key: &str) -> Option<(u16, u16)> {
        match self.get(key)? {
            PropValue::Pair(v, h) => Some((v, h)),
            _ => None,
        }
    }

    /// The word stored under `key`, such as a scrollbar `style`.
    #[must_use]
    pub fn word(&self, key: &str) -> Option<&'static str> {
        match self.get(key)? {
            PropValue::Word(word) => Some(word),
            _ => None,
        }
    }

    /// Whether any property uses an animated paint.
    #[must_use]
    pub fn is_animated(&self) -> bool {
        self.values.values().any(|value| matches!(value, PropValue::Paint(paint) if paint.is_animated()))
    }

    /// Whether no property is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// All properties, sorted by name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, PropValue)> {
        self.values.iter().map(|(key, value)| (key.as_str(), *value))
    }

    pub(crate) fn set(&mut self, key: &str, value: PropValue) {
        Arc::make_mut(&mut self.values).insert(key.to_owned(), value);
    }

    /// Copies every property of `other` over this one.
    pub(crate) fn overlay(&mut self, other: &Self) {
        if other.values.is_empty() {
            return;
        }
        let values = Arc::make_mut(&mut self.values);
        for (key, value) in other.values.iter() {
            values.insert(key.clone(), *value);
        }
    }
}

/// A property value as written in the file, before colour tokens are resolved.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RawProp {
    Expr(Expr),
    Flag(bool),
    Cells(u16),
    Pair(u16, u16),
    Word(&'static str),
}
