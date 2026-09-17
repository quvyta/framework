//! Problems found while loading theme, icon, locale and keymap files.
//!
//! Loading never panics and never aborts on the first mistake: every problem becomes a
//! [`Diagnostic`] that points at the file, line and column, the broken entry is skipped,
//! and the rest of the file is still used.

use std::fmt;

/// A position inside a loaded file. Lines and columns start at 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// Display name of the file, e.g. `nordic.toml`.
    pub file: String,
    /// Line number, starting at 1.
    pub line: usize,
    /// Column number in characters, starting at 1.
    pub column: usize,
}

impl Location {
    /// Converts a byte offset in `text` into a line and column.
    ///
    /// Offsets past the end of `text` point at the end of the file, and offsets inside a
    /// character at that character.
    #[must_use]
    pub fn from_offset(file: &str, text: &str, offset: usize) -> Self {
        let before = &text[..text.floor_char_boundary(offset)];
        let line = before.matches('\n').count() + 1;
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        let column = before[line_start..].chars().count() + 1;
        Self { file: file.to_owned(), line, column }
    }
}

/// Where every line of a text starts, for turning many byte offsets of one file into locations
/// without counting its lines again each time: a theme file has a location for every property.
#[derive(Debug, Clone)]
pub(crate) struct LineStarts(Vec<usize>);

impl LineStarts {
    pub(crate) fn new(text: &str) -> Self {
        Self(std::iter::once(0).chain(text.match_indices('\n').map(|(index, _)| index + 1)).collect())
    }

    /// The same location as [`Location::from_offset`] gives for `text`, the text these line
    /// starts were made from.
    pub(crate) fn locate(&self, file: &str, text: &str, offset: usize) -> Location {
        let offset = text.floor_char_boundary(offset);
        // Line starts are sorted and the first is 0, so at least one is at or before `offset`.
        let line = self.0.partition_point(|start| *start <= offset);
        let line_start = self.0[line.saturating_sub(1)];
        let column = text[line_start..offset].chars().count() + 1;
        Location { file: file.to_owned(), line, column }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The entry was ignored; the value it tried to set comes from elsewhere.
    Error,
    /// The entry was used, but it is probably not what the author wants.
    Warning,
}

/// One problem found while loading a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Error or warning.
    pub severity: Severity,
    /// Where the problem is, when it can be pinned to a place in a file.
    pub location: Option<Location>,
    /// What is wrong and, where possible, how to fix it.
    pub message: String,
}

impl Diagnostic {
    /// Creates an error.
    #[must_use]
    pub fn error(location: Option<Location>, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, location, message: message.into() }
    }

    /// Creates a warning.
    #[must_use]
    pub fn warning(location: Option<Location>, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, location, message: message.into() }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        match &self.location {
            Some(location) => write!(f, "{location}: {kind}: {}", self.message),
            None => write!(f, "{kind}: {}", self.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_maps_to_line_and_column() {
        let text = "a = 1\nbé = 2\n";
        let at = Location::from_offset("x.toml", text, text.find('=').unwrap_or(0));
        assert_eq!((at.line, at.column), (1, 3));
        let second = text.rfind('=').unwrap_or(0);
        let at = Location::from_offset("x.toml", text, second);
        assert_eq!((at.line, at.column), (2, 4));
        assert_eq!(at.to_string(), "x.toml:2:4");
    }

    #[test]
    fn offset_inside_a_character_points_at_that_character() {
        let at = Location::from_offset("x.toml", "a = 1\nbé = 2\n", 8);
        assert_eq!((at.line, at.column), (2, 2));
    }

    #[test]
    fn line_starts_locate_every_offset_like_from_offset() {
        for text in
            ["", "ab", "a = 1\nbé = 2\n", "\n\nx", "k = \"界\"\r\n\r\nv = 'é😀'\r\n# ü\n", "no newline at end é"]
        {
            let starts = LineStarts::new(text);
            for offset in 0..=text.len() + 2 {
                assert_eq!(
                    starts.locate("x.toml", text, offset),
                    Location::from_offset("x.toml", text, offset),
                    "{text:?} at {offset}"
                );
            }
        }
    }

    #[test]
    fn offset_past_end_is_clamped() {
        let at = Location::from_offset("x.toml", "ab", 99);
        assert_eq!((at.line, at.column), (1, 3));
    }

    #[test]
    fn display_includes_severity_and_location() {
        let d = Diagnostic::error(Some(Location::from_offset("t.toml", "x", 0)), "bad");
        assert_eq!(d.to_string(), "t.toml:1:1: error: bad");
        assert_eq!(Diagnostic::warning(None, "hm").to_string(), "warning: hm");
    }
}
