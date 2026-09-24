//! The key file format desktop entries and `mimeapps.list` are written in: `[Group]` headers and
//! `key=value` lines, with backslash escapes in the values.

use std::path::Path;

use super::{lines, warn};
use crate::diagnostics::Diagnostic;

/// One `[Group]` and its entries, in the order they were written.
pub(super) struct Group {
    pub(super) name: String,
    /// The line of the `[Group]` header.
    pub(super) line: usize,
    /// Every key, its raw value and its line.
    entries: Vec<(String, String, usize)>,
}

impl Group {
    /// The raw value of a key; the first one wins when a key is written twice.
    pub(super) fn get(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|(name, _, _)| name == key).map(|(_, value, _)| value.as_str())
    }

    /// The line a key is written on, the first time.
    pub(super) fn line_of(&self, key: &str) -> Option<usize> {
        self.entries.iter().find(|(name, _, _)| name == key).map(|(_, _, line)| *line)
    }

    /// Every key and its raw value.
    pub(super) fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(key, value, _)| (key.as_str(), value.as_str()))
    }
}

/// The groups of the key file `path`, whose bytes are `bytes`. A line that is neither a header,
/// an entry nor a comment is skipped with a warning, as is an entry before the first header.
pub(super) fn parse(bytes: &[u8], path: &Path, diagnostics: &mut Vec<Diagnostic>) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for (number, line) in lines(bytes, path, diagnostics) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            groups.push(Group { name: name.to_owned(), line: number, entries: Vec::new() });
            continue;
        }
        let entry = line.split_once('=').map(|(key, value)| (key.trim_end(), value)).filter(|(key, _)| !key.is_empty());
        match (entry, groups.last_mut()) {
            (Some((key, value)), Some(group)) => {
                group.entries.push((key.to_owned(), value.trim_start().to_owned(), number));
            }
            (Some(_), None) => warn(diagnostics, path, number, "an entry before any [Group] header; it is skipped"),
            (None, _) => {
                warn(diagnostics, path, number, "the line is neither a [Group] header nor key=value; it is skipped");
            }
        }
    }
    groups
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
