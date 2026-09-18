//! Spanned TOML reading shared by every file format the framework loads.
//!
//! Each loader walks the parsed document itself instead of using serde, so every value keeps
//! its byte span and every problem can be reported with a file, line and column.

use std::ops::Range;

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::diagnostics::{Diagnostic, LineStarts, Location};

/// A spanned TOML value.
pub(crate) type Value<'a> = Spanned<DeValue<'a>>;

/// A TOML file being read: its display name and text, for turning spans into locations.
#[derive(Clone)]
pub(crate) struct Doc<'a> {
    file: &'a str,
    text: &'a str,
    lines: LineStarts,
}

impl<'a> Doc<'a> {
    pub(crate) fn new(file: &'a str, text: &'a str) -> Self {
        Self { file, text, lines: LineStarts::new(text) }
    }

    /// Parses the whole file. A syntax error is returned as a located diagnostic.
    pub(crate) fn parse(&self) -> Result<DeTable<'a>, Diagnostic> {
        DeTable::parse(self.text).map(Spanned::into_inner).map_err(|err| {
            let location = match err.span() {
                Some(span) => Some(self.locate(&span)),
                None => self.find_unspanned(err.message()),
            };
            Diagnostic::error(location, err.message().to_owned())
        })
    }

    /// Parses the whole file, keeping what could be read. Every syntax error becomes a located
    /// diagnostic, and the part of the document the parser could still make sense of comes back
    /// with it, so one broken line does not hide the rest of the file.
    pub(crate) fn parse_recoverable(&self) -> (DeTable<'a>, Vec<Diagnostic>) {
        let (root, errors) = DeTable::parse_recoverable(self.text);
        let diagnostics = errors
            .iter()
            .map(|error| {
                let location = match error.span() {
                    Some(span) => Some(self.locate(&span)),
                    None => self.find_unspanned(error.message()),
                };
                Diagnostic::error(location, error.message().to_owned())
            })
            .collect();
        (root.into_inner(), diagnostics)
    }

    /// The best place for an error the parser reported without a span, such as the recursion
    /// limit of a dotted key or a table header nested too deep: the first line that makes the
    /// file give that error, at its first character that is not a space. The file is parsed in
    /// ever longer runs of whole lines, halving the search each time, so this costs a few parses
    /// on a path only a broken file takes. `None` when no run of lines gives the error alone.
    fn find_unspanned(&self, message: &str) -> Option<Location> {
        let ends: Vec<usize> = self
            .text
            .split_inclusive('\n')
            .scan(0, |end, line| {
                *end += line.len();
                Some(*end)
            })
            .collect();
        let fails = |lines: usize| {
            let (_, errors) = DeTable::parse_recoverable(&self.text[..ends[lines - 1]]);
            errors.iter().any(|error| error.span().is_none() && error.message() == message)
        };
        if ends.is_empty() || !fails(ends.len()) {
            return None;
        }
        // The smallest number of lines that fails: more lines never take a line's key away.
        let (mut passing, mut failing) = (0, ends.len());
        while failing - passing > 1 {
            let middle = passing + (failing - passing) / 2;
            if fails(middle) { failing = middle } else { passing = middle }
        }
        let start = if failing == 1 { 0 } else { ends[failing - 2] };
        let line = &self.text[start..ends[failing - 1]];
        let indent = line.len() - line.trim_start().len();
        Some(self.locate(&(start + indent..start + indent)))
    }

    pub(crate) fn locate(&self, span: &Range<usize>) -> Location {
        self.lines.locate(self.file, self.text, span.start)
    }

    pub(crate) fn error(&self, span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(Some(self.locate(span)), message)
    }

    pub(crate) fn warning(&self, span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
        Diagnostic::warning(Some(self.locate(span)), message)
    }

    /// Reads `value` as a table, or explains what was found instead.
    pub(crate) fn table<'t>(&self, value: &'t Value<'a>, what: &str) -> Result<&'t DeTable<'a>, Diagnostic> {
        value.get_ref().as_table().ok_or_else(|| {
            self.error(&value.span(), format!("{what} must be a table, found {}", value.get_ref().type_str()))
        })
    }

    /// Reads `value` as a string, or explains what was found instead.
    pub(crate) fn string<'t>(&self, value: &'t Value<'a>, what: &str) -> Result<&'t str, Diagnostic> {
        value.get_ref().as_str().ok_or_else(|| {
            self.error(&value.span(), format!("{what} must be a string, found {}", value.get_ref().type_str()))
        })
    }
}

/// Returns the value stored under `key`, if any.
pub(crate) fn get<'t, 'a>(table: &'t DeTable<'a>, key: &str) -> Option<&'t Value<'a>> {
    table.get(key)
}

/// The number of a TOML integer, whatever base it is written in; `None` for any other value.
pub(crate) fn integer(value: &DeValue<'_>) -> Option<i64> {
    match value {
        DeValue::Integer(number) => i64::from_str_radix(number.as_str(), number.radix()).ok(),
        _ => None,
    }
}

/// The value as it stands in the file, for diagnostics: the text of a string, a number in the
/// base a reader thinks in whatever base the file wrote it in, `true` or `false`, and the type
/// name of anything a message cannot show in one piece.
pub(crate) fn shown(value: &DeValue<'_>) -> String {
    match value {
        DeValue::String(text) => format!("{:?}", text.as_ref()),
        DeValue::Integer(_) => integer(value).map_or_else(|| "integer".to_owned(), |number| number.to_string()),
        DeValue::Boolean(flag) => flag.to_string(),
        other => other.type_str().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_error_is_located() {
        let doc = Doc::new("bad.toml", "a = 1\nb = \n");
        let Err(err) = doc.parse() else {
            panic!("expected a syntax error");
        };
        let location = err.location.unwrap_or_else(|| panic!("error must be located"));
        assert_eq!(location.file, "bad.toml");
        assert_eq!(location.line, 2);
    }

    #[test]
    fn a_key_nested_too_deep_is_located_at_its_line() {
        // The parser gives no span for these; the line that holds the key is the best place.
        let dotted = format!("a = 1\n\n  {} = 1\nc = 2\n", vec!["k"; 300].join("."));
        let header = format!("a = 1\n[{}]\nb = 2\n", vec!["k"; 300].join("."));
        let entries = format!("a = 1\nb = 2\n\n[[{}]]\n", vec!["k"; 300].join("."));
        for (text, line, column) in [(dotted, 3, 3), (header, 2, 1), (entries, 4, 1)] {
            let doc = Doc::new("deep.toml", &text);
            let Err(err) = doc.parse() else {
                panic!("nesting this deep is refused");
            };
            assert!(err.message.contains("recursion"), "{err}");
            let at = err.location.as_ref().unwrap_or_else(|| panic!("the refusal is located: {err}"));
            assert_eq!((at.file.as_str(), at.line, at.column), ("deep.toml", line, column), "{err}");
            let (_, diagnostics) = doc.parse_recoverable();
            assert!(!diagnostics.is_empty());
            assert!(
                diagnostics.iter().all(|d| d.location.as_ref().is_some_and(|at| at.line == line)),
                "{diagnostics:?}"
            );
        }
    }

    #[test]
    fn nesting_too_deep_is_located_like_any_other_error() {
        // Arrays nested past the parser's limit, starting on the second line.
        let text = format!("a = 1\nb = {}{}\n", "[".repeat(200), "]".repeat(200));
        let doc = Doc::new("deep.toml", &text);
        let Err(err) = doc.parse() else {
            panic!("nesting this deep is refused");
        };
        assert!(err.message.contains("recursion"), "{err}");
        let location = err.location.as_ref().unwrap_or_else(|| panic!("the refusal is located: {err}"));
        assert_eq!((location.file.as_str(), location.line), ("deep.toml", 2), "{err}");
        let (_, diagnostics) = doc.parse_recoverable();
        assert!(!diagnostics.is_empty());
        for diagnostic in &diagnostics {
            let at = diagnostic.location.as_ref().unwrap_or_else(|| panic!("located: {diagnostic}"));
            assert_eq!(at.line, 2, "{diagnostic}");
        }
    }

    #[test]
    fn a_recoverable_parse_locates_the_error_and_keeps_the_rest() {
        let doc = Doc::new("bad.toml", "a = 1\nb = \nc = 3\n");
        let (root, diagnostics) = doc.parse_recoverable();
        let located: Vec<(usize, usize)> =
            diagnostics.iter().filter_map(|d| d.location.as_ref()).map(|at| (at.line, at.column)).collect();
        assert_eq!(located, vec![(2, 5)], "{diagnostics:?}");
        assert_eq!(get(&root, "a").and_then(|v| integer(v.get_ref())), Some(1));
        assert_eq!(get(&root, "c").and_then(|v| integer(v.get_ref())), Some(3), "the line after the error is read");
    }

    #[test]
    fn values_are_shown_as_the_file_writes_them() {
        let doc = Doc::new("t.toml", "count = 0x1f\nname = \"tr\"\non = true\nwhen = 2026-09-18\n");
        let Ok(root) = doc.parse() else {
            panic!("valid toml");
        };
        let show = |key: &str| get(&root, key).map(|value| shown(value.get_ref()));
        assert_eq!(show("count").as_deref(), Some("31"), "a hexadecimal number is shown as a number");
        assert_eq!(get(&root, "count").and_then(|v| integer(v.get_ref())), Some(31), "hexadecimal reads as a number");
        assert_eq!(show("name").as_deref(), Some("\"tr\""));
        assert_eq!(show("on").as_deref(), Some("true"));
        assert_eq!(show("when").as_deref(), Some("datetime"));
        assert_eq!(get(&root, "name").and_then(|v| integer(v.get_ref())), None);
    }

    #[test]
    fn typed_readers_explain_mismatches() {
        let doc = Doc::new("t.toml", "name = 3\n[section]\n");
        let Ok(root) = doc.parse() else {
            panic!("valid toml");
        };
        let Some(name) = get(&root, "name") else {
            panic!("name exists");
        };
        let err = doc.string(name, "meta.name").err().map(|d| d.message);
        assert_eq!(err.as_deref(), Some("meta.name must be a string, found integer"));
        let Some(section) = get(&root, "section") else {
            panic!("section exists");
        };
        assert!(doc.table(section, "section").is_ok());
    }
}
