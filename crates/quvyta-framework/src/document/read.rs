//! Walking a parsed TOML document along a [`Shape`]: what the shape declares is stored, what it
//! does not declare is reported and skipped, and nothing is ever changed on the way.

use toml::de::{DeTable, DeValue};

use super::shape::{Kind, Shape};
use super::{Stored, Table};
use crate::diagnostics::{Diagnostic, Location};
use crate::doc::{self, Doc, Value};

/// Reads `source` as the table `shape` describes. `path` is the dotted way to `source` for
/// diagnostics (empty for the root), and `at` is where `source` starts in the file, which is
/// where a key it is missing is reported.
pub(super) fn table(
    doc: &Doc<'_>,
    source: &DeTable<'_>,
    shape: &Shape,
    path: &str,
    at: &Location,
    found: &mut Vec<Diagnostic>,
) -> Table {
    let mut read = Table::default();
    let mut seen: Vec<String> = Vec::new();
    for (name, value) in source {
        let key = name.get_ref().as_ref();
        let full = join(path, key);
        seen.push(key.to_owned());
        let key_at = doc.locate(&name.span());
        if let Some(declared) = shape.key(key) {
            if let Some(stored) = scalar(doc, value, &declared.kind, &full, declared.required, found) {
                read.values.push((key.to_owned(), stored, key_at, doc.locate(&value.span())));
            }
        } else if let Some(inner) = shape.inner(key) {
            match value.get_ref() {
                DeValue::Table(source) => {
                    read.tables.push((key.to_owned(), table(doc, source, inner, &full, &key_at, found)));
                }
                other => found.push(doc.warning(&value.span(), mismatch(&full, "a table", other))),
            }
        } else if let Some(entry) = shape.entry(key) {
            match value.get_ref() {
                DeValue::Array(items) => {
                    let entries = items
                        .iter()
                        .enumerate()
                        .filter_map(|(index, item)| {
                            let path = format!("{full}[{index}]");
                            match item.get_ref() {
                                DeValue::Table(source) => {
                                    let at = doc.locate(&item.span());
                                    Some(table(doc, source, entry, &path, &at, found))
                                }
                                other => {
                                    found.push(doc.warning(&item.span(), mismatch(&path, "a table", other)));
                                    None
                                }
                            }
                        })
                        .collect();
                    read.arrays.push((key.to_owned(), entries));
                }
                other => found.push(doc.warning(&value.span(), mismatch(&full, "an array of tables", other))),
            }
        } else {
            found.push(doc.warning(&name.span(), format!("`{full}` is not part of the document; it is ignored")));
        }
    }
    for key in shape.required_keys() {
        // A key that is there but unreadable was reported where it stands; saying it is missing
        // as well would report one problem twice.
        if !seen.iter().any(|name| name == key) {
            found.push(Diagnostic::error(Some(at.clone()), format!("`{}` is required and missing", join(path, key))));
        }
    }
    read
}

/// Reads one value of the declared `kind`, or reports what stands there instead.
fn scalar(
    doc: &Doc<'_>,
    value: &Value<'_>,
    kind: &Kind,
    path: &str,
    required: bool,
    found: &mut Vec<Diagnostic>,
) -> Option<Stored> {
    let stored = match (kind, value.get_ref()) {
        (Kind::Text, DeValue::String(text)) => Some(Stored::Text(text.to_string())),
        (Kind::Choice(choices), DeValue::String(text)) if choices.iter().any(|choice| choice == text.as_ref()) => {
            Some(Stored::Text(text.to_string()))
        }
        (Kind::Integer, number) => doc::integer(number).map(Stored::Integer),
        (Kind::Flag, DeValue::Boolean(flag)) => Some(Stored::Flag(*flag)),
        _ => None,
    };
    if stored.is_none() {
        let message = mismatch(path, &kind.describe(), value.get_ref());
        // A required key the document cannot give is an error: the document is not the one the
        // application asked for. An optional one only loses its value.
        found.push(if required { doc.error(&value.span(), message) } else { doc.warning(&value.span(), message) });
    }
    stored
}

/// The message for a value that is not what the shape declared.
fn mismatch(path: &str, expected: &str, found: &DeValue<'_>) -> String {
    format!("`{path}` must be {expected}, found {}; it is ignored", doc::shown(found))
}

/// `key` below `path`, as a message writes it.
fn join(path: &str, key: &str) -> String {
    if path.is_empty() { key.to_owned() } else { format!("{path}.{key}") }
}
