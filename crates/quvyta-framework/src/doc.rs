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
            let location = err.span().map(|span| self.locate(&span));
            Diagnostic::error(location, err.message().to_owned())
        })
    }

    pub(crate) fn locate(&self, span: &Range<usize>) -> Location {
        self.lines.locate(self.file, self.text, span.start)
    }

    pub(crate) fn error(&self, span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(Some(self.locate(span)), message)
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
