//! An application's own data file: a TOML document with a declared shape, which can hold arrays
//! of tables and is never repaired behind the application's back.
//!
//! ```toml
//! id = "api"
//! name = "Payments API"
//! created = "2026-09-18"
//!
//! [[profile]]
//! name = "review"
//! added = "2026-09-18"
//!
//! [[profile]]
//! name = "nightly"
//! ```
//!
//! This is not [`Settings`](crate::storage::Settings). Settings are the user's preferences: they
//! fall back to defaults, they self-heal, and a value they cannot store is dropped. A document is
//! the application's own file — a project, a profile, a record, a list of sources. It has no
//! defaults, it holds tables and arrays of tables, and **nothing is ever written back**: reading
//! a broken document reports it, and no `.bak` file is left behind either. Saving stays the
//! application's own step, with [`atomic_write`](crate::storage::atomic_write).
//!
//! A [`Shape`] declares what the document holds. Reading gives a [`Document`]: the part that
//! could be read, and a located [`Diagnostic`] for everything that could not.
//!
//! - A key of the declared type is read.
//! - A key the shape does not declare is a warning and is skipped.
//! - A value of another type is skipped: an error when the key is required, a warning when it is
//!   optional.
//! - A required key that is not there at all is an error, reported where its table starts.
//! - A syntax error is an error, and the rest of the file is still read.
//!
//! Reading gives the valid part of a broken document rather than nothing, for the same reason
//! every other loader in the framework does: a file the user wrote by hand is usually wrong in
//! one place, and an application that can still name a project with one unreadable profile is
//! more use than one that opens nothing. The application decides what to do: the diagnostics say
//! what is wrong, and a required key that is missing reads as `None`.
//!
//! ```
//! use qframe::document::{Document, Shape, ValueKind};
//!
//! let profile = Shape::new().required("name", ValueKind::text()).optional("added", ValueKind::text());
//! let shape = Shape::new()
//!     .required("id", ValueKind::text())
//!     .optional("name", ValueKind::text())
//!     .entries("profile", profile);
//!
//! let text = "id = \"api\"\nname = \"Payments API\"\n\n[[profile]]\nname = \"review\"\n\n[[profile]]\n";
//! let document = Document::parse("project.qcode", text, &shape);
//! assert_eq!(document.root().text("id"), Some("api"));
//! let names: Vec<&str> = document.root().entries("profile").iter().filter_map(|e| e.text("name")).collect();
//! assert_eq!(names, vec!["review"]);
//! assert_eq!(
//!     document.diagnostics()[0].to_string(),
//!     "project.qcode:7:1: error: `profile[1].name` is required and missing"
//! );
//! ```

mod read;
mod shape;

pub use shape::{Shape, ValueKind};

use std::fs;
use std::io;
use std::path::Path;

use crate::diagnostics::{Diagnostic, Location};
use crate::doc::Doc;

/// A value a document holds, in the type its [`Shape`] declared.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Stored {
    Text(String),
    Integer(i64),
    Flag(bool),
}

/// One table of a document, read against its [`Shape`]: the values it holds, the tables inside
/// it and its arrays of tables.
///
/// Every reader answers `None` (or no entries) for a key that was missing or unreadable, so a
/// broken document is read exactly as far as it is readable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    /// Each value in file order, with the place its key and the value itself were written.
    values: Vec<(String, Stored, Location, Location)>,
    tables: Vec<(String, Table)>,
    arrays: Vec<(String, Vec<Table>)>,
}

impl Table {
    /// The text under `key`, declared with [`ValueKind::text`] or [`ValueKind::choice`].
    #[must_use]
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.stored(key) {
            Some(Stored::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// The number under `key`, declared with [`ValueKind::integer`].
    #[must_use]
    pub fn integer(&self, key: &str) -> Option<i64> {
        match self.stored(key) {
            Some(Stored::Integer(number)) => Some(*number),
            _ => None,
        }
    }

    /// The `true` or `false` under `key`, declared with [`ValueKind::flag`].
    #[must_use]
    pub fn flag(&self, key: &str) -> Option<bool> {
        match self.stored(key) {
            Some(Stored::Flag(flag)) => Some(*flag),
            _ => None,
        }
    }

    /// The table `key` holds, declared with [`Shape::table`].
    #[must_use]
    pub fn table(&self, key: &str) -> Option<&Table> {
        self.tables.iter().find(|(name, _)| name == key).map(|(_, table)| table)
    }

    /// The entries `key` holds, declared with [`Shape::entries`]; empty when the document lists
    /// none. The order is the file's own.
    #[must_use]
    pub fn entries(&self, key: &str) -> &[Table] {
        self.arrays.iter().find(|(name, _)| name == key).map_or(&[], |(_, entries)| entries.as_slice())
    }

    /// Where `key` was written, so an application can report a problem only it can see — a name
    /// no file system accepts, a date the calendar does not have — at the place the user wrote.
    #[must_use]
    pub fn location(&self, key: &str) -> Option<&Location> {
        self.values.iter().find(|(name, _, _, _)| name == key).map(|(_, _, at, _)| at)
    }

    /// Where the value of `key` was written: `8` in `mode = "halb"` is the column of `"halb"`,
    /// not of `mode`. This is the place the document's own diagnostics point at when a value
    /// has the wrong type or is not one of its choices, so an application that reports its own
    /// findings here lands on the same column. `None` when the key was missing or unreadable.
    #[must_use]
    pub fn value_location(&self, key: &str) -> Option<&Location> {
        self.values.iter().find(|(name, _, _, _)| name == key).map(|(_, _, _, at)| at)
    }

    fn stored(&self, key: &str) -> Option<&Stored> {
        self.values.iter().find(|(name, _, _, _)| name == key).map(|(_, value, _, _)| value)
    }
}

/// A document read against a [`Shape`]: the part that could be read, and a diagnostic for
/// everything that could not.
///
/// Reading never writes, never repairs and never panics. See the [module
/// documentation](self) for what each kind of problem becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    root: Table,
    diagnostics: Vec<Diagnostic>,
}

impl Document {
    /// Reads TOML `text` as the document `shape` describes, reporting problems against `file`.
    #[must_use]
    pub fn parse(file: &str, text: &str, shape: &Shape) -> Self {
        let doc = Doc::new(file, text);
        let (root, mut diagnostics) = doc.parse_recoverable();
        // A key missing from the whole document is reported at its first character.
        let start = doc.locate(&(0..0));
        let root = read::table(&doc, &root, shape, "", &start, &mut diagnostics);
        Self { root, diagnostics }
    }

    /// Reads the document at `path`, reporting problems against the file's name.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file cannot be read. A file that is not there is one of
    /// those errors, not an empty document: a data file the application has not written yet and
    /// one it wrote empty mean different things, and only the application knows which it expects.
    pub fn open(path: impl AsRef<Path>, shape: &Shape) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        let name = path.file_name().and_then(|name| name.to_str());
        Ok(match name {
            Some(name) => Self::parse(name, &text, shape),
            None => Self::parse(&path.display().to_string(), &text, shape),
        })
    }

    /// The document's root table.
    #[must_use]
    pub fn root(&self) -> &Table {
        &self.root
    }

    /// Every problem found while reading, in the order they were found.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether nothing at all was wrong with the document.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[cfg(test)]
mod tests;
