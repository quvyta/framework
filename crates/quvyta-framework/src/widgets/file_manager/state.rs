//! What a [`FileManager`](super::FileManager) knows about the folder it shows, and what happens to
//! it.
//!
//! Reading a folder is I/O, so it never happens while drawing: the state asks for a folder when it
//! is opened and keeps the answer, and the view is built from what is already known.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::runtime::{Command, Confirm, Task, TaskEvent, TaskId, TaskOutcome};
use crate::widgets::{Toast, TreeDrop};

use super::details::{FileDetails, PAGE};
use super::flat::FlatEntries;
use super::ops::{self, FileChange, FileError, NameProblem, is_within, name_of, parent_key};
use super::sort::Sort;
use super::trash::Trashed;
use super::watch::Live;

/// The key of the folder the manager is rooted at. Keys below it are paths relative to the root,
/// written with `/` whatever the platform, because they are identities rather than paths.
pub const ROOT: &str = "";

/// The key of the entry `name` inside the folder `parent`.
#[must_use]
pub fn child_key(parent: &str, name: &str) -> String {
    if parent.is_empty() { name.to_owned() } else { format!("{parent}/{name}") }
}

/// One entry of a folder, as a file manager reads it: a name, whether it can be opened and whether
/// it is a program.
///
/// A tree row shows nothing else, so nothing else is read. Size, date and permissions each mean
/// another call to the system for every entry, which a folder of ten thousand entries cannot
/// afford, so they belong to the views that show them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FolderEntry {
    /// Its name, without any folder before it.
    pub name: String,
    /// Whether it is a folder, and so can be opened. A symbolic link is never a folder: opening
    /// one would leave the tree it is drawn in.
    pub folder: bool,
    /// Whether it is a file that may be run, which gives it a program's icon.
    ///
    /// Only asked of a file whose name says nothing of its kind, since the name wins where it
    /// does (see [`file_kind`](crate::icons::file_kind)): a folder of ten thousand photos asks the
    /// system nothing more, and a folder of programs asks once for each of them. Always `false`
    /// where the system has no such permission.
    pub executable: bool,
}

impl FolderEntry {
    /// Reads one folder the way [`read_folder`](Self::read_folder) does, keeping why it could not
    /// be read rather than what the system said about it, so the words are chosen where the
    /// person's language is known.
    pub(super) fn list(path: &Path) -> Result<Vec<Self>, FileError> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path).map_err(ops::read_error)? {
            let entry = entry.map_err(ops::read_error)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
            let executable = !folder && runs(&entry, &name);
            entries.push(Self { name, folder, executable });
        }
        entries.sort_by(|a, b| b.folder.cmp(&a.folder).then_with(|| a.name.cmp(&b.name)));
        Ok(entries)
    }

    /// Whether the entry is one the platform hides: on every system the framework runs on, a name
    /// that starts with a dot.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }

    /// Reads one folder, folders first and then files, each group in name order.
    ///
    /// This touches the disk, so it belongs on a background thread; [`FileManagerState`] asks for
    /// it with [`Command::perform`] and never while drawing.
    ///
    /// # Errors
    ///
    /// What the operating system said, when the folder cannot be read. The manager's own reads say
    /// it in the person's language instead; see [`FileManagerMsg::Listed`].
    pub fn read_folder(path: &Path) -> Result<Vec<Self>, String> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            // A name the platform does not spell as text is still an entry; showing it lossily is
            // better than pretending the folder holds less than it does.
            let name = entry.file_name().to_string_lossy().into_owned();
            // `file_type` does not follow a link, so a link to a folder is an entry and not a way
            // out of the tree.
            let folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
            let executable = !folder && runs(&entry, &name);
            entries.push(Self { name, folder, executable });
        }
        entries.sort_by(|a, b| b.folder.cmp(&a.folder).then_with(|| a.name.cmp(&b.name)));
        Ok(entries)
    }
}

/// Whether the file `entry`, called `name`, is a program: one whose name says nothing of its kind
/// and that may be run. A link answers for what it points to, since its own permissions are
/// always all of them.
pub(super) fn runs(entry: &std::fs::DirEntry, name: &str) -> bool {
    if crate::icons::file_kind(name, false, false).family() != crate::icons::KindFamily::File {
        return false;
    }
    may_run(&entry.path())
}

/// Whether the file at `path` may be run by someone.
#[cfg(unix)]
fn may_run(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|data| data.is_file() && data.permissions().mode() & 0o111 != 0)
}

/// Whether the file at `path` may be run: never known here, so never said.
#[cfg(not(unix))]
fn may_run(_path: &Path) -> bool {
    false
}

/// What the name the dialog asks for is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameFor {
    /// A new, empty file.
    File,
    /// A new, empty folder.
    Folder,
    /// The entry of this key, which keeps its place and takes the new name.
    Rename(String),
}

/// The dialog that asks for a name, while it is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    /// What the name is for.
    pub purpose: NameFor,
    /// The key of the folder the name is to be used in.
    pub folder: String,
    /// What has been typed.
    pub value: String,
    /// Whether the person tried to confirm; an empty name is only pointed out after that, so a
    /// dialog that has just opened does not start by scolding.
    pub tried: bool,
}

/// Something that happened in a [`FileManager`](super::FileManager).
///
/// Hand every one of these to [`FileManagerState::update`]; the application's own messages come
/// from [`FileManager::on_open`](super::FileManager::on_open) and the items it adds to the menu,
/// never from here.
///
/// More things may happen as the manager grows, so match with a `_` arm; an application normally
/// hands every one of these straight over without looking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum FileManagerMsg {
    /// The cursor moved to this key.
    Select(String),
    /// The entries of these keys became the selection.
    Choose(Vec<String>),
    /// The folder of this key was opened, or closed when `false`.
    Expand(String, bool),
    /// The folder of this key was read in the background, or could not be, by the application's
    /// own reading. The text is the system's own; hand it over as it comes.
    Read(String, Result<Vec<FolderEntry>, String>),
    /// The folder of this key was read by the manager itself.
    ///
    /// Why a read failed is kept as a [`FileError`] rather than as words, because the thread that
    /// read the folder does not know the person's language; the words are chosen here.
    Listed(String, Result<Vec<FolderEntry>, FileError>),
    /// A new file was asked for in the folder of this key.
    NewFile(String),
    /// A new folder was asked for in the folder of this key.
    NewFolder(String),
    /// The entry of this key was asked to take another name.
    Rename(String),
    /// The entry of this key was cut, with the rest of the selection when it is part of it, to be
    /// pasted into another folder.
    Cut(String),
    /// The entry of this key was copied, with the rest of the selection when it is part of it, to
    /// be pasted into another folder. What was copied stays where it is.
    Copy(String),
    /// What was cut was asked to go into the folder of this key.
    Paste(String),
    /// What was cut is to stay where it is after all.
    DropCut,
    /// Entries were dragged onto a folder, or onto the free space that stands for the root.
    Drop(TreeDrop),
    /// Entries were dragged onto a folder with Ctrl held at the release, to be copied there. What
    /// was copied stays where it is.
    DropCopy(TreeDrop),
    /// The entry of this key was asked to be deleted, with the rest of the selection when it is
    /// part of it; the person is asked first.
    Delete(String),
    /// The person said yes to deleting the entries of these keys.
    DeleteConfirmed(Vec<String>),
    /// The entry of this key was asked to go to the trash, with the rest of the selection when it
    /// is part of it. Nothing is asked: the trash can be looked in again.
    Trash(String),
    /// The trash was asked to be shown in place of the manager's own folder.
    OpenTrash,
    /// The trash was read, or could not be, and holds these entries. It is read on a background
    /// thread, so why a read failed is kept as a [`FileError`] rather than as words, as in
    /// [`FileManagerMsg::Listed`].
    Trashed(Result<Vec<Trashed>, FileError>),
    /// The entries of the trash with these keys were asked to be put back where they came from,
    /// the rest of the selection included when one of them is part of it. Nothing is overwritten:
    /// an entry whose name is taken again is said so with a name that is free there.
    Restore(Vec<String>),
    /// The entry of the trash with this key was asked to be put back under the name `name`, after
    /// the question about a name that was taken again was answered yes. A key the trash does not
    /// list, and a name that is not one name in that folder, put nothing anywhere.
    RestoreAs(String, String),
    /// The entries of the trash with these keys were asked to be deleted for good, the rest of the
    /// selection included when one of them is part of it; the person is asked first.
    Purge(Vec<String>),
    /// The person said yes to deleting the entries of the trash with these keys for good. Only
    /// the keys the trash lists are deleted: one written by hand, `..` or `../x` among them, is
    /// never taken as a path out of the trash.
    PurgeConfirmed(Vec<String>),
    /// Everything in the trash was asked to be deleted for good; the person is asked first.
    EmptyTrash,
    /// The person said yes to emptying the trash.
    EmptyTrashConfirmed,
    /// Hidden entries are shown from now on, or hidden again when `false`.
    ShowHidden(bool),
    /// The long operation running now started, came further along or ended.
    Work(TaskEvent),
    /// The long operation running now was asked to stop.
    Stop,
    /// Every open folder was asked to be read again.
    Refresh,
    /// The name in the dialog changed.
    Name(String),
    /// The name in the dialog was confirmed.
    Submit,
    /// The dialog was closed without a name.
    CloseNaming,
    /// Operations finished or were refused: for each entry its key and what came of it.
    Done(Vec<(String, Result<FileChange, FileError>)>),
    /// A batch of outside changes of the watch `u64` arrived; empty once that watch was let go.
    Changed(u64, Vec<crate::storage::FolderChange>),
    /// A [bounded](FileManagerState::following_within) wait of the watch `u64` ended with nothing
    /// changed; the watch waits again.
    Quiet(u64),
    /// The details of the entries of these keys were asked for: their size, when they changed and
    /// their permissions. Keys that are already known, or already on their way, cost nothing.
    Detail(Vec<String>),
    /// The details of these keys were read, `None` for an entry the system said nothing about.
    Detailed(Vec<(String, Option<FileDetails>)>),
    /// A flat view stepped into the folder of this key; it is read if it has not been.
    Enter(String),
    /// A flat view stepped out of the folder it shows, into the one above it.
    Leave,
    /// The list was asked to be put in this order, by a press on a column's title; what an order
    /// by size or date needs is read in the background.
    Sort(Sort),
    /// The flat views were narrowed to the entries whose name holds this text, or shown whole
    /// again with `None`; see [`FileManagerState::set_filter`].
    Filter(Option<String>),
    /// The first entry the filter left was chosen: the cursor goes to it and the filter stays.
    FilterChosen,
}

/// A long operation the manager is running in the background right now.
///
/// Copying is the one operation that can take a while: a rename is instant whatever it moves, but
/// a folder of photographs is read and written byte by byte. So a copy runs as a
/// [`Task`](crate::runtime::Task), tells how far it has come and can be stopped; everything else
/// still runs straight through.
#[derive(Debug, Clone)]
pub struct FileWork {
    id: TaskId,
    entries: usize,
    done: f32,
    note: String,
}

impl FileWork {
    /// The task doing the work, so an application can show it in a
    /// [`Tasks`](crate::runtime::Tasks) model of its own beside its other background work, or stop
    /// it without going through the manager's own button.
    #[must_use]
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// The share done, from 0 to 1.
    #[must_use]
    pub fn done(&self) -> f32 {
        self.done
    }

    /// The name of the entry being copied right now, empty before the first one starts.
    #[must_use]
    pub fn note(&self) -> &str {
        &self.note
    }

    /// How many entries the operation was given.
    #[must_use]
    pub fn entries(&self) -> usize {
        self.entries
    }
}

/// Where a deleted entry goes.
#[derive(Debug, Default)]
enum Trash {
    /// Nowhere: deleting takes an entry away for good, which is all a manager did before.
    #[default]
    Off,
    /// The person's own trash, where the freedesktop specification puts it.
    Home,
    /// This folder, for an application with a trash of its own.
    In(PathBuf),
}

impl Trash {
    /// The folder entries go to, when there is one.
    fn folder(&self) -> Option<PathBuf> {
        match self {
            Self::Off => None,
            Self::Home => super::trash::home_trash(),
            Self::In(folder) => Some(folder.clone()),
        }
    }
}

/// What the manager is showing: its own root, or the trash beside it.
///
/// The trash is not under the root, which is the whole of what separates the two: a row of the root
/// is keyed by its path below the root and every operation on it works there, while a row of the
/// trash is named by what it is called in the trash and is put back or deleted by that name. So the
/// two places keep their entries in maps of their own and neither can be acted on with the other's
/// keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Place {
    /// The manager's root and the folders inside it, the place a manager is for.
    #[default]
    Files,
    /// The trash that deleting puts entries in.
    Trash,
}

/// Turns a manager's messages into the application's own.
pub(super) type Wrap<Msg> = Arc<dyn Fn(FileManagerMsg) -> Msg + Send + Sync>;

/// What a file manager has read of its root folder, what is open in it, and what is selected.
///
/// The application owns one of these per manager on screen, hands it every [`FileManagerMsg`] and
/// draws it with [`FileManager`](super::FileManager). It reads folders in the background, keeps no
/// settings and writes nothing of its own to disk.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::{FileManager, FileManagerMsg, FileManagerState};
///
/// struct Files {
///     manager: FileManagerState,
///     opened: Option<std::path::PathBuf>,
/// }
///
/// #[derive(Clone)]
/// enum Msg {
///     Manager(FileManagerMsg),
///     Open(std::path::PathBuf),
/// }
///
/// impl App for Files {
///     type Msg = Msg;
///     fn init(&mut self) -> Command<Msg> {
///         self.manager.load(Msg::Manager)
///     }
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::Manager(message) => self.manager.update(message, Msg::Manager),
///             Msg::Open(path) => {
///                 self.opened = Some(path);
///                 Command::none()
///             }
///         }
///     }
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         FileManager::new(&self.manager, Msg::Manager)
///             .on_open(|path| Msg::Open(path.to_path_buf()))
///             .show(ui)
///             .fill();
///     }
/// }
/// ```
#[derive(Debug)]
pub struct FileManagerState {
    root: PathBuf,
    confined: bool,
    following: bool,
    /// The folder a flat view shows; the root until one is stepped into.
    shown: String,
    children: BTreeMap<String, Vec<FolderEntry>>,
    /// What is known about an entry besides its name, for the entries it was asked for. Never a
    /// whole folder: a folder of ten thousand entries must not become ten thousand calls. Shared,
    /// so a frame's rows read it as they are drawn without copying it.
    details: Arc<BTreeMap<String, Option<FileDetails>>>,
    /// The keys whose details were asked for and have not come back yet, so one ask is not made
    /// twice.
    reading: BTreeSet<String>,
    open: BTreeSet<String>,
    loading: BTreeSet<String>,
    selected: Option<String>,
    chosen: Vec<String>,
    /// Why a folder could not be read, by key, for every folder whose last read failed. The root
    /// is in here like any other: one place holds the reason, so a folder that failed can never
    /// be drawn as one that is simply empty.
    errors: BTreeMap<String, String>,
    cut: Vec<String>,
    /// Whether what waits to be pasted is to be copied rather than moved.
    copying: bool,
    trash: Trash,
    /// Which of the two places is shown.
    place: Place,
    /// What the trash holds, by the name each entry has there. Its own map, never the map of the
    /// root's children: an entry of the trash is not below the root and must never be reached as
    /// if it were.
    trashed: BTreeMap<String, Trashed>,
    /// Whether the trash is being read right now, which the row of the place shows.
    trash_reading: bool,
    /// Why the trash could not be read, when its last read failed. A trash nobody has put anything
    /// into has no `files` folder at all, which is empty rather than broken and never a reason.
    trash_problem: Option<String>,
    hidden: bool,
    work: Option<FileWork>,
    naming: Option<Naming>,
    pub(super) live: Live,
    /// Counts the watches started, so a batch of one that was let go is recognised.
    pub(super) runs: u64,
    /// Whether a view that shows details asked for them, so a folder read again on the
    /// application's word asks for its page again without waiting for a key.
    details_wanted: bool,
    /// How long one wait for outside changes may last; `None` waits until something changes.
    pub(super) patience: Option<std::time::Duration>,
    /// Counts the changes made to the state, so what a frame worked out from it is known to be
    /// current without being worked out again.
    revision: u64,
    /// Counts the changes made to what is known about the entries, so an order that goes by their
    /// size or date is made again when more of them are known.
    details_revision: u64,
    /// The order the flat views list a folder in.
    sort: Sort,
    /// The text the flat views narrow their folder to, with the folder it was typed in: stepping
    /// into another folder lets it go.
    filter: Option<(String, String)>,
    /// The entries of the shown folder as the flat views list them, kept while the state does not
    /// change: a frame of a folder of a hundred thousand entries reads them instead of making them.
    flat: std::sync::Mutex<Option<Arc<FlatEntries>>>,
}

impl FileManagerState {
    /// The key of the root folder: the manager's top row, and the folder every other key is
    /// written relative to.
    pub const ROOT: &'static str = ROOT;

    /// A manager of the folder at `root`, with nothing read yet.
    ///
    /// The root is the manager's top row and starts open: its entries are what the manager is for.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            confined: false,
            following: false,
            shown: ROOT.to_owned(),
            children: BTreeMap::new(),
            details: Arc::default(),
            reading: BTreeSet::new(),
            open: BTreeSet::from([ROOT.to_owned()]),
            loading: BTreeSet::new(),
            selected: None,
            chosen: Vec::new(),
            errors: BTreeMap::new(),
            cut: Vec::new(),
            copying: false,
            trash: Trash::Off,
            place: Place::Files,
            trashed: BTreeMap::new(),
            trash_reading: false,
            trash_problem: None,
            hidden: false,
            work: None,
            naming: None,
            live: Live::Off,
            runs: 0,
            details_wanted: false,
            patience: None,
            revision: 0,
            details_revision: 0,
            sort: Sort::default(),
            filter: None,
            flat: std::sync::Mutex::new(None),
        }
    }

    /// Keeps every operation inside the root: a key that climbs out of it is refused, and so is
    /// one that goes through a symbolic link, because a link can point anywhere.
    ///
    /// An application that shows a folder the person must not leave (a project, a sandbox) asks
    /// for this; one that shows the whole file system does not. It is a rule about what may be
    /// changed on disk, so it belongs to the state the operations run from and not to the view,
    /// which is built afresh every frame.
    #[must_use]
    pub fn confined(mut self) -> Self {
        self.confined = true;
        self
    }

    /// Whether operations are kept inside the root.
    #[must_use]
    pub fn is_confined(&self) -> bool {
        self.confined
    }

    /// Deleting puts an entry in the person's own trash instead of taking it away for good, the
    /// way the freedesktop trash specification says: `$XDG_DATA_HOME/Trash`, or
    /// `~/.local/share/Trash` when that variable says nothing.
    ///
    /// An entry that cannot be renamed into that folder — one on another file system, or a run with
    /// no home folder at all — is not deleted quietly instead: the manager says there is no trash
    /// for it and asks whether to delete it for good, in the danger colour, as its own question.
    ///
    /// Windows and macOS have a trash of their own that this specification does not describe, so
    /// there this asks the same question rather than inventing a folder.
    #[must_use]
    pub fn trashing(mut self) -> Self {
        self.trash = Trash::Home;
        self
    }

    /// Deleting puts an entry in the trash folder `folder` instead of the person's own, for an
    /// application that keeps a trash of its own and for a test, which must never touch the
    /// person's.
    ///
    /// The folder is made when the first entry goes into it, with the `files` and `info` folders
    /// the specification asks for.
    #[must_use]
    pub fn trashing_in(mut self, folder: impl Into<PathBuf>) -> Self {
        self.trash = Trash::In(folder.into());
        self
    }

    /// Whether deleting puts entries in a trash.
    #[must_use]
    pub fn is_trashing(&self) -> bool {
        !matches!(self.trash, Trash::Off)
    }

    /// The folder entries go to when one is trashed, and the folder [`open_trash`](Self::open_trash)
    /// looks in: the one [`trashing`](Self::trashing) and [`trashing_in`](Self::trashing_in) say,
    /// so a test's trash is the one the test uses.
    ///
    /// `None` where there is no trash to speak of: none was asked for, or the person's own cannot
    /// be found on this machine.
    #[must_use]
    pub fn trash_folder(&self) -> Option<PathBuf> {
        self.trash.folder()
    }

    /// Whether the trash is shown rather than the manager's own root. See
    /// [`open_trash`](Self::open_trash).
    #[must_use]
    pub fn in_trash(&self) -> bool {
        self.place == Place::Trash
    }

    /// Which of the two places is shown, for the views that draw whichever it is.
    pub(super) fn place(&self) -> Place {
        self.place
    }

    /// What the trash holds, as far as it has been read, each entry with the name it has there.
    ///
    /// Folders first and then files, each group in name order, as a folder's own entries are
    /// shown; the name here is the one in the trash, which is what an operation on an entry is
    /// given.
    #[must_use]
    pub fn trashed(&self) -> Vec<&Trashed> {
        let mut entries: Vec<&Trashed> = self.trashed.values().collect();
        entries.sort_by(|a, b| b.folder.cmp(&a.folder).then_with(|| a.name.cmp(&b.name)));
        entries
    }

    /// The entry of the trash with the name `name` there, when it has been read. A row of the trash
    /// is keyed by that name, so it is what an operation on the row is given.
    #[must_use]
    pub fn trashed_entry(&self, name: &str) -> Option<&Trashed> {
        self.trashed.get(name)
    }

    /// Whether the trash is being read right now.
    #[must_use]
    pub fn is_reading_trash(&self) -> bool {
        self.trash_reading
    }

    /// Why the trash could not be read, when its last read failed.
    ///
    /// A trash nobody has put anything into has no `files` folder at all, which is empty rather than
    /// broken and is never a reason.
    #[must_use]
    pub fn trash_error(&self) -> Option<&str> {
        self.trash_problem.as_deref()
    }

    /// Shows the entries the platform hides: the ones whose name starts with a dot.
    ///
    /// Off by default, the way a folder is usually looked at. The entries are read either way —
    /// one read of a folder is one read — so turning this on shows them without going to the disk,
    /// and a new entry's name is checked against a hidden one that is already there whether they
    /// are shown or not.
    #[must_use]
    pub fn showing_hidden(mut self, showing: bool) -> Self {
        self.hidden = showing;
        self
    }

    /// Shows or hides the hidden entries; see [`showing_hidden`](Self::showing_hidden).
    pub fn set_showing_hidden(&mut self, showing: bool) {
        self.hidden = showing;
    }

    /// Lists the flat views' folder in the order `sort` from the start, such as the order the
    /// person chose last time; see [`set_sort`](Self::set_sort).
    #[must_use]
    pub fn sorted_by(mut self, sort: Sort) -> Self {
        self.sort = sort;
        self
    }

    /// Lists the flat views' folder in the order `sort`. Folders always come first. An order by
    /// size or by date needs what is known about every entry of the folder, so a view asks for it
    /// with [`FileManagerMsg::Sort`], which reads what is missing in the background; an entry not
    /// read yet stands after the read ones and takes its place when it comes. The cursor and the
    /// selection stay on their entries, wherever those move. The tree keeps its order by name.
    pub fn set_sort(&mut self, sort: Sort) {
        self.sort = sort;
    }

    /// The order the flat views list a folder in.
    #[must_use]
    pub fn sort(&self) -> Sort {
        self.sort
    }

    /// Narrows the flat views to the entries whose name holds `text` anywhere, whatever the case
    /// (Turkish dotted and dotless i included), or shows them all again with `None`. It belongs to
    /// the folder shown now: stepping into another lets it go. Hidden entries stay hidden while
    /// they are not shown. A view shows the field it is typed in while it is set.
    pub fn set_filter(&mut self, text: Option<String>) {
        self.filter = text.map(|text| (self.shown.clone(), text));
    }

    /// The text the flat views are narrowed to, while it is set for the folder shown now.
    #[must_use]
    pub fn filter(&self) -> Option<&str> {
        self.filter.as_ref().filter(|(folder, _)| *folder == self.shown).map(|(_, text)| text.as_str())
    }

    /// Whether the entries the platform hides are shown.
    #[must_use]
    pub fn shows_hidden(&self) -> bool {
        self.hidden
    }

    /// Follows the folders on screen with a [`FolderWatch`](crate::storage::FolderWatch), so what
    /// another program changes in them is read again without being asked.
    ///
    /// Off by default: the watch waits on a background thread, which a screen that is not shown
    /// has no reason to keep, and a test drives the batches itself. Turn it on for a manager the
    /// person is looking at and off again when it leaves the screen.
    #[must_use]
    pub fn following(mut self, following: bool) -> Self {
        self.following = following;
        self
    }

    /// Follows outside changes like [`following(true)`](Self::following), with each wait for them
    /// lasting at most `bound`; a wait that ends with nothing changed simply waits again.
    ///
    /// Made for screen tests: a [`Harness`](crate::runtime::Harness) runs the wait on the spot, and
    /// an unbounded one would never come back, so a test that wants to see another program's
    /// change arrive gives a short bound and steps the harness. A running application keeps
    /// `following(true)`.
    #[must_use]
    pub fn following_within(mut self, bound: std::time::Duration) -> Self {
        self.following = true;
        self.patience = Some(bound);
        self
    }

    /// Turns following outside changes on or off; see [`following`](Self::following).
    pub fn set_following(&mut self, following: bool) {
        self.following = following;
        if !following {
            self.live = Live::Off;
        }
    }

    /// Whether outside changes are followed.
    #[must_use]
    pub fn follows_changes(&self) -> bool {
        self.following
    }

    /// The folder the manager shows.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The entries of the folder `key`, when they have been read.
    #[must_use]
    pub fn children(&self, key: &str) -> Option<&[FolderEntry]> {
        self.children.get(key).map(Vec::as_slice)
    }

    /// The folder a flat view shows, the root until one is stepped into.
    ///
    /// The tree shows the whole root and takes no notice of this; the list and the icons show this
    /// one folder, and [`FileManagerMsg::Enter`] and [`FileManagerMsg::Leave`] move through it.
    ///
    /// While the trash is shown this is still the root: the trash is a place beside the root rather
    /// than a folder in it, so nothing here names it and
    /// [`FileManagerMsg::Leave`](FileManagerMsg::Leave) is the way back out of it.
    #[must_use]
    pub fn folder(&self) -> &str {
        &self.shown
    }

    /// What is known about the entry `key` besides its name: its size, when it changed last and
    /// its permissions.
    ///
    /// `None` while nothing has been asked for that entry, and `Some(None)` for one the system
    /// said nothing about — it went away, or may not be looked at. Ask for details with
    /// [`detail`](Self::detail) or [`FileManagerMsg::Detail`]; a tree row never asks.
    #[must_use]
    pub fn details(&self, key: &str) -> Option<Option<&FileDetails>> {
        self.details.get(key).map(Option::as_ref)
    }

    /// What is known about the entries besides their names, shared, for rows drawn after the view
    /// was built.
    pub(super) fn details_shared(&self) -> Arc<BTreeMap<String, Option<FileDetails>>> {
        Arc::clone(&self.details)
    }

    /// The entries of the shown folder as the flat views list them, made again only after the
    /// folder, its entries or whether hidden entries show have changed.
    pub(super) fn flat_entries(&self) -> Arc<FlatEntries> {
        let mut kept = self.flat.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(flat) = kept.as_ref()
            && flat.revision == self.revision
            && flat.place == self.place
            && flat.folder == self.shown
            && flat.hidden == self.hidden
            && flat.sort == self.sort
            && flat.filter.as_deref() == self.filter()
            && (!self.sort.needs_details() || flat.details_revision == self.details_revision)
        {
            return Arc::clone(flat);
        }
        let flat = Arc::new(FlatEntries::of(self, self.revision, self.details_revision));
        *kept = Some(Arc::clone(&flat));
        flat
    }

    /// Whether the details of `key` have been asked for, whether or not the answer has come.
    #[must_use]
    pub fn has_details(&self, key: &str) -> bool {
        self.details.contains_key(key) || self.reading.contains(key)
    }

    /// Whether the folder `key` is open.
    #[must_use]
    pub fn is_open(&self, key: &str) -> bool {
        self.open.contains(key)
    }

    /// Whether the folder `key` is being read right now.
    #[must_use]
    pub fn is_loading(&self, key: &str) -> bool {
        self.loading.contains(key)
    }

    /// The key of the entry the cursor is on: the row with the pillar, which the keys move from.
    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// The keys of every selected entry, the cursor's among them unless it was taken out.
    #[must_use]
    pub fn chosen(&self) -> &[String] {
        &self.chosen
    }

    /// Why the root folder could not be read, when it could not be.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.folder_error(ROOT)
    }

    /// Why the folder `key` could not be read, when its last read failed.
    ///
    /// A folder the system refused keeps its row and says so; before this the manager kept only
    /// the root's reason and drew every other refused folder as an empty one, which told the
    /// person a folder they may not look into holds nothing. `key` may be
    /// [`ROOT`](Self::ROOT), which is what [`error`](Self::error) asks for.
    #[must_use]
    pub fn folder_error(&self, key: &str) -> Option<&str> {
        self.errors.get(key).map(String::as_str)
    }

    /// The keys of the entries that were cut and wait to be pasted; empty while what waits is to
    /// be copied instead.
    #[must_use]
    pub fn cut(&self) -> &[String] {
        if self.copying { &[] } else { &self.cut }
    }

    /// The keys of the entries that were copied and wait to be pasted.
    #[must_use]
    pub fn copied(&self) -> &[String] {
        if self.copying { &self.cut } else { &[] }
    }

    /// The keys of the entries that wait to be pasted, whether pasting will move or copy them.
    /// Only one of the two waits at a time: copying something lets go of what was cut.
    #[must_use]
    pub fn pending(&self) -> &[String] {
        &self.cut
    }

    /// Whether pasting what waits will copy it rather than move it.
    #[must_use]
    pub fn is_copying(&self) -> bool {
        self.copying
    }

    /// Whether the entry `key` was cut, or is inside a folder that was. A copied entry is not: it
    /// stays where it is, so nothing about it is faint.
    #[must_use]
    pub fn is_cut(&self, key: &str) -> bool {
        !self.copying && self.cut.iter().any(|cut| is_within(key, cut))
    }

    /// The entries of the folder `key` that are drawn: all of them, or the ones the platform does
    /// not hide while [`shows_hidden`](Self::shows_hidden) is off.
    #[must_use]
    pub fn shown_children(&self, key: &str) -> Option<Vec<&FolderEntry>> {
        let entries = self.children(key)?;
        Some(entries.iter().filter(|entry| self.hidden || !entry.is_hidden()).collect())
    }

    /// The long operation running in the background, while one is running. See [`FileWork`].
    #[must_use]
    pub fn work(&self) -> Option<&FileWork> {
        self.work.as_ref()
    }

    /// The dialog asking for a name, while it is open.
    #[must_use]
    pub fn naming(&self) -> Option<&Naming> {
        self.naming.as_ref()
    }

    /// Whether the entry `key` is a folder, as far as the manager has read.
    #[must_use]
    pub fn is_folder(&self, key: &str) -> bool {
        let name = name_of(key);
        self.children(parent_key(key)).is_some_and(|entries| entries.iter().any(|e| e.folder && e.name == name))
    }

    /// The keys of every folder the manager has read so far, the root excepted.
    #[must_use]
    pub fn folder_keys(&self) -> BTreeSet<String> {
        self.children
            .iter()
            .flat_map(|(parent, entries)| {
                entries.iter().filter(|entry| entry.folder).map(move |entry| child_key(parent, &entry.name))
            })
            .collect()
    }

    /// The path on disk of the entry `key`, the root itself for [`Self::ROOT`].
    #[must_use]
    pub fn path(&self, key: &str) -> PathBuf {
        key.split('/').filter(|part| !part.is_empty()).fold(self.root.clone(), |path, part| path.join(part))
    }

    /// Makes `key` the one selected entry, with the cursor on it.
    pub fn select(&mut self, key: &str) {
        self.selected = Some(key.to_owned());
        self.chosen = vec![key.to_owned()];
    }

    /// What an action asked for on the row `key` acts on: the whole selection when the row is one
    /// of several selected, and the row alone otherwise, the way a right click on a row is meant.
    ///
    /// An entry inside a folder that is also taken is left out, because it goes wherever the
    /// folder goes; the root itself is never taken.
    #[must_use]
    pub fn targets(&self, key: &str) -> Vec<String> {
        targets_of(&self.chosen, key)
    }

    /// What is wrong with the name typed so far, if anything; an empty name only once the person
    /// has tried to confirm it.
    #[must_use]
    pub fn naming_problem(&self) -> Option<NameProblem> {
        let naming = self.naming.as_ref()?;
        let siblings = self.children(&naming.folder).unwrap_or_default().iter().map(|entry| entry.name.as_str());
        let current = match &naming.purpose {
            NameFor::Rename(key) => Some(name_of(key)),
            NameFor::File | NameFor::Folder => None,
        };
        let problem = ops::check_name(&naming.value, siblings, current).err()?;
        (problem != NameProblem::Empty || naming.tried).then_some(problem)
    }

    /// The folders whose rows are on screen: every open folder whose folders above it are all open
    /// too, the root first when it is. A folder left open inside a closed one is remembered, not
    /// shown.
    #[must_use]
    pub fn visible_folders(&self) -> Vec<String> {
        self.open
            .iter()
            .filter(|key| {
                let mut above = key.as_str();
                while above != ROOT {
                    above = parent_key(above);
                    if !self.open.contains(above) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// The folders whose entries are shown: the root and every open folder that has been read.
    fn shown_folders(&self) -> Vec<String> {
        std::iter::once(ROOT.to_owned())
            .chain(self.open.iter().filter(|key| *key != ROOT && self.children.contains_key(*key)).cloned())
            .collect()
    }

    /// Opens or closes the folder `key`, and answers whether its entries still have to be read.
    ///
    /// A folder is read once; opening it again shows what is already known and reads nothing, so
    /// clicking a chevron twice does not go to the disk twice.
    fn expand(&mut self, key: &str, open: bool) -> bool {
        if !open {
            self.open.remove(key);
            return false;
        }
        self.open.insert(key.to_owned());
        if self.children.contains_key(key) || self.loading.contains(key) {
            return false;
        }
        self.loading.insert(key.to_owned());
        true
    }

    /// Takes the answer for the folder `key`.
    ///
    /// A folder that could not be read keeps its place and the root says what happened, so the
    /// person sees the reason rather than a gap.
    fn take_read(&mut self, key: &str, entries: Result<Vec<FolderEntry>, String>) {
        // What the flat views list is worked out again from what changes here.
        self.revision += 1;
        self.loading.remove(key);
        // A folder closed or gone while it was being read keeps nothing of the answer, so opening
        // it again, or a new folder of its name, reads afresh.
        if key != ROOT && !self.open.contains(key) {
            return;
        }
        match entries {
            Ok(entries) => {
                self.errors.remove(key);
                self.children.insert(key.to_owned(), entries);
                // A folder read again may hold entries that changed since, so what was known about
                // them is let go and asked for afresh rather than shown out of date.
                Arc::make_mut(&mut self.details).retain(|entry, _| parent_key(entry) != key);
                self.details_revision += 1;
                self.reading.retain(|entry| parent_key(entry) != key);
                self.prune(key);
            }
            Err(problem) => {
                // The reason is kept whichever folder it was. The root draws it in place of the
                // tree; any other folder keeps its row, holds no entries and says on the row that
                // it could not be read, so an empty folder and a refused one never look alike.
                self.errors.insert(key.to_owned(), problem);
                if key != ROOT {
                    self.children.insert(key.to_owned(), Vec::new());
                }
            }
        }
    }

    /// Forgets what the manager held below the folder `key` that its entries, just read, no longer
    /// have: an entry removed or moved away by another program leaves no open folder, cut or
    /// selection behind that would act on a name that is not there.
    fn prune(&mut self, key: &str) {
        let Some(entries) = self.children.get(key) else { return };
        let names: BTreeSet<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
        let held = self
            .open
            .iter()
            .chain(self.children.keys())
            .chain(&self.loading)
            .chain(&self.cut)
            .chain(&self.chosen)
            .chain(&self.selected);
        let gone: BTreeSet<String> = held
            .filter(|held| *held != key && (key == ROOT || is_within(held, key)))
            .map(|held| {
                let below = if key == ROOT { held.as_str() } else { &held[key.len() + 1..] };
                below.split('/').next().unwrap_or(below)
            })
            .filter(|name| !names.contains(name))
            .map(|name| child_key(key, name))
            .collect();
        for child in gone {
            self.forget(&child);
        }
    }

    /// Gives every key at or below `from` the place `to` instead, after a rename or a move, so an
    /// open folder stays open and the selection stays on what moved.
    fn rekey(&mut self, from: &str, to: &str) {
        // What the flat views list is worked out again from what changes here.
        self.revision += 1;
        let moved = |key: &str| is_within(key, from).then(|| format!("{to}{}", &key[from.len()..]));
        self.open = self.open.iter().map(|key| moved(key).unwrap_or_else(|| key.clone())).collect();
        self.loading.retain(|key| !is_within(key, from));
        self.children = std::mem::take(&mut self.children)
            .into_iter()
            .map(|(key, entries)| (moved(&key).unwrap_or(key), entries))
            .collect();
        self.details = Arc::new(
            std::mem::take(Arc::make_mut(&mut self.details))
                .into_iter()
                .map(|(key, details)| (moved(&key).unwrap_or(key), details))
                .collect(),
        );
        self.errors = std::mem::take(&mut self.errors)
            .into_iter()
            .map(|(key, problem)| (moved(&key).unwrap_or(key), problem))
            .collect();
        self.reading.retain(|key| !is_within(key, from));
        if let Some(new) = moved(&self.shown) {
            self.shown = new;
        }
        for key in self.selected.iter_mut().chain(&mut self.cut).chain(&mut self.chosen) {
            if let Some(new) = moved(key) {
                *key = new;
            }
        }
    }

    /// Forgets everything at or below `key`, after it was deleted. The cursor goes to the folder it
    /// was in, the nearest thing still there.
    fn forget(&mut self, key: &str) {
        // What the flat views list is worked out again from what changes here.
        self.revision += 1;
        // A folder a flat view shows that is taken away leaves the view in the one above it.
        if is_within(&self.shown, key) {
            self.shown = parent_key(key).to_owned();
        }
        Arc::make_mut(&mut self.details).retain(|entry, _| !is_within(entry, key));
        self.reading.retain(|entry| !is_within(entry, key));
        self.open.retain(|open| !is_within(open, key));
        self.loading.retain(|loading| !is_within(loading, key));
        self.children.retain(|folder, _| !is_within(folder, key));
        self.errors.retain(|folder, _| !is_within(folder, key));
        self.cut.retain(|cut| !is_within(cut, key));
        self.chosen.retain(|chosen| !is_within(chosen, key));
        if self.selected.as_deref().is_some_and(|selected| is_within(selected, key)) {
            let parent = parent_key(key);
            self.selected = (parent != ROOT).then(|| parent.to_owned());
        }
    }
}

/// What an action on the row `key` acts on while `chosen` is selected; see
/// [`FileManagerState::targets`]. The menu is built after the view, without the state, so it takes
/// the selection along.
pub(super) fn targets_of(chosen: &[String], key: &str) -> Vec<String> {
    let keys = if chosen.len() > 1 && chosen.iter().any(|selected| selected == key) {
        chosen.to_vec()
    } else {
        vec![key.to_owned()]
    };
    outermost(keys)
}

/// `keys` without any key inside a folder that is also among them, and without the root, in their
/// order.
pub(super) fn outermost(keys: Vec<String>) -> Vec<String> {
    let all = keys.clone();
    keys.into_iter()
        .filter(|key| key != ROOT && !all.iter().any(|other| other != key && is_within(key, other)))
        .collect()
}

/// How many names a question lists before it only counts the rest.
const NAMES_SHOWN: usize = 5;

impl FileManagerState {
    /// Reads the root the first time, and every folder on screen again after that: the files may
    /// have changed while the manager was away.
    ///
    /// Call it when the manager comes on screen. `wrap` turns the manager's messages into the
    /// application's own, as in [`update`](Self::update).
    pub fn load<Msg: Clone + Send + 'static>(
        &mut self,
        wrap: impl Fn(FileManagerMsg) -> Msg + Send + Sync + 'static,
    ) -> Command<Msg> {
        let wrap: Wrap<Msg> = Arc::new(wrap);
        self.with_wrap(&wrap, Self::load_with)
    }

    /// Applies `message` and answers with the work it asks for: folders are read and file
    /// operations run on background threads, never while drawing.
    ///
    /// `wrap` is a function such as `Msg::Manager`, or a closure that captures what it needs, such
    /// as a screen's own conversion. It is used for every message the work sends back, so it must
    /// be safe to move to another thread.
    pub fn update<Msg: Clone + Send + 'static>(
        &mut self,
        message: FileManagerMsg,
        wrap: impl Fn(FileManagerMsg) -> Msg + Send + Sync + 'static,
    ) -> Command<Msg> {
        let wrap: Wrap<Msg> = Arc::new(wrap);
        self.with_wrap(&wrap, |state, wrap| state.apply(message, wrap))
    }

    /// Runs `work` and then keeps the watch on exactly the folders on screen, whatever the work
    /// changed about them.
    fn with_wrap<Msg: Clone + Send + 'static>(
        &mut self,
        wrap: &Wrap<Msg>,
        work: impl FnOnce(&mut Self, &Wrap<Msg>) -> Command<Msg>,
    ) -> Command<Msg> {
        let command = work(self, wrap);
        let followed = super::watch::follow(self, wrap);
        Command::batch([command, followed])
    }

    fn load_with<Msg: Clone + Send + 'static>(&mut self, wrap: &Wrap<Msg>) -> Command<Msg> {
        if self.children.contains_key(ROOT) {
            let shown = self.shown_folders();
            return self.reread(shown, wrap);
        }
        if !self.loading.insert(ROOT.to_owned()) {
            return Command::none();
        }
        self.read_folder(ROOT, wrap)
    }

    /// Asks for the details of the entries `keys`: their size, when they changed last and their
    /// permissions.
    ///
    /// Only the keys nothing is known about yet are read, so asking for the same page twice costs
    /// nothing. Use this when the application knows exactly which rows it draws; a view that shows
    /// details asks for a page around the cursor by itself. Never hand it a whole folder: every
    /// key is one more call to the system, which over a remote file system is what a large folder
    /// cannot afford.
    ///
    /// The answers come back as [`FileManagerMsg::Detailed`] and are read with
    /// [`details`](Self::details).
    pub fn detail<Msg: Clone + Send + 'static>(
        &mut self,
        keys: Vec<String>,
        wrap: impl Fn(FileManagerMsg) -> Msg + Send + Sync + 'static,
    ) -> Command<Msg> {
        let wrap: Wrap<Msg> = Arc::new(wrap);
        self.read_details(keys, &wrap)
    }

    /// Asks for the details of a page of entries of the folder `folder`, around the cursor.
    ///
    /// A page of two hundred entries is always more than a screen holds and far less than a large
    /// folder, so a person scrolling rarely waits and a folder of ten thousand entries never turns
    /// into ten thousand calls to the system. The cursor decides where the page sits; a cursor
    /// somewhere else, or nowhere, starts it at the top of the folder.
    ///
    /// The views that show details use this by themselves; an application that knows exactly which
    /// rows it draws asks for those with [`detail`](Self::detail) instead.
    pub fn detail_page<Msg: Clone + Send + 'static>(
        &mut self,
        folder: &str,
        wrap: impl Fn(FileManagerMsg) -> Msg + Send + Sync + 'static,
    ) -> Command<Msg> {
        let wrap: Wrap<Msg> = Arc::new(wrap);
        self.read_page(folder, &wrap)
    }

    /// Shows the folder `key` in the flat views, reading it if it has not been read.
    ///
    /// The cursor starts at the folder's own row, so the keys go on from the top of what is now
    /// shown rather than from a row of the folder that was left.
    fn enter<Msg: Clone + Send + 'static>(&mut self, key: &str, wrap: &Wrap<Msg>) -> Command<Msg> {
        if key != ROOT && !self.is_folder(key) {
            return Command::none();
        }
        self.shown = key.to_owned();
        self.selected = None;
        self.chosen.clear();
        // The folder is opened as well as shown, so the tree and the flat views agree about where
        // the person is when the shape is changed.
        let read = self.expand(key, true);
        if read { self.read_folder(key, wrap) } else { Command::none() }
    }

    /// Shows the trash in place of the manager's own folder, and reads it.
    ///
    /// The trash is the folder [`trashing`](Self::trashing) and [`trashing_in`](Self::trashing_in)
    /// put entries in, read back the way the freedesktop specification writes it: what is in
    /// `files`, and what each entry's note in `info` says about where it came from and when it
    /// went. An entry whose note is missing or says nothing usable is listed all the same, under
    /// the name it has there.
    ///
    /// It is a place beside the root rather than a folder in it, so a row of it is named by what it
    /// is called in the trash and is put back or deleted by that name;
    /// [`FileManagerMsg::Leave`] comes back to the root, and
    /// [`FileManagerMsg::Restore`], [`FileManagerMsg::Purge`] and [`FileManagerMsg::EmptyTrash`] do
    /// what they say.
    ///
    /// Nothing is shown where there is no trash to show, and nothing is asked: a manager that
    /// deletes for good has none, and neither has a run with no home folder.
    /// [`in_trash`](Self::in_trash) says which place is shown.
    ///
    /// Reading is I/O, so it runs on a background thread, as every read of a folder does.
    pub fn open_trash<Msg: Clone + Send + 'static>(
        &mut self,
        wrap: impl Fn(FileManagerMsg) -> Msg + Send + Sync + 'static,
    ) -> Command<Msg> {
        let wrap: Wrap<Msg> = Arc::new(wrap);
        self.with_wrap(&wrap, FileManagerState::enter_trash)
    }

    /// Shows the trash and reads it, as [`open_trash`](Self::open_trash) does.
    fn enter_trash<Msg: Clone + Send + 'static>(&mut self, wrap: &Wrap<Msg>) -> Command<Msg> {
        let Some(trash) = self.trash.folder() else { return Command::none() };
        self.place = Place::Trash;
        // The cursor and the selection belong to the place that is left, so nothing carries into
        // the trash that would act on a name of the root. The filter belongs to the folder it was
        // typed in, so it goes with it.
        self.selected = None;
        self.chosen.clear();
        self.filter = None;
        self.read_trash(&trash, wrap)
    }

    /// Reads the trash folder `trash` on a background thread.
    ///
    /// It is read every time it is shown rather than once and kept: a trash holds what was deleted
    /// since, and what it holds changes while the manager looks at its own folder.
    fn read_trash<Msg: Clone + Send + 'static>(&mut self, trash: &Path, wrap: &Wrap<Msg>) -> Command<Msg> {
        let (trash, wrap) = (trash.to_path_buf(), Arc::clone(wrap));
        self.trash_reading = true;
        Command::perform(move || wrap(FileManagerMsg::Trashed(super::trash::entries(&trash))))
    }

    /// Takes the answer for the trash.
    ///
    /// What is known of an entry the trash no longer holds is let go, so a cursor or a selection
    /// cannot act on a name nothing is there for.
    fn take_trash(&mut self, entries: Result<Vec<Trashed>, FileError>) {
        // What the flat views list is worked out again from what changes here.
        self.revision += 1;
        self.trash_reading = false;
        match entries {
            Ok(entries) => {
                self.trash_problem = None;
                let held: BTreeSet<String> = entries.iter().map(|entry| entry.name.clone()).collect();
                self.trashed.retain(|name, _| held.contains(name));
                self.chosen.retain(|chosen| held.contains(chosen));
                if self.selected.as_deref().is_some_and(|selected| !held.contains(selected)) {
                    self.selected = None;
                }
                self.trashed = entries.into_iter().map(|entry| (entry.name.clone(), entry)).collect();
            }
            Err(problem) => {
                // The trash is emptied, not gone: what it held before is still on disk and still
                // put back, so the rows go rather than a reason being drawn over them.
                self.trashed.clear();
                self.selected = None;
                self.chosen.clear();
                self.trash_problem = Some(problem.message());
            }
        }
    }

    /// Comes back from the trash to the manager's own folder, and reads what is on screen again:
    /// what was deleted from another program while the trash was shown is not known yet.
    fn leave_trash<Msg: Clone + Send + 'static>(&mut self, wrap: &Wrap<Msg>) -> Command<Msg> {
        self.place = Place::Files;
        self.selected = None;
        self.chosen.clear();
        self.filter = None;
        self.refresh(wrap)
    }

    /// Puts the entries of the trash with the keys `keys` back where their notes say they came from,
    /// each on its own, so one that cannot go does not keep the others from going.
    fn restore<Msg: Clone + Send + 'static>(&self, keys: Vec<String>, wrap: &Wrap<Msg>) -> Command<Msg> {
        let keys: Vec<String> = keys.into_iter().filter(|key| self.trashed.contains_key(key)).collect();
        self.run_in_trash(keys, wrap, |trash, key| (key.to_owned(), super::trash::restore(trash, key, None)))
    }

    /// Puts the entry of the trash with the key `key` back under the name `name`, which the person
    /// was offered after its own name was found to be taken again.
    fn restore_as<Msg: Clone + Send + 'static>(&self, key: String, name: String, wrap: &Wrap<Msg>) -> Command<Msg> {
        if !self.trashed.contains_key(&key) {
            return Command::none();
        }
        self.run_in_trash(vec![key], wrap, move |trash, key| {
            (key.to_owned(), super::trash::restore(trash, key, Some(&name)))
        })
    }

    /// Deletes the entries of the trash with the keys `keys` for good, each on its own.
    fn purge<Msg: Clone + Send + 'static>(&self, keys: Vec<String>, wrap: &Wrap<Msg>) -> Command<Msg> {
        let keys: Vec<String> = keys.into_iter().filter(|key| self.trashed.contains_key(key)).collect();
        self.run_in_trash(keys, wrap, |trash, key| (key.to_owned(), super::trash::purge(trash, key)))
    }

    /// Deletes everything in the trash for good.
    ///
    /// What went and what was refused are answered together, so the trash is read again either way:
    /// an entry that did go must not be left in the list as though it were still there.
    fn empty_trash<Msg: Clone + Send + 'static>(&mut self, wrap: &Wrap<Msg>) -> Command<Msg> {
        let Some(trash) = self.trash.folder() else { return Command::none() };
        let wrap = Arc::clone(wrap);
        Command::perform(move || {
            let (gone, refused) = super::trash::empty(&trash);
            let items = gone
                .into_iter()
                .map(|name| (name.clone(), Ok(FileChange::Deleted(name))))
                // An entry that could not be deleted is named by nothing, which is what the refusal
                // says: there is no entry left to name.
                .chain(refused.into_iter().map(|problem| (String::new(), Err(problem))))
                .collect();
            wrap(FileManagerMsg::Done(items))
        })
    }

    /// The entries of a page around the cursor in the folder `folder` that nothing is known about
    /// yet and that nothing is on its way for.
    ///
    /// This is what a view showing details asks for while it draws: it is empty once the page is
    /// known or already being read, so the view asks once and then stops asking.
    #[must_use]
    pub fn detail_gaps(&self, folder: &str) -> Vec<String> {
        // The shown folder is asked about on every frame of a list, so its keys come from what the
        // flat views keep rather than being made again.
        if folder == self.shown {
            let flat = self.flat_entries();
            let at = self.selected.as_deref().and_then(|cursor| flat.position(cursor)).unwrap_or(0);
            let start = at.saturating_sub(PAGE / 2);
            return flat.rows[start.min(flat.rows.len())..]
                .iter()
                .take(PAGE)
                .filter(|row| !self.has_details(&row.key))
                .map(|row| row.key.clone())
                .collect();
        }
        let Some(entries) = self.shown_children(folder) else { return Vec::new() };
        let keys: Vec<String> = entries.iter().map(|entry| child_key(folder, &entry.name)).collect();
        let at = self.selected.as_deref().and_then(|cursor| keys.iter().position(|key| key == cursor)).unwrap_or(0);
        // The page is put around the cursor, so moving on in either direction stays inside it.
        let start = at.saturating_sub(PAGE / 2);
        keys.into_iter().skip(start).take(PAGE).filter(|key| !self.has_details(key)).collect()
    }

    /// A folder that was just read, when it is the one shown and a view asked for details before,
    /// asks for its page at once. The view asks when the person is quiet, which a read the
    /// application started does not end, so without this the rows would stay bare until a key.
    fn details_again<Msg: Clone + Send + 'static>(&mut self, key: &str, wrap: &Wrap<Msg>) -> Command<Msg> {
        if self.details_wanted && key == self.shown { self.read_page(key, wrap) } else { Command::none() }
    }

    /// The page of [`detail_page`](Self::detail_page), for the manager's own asking.
    pub(super) fn read_page<Msg: Clone + Send + 'static>(&mut self, folder: &str, wrap: &Wrap<Msg>) -> Command<Msg> {
        let page = self.detail_gaps(folder);
        self.read_details(page, wrap)
    }

    /// Reads the details of `keys` on a background thread, leaving out what is known or on its way.
    pub(super) fn read_details<Msg: Clone + Send + 'static>(
        &mut self,
        keys: Vec<String>,
        wrap: &Wrap<Msg>,
    ) -> Command<Msg> {
        let wanted: Vec<String> = keys.into_iter().filter(|key| key != ROOT && !self.has_details(key)).collect();
        if wanted.is_empty() {
            return Command::none();
        }
        for key in &wanted {
            self.reading.insert(key.clone());
        }
        let (root, wrap) = (self.root.clone(), Arc::clone(wrap));
        Command::perform(move || {
            let read = wanted
                .iter()
                .map(|key| {
                    let path =
                        key.split('/').filter(|part| !part.is_empty()).fold(root.clone(), |at, part| at.join(part));
                    (key.clone(), FileDetails::read(&path))
                })
                .collect();
            wrap(FileManagerMsg::Detailed(read))
        })
    }

    /// Reads one folder on a background thread.
    fn read_folder<Msg: Clone + Send + 'static>(&self, key: &str, wrap: &Wrap<Msg>) -> Command<Msg> {
        let (key, path, wrap) = (key.to_owned(), self.path(key), Arc::clone(wrap));
        Command::perform(move || wrap(FileManagerMsg::Listed(key.clone(), FolderEntry::list(&path))))
    }

    /// Reads the folders `keys` again, each while it is still shown.
    pub(super) fn reread<Msg: Clone + Send + 'static>(&mut self, keys: Vec<String>, wrap: &Wrap<Msg>) -> Command<Msg> {
        let keys: Vec<String> = keys.into_iter().filter(|key| key == ROOT || self.is_open(key)).collect();
        let commands: Vec<Command<Msg>> = keys
            .into_iter()
            .map(|key| {
                self.loading.insert(key.clone());
                self.read_folder(&key, wrap)
            })
            .collect();
        Command::batch(commands)
    }

    /// Reads again every folder on screen: when the person asks, when the manager is returned to,
    /// and when a watch says anything may have changed.
    pub(super) fn refresh<Msg: Clone + Send + 'static>(&mut self, wrap: &Wrap<Msg>) -> Command<Msg> {
        let shown = self.shown_folders();
        self.reread(shown, wrap)
    }

    fn apply<Msg: Clone + Send + 'static>(&mut self, message: FileManagerMsg, wrap: &Wrap<Msg>) -> Command<Msg> {
        // The trash is beside the root, so every message that acts on a key below the root is
        // refused while it is shown: a name of the trash is no key, and handing one to an operation
        // that works below the root would act on whatever the root holds under that name. What can
        // be done to an entry of the trash is asked of it by name instead.
        if self.place == Place::Trash
            && matches!(
                message,
                FileManagerMsg::Expand(_, _)
                    | FileManagerMsg::NewFile(_)
                    | FileManagerMsg::NewFolder(_)
                    | FileManagerMsg::Rename(_)
                    | FileManagerMsg::Cut(_)
                    | FileManagerMsg::Copy(_)
                    | FileManagerMsg::Paste(_)
                    | FileManagerMsg::Drop(_)
                    | FileManagerMsg::DropCopy(_)
                    | FileManagerMsg::Delete(_)
                    | FileManagerMsg::DeleteConfirmed(_)
                    | FileManagerMsg::Trash(_)
                    | FileManagerMsg::Enter(_)
            )
        {
            return Command::none();
        }
        match message {
            FileManagerMsg::Select(key) => {
                self.selected = Some(key);
                Command::none()
            }
            FileManagerMsg::Filter(text) => {
                self.set_filter(text);
                Command::none()
            }
            FileManagerMsg::FilterChosen => {
                if let Some(first) = self.flat_entries().rows.first() {
                    self.selected = Some(first.key.clone());
                }
                Command::none()
            }
            FileManagerMsg::Sort(sort) => {
                self.sort = sort;
                // The trash is a place beside the root and not below it, so a key there is not a
                // path to read: there is no size to ask the system about and the deletion date a
                // note carries is already known.
                if !sort.needs_details() || self.place == Place::Trash {
                    return Command::none();
                }
                // An order by size or date is an order of the whole folder, so the whole folder is
                // read for it, once: what is known already costs nothing.
                let keys = self.flat_entries().rows.iter().map(|row| row.key.clone()).collect();
                self.read_details(keys, wrap)
            }
            FileManagerMsg::Choose(keys) => {
                self.chosen = keys;
                Command::none()
            }
            FileManagerMsg::Expand(key, open) => {
                if !self.expand(&key, open) {
                    return Command::none();
                }
                self.read_folder(&key, wrap)
            }
            FileManagerMsg::Read(key, entries) => {
                self.take_read(&key, entries);
                self.details_again(&key, wrap)
            }
            FileManagerMsg::Listed(key, entries) => {
                self.take_read(&key, entries.map_err(|problem| problem.message()));
                self.details_again(&key, wrap)
            }
            FileManagerMsg::NewFile(folder) => self.ask_name(NameFor::File, folder, String::new(), wrap),
            FileManagerMsg::NewFolder(folder) => self.ask_name(NameFor::Folder, folder, String::new(), wrap),
            FileManagerMsg::Rename(key) => {
                let (folder, name) = (parent_key(&key).to_owned(), name_of(&key).to_owned());
                self.ask_name(NameFor::Rename(key), folder, name, wrap)
            }
            FileManagerMsg::Cut(key) => {
                self.cut = self.targets(&key);
                self.copying = false;
                Command::none()
            }
            FileManagerMsg::Copy(key) => {
                self.cut = self.targets(&key);
                self.copying = true;
                Command::none()
            }
            FileManagerMsg::DropCut => {
                self.cut.clear();
                self.copying = false;
                Command::none()
            }
            FileManagerMsg::Paste(into) => {
                let keys = self.cut.clone();
                if self.copying {
                    return self.copy_all(keys, into, wrap);
                }
                self.move_all(keys, into, wrap)
            }
            FileManagerMsg::Drop(TreeDrop { keys, into }) => {
                self.move_all(outermost(keys), into.unwrap_or_default(), wrap)
            }
            FileManagerMsg::DropCopy(TreeDrop { keys, into }) => {
                self.copy_all(outermost(keys), into.unwrap_or_default(), wrap)
            }
            FileManagerMsg::Delete(key) => self.ask_delete(self.targets(&key), wrap),
            FileManagerMsg::DeleteConfirmed(keys) => {
                let confined = self.confined;
                self.run_each(keys, wrap, move |root, key| (key.to_owned(), ops::delete(root, key, confined)))
            }
            FileManagerMsg::Trash(key) => {
                let keys = self.targets(&key);
                self.trash_all(keys, wrap)
            }
            FileManagerMsg::OpenTrash => self.enter_trash(wrap),
            FileManagerMsg::Trashed(entries) => {
                self.take_trash(entries);
                Command::none()
            }
            FileManagerMsg::Restore(keys) => self.restore(keys, wrap),
            FileManagerMsg::RestoreAs(key, name) => self.restore_as(key, name, wrap),
            FileManagerMsg::Purge(keys) => self.ask_purge(keys, wrap),
            FileManagerMsg::PurgeConfirmed(keys) => self.purge(keys, wrap),
            FileManagerMsg::EmptyTrash => self.ask_empty(wrap),
            FileManagerMsg::EmptyTrashConfirmed => self.empty_trash(wrap),
            FileManagerMsg::ShowHidden(showing) => {
                self.hidden = showing;
                Command::none()
            }
            FileManagerMsg::Work(event) => self.took_event(event, wrap),
            FileManagerMsg::Stop => match &self.work {
                Some(work) => Command::cancel_task(work.id),
                None => Command::none(),
            },
            FileManagerMsg::Refresh if self.place == Place::Trash => match self.trash.folder() {
                Some(trash) => self.read_trash(&trash, wrap),
                None => Command::none(),
            },
            FileManagerMsg::Refresh => self.refresh(wrap),
            FileManagerMsg::Name(value) => {
                if let Some(naming) = &mut self.naming {
                    naming.value = value;
                }
                Command::none()
            }
            FileManagerMsg::Submit => self.submit(wrap),
            FileManagerMsg::CloseNaming => {
                self.naming = None;
                Command::none()
            }
            FileManagerMsg::Done(results) => self.done(results, wrap),
            FileManagerMsg::Changed(run, batch) => super::watch::changed(self, run, batch, wrap),
            FileManagerMsg::Quiet(run) => super::watch::quiet(self, run, wrap),
            FileManagerMsg::Enter(key) => self.enter(&key, wrap),
            FileManagerMsg::Leave => {
                if self.place == Place::Trash {
                    return self.leave_trash(wrap);
                }
                if self.shown == ROOT {
                    return Command::none();
                }
                let left = self.shown.clone();
                let up = parent_key(&left).to_owned();
                let command = self.enter(&up, wrap);
                // The cursor lands on the folder that was left, which is where the eye already is.
                self.selected = Some(left);
                command
            }
            FileManagerMsg::Detail(keys) => {
                self.details_wanted = true;
                self.read_details(keys, wrap)
            }
            FileManagerMsg::Detailed(read) => {
                for (key, details) in read {
                    self.reading.remove(&key);
                    Arc::make_mut(&mut self.details).insert(key, details);
                }
                self.details_revision += 1;
                Command::none()
            }
        }
    }

    /// Opens the dialog asking for a name in `folder`, opening the folder too: the new entry will
    /// be shown there, and the names already in it are what the name is checked against.
    fn ask_name<Msg: Clone + Send + 'static>(
        &mut self,
        purpose: NameFor,
        folder: String,
        value: String,
        wrap: &Wrap<Msg>,
    ) -> Command<Msg> {
        let read = folder != ROOT && self.expand(&folder, true);
        let command = if read { self.read_folder(&folder, wrap) } else { Command::none() };
        self.naming = Some(Naming { purpose, folder, value, tried: false });
        command
    }

    /// Takes the name in the dialog, or points out what is wrong with it.
    fn submit<Msg: Clone + Send + 'static>(&mut self, wrap: &Wrap<Msg>) -> Command<Msg> {
        let Some(naming) = self.naming.as_mut() else { return Command::none() };
        naming.tried = true;
        if self.naming_problem().is_some() {
            return Command::none();
        }
        let Some(Naming { purpose, folder, value: name, .. }) = self.naming.take() else { return Command::none() };
        let confined = self.confined;
        match purpose {
            NameFor::File => self.run_each(vec![folder], wrap, move |root, folder| {
                (child_key(folder, &name), ops::create_file(root, folder, &name, confined))
            }),
            NameFor::Folder => self.run_each(vec![folder], wrap, move |root, folder| {
                (child_key(folder, &name), ops::create_folder(root, folder, &name, confined))
            }),
            // The same name again is nothing to do, not a clash with itself.
            NameFor::Rename(key) if name_of(&key) == name => Command::none(),
            NameFor::Rename(key) => self
                .run_each(vec![key], wrap, move |root, key| (key.to_owned(), ops::rename(root, key, &name, confined))),
        }
    }

    /// Moves the entries `keys` into the folder `into`, each on its own: one that cannot go does
    /// not keep the others from going, and the answer says which stayed and why.
    fn move_all<Msg: Clone + Send + 'static>(
        &mut self,
        keys: Vec<String>,
        into: String,
        wrap: &Wrap<Msg>,
    ) -> Command<Msg> {
        if keys.is_empty() {
            return Command::none();
        }
        let confined = self.confined;
        self.run_each(keys, wrap, move |root, key| (key.to_owned(), ops::move_into(root, key, &into, confined)))
    }

    /// Copies the entries `keys` into the folder `into`, each on its own, as a background task
    /// that says how far it has come and can be stopped.
    ///
    /// A copy is the one operation with no upper bound: a rename is instant whatever it moves,
    /// while a folder of photographs is read and written byte by byte. So this does not hold a
    /// thread until it is over and hope it is quick; it is a task like a build or a download, with
    /// a share done and a way to say stop.
    ///
    /// One copy runs at a time: starting another while one is going would make two progress bars
    /// for one row of the manager, so a copy asked for while one runs is refused by doing nothing.
    fn copy_all<Msg: Clone + Send + 'static>(
        &mut self,
        keys: Vec<String>,
        into: String,
        wrap: &Wrap<Msg>,
    ) -> Command<Msg> {
        if keys.is_empty() || self.work.is_some() {
            return Command::none();
        }
        let (confined, root, count) = (self.confined, self.root.clone(), keys.len());
        let label = crate::t!("quvyta.file-manager.copying", n = count);
        let finish = Arc::clone(wrap);
        let events = Arc::clone(wrap);
        let task = Task::new(label, move |cx| {
            let mut results = Vec::new();
            let step = 1.0 / count as f32;
            for (index, key) in keys.iter().enumerate() {
                cx.note(name_of(key).to_owned());
                let base = index as f32 * step;
                let progress = |done: u64, total: u64| {
                    let share = if total == 0 { 1.0 } else { done as f32 / total as f32 };
                    cx.progress(base + share * step);
                };
                let stopped = || cx.is_cancelled();
                let watch = ops::Watch { progress: &progress, stopped: &stopped };
                results.push((key.clone(), ops::copy_watched(&root, key, &into, confined, &watch)));
                if cx.is_cancelled() {
                    // What was copied before the person said stop stays; the half-written one was
                    // taken away again by the copy itself.
                    break;
                }
            }
            Ok(finish(FileManagerMsg::Done(results)))
        })
        .on_event(move |event| events(FileManagerMsg::Work(event)));
        self.work = Some(FileWork { id: task.id(), entries: count, done: 0.0, note: String::new() });
        Command::task(task)
    }

    /// Takes what the running operation said about itself.
    ///
    /// A task that was stopped never delivers its result, so the folders it touched are read again
    /// here: what it had already copied is on disk and belongs on the screen.
    fn took_event<Msg: Clone + Send + 'static>(&mut self, event: TaskEvent, wrap: &Wrap<Msg>) -> Command<Msg> {
        let Some(work) = &mut self.work else { return Command::none() };
        if event.id() != work.id {
            return Command::none();
        }
        match event {
            TaskEvent::Started { .. } => Command::none(),
            TaskEvent::Progress { fraction, note, .. } => {
                if let Some(fraction) = fraction {
                    work.done = fraction;
                }
                if let Some(note) = note {
                    work.note = note;
                }
                Command::none()
            }
            TaskEvent::Finished { outcome, .. } => {
                self.work = None;
                match outcome {
                    TaskOutcome::Cancelled => self.refresh(wrap),
                    // A task that ended by itself delivered its `Done` first, which read the
                    // folders it touched already.
                    TaskOutcome::Done | TaskOutcome::Failed(_) => Command::none(),
                }
            }
        }
    }

    /// Puts the entries `keys` in the trash, each on its own.
    ///
    /// Nothing is asked first: the trash can be looked in again, and a question about something
    /// that can be undone is a question not worth asking. An entry the trash cannot take is not
    /// deleted quietly: [`done`](Self::done) asks about that one on its own.
    fn trash_all<Msg: Clone + Send + 'static>(&mut self, keys: Vec<String>, wrap: &Wrap<Msg>) -> Command<Msg> {
        let Some(trash) = self.trash.folder() else {
            // No trash at all is the same answer for every entry, and the question comes at once.
            return self.ask_delete_forever(keys, wrap);
        };
        if keys.is_empty() {
            return Command::none();
        }
        let confined = self.confined;
        self.run_each(keys, wrap, move |root, key| (key.to_owned(), ops::to_trash(&trash, root, key, confined)))
    }

    /// Asks before deleting, which cannot be undone; a folder says it takes everything in it along.
    fn ask_delete<Msg: Clone + Send + 'static>(&self, keys: Vec<String>, wrap: &Wrap<Msg>) -> Command<Msg> {
        let folders = keys.iter().any(|key| self.is_folder(key));
        let (title, message) = match keys.as_slice() {
            [] => return Command::none(),
            [key] => {
                let name = name_of(key);
                let text = if folders {
                    "quvyta.file-manager.delete-folder-text"
                } else {
                    "quvyta.file-manager.delete-file-text"
                };
                (crate::t!("quvyta.file-manager.delete-title", name = name), crate::t!(text, name = name))
            }
            many => {
                let mut names = many.iter().take(NAMES_SHOWN).map(|key| name_of(key)).collect::<Vec<_>>().join(", ");
                if many.len() > NAMES_SHOWN {
                    names =
                        format!("{names} {}", crate::t!("quvyta.file-manager.and-more", n = many.len() - NAMES_SHOWN));
                }
                let text = if folders {
                    "quvyta.file-manager.delete-many-folders-text"
                } else {
                    "quvyta.file-manager.delete-many-text"
                };
                (
                    crate::t!("quvyta.file-manager.delete-many-title", n = many.len()),
                    crate::t!(text, names = names.as_str()),
                )
            }
        };
        let label = if keys.len() > 1 {
            crate::t!("quvyta.file-manager.delete-many", n = keys.len())
        } else {
            crate::t!("quvyta.file-manager.delete")
        };
        let confirmed = wrap(FileManagerMsg::DeleteConfirmed(keys));
        Command::confirm(Confirm::new(title, confirmed).message(message).confirm_label(label).danger())
    }

    /// Asks whether to delete for good what the trash could not take, saying why the trash was no
    /// use: on another file system, or there is none.
    fn ask_delete_forever<Msg: Clone + Send + 'static>(&self, keys: Vec<String>, wrap: &Wrap<Msg>) -> Command<Msg> {
        let (title, message) = match keys.as_slice() {
            [] => return Command::none(),
            [key] => {
                let name = name_of(key);
                (
                    crate::t!("quvyta.file-manager.no-trash-title", name = name),
                    crate::t!("quvyta.file-manager.no-trash-text", name = name),
                )
            }
            many => (
                crate::t!("quvyta.file-manager.no-trash-many-title", n = many.len()),
                crate::t!("quvyta.file-manager.no-trash-many-text"),
            ),
        };
        let confirmed = wrap(FileManagerMsg::DeleteConfirmed(keys));
        let label = crate::t!("quvyta.file-manager.delete-forever");
        Command::confirm(Confirm::new(title, confirmed).message(message).confirm_label(label).danger())
    }

    /// Asks whether to put an entry of the trash back under another name, because its own name is
    /// taken again where it came from.
    ///
    /// Nothing there is overwritten either way, which is why this is not a question of danger: the
    /// entry goes back either not at all or under the name offered here.
    fn ask_restore<Msg: Clone + Send + 'static>(&self, key: &str, free: &str, wrap: &Wrap<Msg>) -> Command<Msg> {
        let Some(entry) = self.trashed.get(key) else { return Command::none() };
        let (title, message) = (
            crate::t!("quvyta.file-manager.restore-clash-title", name = entry.label.as_str()),
            crate::t!("quvyta.file-manager.restore-clash-text", free = free),
        );
        let label = crate::t!("quvyta.file-manager.restore-as", name = free);
        let confirmed = wrap(FileManagerMsg::RestoreAs(key.to_owned(), free.to_owned()));
        Command::confirm(Confirm::new(title, confirmed).message(message).confirm_label(label))
    }

    /// Asks before deleting entries of the trash for good: there is no trash behind this one.
    ///
    /// A folder says it takes everything in it along, as a delete of a folder of the root does.
    fn ask_purge<Msg: Clone + Send + 'static>(&self, keys: Vec<String>, wrap: &Wrap<Msg>) -> Command<Msg> {
        let held: Vec<&Trashed> = keys.iter().filter_map(|key| self.trashed.get(key)).collect();
        let (title, message) = match held.as_slice() {
            [] => return Command::none(),
            [entry] => (
                crate::t!("quvyta.file-manager.purge-title", name = entry.label.as_str()),
                crate::t!("quvyta.file-manager.purge-text", name = entry.label.as_str()),
            ),
            many => (
                crate::t!("quvyta.file-manager.purge-many-title", n = many.len()),
                crate::t!("quvyta.file-manager.purge-many-text", n = many.len()),
            ),
        };
        let label = if held.len() > 1 {
            crate::t!("quvyta.file-manager.delete-permanently-many", n = held.len())
        } else {
            crate::t!("quvyta.file-manager.delete-permanently")
        };
        let confirmed = wrap(FileManagerMsg::PurgeConfirmed(keys));
        Command::confirm(Confirm::new(title, confirmed).message(message).confirm_label(label).danger())
    }

    /// Asks before emptying the trash: every entry in it goes for good, and there is nothing behind
    /// this one to put them back.
    fn ask_empty<Msg: Clone + Send + 'static>(&self, wrap: &Wrap<Msg>) -> Command<Msg> {
        if self.trashed.is_empty() {
            return Command::none();
        }
        let title = crate::t!("quvyta.file-manager.empty-title");
        let message = crate::t!("quvyta.file-manager.empty-text", n = self.trashed.len());
        let label = crate::t!("quvyta.file-manager.empty-trash");
        let confirmed = wrap(FileManagerMsg::EmptyTrashConfirmed);
        Command::confirm(Confirm::new(title, confirmed).message(message).confirm_label(label).danger())
    }

    /// Runs a file operation for each of `items` on a background thread, one after the other: a
    /// move of several entries is done in the order they were given, so a clash between two of
    /// them is decided the same way every time.
    fn run_each<Msg: Clone + Send + 'static>(
        &self,
        items: Vec<String>,
        wrap: &Wrap<Msg>,
        work: impl Fn(&Path, &str) -> (String, Result<FileChange, FileError>) + Send + 'static,
    ) -> Command<Msg> {
        let (root, wrap) = (self.root.clone(), Arc::clone(wrap));
        Command::perform(move || wrap(FileManagerMsg::Done(items.iter().map(|item| work(&root, item)).collect())))
    }

    /// Runs a trash operation for each of `items` on a background thread, one after the other.
    ///
    /// The folder every operation works in is the trash's own rather than the root's, and the items
    /// are the names the entries have there. Where there is no trash, nothing is done: there is
    /// nothing to restore and nothing to empty.
    fn run_in_trash<Msg: Clone + Send + 'static>(
        &self,
        items: Vec<String>,
        wrap: &Wrap<Msg>,
        work: impl Fn(&Path, &str) -> (String, Result<FileChange, FileError>) + Send + 'static,
    ) -> Command<Msg> {
        let Some(trash) = self.trash.folder() else { return Command::none() };
        if items.is_empty() {
            return Command::none();
        }
        let wrap = Arc::clone(wrap);
        Command::perform(move || wrap(FileManagerMsg::Done(items.iter().map(|item| work(&trash, item)).collect())))
    }

    /// Takes what the operations changed: the manager follows it, the folders they touched are
    /// read again and the selection goes to what was made or moved. What was refused is said,
    /// entry by entry when there were several.
    fn done<Msg: Clone + Send + 'static>(
        &mut self,
        results: Vec<(String, Result<FileChange, FileError>)>,
        wrap: &Wrap<Msg>,
    ) -> Command<Msg> {
        let total = results.len();
        let mut touched = Vec::new();
        let mut arrived = Vec::new();
        let mut refused = Vec::new();
        let mut without_trash = Vec::new();
        let mut clashes = Vec::new();
        let mut emptied = false;
        for (key, result) in results {
            match result {
                // The trash could not take it, so the person is asked about that one entry instead
                // of being told off: it is a limit of the trash, not a mistake of theirs.
                Err(FileError::NoTrash) => without_trash.push(key),
                // A name that is taken again is not a refusal but a question with a name that is
                // free there: nothing of the person's is overwritten either way. One entry on its own
                // is asked about; several are refused with the reason, because a question is one
                // dialog and a name is given to one entry at a time.
                Err(FileError::TakenAs(free)) if total == 1 && self.trashed.contains_key(&key) => {
                    clashes.push((key, free));
                }
                Err(error) => refused.push((key, error)),
                Ok(FileChange::Copied(key)) => {
                    // The copy is selected, so the folder it went into opens to show it, as a
                    // move's does; a cursor on a hidden row is lost to the keys.
                    let parent = parent_key(&key).to_owned();
                    if parent != ROOT {
                        self.open.insert(parent.clone());
                    }
                    touched.push(parent);
                    arrived.push(key);
                }
                Ok(FileChange::Trashed(key)) => {
                    self.forget(&key);
                    touched.push(parent_key(&key).to_owned());
                }
                Ok(FileChange::Created(key)) => {
                    touched.push(parent_key(&key).to_owned());
                    arrived.push(key);
                }
                Ok(FileChange::Moved(from, to)) => {
                    // A cut entry that went somewhere is used up; one that was refused stays cut,
                    // to be tried elsewhere.
                    self.cut.retain(|cut| *cut != from);
                    if from == to {
                        continue;
                    }
                    self.rekey(&from, &to);
                    let parent = parent_key(&to).to_owned();
                    if parent != ROOT {
                        self.open.insert(parent.clone());
                    }
                    touched.extend([parent_key(&from).to_owned(), parent]);
                    arrived.push(to);
                }
                Ok(FileChange::Deleted(key)) => {
                    let purged = self.trashed.remove(&key).is_some();
                    self.forget(&key);
                    // A purge takes the entry out of the trash rather than out of a folder of the
                    // root, so it is the trash that is read again and not the root behind it.
                    if purged {
                        emptied = true;
                    } else {
                        touched.push(parent_key(&key).to_owned());
                    }
                }
                Ok(FileChange::Restored(key)) => {
                    // The entry went back to where its note said, which is often outside the root
                    // and sometimes inside it, so the folder it came back to is read again only when
                    // that is where it landed.
                    let back_inside = self
                        .trashed
                        .get(&key)
                        .and_then(|entry| entry.origin.as_deref())
                        .is_some_and(|origin| origin.starts_with(&self.root));
                    self.trashed.remove(&key);
                    self.forget(&key);
                    emptied = true;
                    if back_inside {
                        touched.push(ROOT.to_owned());
                    }
                }
            }
        }
        if let Some(first) = arrived.first() {
            self.selected = Some(first.clone());
            self.chosen = arrived;
        }
        touched.sort();
        touched.dedup();
        // An entry the trash could not take, and one whose name is taken again, are asked about rather
        // than counted among the refusals.
        let total = total - without_trash.len() - clashes.len();
        let asked = self.ask_delete_forever(without_trash, wrap);
        let about_name = Command::batch(clashes.iter().map(|(key, free)| self.ask_restore(key, free, wrap)));
        let read = match self.trash.folder() {
            Some(trash) if emptied => self.read_trash(&trash, wrap),
            _ => Command::none(),
        };
        Command::batch([refusal(total, &refused), asked, about_name, self.reread(touched, wrap), read])
    }
}

/// Says what was refused: the reason alone when there was one entry, and which entries stayed with
/// each one's reason when there were several.
fn refusal<Msg: Clone + Send + 'static>(total: usize, refused: &[(String, FileError)]) -> Command<Msg> {
    match refused {
        [] => Command::none(),
        [(_, error)] if total == 1 => {
            Command::toast(Toast::danger(crate::t!("quvyta.file-manager.failed")).body(error.message()))
        }
        _ => {
            let lines: Vec<String> =
                refused.iter().map(|(key, error)| format!("{}: {}", name_of(key), error.message())).collect();
            let title = crate::t!("quvyta.file-manager.failed-some", failed = refused.len(), total = total);
            Command::toast(Toast::danger(title).body(lines.join("\n")))
        }
    }
}
