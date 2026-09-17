//! What a [`Table`](super::Table) shows: its columns, rows and cells.

use crate::text;
use crate::widget::Align;

/// How wide a [`Column`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnWidth {
    /// Exactly this many cells.
    Fixed(u16),
    /// As wide as its widest cell or its title.
    Fit,
    /// A share of the room left by the other columns, by weight.
    Fill(u16),
}

/// Which way a sorted column is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Smallest first.
    Ascending,
    /// Largest first.
    Descending,
}

impl SortDirection {
    /// The other direction.
    #[must_use]
    pub fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// A column of a [`Table`](super::Table): a title, a width rule, an alignment and whether it can be sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub(super) title: String,
    pub(super) width: ColumnWidth,
    pub(super) min: Option<u16>,
    pub(super) align: Align,
    pub(super) sortable: bool,
}

impl Column {
    /// A left-aligned column that shares the free room equally with other filling columns.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), width: ColumnWidth::Fill(1), min: None, align: Align::Start, sortable: false }
    }

    /// How wide the column is.
    #[must_use]
    pub fn width(mut self, width: ColumnWidth) -> Self {
        self.width = width;
        self
    }

    /// The fewest cells a fitting or filling column shrinks to. When the columns' minimums do
    /// not fit, the table scrolls sideways instead of shrinking further. Filling columns keep
    /// their title's width by default.
    #[must_use]
    pub fn min(mut self, cells: u16) -> Self {
        self.min = Some(cells);
        self
    }

    /// Where cell text sits; `Align::End` for numbers so their digits line up.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Lets the user sort by this column (needs [`Table::on_sort`](super::Table::on_sort)).
    #[must_use]
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub(super) fn title_width(&self) -> u16 {
        text::width(&self.title).saturating_add(if self.sortable { 2 } else { 0 })
    }
}

/// One cell: text, optionally with an icon and a colour.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableCell {
    pub(super) text: String,
    pub(super) icon: Option<String>,
    pub(super) icon_color: Option<String>,
    pub(super) color: Option<String>,
}

impl TableCell {
    /// A cell showing `text`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), ..Self::default() }
    }

    /// Icon key drawn before the text, optionally in theme colour `color`.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>, color: Option<&str>) -> Self {
        self.icon = Some(key.into());
        self.icon_color = color.map(str::to_owned);
        self
    }

    /// Draws the text in theme colour `token`, e.g. `"success"` next to a status icon.
    #[must_use]
    pub fn color(mut self, token: impl Into<String>) -> Self {
        self.color = Some(token.into());
        self
    }

    pub(super) fn width(&self) -> u16 {
        text::width(&self.text).saturating_add(self.icon.as_ref().map_or(0, |_| 2))
    }
}

impl From<&str> for TableCell {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for TableCell {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

/// One row of a [`Table`](super::Table).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableRow {
    pub(super) cells: Vec<TableCell>,
    pub(super) faint: bool,
}

impl TableRow {
    /// A row of `cells`, one per column.
    #[must_use]
    pub fn new(cells: impl IntoIterator<Item = impl Into<TableCell>>) -> Self {
        Self { cells: cells.into_iter().map(Into::into).collect(), faint: false }
    }

    /// Draws the row faint while keeping it selectable.
    #[must_use]
    pub fn faint(mut self, faint: bool) -> Self {
        self.faint = faint;
        self
    }
}
