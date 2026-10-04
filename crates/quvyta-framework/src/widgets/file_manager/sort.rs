//! The order a file manager's flat views list a folder in.

use std::cmp::Ordering;

use crate::icons::file_kind;

use super::details::FileDetails;

/// What the entries of a folder are put in order by. Folders always come first, whatever it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum SortBy {
    /// By name, as a person reads them: the default.
    #[default]
    Name,
    /// By size, smallest first.
    Size,
    /// By when they changed last, oldest first.
    Changed,
    /// By kind, so the pictures stand together and the code together, then by name.
    Kind,
}

/// The order of a file manager's list and icons: what they are sorted by and whether it is turned
/// round. See [`FileManagerState::set_sort`](super::FileManagerState::set_sort).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Sort {
    /// What the entries are put in order by.
    pub by: SortBy,
    /// Whether the order is turned round: largest, newest or last first. Folders still come first.
    pub reverse: bool,
}

impl Sort {
    /// The entries in order of `by`, the smallest, oldest or first first.
    #[must_use]
    pub fn by(by: SortBy) -> Self {
        Self { by, reverse: false }
    }

    /// The same order turned round, or put back when `reverse` is `false`.
    #[must_use]
    pub fn reversed(self, reverse: bool) -> Self {
        Self { reverse, ..self }
    }

    /// Whether this order needs what is known about each entry besides its name, which is read in
    /// the background and may not be there yet.
    pub(super) fn needs_details(self) -> bool {
        matches!(self.by, SortBy::Size | SortBy::Changed)
    }
}

/// One entry as it is put in order.
pub(super) struct Ranked<'a> {
    pub(super) name: &'a str,
    pub(super) folder: bool,
    pub(super) executable: bool,
    pub(super) details: Option<&'a FileDetails>,
    /// When the entry changed where the place that shows it says so itself: the trash knows when
    /// an entry went and not when it last changed on disk. It wins over the date read from the
    /// entry, which is nothing in a place that is not below the root.
    pub(super) changed: Option<i64>,
}

/// How `a` and `b` stand in `sort`: folders first, then the order asked for, then the name, so two
/// entries the order cannot tell apart keep one place. An entry whose size or date is not known yet
/// stands after the known ones in either direction, and takes its place when it comes.
pub(super) fn compare(sort: Sort, a: &Ranked<'_>, b: &Ranked<'_>) -> Ordering {
    let by_name = || a.name.cmp(b.name);
    let turned = |order: Ordering| if sort.reverse { order.reverse() } else { order };
    let known = |order: Option<Ordering>, a_known: bool, b_known: bool| match (a_known, b_known) {
        (true, true) => turned(order.unwrap_or(Ordering::Equal)),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => Ordering::Equal,
    };
    let changed = |entry: &Ranked<'_>| entry.changed.or_else(|| entry.details.and_then(|details| details.modified));
    let order = match sort.by {
        SortBy::Name => turned(by_name()),
        SortBy::Size => {
            let (x, y) = (a.details.map(|d| d.size), b.details.map(|d| d.size));
            known(x.zip(y).map(|(x, y)| x.cmp(&y)), x.is_some(), y.is_some())
        }
        SortBy::Changed => {
            let (x, y) = (changed(a), changed(b));
            known(x.zip(y).map(|(x, y)| x.cmp(&y)), x.is_some(), y.is_some())
        }
        SortBy::Kind => {
            let kind = |entry: &Ranked<'_>| file_kind(entry.name, entry.folder, entry.executable).icon();
            turned(kind(a).cmp(kind(b)))
        }
    };
    b.folder.cmp(&a.folder).then(order).then_with(by_name)
}
