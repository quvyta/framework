//! The flat views of a file manager: one folder at a time, drawn as a list of rows with their
//! details or as a grid of icons.
//!
//! A tree shows folders inside folders; a flat view shows one folder, and stepping into a folder
//! or out of it is how it is moved through. Which folder that is belongs to the state, because
//! reading a folder is the state's work, while which shape it is drawn in belongs to the view.
//!
//! Both shapes are built from the same rows: the folder itself first, so it has a place of its own
//! for its menu and a way out of it, and then its entries. Only the rows on screen are painted,
//! whatever the folder holds, which is what [`Table`] and [`CardGrid`] do by themselves.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{Align, Length, MeasureCx, PaintCx, View, Widget};

use super::super::delayed::DelayedIndicator;
use super::super::{CardGrid, Column, ColumnWidth, ContextItem, Span, SpinnerStyle, Table, TableCell, TableRow, Text};
use super::state::ROOT;
use super::{FileManager, FileManagerMsg, child_key, root_name};

/// Cells the date column takes: `2026-09-20 14:32` and no more.
const DATE_WIDTH: u16 = 16;

/// Cells the permissions column takes: the ten letters unix writes them with.
const PERMISSIONS_WIDTH: u16 = 11;

/// The fewest cells the name column shrinks to before the columns scroll sideways.
const NAME_WIDTH: u16 = 12;

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

/// What one card of the icon view shows: its name, its icon, the icon's tone and whether the card
/// is drawn faint.
type Card = (String, String, Option<String>, bool);

/// One row of a flat view: an entry of the shown folder, or the folder itself at the top.
pub(super) struct FlatRow {
    /// The key it acts on.
    pub(super) key: String,
    /// What it says.
    pub(super) name: String,
    /// Whether it can be stepped into.
    pub(super) folder: bool,
    /// Whether it is the row of the shown folder itself, which steps out of it.
    pub(super) itself: bool,
}

impl<'a, Msg: Clone + 'static> FileManager<'a, Msg> {
    /// The rows a flat view shows: the folder itself, then the entries it has been read to hold.
    pub(super) fn flat_rows(&self) -> Vec<FlatRow> {
        let state = self.state;
        let shown = state.folder();
        let label = if shown == ROOT {
            self.root_label.clone().unwrap_or_else(|| root_name(state.root()))
        } else {
            super::name_of(shown).to_owned()
        };
        let mut rows = vec![FlatRow { key: shown.to_owned(), name: label, folder: true, itself: true }];
        if let Some(entries) = state.shown_children(shown) {
            rows.extend(entries.into_iter().map(|entry| FlatRow {
                key: child_key(shown, &entry.name),
                name: entry.name.clone(),
                folder: entry.folder,
                itself: false,
            }));
        }
        rows
    }

    /// The icon and the colour a flat row is drawn with, and whether it is drawn faint.
    fn flat_look(&self, row: &FlatRow) -> (String, Option<String>, bool) {
        let mark = self.mark_of(&row.key);
        let own = if row.folder { "folder" } else { "file" };
        let (icon, tone) = self.sign_of(&mark, own);
        // The row of the folder itself is never faint for being cut: it is where the person is.
        let cut = !row.itself && self.state.is_cut(&row.key);
        (icon, tone, self.disabled || cut || mark.is_faint())
    }

    /// What a flat view's rows are selected and checked, as the widgets count them.
    fn flat_selection(&self, rows: &[FlatRow]) -> (Option<usize>, Vec<bool>) {
        let state = self.state;
        let selected = state.selected().and_then(|cursor| rows.iter().position(|row| row.key == cursor));
        let checked = rows.iter().map(|row| !row.itself && state.chosen().contains(&row.key)).collect();
        (selected, checked)
    }

    /// The message a flat row's activation sends: the folder row steps out of the folder, another
    /// folder is stepped into, and a file is the application's to open.
    fn flat_activate(&self, row: &FlatRow) -> Msg {
        if row.itself {
            return (self.wrap)(FileManagerMsg::Leave);
        }
        if row.folder {
            return (self.wrap)(FileManagerMsg::Enter(row.key.clone()));
        }
        match &self.on_open {
            Some(open) => open(&self.state.path(&row.key)),
            // Without a way to open files, a click on one only moves the cursor to it.
            None => (self.wrap)(FileManagerMsg::Select(row.key.clone())),
        }
    }

    /// The keys, the activations and the checks a flat view's widgets are wired with.
    fn flat_wiring(&self, rows: &[FlatRow]) -> Wiring<Msg> {
        Wiring {
            keys: rows.iter().map(|row| row.key.clone()).collect(),
            activations: rows.iter().map(|row| self.flat_activate(row)).collect(),
            chosen: self.state.chosen().to_vec(),
            itself: rows.iter().map(|row| row.itself).collect(),
        }
    }

    /// The list view: a row per entry with its size, when it changed last and its permissions.
    pub(super) fn table(&self, rows: &[FlatRow]) -> Table<Msg> {
        let state = self.state;
        let columns = vec![
            Column::new(crate::t!("quvyta.file-manager.column-name")).min(NAME_WIDTH),
            Column::new(crate::t!("quvyta.file-manager.column-size")).width(ColumnWidth::Fit).align(Align::End),
            Column::new(crate::t!("quvyta.file-manager.column-modified")).width(ColumnWidth::Fixed(DATE_WIDTH)),
            Column::new(crate::t!("quvyta.file-manager.column-permissions"))
                .width(ColumnWidth::Fixed(PERMISSIONS_WIDTH)),
        ];
        let table_rows: Vec<TableRow> = rows
            .iter()
            .map(|row| {
                let (icon, tone, faint) = self.flat_look(row);
                let name = TableCell::new(row.name.clone()).icon(icon, tone.as_deref());
                // The folder's own row says nothing about itself: its size is the folder's own,
                // which says nothing about what is in it, and the row is a way out rather than an
                // entry of the list.
                let details = if row.itself { None } else { state.details(&row.key).flatten() };
                let (size, modified, permissions) = match details {
                    Some(details) => {
                        (details.size_text(row.folder), details.modified_text(), details.permissions_text(row.folder))
                    }
                    None => (String::new(), String::new(), String::new()),
                };
                TableRow::new([name, TableCell::new(size), TableCell::new(modified), TableCell::new(permissions)])
                    .faint(faint)
            })
            .collect();
        let table = Table::new(columns, table_rows);
        if self.disabled {
            return table;
        }
        let (selected, checked) = self.flat_selection(rows);
        let wiring = self.flat_wiring(rows);
        let (select, activate, toggle) = (wiring.clone(), wiring.clone(), wiring.clone());
        let (wrap, choose) = (Rc::clone(&self.wrap), Rc::clone(&self.wrap));
        table
            .selected(selected)
            .checked(checked)
            .on_select(move |index| select.select(index, &wrap))
            .on_activate(move |index| activate.activate(index))
            .on_toggle(move |index| choose(FileManagerMsg::Choose(toggle.toggled(index))))
            .context_menu(self.flat_menu(rows))
    }

    /// The icon view: a card per entry, each an icon and a name.
    pub(super) fn grid(&self, rows: &[FlatRow]) -> CardGrid<Msg> {
        let cards: Arc<[Card]> = rows
            .iter()
            .map(|row| {
                let (icon, tone, faint) = self.flat_look(row);
                (row.name.clone(), icon, tone, faint)
            })
            .collect();
        let built = Arc::clone(&cards);
        let grid = CardGrid::new(rows.len()).card_width(18, 26).card_height(1).disabled(self.disabled).card(
            move |ui, index| {
                let Some((name, icon, tone, faint)) = built.get(index) else { return };
                let glyph = ui.env().icons().glyph(icon).into_owned();
                let mut mark = Span::new(format!("{glyph} "));
                if let Some(tone) = tone {
                    mark = mark.color(tone.as_str());
                }
                let mut label = Span::new(name.clone());
                if *faint {
                    (mark, label) = (mark.role("faint"), label.role("faint"));
                }
                ui.add(Text::rich([mark, label]).no_wrap());
            },
        );
        if self.disabled {
            return grid;
        }
        let (selected, checked) = self.flat_selection(rows);
        let wiring = self.flat_wiring(rows);
        let (select, activate, toggle) = (wiring.clone(), wiring.clone(), wiring.clone());
        let (wrap, choose) = (Rc::clone(&self.wrap), Rc::clone(&self.wrap));
        grid.selected(selected)
            .checked(checked)
            .on_select(move |index| select.select(index, &wrap))
            .on_activate(move |index| activate.activate(index))
            .on_toggle(move |index| choose(FileManagerMsg::Choose(toggle.toggled(index))))
            .context_menu(self.flat_menu(rows))
    }

    /// The menu of a flat row, which is the menu of the entry it stands for.
    fn flat_menu(&self, rows: &[FlatRow]) -> impl Fn(usize) -> Vec<ContextItem<Msg>> + 'static {
        let keys: Vec<String> = rows.iter().map(|row| row.key.clone()).collect();
        let items = self.menu_for();
        // The way out of the folder is on the folder's own row, where the way in was.
        let leave = (self.state.folder() != ROOT).then(|| (self.wrap)(FileManagerMsg::Leave));
        move |index| {
            let mut own = keys.get(index).map(|key| items(key)).unwrap_or_default();
            if index == 0
                && let Some(leave) = &leave
            {
                own.insert(0, ContextItem::new(crate::t!("quvyta.file-manager.up"), leave.clone()));
                own.insert(1, ContextItem::gap());
            }
            own
        }
    }

    /// The foot of a flat view: the spinner of a slow read, and what the folder holds otherwise.
    ///
    /// It keeps its row whether it shows the spinner or the count, so the rows above it never jump
    /// when a read takes long enough to be worth saying something about.
    pub(super) fn foot(&self, ui: &mut View<'_, Msg>, rows: &[FlatRow]) {
        let state = self.state;
        let shown = state.folder();
        let known = state.shown_children(shown);
        let label = match &known {
            None => String::new(),
            // A folder that could not be read is not an empty one, and the foot is the only place
            // a flat view has to say which of the two it is showing.
            Some(_) if state.folder_error(shown).is_some() => {
                crate::t!("quvyta.file-manager.unreadable-short")
            }
            Some(_) if rows.len() <= 1 => crate::t!("quvyta.file-manager.empty"),
            Some(_) => crate::t!("quvyta.file-manager.entries", n = rows.len() - 1),
        };
        ui.add(Reading { busy: state.is_loading(shown), label }).fill_width().height(Length::Cells(1));
    }
}

/// What a flat view's widgets need to turn a row number back into the entry it stands for.
struct Wiring<Msg> {
    keys: Vec<String>,
    activations: Vec<Msg>,
    chosen: Vec<String>,
    itself: Vec<bool>,
}

impl<Msg: Clone> Clone for Wiring<Msg> {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            activations: self.activations.clone(),
            chosen: self.chosen.clone(),
            itself: self.itself.clone(),
        }
    }
}

impl<Msg: Clone> Wiring<Msg> {
    /// Moving the cursor to row `index`.
    fn select(&self, index: usize, wrap: &Rc<dyn Fn(FileManagerMsg) -> Msg>) -> Msg {
        let key = self.keys.get(index).cloned().unwrap_or_default();
        wrap(FileManagerMsg::Select(key))
    }

    /// What row `index` does when it is opened.
    fn activate(&self, index: usize) -> Msg {
        self.activations
            .get(index)
            .cloned()
            .unwrap_or_else(|| self.activations.first().cloned().expect("a flat view always has the folder's own row"))
    }

    /// The selection after row `index` was checked or unchecked. The folder's own row is not an
    /// entry, so it is never part of the selection.
    fn toggled(&self, index: usize) -> Vec<String> {
        let mut chosen = self.chosen.clone();
        let Some(key) = self.keys.get(index).filter(|_| self.itself.get(index) == Some(&false)) else {
            return chosen;
        };
        match chosen.iter().position(|held| held == key) {
            Some(at) => {
                chosen.remove(at);
            }
            None => chosen.push(key.clone()),
        }
        chosen
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
