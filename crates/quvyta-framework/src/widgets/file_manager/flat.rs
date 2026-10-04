//! The flat views of a file manager: one folder at a time, drawn as a list of rows with their
//! details or as a grid of icons.
//!
//! A tree shows folders inside folders; a flat view shows one folder, and stepping into a folder
//! or out of it is how it is moved through. Which folder that is belongs to the state, because
//! reading a folder is the state's work, while which shape it is drawn in belongs to the view.
//!
//! Both shapes are built from the same rows: the folder itself first, so it has a place of its own
//! for its menu and a way out of it, and then its entries. A frame costs what the rows on screen
//! cost, whatever the folder holds: the entries are listed once each time the folder changes and
//! kept in the state, and a row's icon, colour and details are worked out only when [`Table`] or
//! [`CardGrid`] draws it.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{Align, Length, MeasureCx, PaintCx, View, Widget};

use super::super::delayed::DelayedIndicator;
use super::super::{
    CardGrid, Column, ColumnWidth, ContextItem, IconTile, RowDrop, SortDirection, SpinnerStyle, Table, TableCell,
    TableRow, TextInput, TreeDrop,
};
use super::details::FileDetails;
use super::kinds::KindLook;
use super::sort::{Ranked, Sort, SortBy};
use super::state::{Place, ROOT};
use super::{FileManager, FileManagerMsg, FileManagerState, Marks, OnPath, child_key, is_within, root_name};

/// Cells the date column takes: `2026-09-20 14:32` and no more.
const DATE_WIDTH: u16 = 16;

/// Cells the size column takes: `12.3 MB`, with room for a longer unit in another language. A fixed width, because the rows are
/// drawn as they come on screen and a column that fitted them would change width while scrolling.
const SIZE_WIDTH: u16 = 9;

/// Cells the permissions column takes: the ten letters unix writes them with.
const PERMISSIONS_WIDTH: u16 = 11;

/// The fewest cells the name column shrinks to before the columns scroll sideways.
const NAME_WIDTH: u16 = 12;

/// The fewest cells the column of where a trashed entry came from shrinks to: a path is long, and
/// one worth reading in full is worth more than the cells beside it.
const FROM_WIDTH: u16 = 24;

/// The shape a file manager draws its folder in.
///
/// The tree is what a manager is without being asked anything; the other two show one folder at a
/// time and are turned on with [`FileManager::view`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FileView {
    /// Folders inside folders, opened and closed where they stand. The default.
    #[default]
    Tree,
    /// One folder as rows: the name, the size, when it changed last and its permissions.
    List,
    /// One folder as cards, each an icon and a name.
    Icons,
}

/// One entry of the shown place, as the flat views list it.
#[derive(Debug)]
pub(super) struct FlatRow {
    /// The key it acts on.
    pub(super) key: String,
    /// What it says.
    pub(super) name: String,
    /// Whether it can be stepped into.
    pub(super) folder: bool,
    /// Whether it is a file that may be run.
    pub(super) executable: bool,
    /// When the entry changed in the place that shows it, where the place says so itself: the
    /// trash knows when an entry went and not when it last changed on disk.
    pub(super) changed: Option<i64>,
    /// Where the entry came from, as a note of the trash says; empty in a folder, where the
    /// columns beside the name are read from the entry itself.
    pub(super) from: String,
    /// When the entry went, as a note of the trash writes it; empty in a folder.
    pub(super) deleted: String,
}

impl FlatRow {
    /// The entry as an order sees it, with what is known about it besides its name.
    fn ranked<'a>(&'a self, details: &'a BTreeMap<String, Option<FileDetails>>) -> Ranked<'a> {
        Ranked {
            name: &self.name,
            folder: self.folder,
            executable: self.executable,
            details: details.get(&self.key).and_then(Option::as_ref),
            changed: self.changed,
        }
    }
}

/// The entries of the shown place in the order the flat views list them, with where each key
/// stands. The state keeps it while the place, its entries and whether hidden entries show stay
/// the same, so a frame of a large folder reads it rather than makes it.
#[derive(Debug)]
pub(super) struct FlatEntries {
    /// The state's revision it was made at.
    pub(super) revision: u64,
    /// Which of the two places it lists.
    pub(super) place: Place,
    /// The folder it lists, which is the root's own row in the trash.
    pub(super) folder: String,
    /// Whether hidden entries were shown.
    pub(super) hidden: bool,
    /// The state's count of what is known about the entries when it was made, for an order that
    /// goes by it.
    pub(super) details_revision: u64,
    /// The order the entries are in.
    pub(super) sort: Sort,
    /// The text the entries were narrowed to.
    pub(super) filter: Option<String>,
    /// Whether the place has been read at all; an unread one lists nothing but is not empty.
    pub(super) known: bool,
    pub(super) rows: Vec<FlatRow>,
    /// The place of every key in `rows`.
    index: HashMap<String, usize>,
}

impl FlatEntries {
    /// The entries of the place `state` shows.
    pub(super) fn of(state: &FileManagerState, revision: u64, details_revision: u64) -> Self {
        #[cfg(test)]
        ENTRIES_LISTED.with(|count| count.set(count.get() + 1));
        let place = state.place();
        let sort = state.sort();
        // The trash is a place beside the root rather than a folder in it, so its own row is the
        // root's and its entries are named by what they are called there.
        let folder = if place == Place::Trash { ROOT.to_owned() } else { state.folder().to_owned() };
        let (mut rows, known) = match place {
            Place::Files => {
                let entries = state.shown_children(&folder);
                let rows: Vec<FlatRow> = entries
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|entry| FlatRow {
                        key: child_key(&folder, &entry.name),
                        name: entry.name.clone(),
                        folder: entry.folder,
                        executable: entry.executable,
                        changed: None,
                        from: String::new(),
                        deleted: String::new(),
                    })
                    .collect();
                (rows, entries.is_some())
            }
            Place::Trash => {
                let rows = state
                    .trashed()
                    .into_iter()
                    .map(|entry| FlatRow {
                        key: entry.name.clone(),
                        name: entry.label.clone(),
                        folder: entry.folder,
                        executable: entry.executable,
                        changed: entry.deleted_at(),
                        from: entry.origin_text(),
                        deleted: entry.deleted_text(),
                    })
                    .collect();
                // A trash that could not be read is not an empty one, so it says why rather than
                // what it holds, and one nobody has put anything into is empty without a fault.
                (rows, !state.is_reading_trash() && state.trash_error().is_none())
            }
        };
        let filter = state.filter().map(str::to_owned);
        if let Some(text) = filter.as_deref().map(folded).filter(|text| !text.is_empty()) {
            rows.retain(|row| folded(&row.name).contains(&text));
        }
        if sort != Sort::default() {
            let details = state.details_shared();
            rows.sort_by(|a, b| super::sort::compare(sort, &a.ranked(&details), &b.ranked(&details)));
        }
        let index = rows.iter().enumerate().map(|(at, row)| (row.key.clone(), at)).collect();
        let hidden = state.shows_hidden();
        Self { revision, details_revision, sort, filter, place, folder, hidden, known, rows, index }
    }

    /// Where the entry `key` stands among the entries.
    pub(super) fn position(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }
}

/// A flat view's rows for one frame: the shown folder's own row first, then its entries. Shared by
/// every closure the widgets are wired with, so a frame copies none of them.
struct Flat<Msg> {
    entries: Arc<FlatEntries>,
    /// What the folder's own row says.
    label: String,
    /// The folder above the shown one, which the shown folder's own row stands for as a drop
    /// target; `None` at the root, which has nothing above it.
    up: Option<String>,
    wrap: Wrap<Msg>,
    on_open: Option<OnPath<Msg>>,
    root: PathBuf,
}

/// How a manager's messages become the application's, as the view holds it.
type Wrap<Msg> = Rc<dyn Fn(FileManagerMsg) -> Msg>;

impl<Msg> Flat<Msg> {
    /// The rows: the folder's own and one per entry.
    fn len(&self) -> usize {
        self.entries.rows.len() + 1
    }

    /// The entry row `index` stands for, or `None` for the place's own row.
    fn entry(&self, index: usize) -> Option<&FlatRow> {
        index.checked_sub(1).and_then(|at| self.entries.rows.get(at))
    }

    /// The key row `index` acts on.
    fn key(&self, index: usize) -> Option<&str> {
        if index == 0 { Some(&self.entries.folder) } else { self.entry(index).map(|row| row.key.as_str()) }
    }

    /// Moving the cursor to row `index`.
    fn select(&self, index: usize) -> Msg {
        (self.wrap)(FileManagerMsg::Select(self.key(index).unwrap_or_default().to_owned()))
    }

    /// What row `index` does when it is opened: the place's own row steps out of it, another
    /// folder is stepped into, and a file is the application's to open.
    fn activate(&self, index: usize) -> Msg {
        let Some(row) = self.entry(index) else { return (self.wrap)(FileManagerMsg::Leave) };
        // An entry of the trash is neither stepped into nor opened: what can be done to it is asked
        // of it by name, and only its row's menu knows that.
        if self.entries.place == Place::Trash {
            return (self.wrap)(FileManagerMsg::Select(row.key.clone()));
        }
        if row.folder {
            return (self.wrap)(FileManagerMsg::Enter(row.key.clone()));
        }
        match &self.on_open {
            Some(open) => {
                let path = row
                    .key
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .fold(self.root.clone(), |at, part| at.join(part));
                open(&path)
            }
            // Without a way to open files, a click on one only moves the cursor to it.
            None => (self.wrap)(FileManagerMsg::Select(row.key.clone())),
        }
    }

    /// The keys of the rows `indexes`, the selection a widget reports. The place's own row is not
    /// an entry, so it is never part of the selection, even when a box covers it.
    fn keys_of(&self, indexes: &[usize]) -> Vec<String> {
        indexes.iter().filter_map(|index| self.entry(*index)).map(|row| row.key.clone()).collect()
    }

    /// Whether row `index` takes a drop: a folder of the shown one, or the shown folder's own
    /// row, which is the way up and takes a drop for the folder above, as a desktop explorer's
    /// path takes one for a parent. At the root that row has nothing above it and takes nothing,
    /// and in the trash no row takes one at all: an entry in it is put back or deleted by name,
    /// and a drop would move whatever the root holds under that name.
    fn takes_drop(&self, index: usize) -> bool {
        if self.entries.place == Place::Trash {
            return false;
        }
        if index == 0 {
            return self.up.is_some();
        }
        self.entry(index).is_some_and(|row| row.folder)
    }

    /// The entries a widget's drop moves, and the folder they go into: the folder above for a
    /// drop on the shown folder's own row.
    fn dropped(&self, dropped: &RowDrop) -> TreeDrop {
        let into =
            if dropped.into == 0 { self.up.clone() } else { self.entry(dropped.into).map(|row| row.key.clone()) };
        TreeDrop { keys: self.keys_of(&dropped.rows), into }
    }
}

// Counts the rows a flat view works out a look for, and the times the entries are listed, so a
// test can tell that a frame's work follows the rows on screen rather than the folder.
#[cfg(test)]
thread_local! {
    pub(super) static ROWS_LOOKED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    pub(super) static ENTRIES_LISTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What a flat row is drawn with, taken along by the widgets so each row works out its own look
/// when it comes on screen.
struct Look {
    marks: Option<Marks>,
    kinds: KindLook,
    /// What waits to be moved, which draws faint; empty while what waits is to be copied.
    cut: Vec<String>,
    disabled: bool,
    /// The name of the root by itself, for the kind of its own row.
    root_name: String,
    /// Whether the trash is the place shown, whose own row is drawn as the trash, which is what it
    /// is, the way the tree draws it.
    trash: bool,
    details: Arc<BTreeMap<String, Option<FileDetails>>>,
}

impl Look {
    /// The icon and the colour the row of `key`, called `name`, is drawn with, and whether it is
    /// drawn faint. `itself` is the shown place's own row, which is never faint for being cut:
    /// it is where the person is.
    fn of(
        &self,
        key: &str,
        name: &str,
        folder: bool,
        executable: bool,
        itself: bool,
    ) -> (String, Option<String>, bool) {
        let mark = self.marks.as_ref().map(|mark| mark(key)).unwrap_or_default();
        // The kind goes by the entry's own name, not by a label the application gave the root.
        let name = if key == ROOT { if self.trash { "trash" } else { self.root_name.as_str() } } else { name };
        let (icon, tone) = match mark.icon() {
            Some(icon) => (icon.to_owned(), mark.tone().map(str::to_owned)),
            None => self.kinds.own_icon(key, name, folder, executable),
        };
        let cut = !itself && self.cut.iter().any(|cut| is_within(key, cut));
        (icon, tone, self.disabled || cut || mark.is_faint())
    }

    /// The look of row `index` of `flat`.
    fn row<Msg>(&self, flat: &Flat<Msg>, index: usize) -> (String, String, Option<String>, bool) {
        #[cfg(test)]
        ROWS_LOOKED.with(|count| count.set(count.get() + 1));
        let (key, name, folder, executable) = match flat.entry(index) {
            Some(row) => (row.key.as_str(), row.name.as_str(), row.folder, row.executable),
            None => (flat.entries.folder.as_str(), flat.label.as_str(), true, false),
        };
        // The kind goes by the name the row says: an entry of the trash is drawn by the name it
        // has where it came from, which is the one a person knows it by, and the number the trash
        // gave it says nothing of its kind.
        let (icon, tone, faint) = self.of(key, name, folder, executable, index == 0);
        (name.to_owned(), icon, tone, faint)
    }
}

impl<'a, Msg: Clone + 'static> FileManager<'a, Msg> {
    /// The rows a flat view shows this frame: the place itself, then the entries it has been
    /// read to hold.
    fn flat(&self) -> Rc<Flat<Msg>> {
        let state = self.state;
        let entries = state.flat_entries();
        // The trash's own row stands where the root's is and says what the trash is called, rather
        // than the name the person gave the root.
        let label = match entries.place {
            Place::Trash => crate::t!("quvyta.file-manager.trash-place"),
            Place::Files if entries.folder == ROOT => {
                self.root_label.clone().unwrap_or_else(|| root_name(state.root()))
            }
            Place::Files => super::name_of(&entries.folder).to_owned(),
        };
        // The place the trash goes back to is the root rather than the folder above it, and the
        // trash takes no drop, so it has no row above it.
        let up = (entries.place == Place::Files && entries.folder != ROOT)
            .then(|| super::parent_key(&entries.folder).to_owned());
        Rc::new(Flat {
            entries,
            label,
            up,
            wrap: Rc::clone(&self.wrap),
            on_open: self.on_open.clone(),
            root: state.root().to_path_buf(),
        })
    }

    /// What the rows of this frame are drawn with.
    fn look(&self) -> Rc<Look> {
        let state = self.state;
        let cut = if state.is_copying() { Vec::new() } else { state.pending().to_vec() };
        Rc::new(Look {
            marks: self.marks.clone(),
            kinds: self.kinds.clone(),
            cut,
            disabled: self.disabled,
            root_name: root_name(state.root()),
            trash: state.in_trash(),
            details: state.details_shared(),
        })
    }

    /// Which of a flat view's rows the cursor is on and which are selected, as the widgets count
    /// them.
    fn flat_selection(&self, flat: &Flat<Msg>) -> (Option<usize>, Vec<usize>) {
        let state = self.state;
        let place = |key: &str| {
            if key == flat.entries.folder { Some(0) } else { flat.entries.position(key).map(|at| at + 1) }
        };
        let selected = state.selected().and_then(place);
        let mut chosen: Vec<usize> = state.chosen().iter().filter_map(|key| place(key)).filter(|at| *at > 0).collect();
        chosen.sort_unstable();
        (selected, chosen)
    }

    /// The list view: a row per entry with its size, when it changed last and its permissions, or
    /// in the trash with where it came from and when it went.
    fn table(&self, flat: &Rc<Flat<Msg>>) -> Table<Msg> {
        let sortable = !self.disabled;
        let trash = flat.entries.place == Place::Trash;
        let name = Column::new(crate::t!("quvyta.file-manager.column-name")).min(NAME_WIDTH).sortable(sortable);
        let changed = Column::new(crate::t!("quvyta.file-manager.column-modified"))
            .width(ColumnWidth::Fixed(DATE_WIDTH))
            .sortable(sortable);
        // A note says two things about an entry of the trash: where it came from and when it went.
        // The changed column carries the second of them, and there is no size or permission to read
        // there, since the trash is the specification's folder and not one of the person's.
        let columns = if trash {
            vec![name, Column::new(crate::t!("quvyta.file-manager.column-from")).min(FROM_WIDTH), changed]
        } else {
            vec![
                name,
                Column::new(crate::t!("quvyta.file-manager.column-size"))
                    .width(ColumnWidth::Fixed(SIZE_WIDTH))
                    .align(Align::End)
                    .sortable(sortable),
                changed,
                Column::new(crate::t!("quvyta.file-manager.column-permissions"))
                    .width(ColumnWidth::Fixed(PERMISSIONS_WIDTH)),
            ]
        };
        let (rows, look) = (Rc::clone(flat), self.look());
        let cells = columns.len() - 1;
        let table = Table::lazy(columns, flat.len(), move |index| {
            let (name, icon, tone, faint) = look.row(&rows, index);
            let name = TableCell::new(name).icon(icon, tone.as_deref());
            // The place's own row says nothing about itself: it is the way in and out of the place
            // rather than an entry of the list.
            let cells = match rows.entry(index) {
                None => vec![TableCell::new(""); cells],
                Some(row) if trash => vec![TableCell::new(row.from.clone()), TableCell::new(row.deleted.clone())],
                Some(row) => {
                    let details = look.details.get(&row.key).and_then(Option::as_ref);
                    match details {
                        Some(details) => vec![
                            TableCell::new(details.size_text(row.folder)),
                            TableCell::new(details.modified_text()),
                            TableCell::new(details.permissions_text(row.folder)),
                        ],
                        None => vec![TableCell::new(""); 3],
                    }
                }
            };
            TableRow::new([name].into_iter().chain(cells)).faint(faint)
        });
        if self.disabled {
            return table;
        }
        let (selected, chosen) = self.flat_selection(flat);
        let [select, activate, choose, drop, copy, accepts] = std::array::from_fn(|_| Rc::clone(flat));
        let [choosing, dropping, copying, sorting] = std::array::from_fn(|_| Rc::clone(&self.wrap));
        let sort = flat.entries.sort;
        let table = match sort_column(sort, trash) {
            Some(column) => {
                table.sort(column, if sort.reverse { SortDirection::Descending } else { SortDirection::Ascending })
            }
            None => table,
        };
        table
            .on_sort(move |column, direction| {
                let by = sorted_by(column, trash);
                sorting(FileManagerMsg::Sort(Sort::by(by).reversed(direction == SortDirection::Descending)))
            })
            .selected(selected)
            .activate_on(self.open_on)
            .multi_select(&chosen, move |indexes| choosing(FileManagerMsg::Choose(choose.keys_of(&indexes))))
            .box_select(true)
            .droppable(
                move |dropped| dropping(FileManagerMsg::Drop(drop.dropped(&dropped))),
                move |index| accepts.takes_drop(index),
            )
            .on_copy_drop(move |dropped| copying(FileManagerMsg::DropCopy(copy.dropped(&dropped))))
            .on_select(move |index| select.select(index))
            .on_activate(move |index| activate.activate(index))
            .context_menu(self.flat_menu(flat))
    }

    /// The icon view: a card per entry, each an icon and a name.
    fn grid(&self, flat: &Rc<Flat<Msg>>) -> CardGrid<Msg> {
        let (rows, look) = (Rc::clone(flat), self.look());
        let (selected, chosen) = self.flat_selection(flat);
        let marked = chosen.clone();
        // Each entry is a desktop icon, the same tile a desktop draws its own with: the glyph over
        // the name, the chosen tiles raised with the pillar, and the cursor's pillar alone on a tile
        // that is not chosen while others are.
        let grid = CardGrid::new(flat.len())
            .card_width(IconTile::WIDTH, IconTile::WIDTH)
            .card_height(IconTile::HEIGHT)
            .gap(1, 1)
            .bare_cards(true)
            .disabled(self.disabled)
            .card(move |ui, index| {
                let (name, icon, tone, faint) = look.row(&rows, index);
                let cursor = selected == Some(index);
                let chosen = if marked.is_empty() { cursor } else { marked.contains(&index) };
                let mut tile = IconTile::new(icon, name).selected(chosen).cursor(cursor && !chosen).faint(faint);
                if let Some(tone) = tone {
                    tile = tile.color(tone);
                }
                ui.add(tile);
            });
        if self.disabled {
            return grid;
        }
        let [select, activate, choose, drop, copy, accepts] = std::array::from_fn(|_| Rc::clone(flat));
        let [choosing, dropping, copying] = std::array::from_fn(|_| Rc::clone(&self.wrap));
        grid.selected(selected)
            .activate_on(self.open_on)
            .multi_select(&chosen, move |indexes| choosing(FileManagerMsg::Choose(choose.keys_of(&indexes))))
            .box_select(true)
            .droppable(
                move |dropped| dropping(FileManagerMsg::Drop(drop.dropped(&dropped))),
                move |index| accepts.takes_drop(index),
            )
            .on_copy_drop(move |dropped| copying(FileManagerMsg::DropCopy(copy.dropped(&dropped))))
            .on_select(move |index| select.select(index))
            .on_activate(move |index| activate.activate(index))
            .context_menu(self.flat_menu(flat))
    }

    /// The menu of a flat row, which is the menu of the entry it stands for.
    fn flat_menu(&self, flat: &Rc<Flat<Msg>>) -> impl Fn(usize) -> Vec<ContextItem<Msg>> + 'static {
        let rows = Rc::clone(flat);
        let items = self.row_menu();
        // The way out is on the row of the place itself, where the way in was: up a folder out of
        // the one shown. The trash's own menu carries its own way out, since the place it goes back
        // to is the root and not the folder above.
        let leave = (!self.state.in_trash() && self.state.folder() != ROOT).then(|| (self.wrap)(FileManagerMsg::Leave));
        move |index| {
            let mut own = rows.key(index).map(&items).unwrap_or_default();
            if index == 0
                && let Some(leave) = &leave
            {
                own.insert(0, ContextItem::new(crate::t!("quvyta.file-manager.up"), leave.clone()));
                own.insert(1, ContextItem::gap());
            }
            own
        }
    }

    /// The rows of a flat view and the foot under them.
    pub(super) fn flat_view(&self, ui: &mut View<'_, Msg>, name: String) {
        let flat = self.flat();
        if let Some(text) = self.state.filter()
            && !self.disabled
        {
            // The field the place is narrowed with stands over the rows while a filter is set:
            // typing narrows, Enter puts the cursor on the first entry left, Esc shows all again.
            let wrap = Rc::clone(&self.wrap);
            let field = TextInput::new(text)
                .placeholder(crate::t!("quvyta.file-manager.filter"))
                .on_change(move |text| wrap(FileManagerMsg::Filter(Some(text))))
                .on_submit({
                    let wrap = Rc::clone(&self.wrap);
                    move |_| wrap(FileManagerMsg::FilterChosen)
                })
                .on_cancel((self.wrap)(FileManagerMsg::Filter(None)));
            ui.add(field).fill_width().id(format!("{name}-filter"));
        }
        if self.view == FileView::Icons {
            let grid = self.grid(&flat);
            ui.add(grid).fill().id(name);
        } else {
            let table = self.table(&flat);
            ui.add(table).fill().id(name);
        }
        self.foot(ui, &flat);
    }

    /// The foot of a flat view: the spinner of a slow read, and what the place holds otherwise.
    ///
    /// It keeps its row whether it shows the spinner or the count, so the rows above it never jump
    /// when a read takes long enough to be worth saying something about.
    fn foot(&self, ui: &mut View<'_, Msg>, flat: &Flat<Msg>) {
        let state = self.state;
        let trash = flat.entries.place == Place::Trash;
        let shown = flat.entries.folder.as_str();
        let entries = flat.entries.rows.len();
        // A place that could not be read is not an empty one, and the foot is the only place a
        // flat view has to say which of the two it is showing.
        let unreadable = state.folder_error(shown).is_some() || (trash && state.trash_error().is_some());
        let label = if !flat.entries.known {
            String::new()
        } else if unreadable {
            crate::t!("quvyta.file-manager.unreadable-short")
        } else if entries == 0 && flat.entries.filter.is_some() {
            crate::t!("quvyta.file-manager.no-match")
        } else if entries == 0 {
            crate::t!("quvyta.file-manager.empty")
        } else {
            crate::t!("quvyta.file-manager.entries", n = entries)
        };
        let busy = if trash { state.is_reading_trash() } else { state.is_loading(shown) };
        ui.add(Reading { busy, label }).fill_width().height(Length::Cells(1));
    }
}

/// `text` as a filter compares it: lowercase, with the Turkish dotted and dotless i read as `i`,
/// so a person typing either finds both.
fn folded(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ı' { 'i' } else { c })
        .filter(|c| *c != '\u{307}')
        .collect()
}

/// The list's column that shows the order `sort` goes by, which carries the order's arrow; an
/// order by kind has no column of its own, and the trash has no column to carry an order by size,
/// since it reads no size of its own.
fn sort_column(sort: Sort, trash: bool) -> Option<usize> {
    match sort.by {
        SortBy::Name => Some(0),
        SortBy::Size if !trash => Some(1),
        SortBy::Changed => Some(2),
        _ => None,
    }
}

/// What a press on the list's column `column` asks for, as the place shown has columns of its own:
/// where the size of a folder's entry stands the trash says where it came from, and nothing there
/// is put in order.
fn sorted_by(column: usize, trash: bool) -> SortBy {
    match (column, trash) {
        (2, _) => SortBy::Changed,
        (1, false) => SortBy::Size,
        _ => SortBy::Name,
    }
}

/// The one row under a flat view's rows: the spinner of a read that takes long enough to notice,
/// and what the folder holds the rest of the time.
struct Reading {
    busy: bool,
    label: String,
}

/// The delayed indicator of the read, kept in the row's memory.
#[derive(Debug, Default)]
struct Mark(DelayedIndicator);

impl<Msg: 'static> Widget<Msg> for Reading {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(available.width, 1)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let now = cx.now();
        let mut mark = cx.memory::<Mark>().0;
        let shown = mark.update(self.busy, now);
        if let Some(change) = mark.next_change(self.busy, now) {
            cx.request_frame_in(change);
        }
        cx.memory::<Mark>().0 = mark;
        let faint = cx.style("list-header", None, &[]).text();
        if !shown {
            cx.text(area.x, area.y, &self.label, faint, area.width);
            return;
        }
        let style = cx.style("spinner", None, &[]).text();
        let cell = cx.animation(SpinnerStyle::Dots.animation(), style, Some(Duration::ZERO));
        let glyph = text::truncate(&cell.glyph, 1).into_owned();
        cx.text(area.x, area.y, &glyph, cell.style, 1);
        let word = crate::t!("quvyta.file-manager.reading");
        cx.text(area.x + 2, area.y, &word, CellStyle { bg: None, ..faint }, area.width.saturating_sub(2));
    }
}
