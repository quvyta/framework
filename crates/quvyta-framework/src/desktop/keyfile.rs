//! The key file format desktop entries and `mimeapps.list` are written in: `[Group]` headers and
//! `key=value` lines, with backslash escapes in the values.

use std::path::Path;

use super::{lines, warn, warn_at};
use crate::diagnostics::Diagnostic;

/// One `[Group]` and its entries, in the order they were written.
pub(super) struct Group {
    pub(super) name: String,
    /// The line of the `[Group]` header.
    pub(super) line: usize,
    /// The column the `[Group]` header stands at.
    pub(super) column: usize,
    /// Every key, its raw value and where it was written.
    entries: Vec<Entry>,
}

/// One `key=value` line of a group, and the place it stands at.
struct Entry {
    key: String,
    value: String,
    line: usize,
    column: usize,
}

impl Group {
    /// The raw value of a key; the first one wins when a key is written twice.
    pub(super) fn get(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|entry| entry.key == key).map(|entry| entry.value.as_str())
    }

    /// The line and the column a key is written at, the first time.
    pub(super) fn at(&self, key: &str) -> Option<(usize, usize)> {
        self.entries.iter().find(|entry| entry.key == key).map(|entry| (entry.line, entry.column))
    }

    /// Every key and its raw value.
    pub(super) fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|entry| (entry.key.as_str(), entry.value.as_str()))
    }
}

/// The groups of the key file `path`, whose bytes are `bytes`. A line that is neither a header,
/// an entry nor a comment is skipped with a warning, as is an entry before the first header.
pub(super) fn parse(bytes: &[u8], path: &Path, diagnostics: &mut Vec<Diagnostic>) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for (number, line) in lines(bytes, path, diagnostics) {
        // A key is written where it is written, not at the line's start: a diagnostic about one
        // points at it.
        let column = start_column(line);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            groups.push(Group { name: name.to_owned(), line: number, column, entries: Vec::new() });
            continue;
        }
        let entry = line.split_once('=').map(|(key, value)| (key.trim_end(), value)).filter(|(key, _)| !key.is_empty());
        match (entry, groups.last_mut()) {
            (Some((key, value)), Some(group)) => {
                group.entries.push(Entry {
                    key: key.to_owned(),
                    value: value.trim_start().to_owned(),
                    line: number,
                    column,
                });
            }
            (Some(_), None) => {
                warn_at(diagnostics, path, number, column, "an entry before any [Group] header; it is skipped");
            }
            (None, _) => {
                warn(diagnostics, path, number, "the line is neither a [Group] header nor key=value; it is skipped");
            }
        }
    }
    groups
}

/// The column the line's first character that is not space stands at, counted in characters, as
/// a [`Location`](crate::diagnostics::Location) counts.
fn start_column(line: &str) -> usize {
    line.chars().count() - line.trim_start().chars().count() + 1
}

/// The character an escape stands for in any value.
fn escape(c: char) -> Option<char> {
    match c {
        's' => Some(' '),
        'n' => Some('\n'),
        't' => Some('\t'),
        'r' => Some('\r'),
        '\\' => Some('\\'),
        _ => None,
    }
}

/// A string value with its escapes resolved. An escape the format does not know is kept as it
/// was written, so a value the author got slightly wrong still reads as they meant it.
pub(super) fn string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some(next) => match escape(next) {
                Some(plain) => out.push(plain),
                None => {
                    out.push('\\');
                    out.push(next);
                }
            },
            None => out.push('\\'),
        }
    }
    out
}

/// A `;`-separated list value, each item with its escapes resolved; `\;` is a semicolon inside an
/// item. Empty items, such as the one after the customary closing `;`, are dropped.
pub(super) fn list(raw: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut item = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        match c {
            ';' => items.push(std::mem::take(&mut item)),
            '\\' => match chars.next() {
                Some(';') => item.push(';'),
                Some(next) => match escape(next) {
                    Some(plain) => item.push(plain),
                    None => {
                        item.push('\\');
                        item.push(next);
                    }
                },
                None => item.push('\\'),
            },
            _ => item.push(c),
        }
    }
    items.push(item);
    items.iter().map(|item| item.trim()).filter(|item| !item.is_empty()).map(str::to_owned).collect()
}
