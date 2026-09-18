//! The shape of a document: the keys one table holds, what each key may hold, the tables that
//! sit inside it and the keys that carry an array of tables.

/// What one key may hold, in the loader's own terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Any text.
    Text,
    /// Text from a fixed list.
    Choice(Vec<String>),
    /// A whole number.
    Integer,
    /// `true` or `false`.
    Flag,
}

impl Kind {
    /// The expectation in words, for diagnostics.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Text => "a string".to_owned(),
            Self::Choice(choices) => format!("one of {}", choices.join(", ")),
            Self::Integer => "a whole number".to_owned(),
            Self::Flag => "a boolean".to_owned(),
        }
    }
}

/// What one key of a [`Shape`] may hold, for [`Shape::required`] and [`Shape::optional`].
///
/// A plain value rather than one builder per kind (`required_text`, `optional_text`, …): the
/// kinds stay listed once, and any kind can be required or optional.
///
/// ```
/// use qframe::document::{Shape, ValueKind};
///
/// let shape = Shape::new()
///     .required("name", ValueKind::text())
///     .required("engine", ValueKind::choice(["podman", "docker"]))
///     .optional("retries", ValueKind::integer())
///     .optional("network", ValueKind::flag());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueKind(pub(crate) Kind);

impl ValueKind {
    /// Any text.
    #[must_use]
    pub fn text() -> Self {
        Self(Kind::Text)
    }

    /// One text of `choices`, e.g. the names an application knows.
    #[must_use]
    pub fn choice(choices: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(Kind::Choice(choices.into_iter().map(Into::into).collect()))
    }

    /// A whole number, in any base TOML writes.
    #[must_use]
    pub fn integer() -> Self {
        Self(Kind::Integer)
    }

    /// `true` or `false`.
    #[must_use]
    pub fn flag() -> Self {
        Self(Kind::Flag)
    }
}

/// One declared key of a [`Shape`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Key {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    /// Whether a document without the key is incomplete.
    pub(crate) required: bool,
}

/// The shape of one table of a document: which keys it holds, which tables sit inside it and
/// which of its keys carry an array of tables.
///
/// A [`Document`](super::Document) is read against the shape of its root table. Keys are single
/// names, never dotted paths: nesting is declared with [`Shape::table`], which reads both
/// `[mounts]` with `project` under it and the same key written as `mounts.project`.
///
/// ```
/// use qframe::document::{Document, Shape, ValueKind};
///
/// let profile = Shape::new().required("name", ValueKind::text()).optional("added", ValueKind::text());
/// let shape = Shape::new()
///     .required("id", ValueKind::text())
///     .optional("created", ValueKind::text())
///     .table("mounts", Shape::new().optional("assets", ValueKind::choice(["rw", "ro"])))
///     .entries("profile", profile);
///
/// let text = "id = \"api\"\n\n[mounts]\nassets = \"ro\"\n\n[[profile]]\nname = \"review\"\n";
/// let document = Document::parse("project.qcode", text, &shape);
/// assert!(document.is_clean());
/// assert_eq!(document.root().text("id"), Some("api"));
/// assert_eq!(document.root().table("mounts").and_then(|mounts| mounts.text("assets")), Some("ro"));
/// assert_eq!(document.root().entries("profile").len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    keys: Vec<Key>,
    /// Tables that sit inside this one, each with its own shape.
    tables: Vec<(String, Shape)>,
    /// Keys carrying an array of tables, with the shape of one entry.
    arrays: Vec<(String, Shape)>,
}

impl Shape {
    /// A table that holds nothing yet. Every key it may hold is declared on it.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A key the document must hold: a document without it, or with a value of another type, is
    /// reported as an error. Declaring a name again replaces what it declared before.
    #[must_use]
    pub fn required(self, key: &str, kind: ValueKind) -> Self {
        self.declare(key, kind, true)
    }

    /// A key the document may hold: it is read when it is there and of the declared type, a
    /// value of another type is a warning, and a missing key is not a problem at all.
    #[must_use]
    pub fn optional(self, key: &str, kind: ValueKind) -> Self {
        self.declare(key, kind, false)
    }

    /// A table inside this one, `[key]` with `shape` below it.
    ///
    /// The table itself is optional: a document without it is not a problem, and the keys of a
    /// missing table read as missing. Its required keys are required once the table is there.
    #[must_use]
    pub fn table(mut self, key: &str, shape: Shape) -> Self {
        self.forget(key);
        self.tables.push((key.to_owned(), shape));
        self
    }

    /// An array of tables, `[[key]]` repeated, each entry shaped by `shape`.
    ///
    /// The array itself is optional: a document that lists no entry simply has none. Each entry
    /// is checked on its own, and an entry that is missing a required key is reported where it
    /// starts, beside the entries that were read.
    #[must_use]
    pub fn entries(mut self, key: &str, shape: Shape) -> Self {
        self.forget(key);
        self.arrays.push((key.to_owned(), shape));
        self
    }

    fn declare(mut self, key: &str, kind: ValueKind, required: bool) -> Self {
        self.forget(key);
        self.keys.push(Key { name: key.to_owned(), kind: kind.0, required });
        self
    }

    /// Drops whatever `key` declared before, so one name means one thing.
    fn forget(&mut self, key: &str) {
        self.keys.retain(|declared| declared.name != key);
        self.tables.retain(|(name, _)| name != key);
        self.arrays.retain(|(name, _)| name != key);
    }

    /// The key declared as `name`, if the shape declares one.
    pub(crate) fn key(&self, name: &str) -> Option<&Key> {
        self.keys.iter().find(|key| key.name == name)
    }

    /// The shape of the table declared as `name`, if the shape holds one.
    pub(crate) fn inner(&self, name: &str) -> Option<&Shape> {
        self.tables.iter().find(|(key, _)| key == name).map(|(_, shape)| shape)
    }

    /// The shape of one entry of the array declared as `name`, if the shape holds one.
    pub(crate) fn entry(&self, name: &str) -> Option<&Shape> {
        self.arrays.iter().find(|(key, _)| key == name).map(|(_, shape)| shape)
    }

    /// The keys a document must hold, in the order they were declared.
    pub(crate) fn required_keys(&self) -> impl Iterator<Item = &str> {
        self.keys.iter().filter(|key| key.required).map(|key| key.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_describe_what_they_expect() {
        assert_eq!(ValueKind::text().0.describe(), "a string");
        assert_eq!(ValueKind::choice(["rw", "ro"]).0.describe(), "one of rw, ro");
        assert_eq!(ValueKind::integer().0.describe(), "a whole number");
        assert_eq!(ValueKind::flag().0.describe(), "a boolean");
    }

    #[test]
    fn one_name_means_one_thing() {
        let shape = Shape::new()
            .required("profile", ValueKind::text())
            .table("profile", Shape::new())
            .entries("profile", Shape::new().required("name", ValueKind::text()));
        assert!(shape.key("profile").is_none() && shape.inner("profile").is_none());
        assert_eq!(shape.entry("profile").map(|entry| entry.required_keys().count()), Some(1));

        let shape = shape.optional("profile", ValueKind::integer());
        assert!(shape.entry("profile").is_none(), "the array is gone once the name is a key");
        assert_eq!(shape.key("profile").map(|key| key.required), Some(false));
        assert_eq!(shape.required_keys().count(), 0);
    }

    #[test]
    fn declaring_a_key_again_replaces_it() {
        let shape = Shape::new().optional("id", ValueKind::text()).required("id", ValueKind::text());
        assert_eq!(shape.keys.len(), 1);
        assert_eq!(shape.required_keys().collect::<Vec<_>>(), vec!["id"]);
        assert_eq!(shape, Shape::new().required("id", ValueKind::text()));
    }
}
