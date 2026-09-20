//! A file manager: a folder shown as a tree, with the operations a person expects on it.

mod details;
mod flat;
mod mark;
mod ops;
mod state;
mod trash;
mod watch;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::rc::Rc;

use crate::widget::{Length, NodeMut, View};

use super::{Button, ContextItem, Field, Form, FormErrors, Modal, ProgressBar, Text, TextInput, Tree, TreeNode};

pub use details::FileDetails;
pub use flat::FileView;
pub use mark::RowMark;
pub use ops::copy_into;
use ops::stem;
pub use ops::{FileChange, FileError, NameProblem, is_inside, is_within, name_of, parent_key};
use state::ROOT;
pub use state::{FileManagerMsg, FileManagerState, FileWork, FolderEntry, NameFor, Naming, child_key};

/// Turns a manager's messages into the application's own, on the drawing side.
type Wrap<Msg> = Rc<dyn Fn(FileManagerMsg) -> Msg>;

/// What the application adds to the menu of the row `key`, which acts on `targets`.
type Menu<Msg> = Rc<dyn Fn(&str, &[String]) -> Vec<ContextItem<Msg>>>;

/// What a row's path becomes for the application.
type OnPath<Msg> = Rc<dyn Fn(&Path) -> Msg>;

/// What the application says about the look of the row `key`.
type Marks = Rc<dyn Fn(&str) -> RowMark>;

/// The name the field of the naming dialog is focused by.
const NAME_ID: &str = "file-manager-name";

/// Width of the naming dialog, in cells: room for a long file name without covering the screen.
const NAMING_WIDTH: u16 = 48;

/// A folder as a tree, with every file operation on it.
///
/// The manager is a **file view**, not an application: it reads the folder, draws it, does the
/// file operations and says what happened. What opening a file means is always the application's:
/// [`on_open`](Self::on_open) says a path was asked to be opened and nothing more.
///
/// The application owns a [`FileManagerState`], hands it every [`FileManagerMsg`] and draws it
/// here. Folders are read on a background thread, never while drawing; a read that takes longer
/// than about 300 ms shows a small spinner on the folder's own row, which then stays about 500 ms,
/// so quick reads never flash one.
///
/// What it does: opening and closing folders, one and several selections, the keyboard's own way
/// through the rows, dragging entries onto a folder to move them, cut and paste, a new file or
/// folder, renaming with the name checked as it is typed, and deleting behind a question. Each
/// operation says what it changed or why it was refused, entry by entry when there were several.
///
/// What it draws: the root as the top row, so the folder itself has a place for its menu; folders
/// then files, each in name order; an entry whose name the platform does not spell as text shown
/// lossily rather than left out; what was cut faint until it is pasted or let go.
///
/// See [`FileManagerState`] for a whole application, and
/// [`FileManagerState::confined`](FileManagerState::confined) for keeping operations inside the
/// root.
///
/// Keys: the tree's own (↑/↓ between rows, ←/→ and Enter to open and close a folder, Enter on a
/// file to open it, Space and Ctrl to select several, Home and End, the menu key on the row the
/// cursor is on).
///
/// Style keys: the tree's (`list-item`, `tree-chevron`, `tree-drop`, `list-detail`, `spinner`),
/// the context menu's and the dialog's. Texts: `quvyta.file-manager.*`.
pub struct FileManager<'a, Msg> {
    state: &'a FileManagerState,
    wrap: Wrap<Msg>,
    root_label: Option<String>,
    on_open: Option<OnPath<Msg>>,
    on_open_terminal: Option<OnPath<Msg>>,
    menu: Option<Menu<Msg>>,
    marks: Option<Marks>,
    view: FileView,
    disabled: bool,
}

impl<'a, Msg: Clone + 'static> FileManager<'a, Msg> {
    /// A manager showing `state`; `wrap` turns the manager's messages into the application's.
    ///
    /// `wrap` is a function such as `Msg::Files`, or a closure that captures what it needs, such
    /// as a screen's own conversion: `move |message| convert(screen::Msg::Files(message))`.
    #[must_use]
    pub fn new(state: &'a FileManagerState, wrap: impl Fn(FileManagerMsg) -> Msg + 'static) -> Self {
        Self {
            state,
            wrap: Rc::new(wrap),
            root_label: None,
            on_open: None,
            on_open_terminal: None,
            menu: None,
            marks: None,
            view: FileView::Tree,
            disabled: false,
        }
    }

    /// What the top row says. The name of the root folder by default; an application with a name
    /// of its own for it, such as a project's, gives that instead.
    #[must_use]
    pub fn root_label(mut self, label: impl Into<String>) -> Self {
        self.root_label = Some(label.into());
        self
    }

    /// A file was asked to be opened: a click or Enter on its row.
    ///
    /// The manager has no viewer, tab or window of its own; one application opens the path in a
    /// tab, another in a window, and a dialog returns it as the answer. Without this a click on a
    /// file only selects it.
    #[must_use]
    pub fn on_open(mut self, message: impl Fn(&Path) -> Msg + 'static) -> Self {
        self.on_open = Some(Rc::new(message));
        self
    }

    /// Offers "Open a terminal here" on a folder's menu, with the folder's path.
    ///
    /// The wording is the framework's, so every application says it the same way; what a terminal
    /// is stays the application's own.
    #[must_use]
    pub fn on_open_terminal(mut self, message: impl Fn(&Path) -> Msg + 'static) -> Self {
        self.on_open_terminal = Some(Rc::new(message));
        self
    }

    /// The application's own items on a row's menu, in a group of their own between the manager's
    /// editing items and its last, destructive one.
    ///
    /// The row's key comes first and what an action there acts on second: the whole selection when
    /// the row is one of several selected, the row alone otherwise, as
    /// [`FileManagerState::targets`] works it out.
    #[must_use]
    pub fn menu_items(mut self, items: impl Fn(&str, &[String]) -> Vec<ContextItem<Msg>> + 'static) -> Self {
        self.menu = Some(Rc::new(items));
        self
    }

    /// What the application says about the look of a row, by key: a sign in a tone, a faint row,
    /// or both. See [`RowMark`].
    ///
    /// The manager knows names and folders; what an entry means to the application it cannot know.
    /// qcode marks an entry its backup leaves out with a warning sign and draws the row faint; a
    /// version control panel marks what is ignored. Return [`RowMark::new()`] for a row with
    /// nothing to say, which is every row by default.
    ///
    /// A mark cannot make a row louder than the manager's own states: a cut entry and a disabled
    /// manager stay faint whatever the mark says, because they are about what can be done rather
    /// than about what the entry is.
    #[must_use]
    pub fn row_mark(mut self, mark: impl Fn(&str) -> RowMark + 'static) -> Self {
        self.marks = Some(Rc::new(mark));
        self
    }

    /// The shape the folder is drawn in: the tree it is without being asked, a list of rows with
    /// their size, date and permissions, or a grid of icons.
    ///
    /// The tree shows folders inside folders, opened where they stand. The other two show one
    /// folder at a time: its own row comes first, so the folder has a place for its menu and a way
    /// back out of it, and stepping into a folder shows that folder instead. Which folder is shown
    /// is [`FileManagerState::folder`], and the keys, the menus and every operation are the same
    /// in all three.
    ///
    /// The list reads the size, the date and the permissions of a page of entries around the
    /// cursor, never of a whole folder; the tree and the icons read none.
    #[must_use]
    pub fn view(mut self, view: FileView) -> Self {
        self.view = view;
        self
    }

    /// Draws the rows faint and answers nothing: no click, key, drag or menu, while the
    /// application has taken the folder away from the person.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Adds the manager to `ui` and answers with its tree, to be given a size and a name.
    ///
    /// The dialog that asks for a name is added too while one is asked for; it is a layer and
    /// takes no room of its own.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let state = self.state;
        if let Some(problem) = state.error() {
            ui.add(Text::new(crate::t!("quvyta.file-manager.unreadable")).role("secondary"));
            return ui.add(Text::new(problem.to_owned()).role("faint")).selectable(true);
        }
        self.naming_dialog(ui);
        self.work_row(ui);
        if self.view == FileView::Tree {
            let tree = self.tree();
            return ui.add(tree);
        }
        // The rows and the foot under them are one thing to place, so they are given a column of
        // their own and the application sizes that.
        let rows = self.flat_rows();
        // The list shows details, so it asks for the page around the cursor it has none of yet;
        // the tree and the icons show names alone and ask for nothing, which is what keeps a
        // folder of ten thousand entries from becoming ten thousand calls to the system.
        if self.view == FileView::List {
            let gaps = state.detail_gaps(state.folder());
            if !gaps.is_empty() {
                let wrap = Rc::clone(&self.wrap);
                ui.on_idle(std::time::Duration::ZERO, move |_| wrap(FileManagerMsg::Detail(gaps.clone())));
            }
        }
        ui.column(|ui| {
            if self.view == FileView::Icons {
                let grid = self.grid(&rows);
                ui.add(grid).fill();
            } else {
                let table = self.table(&rows);
                ui.add(table).fill();
            }
            self.foot(ui, &rows);
        })
    }

    /// The row above the rows while a long operation runs: what it is doing, how far it has come
    /// and a way to say stop.
    ///
    /// It sits above the tree rather than over it: the rows stay readable while a copy goes on, and
    /// the row goes away by itself when the work ends. It takes no room at all while nothing runs.
    fn work_row(&self, ui: &mut View<'_, Msg>) {
        let Some(work) = self.state.work() else { return };
        let label = crate::t!("quvyta.file-manager.copying", n = work.entries());
        let stop = (self.wrap)(FileManagerMsg::Stop);
        let note = work.note().to_owned();
        let done = work.done();
        ui.row(|ui| {
            ui.add(Text::new(label).role("secondary").no_wrap());
            ui.add(ProgressBar::new(done).percent(true)).width(Length::Fill(1));
            if !note.is_empty() {
                ui.add(Text::new(note).role("faint").no_wrap());
            }
            ui.add(Button::new(crate::t!("quvyta.file-manager.stop")).on_press(stop));
        })
        .fill_width();
    }

    /// The tree of the whole manager, with the root as its one top row.
    fn tree(&self) -> Tree<Msg> {
        let state = self.state;
        let tree = Tree::new([self.root_node()]);
        if self.disabled {
            return tree;
        }
        let wrap = Rc::clone(&self.wrap);
        let expand = Rc::clone(&self.wrap);
        let choose = Rc::clone(&self.wrap);
        let drop = Rc::clone(&self.wrap);
        let accepts = state.folder_keys();
        let tree = tree
            .selected(state.selected())
            .on_select(move |key| wrap(FileManagerMsg::Select(key.to_owned())))
            .multi_select(state.chosen(), move |keys| choose(FileManagerMsg::Choose(keys)))
            .droppable(
                move |dropped| drop(FileManagerMsg::Drop(dropped)),
                move |key| key == ROOT || accepts.contains(key),
            )
            .on_expand(move |key, open| expand(FileManagerMsg::Expand(key.to_owned(), open)))
            .context_menu(self.menu_for());
        // Enter or a click on a file opens it; on a folder they open the folder, which the tree
        // does itself. Space and the modified clicks select instead.
        match &self.on_open {
            Some(open) => {
                let (open, root) = (Rc::clone(open), state.root().to_path_buf());
                tree.on_activate(move |key| open(&path_of(&root, key)))
            }
            None => tree,
        }
    }

    /// The root folder itself, as the one row at the top.
    ///
    /// It is a row rather than nothing so the folder has a place of its own: its menu makes entries
    /// and pastes at the top, reached by a right click or by selecting it and pressing the menu key
    /// like any other row. A tree only as tall as its rows has no empty part below them to click,
    /// so the row is the one way the mouse and the keyboard reach the folder alike.
    fn root_node(&self) -> TreeNode {
        let state = self.state;
        let label = self.root_label.clone().unwrap_or_else(|| root_name(state.root()));
        let mark = self.mark_of(ROOT);
        let (icon, tone) = self.sign_of(&mark, "folder");
        let mut root = TreeNode::new(ROOT, label).icon(icon, tone.as_deref()).faint(self.disabled || mark.is_faint());
        // An unread folder is not an empty one, so it never says "empty" before it is known.
        if state.shown_children(ROOT).is_some_and(|entries| entries.is_empty()) {
            root = root.detail(crate::t!("quvyta.file-manager.empty"));
        }
        root.expandable(true).expanded(state.is_open(ROOT)).loading(state.is_loading(ROOT)).children(self.nodes(ROOT))
    }

    /// The rows below the folder `key`, as far as it has been read.
    fn nodes(&self, key: &str) -> Vec<TreeNode> {
        let state = self.state;
        let Some(entries) = state.shown_children(key) else { return Vec::new() };
        entries
            .into_iter()
            .map(|entry| {
                let child = child_key(key, &entry.name);
                let mark = self.mark_of(&child);
                let (icon, tone) = self.sign_of(&mark, if entry.folder { "folder" } else { "file" });
                // What was cut is drawn faint until it is pasted or let go, with everything in it.
                let faint = self.disabled || state.is_cut(&child) || mark.is_faint();
                let mut node =
                    TreeNode::new(child.clone(), entry.name.clone()).icon(icon, tone.as_deref()).faint(faint);
                if entry.folder {
                    let open = state.is_open(&child);
                    node = node.expandable(true).expanded(open).loading(state.is_loading(&child));
                    if open {
                        node = node.children(self.nodes(&child));
                    }
                }
                node
            })
            .collect()
    }

    /// What the application says about the row `key`, nothing when it says nothing.
    fn mark_of(&self, key: &str) -> RowMark {
        self.marks.as_ref().map(|mark| mark(key)).unwrap_or_default()
    }

    /// The icon and the colour a row is drawn with: the mark's sign when it has one, and the
    /// manager's own folder or file icon in the row's own colour otherwise.
    fn sign_of(&self, mark: &RowMark, own: &'static str) -> (String, Option<String>) {
        match mark.icon() {
            Some(icon) => (icon.to_owned(), mark.tone().map(str::to_owned)),
            None => (own.to_owned(), None),
        }
    }

    /// What every row's menu holds.
    fn menu_for(&self) -> impl Fn(&str) -> Vec<ContextItem<Msg>> + 'static {
        let state = self.state;
        // The menu is built long after the view, so it takes what it needs along rather than the
        // state itself.
        let wrap = Rc::clone(&self.wrap);
        let chosen = state.chosen().to_vec();
        let pending = Pending { keys: state.pending().to_vec(), copying: state.is_copying() };
        let trashing = state.is_trashing();
        let folders = state.folder_keys();
        let extra = self.menu.clone();
        let terminal = self.on_open_terminal.clone();
        let root = state.root().to_path_buf();
        move |key: &str| {
            // The tree keeps the selection when the click is on one of its rows and makes the row
            // the selection otherwise, so the menu acts on what the click was on.
            let targets = state::targets_of(&chosen, key);
            let mut own = extra.as_ref().map(|items| items(key, &targets)).unwrap_or_default();
            if let Some(message) = &terminal
                && (key == ROOT || folders.contains(key))
            {
                let label = crate::t!("quvyta.file-manager.open-terminal");
                own.push(ContextItem::new(label, message(&path_of(&root, key))));
            }
            let send = |message: FileManagerMsg| wrap(message);
            if targets.len() > 1 {
                return many_menu(key, targets.len(), &pending, trashing, own, &send);
            }
            if key == ROOT || folders.contains(key) {
                return folder_menu(key, &pending, trashing, own, &send);
            }
            file_menu(key, &pending, trashing, own, &send)
        }
    }

    /// The dialog that asks for a name, while one is asked for. It waits for an answer rather than
    /// sitting beside the tree: the name is all there is to do until it is given or dropped.
    fn naming_dialog(&self, ui: &mut View<'_, Msg>) {
        let state = self.state;
        let Some(naming) = state.naming() else { return };
        let (title, confirm) = match &naming.purpose {
            NameFor::File => (crate::t!("quvyta.file-manager.new-file-title"), crate::t!("quvyta.file-manager.create")),
            NameFor::Folder => {
                (crate::t!("quvyta.file-manager.new-folder-title"), crate::t!("quvyta.file-manager.create"))
            }
            NameFor::Rename(key) => (
                crate::t!("quvyta.file-manager.rename-title", name = name_of(key)),
                crate::t!("quvyta.file-manager.rename-do"),
            ),
        };
        let close = (self.wrap)(FileManagerMsg::CloseNaming);
        let submit = (self.wrap)(FileManagerMsg::Submit);
        let dialog = Modal::new()
            .title(title)
            .width(NAMING_WIDTH)
            .on_close(close.clone())
            .action(Button::new(crate::t!("quvyta.file-manager.cancel")).on_press(close))
            .action(Button::new(confirm).variant("primary").on_press(submit.clone()));
        let mut errors = FormErrors::new();
        if let Some(problem) = state.naming_problem() {
            errors.set(NAME_ID, problem.message());
        }
        let value = naming.value.clone();
        // A rename selects the name without its extension, so typing gives a new name and keeps
        // the kind of file; a new entry starts empty and has nothing to select.
        let selection = match &naming.purpose {
            NameFor::Rename(key) => Some(stem(&value, state.is_folder(key))),
            NameFor::File | NameFor::Folder => None,
        };
        let typed = Rc::clone(&self.wrap);
        ui.add_with(dialog, |ui| {
            Form::new().show(ui, |fields| {
                let label = crate::t!("quvyta.file-manager.name-label");
                fields.field(Field::new(label).error(errors.get(NAME_ID)), |ui| {
                    let mut input = TextInput::new(value)
                        .invalid(errors.has(NAME_ID))
                        .on_change(move |value| typed(FileManagerMsg::Name(value)))
                        .on_submit(move |_| submit.clone());
                    if let Some(range) = selection {
                        input = input.select_on_focus(range);
                    }
                    ui.add(input).id(NAME_ID).fill_width();
                });
            });
        });
    }
}

/// The path of the entry `key` under `root`.
fn path_of(root: &Path, key: &str) -> std::path::PathBuf {
    key.split('/').filter(|part| !part.is_empty()).fold(root.to_path_buf(), |path, part| path.join(part))
}

/// What the top row says about the folder at `root`: its own name, or the whole path when it has
/// none, as the file system's own root has none.
fn root_name(root: &Path) -> String {
    root.file_name().map_or_else(|| root.display().to_string(), |name| name.to_string_lossy().into_owned())
}

/// Puts `own`, when there is any, into a menu as a group of its own.
fn add_own<Msg>(items: &mut Vec<ContextItem<Msg>>, own: Vec<ContextItem<Msg>>) {
    if !own.is_empty() {
        items.push(ContextItem::gap());
        items.extend(own);
    }
}

/// What waits to be pasted, and whether pasting it will copy it.
struct Pending {
    keys: Vec<String>,
    copying: bool,
}

impl Pending {
    /// Whether anything waits to be pasted.
    fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// What letting go of it is called: a move is cancelled, a copy is cancelled.
    fn drop_label(&self) -> String {
        let key = if self.copying { "quvyta.file-manager.drop-copy" } else { "quvyta.file-manager.drop-cut" };
        crate::t!(key)
    }
}

/// The items for what waits to be pasted: pasting it here, and letting it go. A folder cannot take
/// itself or a folder that holds it, so pasting there is shown but cannot be chosen.
fn paste_items<Msg: Clone + 'static>(
    items: &mut Vec<ContextItem<Msg>>,
    key: &str,
    pending: &Pending,
    send: &impl Fn(FileManagerMsg) -> Msg,
) {
    if pending.is_empty() {
        return;
    }
    let paste = ContextItem::new(crate::t!("quvyta.file-manager.paste"), send(FileManagerMsg::Paste(key.to_owned())));
    items.push(paste.disabled(pending.keys.iter().any(|waiting| is_within(key, waiting))));
    items.push(ContextItem::new(pending.drop_label(), send(FileManagerMsg::DropCut)));
}

/// The last item of a row's menu: the trash when the manager has one, and deleting for good
/// otherwise. Both are the destructive item, so both stand alone at the end in the danger colour.
fn away_item<Msg: Clone + 'static>(
    key: &str,
    count: usize,
    trashing: bool,
    send: &impl Fn(FileManagerMsg) -> Msg,
) -> ContextItem<Msg> {
    let many = count > 1;
    let label = match (trashing, many) {
        (true, false) => crate::t!("quvyta.file-manager.trash"),
        (true, true) => crate::t!("quvyta.file-manager.trash-many", n = count),
        (false, false) => crate::t!("quvyta.file-manager.delete"),
        (false, true) => crate::t!("quvyta.file-manager.delete-many", n = count),
    };
    let message = if trashing {
        send(FileManagerMsg::Trash(key.to_owned()))
    } else {
        send(FileManagerMsg::Delete(key.to_owned()))
    };
    ContextItem::new(label, message).danger(true)
}

/// The menu of a folder, or of the root itself when `key` is [`ROOT`]: what can be made in it and,
/// while something is cut, pasting it here.
fn folder_menu<Msg: Clone + 'static>(
    key: &str,
    pending: &Pending,
    trashing: bool,
    own: Vec<ContextItem<Msg>>,
    send: &impl Fn(FileManagerMsg) -> Msg,
) -> Vec<ContextItem<Msg>> {
    let mut items = vec![
        ContextItem::new(crate::t!("quvyta.file-manager.new-file"), send(FileManagerMsg::NewFile(key.to_owned()))),
        ContextItem::new(crate::t!("quvyta.file-manager.new-folder"), send(FileManagerMsg::NewFolder(key.to_owned()))),
    ];
    let root = key == ROOT;
    if !root {
        items.push(ContextItem::gap());
        items.push(ContextItem::new(
            crate::t!("quvyta.file-manager.rename"),
            send(FileManagerMsg::Rename(key.to_owned())),
        ));
        items.push(ContextItem::new(crate::t!("quvyta.file-manager.cut"), send(FileManagerMsg::Cut(key.to_owned()))));
        items.push(ContextItem::new(crate::t!("quvyta.file-manager.copy"), send(FileManagerMsg::Copy(key.to_owned()))));
    }
    paste_items(&mut items, key, pending, send);
    add_own(&mut items, own);
    items.push(ContextItem::gap());
    if root {
        items.push(ContextItem::new(crate::t!("quvyta.file-manager.refresh"), send(FileManagerMsg::Refresh)));
    } else {
        items.push(away_item(key, 1, trashing, send));
    }
    items
}

/// The menu of a file.
fn file_menu<Msg: Clone + 'static>(
    key: &str,
    pending: &Pending,
    trashing: bool,
    own: Vec<ContextItem<Msg>>,
    send: &impl Fn(FileManagerMsg) -> Msg,
) -> Vec<ContextItem<Msg>> {
    let mut items = vec![
        ContextItem::new(crate::t!("quvyta.file-manager.rename"), send(FileManagerMsg::Rename(key.to_owned()))),
        ContextItem::new(crate::t!("quvyta.file-manager.cut"), send(FileManagerMsg::Cut(key.to_owned()))),
        ContextItem::new(crate::t!("quvyta.file-manager.copy"), send(FileManagerMsg::Copy(key.to_owned()))),
    ];
    if !pending.is_empty() {
        items.push(ContextItem::new(pending.drop_label(), send(FileManagerMsg::DropCut)));
    }
    add_own(&mut items, own);
    items.push(ContextItem::gap());
    items.push(away_item(key, 1, trashing, send));
    items
}

/// The menu of a row that is one of `count` selected entries: what can be done to all of them at
/// once. A name is given to one entry at a time, so renaming is not offered.
fn many_menu<Msg: Clone + 'static>(
    key: &str,
    count: usize,
    pending: &Pending,
    trashing: bool,
    own: Vec<ContextItem<Msg>>,
    send: &impl Fn(FileManagerMsg) -> Msg,
) -> Vec<ContextItem<Msg>> {
    let mut items = vec![
        ContextItem::new(
            crate::t!("quvyta.file-manager.cut-many", n = count),
            send(FileManagerMsg::Cut(key.to_owned())),
        ),
        ContextItem::new(
            crate::t!("quvyta.file-manager.copy-many", n = count),
            send(FileManagerMsg::Copy(key.to_owned())),
        ),
    ];
    if !pending.is_empty() {
        items.push(ContextItem::new(pending.drop_label(), send(FileManagerMsg::DropCut)));
    }
    add_own(&mut items, own);
    items.push(ContextItem::gap());
    items.push(away_item(key, count, trashing, send));
    items
}
