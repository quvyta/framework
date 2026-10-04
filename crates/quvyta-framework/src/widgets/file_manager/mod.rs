//! A file manager: a folder shown as a tree, with the operations a person expects on it.

mod details;
mod flat;
mod keys;
mod kinds;
mod mark;
mod ops;
mod sort;
mod state;
mod trash;
mod watch;

#[cfg(test)]
mod kinds_tests;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::rc::Rc;

use crate::icons::UserFolders;
use crate::widget::{Length, NodeMut, View};

use super::{Button, Click, ContextItem, Field, Form, FormErrors, Modal, ProgressBar, Text, TextInput, Tree, TreeNode};

pub use details::FileDetails;
pub use flat::FileView;
pub use mark::RowMark;
pub use ops::copy_into;
use ops::stem;
pub use ops::{FileChange, FileError, NameProblem, is_inside, is_within, name_of, parent_key};
pub use sort::{Sort, SortBy};
use state::ROOT;
pub use state::{FileManagerMsg, FileManagerState, FileWork, FolderEntry, NameFor, Naming, child_key};
pub use trash::Trashed;

/// Turns a manager's messages into the application's own, on the drawing side.
type Wrap<Msg> = Rc<dyn Fn(FileManagerMsg) -> Msg>;

/// What the application adds to the menu of the row `key`, which acts on `targets`.
type Menu<Msg> = Rc<dyn Fn(&MenuTarget<'_>) -> Vec<ContextItem<Msg>>>;

/// The row a menu was opened on, as [`FileManager::menu_for`] hands it to the application.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MenuTarget<'a> {
    /// The row's key, as [`FileManagerState`] names entries.
    pub key: &'a str,
    /// Where the entry is on disk.
    pub path: &'a Path,
    /// Whether the entry is a folder, the manager's own top row included.
    pub folder: bool,
    /// What an action from this menu acts on: the whole selection when the row is one of
    /// several selected, the row alone otherwise, as [`FileManagerState::targets`] works it out.
    pub selection: &'a [String],
}

/// What a row's path becomes for the application.
type OnPath<Msg> = Rc<dyn Fn(&Path) -> Msg>;

/// What the application says about the look of the row `key`.
type Marks = Rc<dyn Fn(&str) -> RowMark>;

/// The name the field of the naming dialog is focused by.
const NAME_ID: &str = "file-manager-name";

/// The name the rows carry when the application gives them none, so they stay one widget, and
/// keep the keyboard, when the view changes.
const ROWS_ID: &str = "file-manager-rows";

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
/// A manager with a trash can be sent to it, as a desktop's trash icon does: the trash is then
/// a place of its own, with every entry under the name it has where it came from, the folder it
/// came from and when it went, and its rows put an entry back, delete it for good or empty the
/// whole trash. See [`FileManagerState::open_trash`].
///
/// The mouse works as it does in a desktop file explorer, in all three views. A click only
/// selects; a double click or Enter opens: a file through [`on_open`](Self::on_open), a folder by
/// stepping into it in the list and the icons and by opening or closing it in the tree, where
/// its chevron and ← and → still do that with one click. Ctrl+click adds an entry to the
/// selection or takes it out, and Shift+click selects the entries from the last one clicked. A
/// drag from the free space draws a box, a tone over the cells it covers, and selects the
/// entries inside it, adding to the selection when Ctrl was held. A drag from a selected entry
/// carries the whole selection: released on a folder it moves there, or is copied there when
/// Ctrl is held at the release, and released anywhere else it does nothing. The folder under the
/// drag takes the accent tone while it can take what is dragged; a folder never takes itself or a
/// folder inside it. In the list and the icons the row of the shown folder is the way up, so a
/// drop on it goes into the folder above, as a desktop explorer's path takes a drop for a parent;
/// at the root it takes nothing. A terminal that does
/// not report Ctrl with the pointer always moves. A name already taken in the folder is never
/// overwritten; the entry says why it stayed. A right click on a selected entry opens the menu of
/// the selection, and on any other entry selects it and opens its menu.
/// [`open_on(Click::Single)`](Self::open_on) opens with one click instead.
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
/// file to open it, Home and End, the menu key on the row the cursor is on), and a desktop file
/// explorer's selection keys in all three views: Shift with the arrows, PgUp/PgDn, Home or End
/// extends the selection from where it started, Space adds or takes out the entry under the
/// cursor, Ctrl+A selects every entry shown and Esc leaves only the entry under the cursor
/// selected. Then Ctrl+X, Ctrl+C and Ctrl+V. Ctrl+X cuts the selection and
/// Ctrl+C copies it, the entry under the cursor when nothing is selected; Ctrl+V pastes what waits
/// into the folder the list and the icons show, and in the tree into the folder under the cursor,
/// or the folder holding the file under it. A name already taken there is refused and said, as a
/// paste from the menu is. The keys are the manager's only while its rows have focus and no text
/// is selected with the mouse: a text field keeps copying and pasting text, and selected text is
/// what Ctrl+C copies, see [`NodeMut::on_clipboard`](crate::widget::NodeMut::on_clipboard). Copy
/// and paste follow the keymap's `copy` and `paste`.
///
/// What it can add: each row's icon by the kind of the entry, see [`kind_icons`](Self::kind_icons),
/// and those icons in the colours of their families, see [`kind_tones`](Self::kind_tones).
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
    open_on: Click,
    disabled: bool,
    kind_icons: bool,
    kind_tones: bool,
    user_folders: Option<&'a UserFolders>,
    rows_id: Option<String>,
    /// How kinds are drawn on this screen, worked out when it is shown.
    kinds: kinds::KindLook,
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
            open_on: Click::Double,
            disabled: false,
            kind_icons: false,
            kind_tones: false,
            user_folders: None,
            rows_id: None,
            kinds: kinds::KindLook::default(),
        }
    }

    /// What the top row says. The name of the root folder by default; an application with a name
    /// of its own for it, such as a project's, gives that instead.
    #[must_use]
    pub fn root_label(mut self, label: impl Into<String>) -> Self {
        self.root_label = Some(label.into());
        self
    }

    /// A file was asked to be opened: a double click or Enter on its row, or a click with
    /// [`open_on(Click::Single)`](Self::open_on).
    ///
    /// The manager has no viewer, tab or window of its own; one application opens the path in a
    /// tab, another in a window, and a dialog returns it as the answer. Without this a double
    /// click on a file only selects it.
    #[must_use]
    pub fn on_open(mut self, message: impl Fn(&Path) -> Msg + 'static) -> Self {
        self.on_open = Some(Rc::new(message));
        self
    }

    /// How many clicks open an entry: [`Click::Double`], the default, the way a desktop file
    /// explorer opens, so a click is free to select or to start a drag; or [`Click::Single`], a
    /// click that selects and opens at once, for a picker whose rows are only ever opened.
    ///
    /// A double click is two presses on the same entry within [`Click::INTERVAL`]. Enter opens
    /// either way, and a folder's chevron in the tree opens and closes it with one click.
    #[must_use]
    pub fn open_on(mut self, click: Click) -> Self {
        self.open_on = click;
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
    ///
    /// [`menu_for`](Self::menu_for) is the same with the row's path and kind as well.
    #[must_use]
    pub fn menu_items(self, items: impl Fn(&str, &[String]) -> Vec<ContextItem<Msg>> + 'static) -> Self {
        self.menu_for(move |target| items(target.key, target.selection))
    }

    /// The application's own items on a row's menu, like [`menu_items`](Self::menu_items), told
    /// everything the manager knows of the row: its key, its path, whether it is a folder and what
    /// an action there acts on. An application offering "Open" for files and "Add to favourites"
    /// for folders needs no list of folders of its own.
    #[must_use]
    pub fn menu_for(mut self, items: impl Fn(&MenuTarget<'_>) -> Vec<ContextItem<Msg>> + 'static) -> Self {
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

    /// Draws each row's icon by the kind of its entry: the Rust logo on a Rust file, a zipper on
    /// an archive, a folder with a branch on `.git`, the downloads folder in the home. Off, every
    /// row is a plain `folder` or `file`.
    ///
    /// A person knows what a file is from its icon before reading its name. The kind comes from
    /// the name alone, see [`file_kind`](crate::icons::file_kind), so no file is opened to draw
    /// it; whether a file whose name says nothing may be run is the one thing read, with the
    /// folder. Outside a Nerd Font each icon is its family's shape, so code, pictures and archives
    /// are still told apart.
    ///
    /// The icons have no colour of their own, as the plain ones have none: they are drawn in the
    /// row's quiet colour and take the selected row's colour with the rest of it. A sign an
    /// application gives with [`row_mark`](Self::row_mark) says something the kind cannot, so it
    /// wins over the kind. Colours by kind are a further layer, [`kind_tones`](Self::kind_tones).
    ///
    /// The folders of the home are found by the names the person's language gives them, read from
    /// `user-dirs.dirs` once the home is on screen; [`user_folders`](Self::user_folders) gives them
    /// instead.
    #[must_use]
    pub fn kind_icons(mut self, on: bool) -> Self {
        self.kind_icons = on;
        self
    }

    /// Colours the icons of [`kind_icons`](Self::kind_icons) by their family: folders take the
    /// accent and the files the theme's series tones, see
    /// [`KindFamily::tone`](crate::icons::KindFamily::tone). A file whose kind is not known keeps
    /// the row's colour.
    ///
    /// The colour only repeats what the shape says, so it adds nothing where tones cannot be told
    /// apart: in sixteen colours and in ASCII it is not drawn. It does nothing without
    /// [`kind_icons`](Self::kind_icons).
    #[must_use]
    pub fn kind_tones(mut self, on: bool) -> Self {
        self.kind_tones = on;
        self
    }

    /// The home and its folders [`kind_icons`](Self::kind_icons) recognises, in place of the
    /// person's own, [`UserFolders::current`].
    ///
    /// For a manager showing another person's home, or a test that means a home of its own.
    #[must_use]
    pub fn user_folders(mut self, folders: &'a UserFolders) -> Self {
        self.user_folders = Some(folders);
        self
    }

    /// Names the rows, so [`Command::focus(name)`](crate::runtime::Command::focus) gives them
    /// the keyboard: an application that takes the person to another folder, from a list of
    /// places, a path bar or a back button, sends it so ↑ and ↓ move through the new folder at
    /// once.
    ///
    /// The name is on the rows themselves, the tree, the list or the icons, whichever is drawn,
    /// not on the column [`show`](Self::show) answers with, which holds the foot too and takes no
    /// focus. The rows are the same widget in all three views, so rows that have the keyboard
    /// keep it when the view changes, named or not.
    #[must_use]
    pub fn id(mut self, name: impl Into<String>) -> Self {
        self.rows_id = Some(name.into());
        self
    }

    /// Draws the rows faint and answers nothing: no click, key, drag or menu, while the
    /// application has taken the folder away from the person.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Adds the manager to `ui` and answers with the column that holds it, to be given a size.
    ///
    /// The column holds the rows and, in the list and the icons, the foot under them. It takes no
    /// focus itself; [`id`](Self::id) names the rows inside it, which do.
    ///
    /// The dialog that asks for a name is added too while one is asked for; it is a layer and
    /// takes no room of its own.
    pub fn show<'v>(mut self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let state = self.state;
        self.kinds = self.kind_look(ui.env());
        // An unreadable folder is said where it stands, but the trash is a place beside the root: while it
        // is shown, what the root could not be read has nothing to say about the rows on screen.
        if let Some(problem) = state.error()
            && !state.in_trash()
        {
            ui.add(Text::new(crate::t!("quvyta.file-manager.unreadable")).role("secondary"));
            return ui.add(Text::new(problem.to_owned()).role("faint")).selectable(true);
        }
        self.naming_dialog(ui);
        self.work_row(ui);
        // The list shows details, so it asks for the page around the cursor it has none of yet;
        // the tree and the icons show names alone and ask for nothing, which is what keeps a
        // folder of ten thousand entries from becoming ten thousand calls to the system. The trash
        // asks for nothing either way: a note says where an entry came from and when it went, and
        // nothing about it is read from the entry.
        if self.view == FileView::List && !state.in_trash() {
            let gaps = state.detail_gaps(state.folder());
            if !gaps.is_empty() {
                let wrap = Rc::clone(&self.wrap);
                ui.on_idle(std::time::Duration::ZERO, move |_| wrap(FileManagerMsg::Detail(gaps.clone())));
            }
        }
        // Every view is the same column with the rows first under the same name, so the rows are
        // one widget whatever shape they take: focus on them outlives a change of view.
        let name = self.rows_id.clone().unwrap_or_else(|| ROWS_ID.to_owned());
        let node = ui.column(|ui| match self.view {
            FileView::Tree => {
                let tree = self.tree();
                ui.add(tree).fill().id(name);
            }
            FileView::List | FileView::Icons => self.flat_view(ui, name),
        });
        self.claim_clipboard(node)
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
        let trashed = state.in_trash();
        let tree = Tree::new([if trashed { self.trash_node() } else { self.root_node() }]);
        if self.disabled {
            return tree;
        }
        let wrap = Rc::clone(&self.wrap);
        let expand = Rc::clone(&self.wrap);
        let choose = Rc::clone(&self.wrap);
        let drop = Rc::clone(&self.wrap);
        let copy = Rc::clone(&self.wrap);
        let accepts = state.folder_keys();
        let tree = tree
            .selected(state.selected())
            .on_select(move |key| wrap(FileManagerMsg::Select(key.to_owned())))
            // The root is where the person is, not an entry to carry away, so a box or Ctrl+A that
            // covers its row leaves it out.
            .multi_select(state.chosen(), move |keys| {
                choose(FileManagerMsg::Choose(keys.into_iter().filter(|key| key != ROOT).collect()))
            })
            // The trash takes no drop: an entry in it is put back or deleted by name, and a drop
            // would move whatever the root holds under that name.
            .droppable(
                move |dropped| drop(FileManagerMsg::Drop(dropped)),
                move |key| !trashed && (key == ROOT || accepts.contains(key)),
            )
            .on_copy_drop(move |dropped| copy(FileManagerMsg::DropCopy(dropped)))
            .activate_on(self.open_on)
            .box_select(true)
            .on_expand(move |key, open| expand(FileManagerMsg::Expand(key.to_owned(), open)))
            .context_menu(self.row_menu());
        // Enter or a double click on a file opens it; on a folder they open the folder, which the
        // tree does itself. Space and the modified clicks select instead.
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
        let (icon, tone) = self.sign_of(&mark, ROOT, &root_name(state.root()), true, false);
        let mut root = TreeNode::new(ROOT, label).icon(icon, tone.as_deref()).faint(self.disabled || mark.is_faint());
        // An unread folder is not an empty one, so it never says "empty" before it is known.
        if state.shown_children(ROOT).is_some_and(|entries| entries.is_empty()) {
            root = root.detail(crate::t!("quvyta.file-manager.empty"));
        }
        root.expandable(true).expanded(state.is_open(ROOT)).loading(state.is_loading(ROOT)).children(self.nodes(ROOT))
    }

    /// The trash itself as the one row at the top, where it is the place the manager shows: its own
    /// shape, its own emptiness, and the entries it holds below it.
    ///
    /// It takes the place of the root's row rather than standing beside it, the way a flat view
    /// shows one folder with that folder's own row at the top: one place at a time, and the row of
    /// the place carries its menu and is the way back out of it.
    fn trash_node(&self) -> TreeNode {
        let state = self.state;
        let mark = self.mark_of(ROOT);
        let label = crate::t!("quvyta.file-manager.trash-place");
        let (icon, tone) = self.sign_of(&mark, ROOT, "trash", true, false);
        let children = self.trash_nodes();
        let mut node = TreeNode::new(ROOT, label).icon(icon, tone.as_deref()).faint(self.disabled || mark.is_faint());
        // A trash nobody has put anything into is empty rather than broken, and says so only once
        // it is neither being read nor saying why it could not be.
        if children.is_empty() && !state.is_reading_trash() && state.trash_error().is_none() {
            node = node.detail(crate::t!("quvyta.file-manager.empty"));
        }
        node.expandable(true).expanded(true).loading(state.is_reading_trash()).children(children)
    }

    /// The rows of the entries of the trash, as far as it has been read.
    ///
    /// A row says nothing about the entry beyond its name, as a row of a folder's does, and where an
    /// entry came from is not said here for the same reason it is not said about a file's size: a
    /// path is longer than a tree row is wide, and the detail that carried it would cover the very
    /// name it belongs to. The list is where the place's own columns have room for it.
    fn trash_nodes(&self) -> Vec<TreeNode> {
        let state = self.state;
        state
            .trashed()
            .into_iter()
            .map(|entry| {
                let mark = self.mark_of(&entry.name);
                let (icon, tone) = self.sign_of(&mark, &entry.name, &entry.label, entry.folder, entry.executable);
                TreeNode::new(entry.name.clone(), entry.label.clone())
                    .icon(icon, tone.as_deref())
                    .faint(self.disabled || mark.is_faint())
            })
            .collect()
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
                let (icon, tone) = self.sign_of(&mark, &child, &entry.name, entry.folder, entry.executable);
                // What was cut is drawn faint until it is pasted or let go, with everything in it.
                let faint = self.disabled || state.is_cut(&child) || mark.is_faint();
                let mut node =
                    TreeNode::new(child.clone(), entry.name.clone()).icon(icon, tone.as_deref()).faint(faint);
                if entry.folder {
                    let open = state.is_open(&child);
                    node = node.expandable(true).expanded(open).loading(state.is_loading(&child));
                    // A folder the system refused says so on its own row. Without this it opened
                    // to nothing, which reads as an empty folder: the person would be told a
                    // folder they may not look into holds nothing.
                    if state.folder_error(&child).is_some() {
                        node = node.detail(crate::t!("quvyta.file-manager.unreadable-short"));
                    }
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

    /// The icon and the colour the row `key` is drawn with: the mark's sign when it has one, and
    /// the manager's own icon for the entry called `name` otherwise, in the row's own colour
    /// unless kinds are coloured.
    fn sign_of(
        &self,
        mark: &RowMark,
        key: &str,
        name: &str,
        folder: bool,
        executable: bool,
    ) -> (String, Option<String>) {
        match mark.icon() {
            Some(icon) => (icon.to_owned(), mark.tone().map(str::to_owned)),
            None => self.own_icon(key, name, folder, executable),
        }
    }

    /// What every row's menu holds.
    fn row_menu(&self) -> impl Fn(&str) -> Vec<ContextItem<Msg>> + 'static {
        let state = self.state;
        // The menu is built long after the view, so it takes what it needs along rather than the
        // state itself.
        let wrap = Rc::clone(&self.wrap);
        let chosen = state.chosen().to_vec();
        let pending = Pending { keys: state.pending().to_vec(), copying: state.is_copying() };
        let trashing = state.is_trashing();
        let trash = state.trash_folder();
        let trashable = trash.is_some();
        let folders = state.folder_keys();
        let extra = self.menu.clone();
        let terminal = self.on_open_terminal.clone();
        let root = state.root().to_path_buf();
        let place = state.in_trash();
        let filled = !state.trashed().is_empty();
        move |key: &str| {
            // The tree keeps the selection when the click is on one of its rows and makes the row
            // the selection otherwise, so the menu acts on what the click was on.
            let targets = state::targets_of(&chosen, key);
            let folder = key == ROOT || folders.contains(key);
            // A row of the trash is where the entry is now, not where it came from: it can be
            // looked at and taken out of there, while its note beside it is what says where it goes
            // back to. The row of the place itself is the trash folder.
            let path = match (place, key == ROOT) {
                (true, true) => trash.clone().unwrap_or_default(),
                (true, false) => trash.clone().unwrap_or_default().join("files").join(key),
                (false, _) => path_of(&root, key),
            };
            let target = MenuTarget { key, path: &path, folder, selection: &targets };
            let mut own = extra.as_ref().map(|items| items(&target)).unwrap_or_default();
            if let Some(message) = &terminal
                && folder
                && !place
            {
                let label = crate::t!("quvyta.file-manager.open-terminal");
                own.push(ContextItem::new(label, message(&path)));
            }
            let send = |message: FileManagerMsg| wrap(message);
            if place {
                let place_row = key == ROOT;
                return if place_row {
                    trash_place_menu(filled, own, &send)
                } else {
                    trash_entry_menu(&targets, own, &send)
                };
            }
            if targets.len() > 1 {
                return many_menu(key, targets.len(), &pending, trashing, own, &send);
            }
            if folder {
                return folder_menu(key, &pending, trashing, trashable, own, &send);
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

/// The path of the entry `key` under `root`, the folder a [`FileManagerState`] shows: the root's
/// own key is `root` itself, and a key's parts separated by `/` are folders below it.
///
/// ```
/// use std::path::Path;
/// use qframe::widgets::path_of;
///
/// assert_eq!(path_of(Path::new("/home/ali"), "notes/2026/june.md"), Path::new("/home/ali/notes/2026/june.md"));
/// ```
#[must_use]
pub fn path_of(root: &Path, key: &str) -> std::path::PathBuf {
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
    trashable: bool,
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
        // The trash is a place of its own rather than a folder here, so it is opened rather than
        // stepped into, and it is offered only where there is one to open.
        if trashable {
            items.push(ContextItem::new(crate::t!("quvyta.file-manager.open-trash"), send(FileManagerMsg::OpenTrash)));
        }
    } else {
        items.push(away_item(key, 1, trashing, send));
    }
    items
}

/// The menu of the trash's own row: the way out of the place, reading it again, and emptying it,
/// which asks first because there is no trash behind this one to put anything back into.
fn trash_place_menu<Msg: Clone + 'static>(
    filled: bool,
    own: Vec<ContextItem<Msg>>,
    send: &impl Fn(FileManagerMsg) -> Msg,
) -> Vec<ContextItem<Msg>> {
    // The way out of a place is on its own row, in every view, as it is for the folder a flat view
    // shows; here the place is not a folder, so it says where it goes back to.
    let mut items = vec![
        ContextItem::new(crate::t!("quvyta.file-manager.leave-trash"), send(FileManagerMsg::Leave)),
        ContextItem::gap(),
        ContextItem::new(crate::t!("quvyta.file-manager.refresh"), send(FileManagerMsg::Refresh)),
    ];
    add_own(&mut items, own);
    if filled {
        items.push(ContextItem::gap());
        items.push(
            ContextItem::new(crate::t!("quvyta.file-manager.empty-trash"), send(FileManagerMsg::EmptyTrash))
                .danger(true),
        );
    }
    items
}

/// The menu of an entry of the trash, or of every entry selected when `targets` holds more than
/// one: putting it back where it came from, and taking it away for good.
///
/// Nothing else is offered, and nothing is offered about where it is: an entry in the trash cannot
/// be made, moved or renamed there, since the trash is a folder of the specification rather than a
/// folder of the person's.
fn trash_entry_menu<Msg: Clone + 'static>(
    targets: &[String],
    own: Vec<ContextItem<Msg>>,
    send: &impl Fn(FileManagerMsg) -> Msg,
) -> Vec<ContextItem<Msg>> {
    let many = targets.len() > 1;
    let restore = if many {
        crate::t!("quvyta.file-manager.restore-many", n = targets.len())
    } else {
        crate::t!("quvyta.file-manager.restore")
    };
    let away = if many {
        crate::t!("quvyta.file-manager.delete-permanently-many", n = targets.len())
    } else {
        crate::t!("quvyta.file-manager.delete-permanently")
    };
    let mut items = vec![ContextItem::new(restore, send(FileManagerMsg::Restore(targets.to_vec())))];
    add_own(&mut items, own);
    items.push(ContextItem::gap());
    items.push(ContextItem::new(away, send(FileManagerMsg::Purge(targets.to_vec()))).danger(true));
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
