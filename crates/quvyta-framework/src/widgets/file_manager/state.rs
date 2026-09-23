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
use super::ops::{self, FileChange, FileError, NameProblem, is_within, name_of, parent_key};
use super::watch::Live;

/// The key of the folder the manager is rooted at. Keys below it are paths relative to the root,
/// written with `/` whatever the platform, because they are identities rather than paths.
pub const ROOT: &str = "";

/// The key of the entry `name` inside the folder `parent`.
#[must_use]
pub fn child_key(parent: &str, name: &str) -> String {
    if parent.is_empty() { name.to_owned() } else { format!("{parent}/{name}") }
}

/// One entry of a folder, as a file manager reads it: a name and whether it can be opened.
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
            entries.push(Self { name, folder });
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
            entries.push(Self { name, folder });
        }
        entries.sort_by(|a, b| b.folder.cmp(&a.folder).then_with(|| a.name.cmp(&b.name)));
        Ok(entries)
    }
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
    /// The entry of this key was asked to be deleted, with the rest of the selection when it is
    /// part of it; the person is asked first.
    Delete(String),
    /// The person said yes to deleting the entries of these keys.
    DeleteConfirmed(Vec<String>),
    /// The entry of this key was asked to go to the trash, with the rest of the selection when it
    /// is part of it. Nothing is asked: the trash can be looked in again.
    Trash(String),
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
    /// whole folder: a folder of ten thousand entries must not become ten thousand calls.
    details: BTreeMap<String, Option<FileDetails>>,
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
    hidden: bool,
    work: Option<FileWork>,
    naming: Option<Naming>,
    pub(super) live: Live,
    /// Counts the watches started, so a batch of one that was let go is recognised.
    pub(super) runs: u64,
    /// How long one wait for outside changes may last; `None` waits until something changes.
    pub(super) patience: Option<std::time::Duration>,
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
            details: BTreeMap::new(),
            reading: BTreeSet::new(),
            open: BTreeSet::from([ROOT.to_owned()]),
            loading: BTreeSet::new(),
            selected: None,
            chosen: Vec::new(),
            errors: BTreeMap::new(),
            cut: Vec::new(),
            copying: false,
            trash: Trash::Off,
            hidden: false,
            work: None,
            naming: None,
            live: Live::Off,
            runs: 0,
            patience: None,
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
                self.details.retain(|entry, _| parent_key(entry) != key);
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
        let moved = |key: &str| is_within(key, from).then(|| format!("{to}{}", &key[from.len()..]));
        self.open = self.open.iter().map(|key| moved(key).unwrap_or_else(|| key.clone())).collect();
        self.loading.retain(|key| !is_within(key, from));
        self.children = std::mem::take(&mut self.children)
            .into_iter()
            .map(|(key, entries)| (moved(&key).unwrap_or(key), entries))
            .collect();
        self.details = std::mem::take(&mut self.details)
            .into_iter()
            .map(|(key, details)| (moved(&key).unwrap_or(key), details))
            .collect();
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
        // A folder a flat view shows that is taken away leaves the view in the one above it.
        if is_within(&self.shown, key) {
            self.shown = parent_key(key).to_owned();
        }
        self.details.retain(|entry, _| !is_within(entry, key));
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

    /// The entries of a page around the cursor in the folder `folder` that nothing is known about
    /// yet and that nothing is on its way for.
    ///
    /// This is what a view showing details asks for while it draws: it is empty once the page is
    /// known or already being read, so the view asks once and then stops asking.
    #[must_use]
    pub fn detail_gaps(&self, folder: &str) -> Vec<String> {
        let Some(entries) = self.shown_children(folder) else { return Vec::new() };
        let keys: Vec<String> = entries.iter().map(|entry| child_key(folder, &entry.name)).collect();
        let at = self.selected.as_deref().and_then(|cursor| keys.iter().position(|key| key == cursor)).unwrap_or(0);
        // The page is put around the cursor, so moving on in either direction stays inside it.
        let start = at.saturating_sub(PAGE / 2);
        keys.into_iter().skip(start).take(PAGE).filter(|key| !self.has_details(key)).collect()
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
        match message {
            FileManagerMsg::Select(key) => {
                self.selected = Some(key);
                Command::none()
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
                Command::none()
            }
            FileManagerMsg::Listed(key, entries) => {
                self.take_read(&key, entries.map_err(|problem| problem.message()));
                Command::none()
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
            FileManagerMsg::Delete(key) => self.ask_delete(self.targets(&key), wrap),
            FileManagerMsg::DeleteConfirmed(keys) => {
                let confined = self.confined;
                self.run_each(keys, wrap, move |root, key| (key.to_owned(), ops::delete(root, key, confined)))
            }
            FileManagerMsg::Trash(key) => {
                let keys = self.targets(&key);
                self.trash_all(keys, wrap)
            }
            FileManagerMsg::ShowHidden(showing) => {
                self.hidden = showing;
                Command::none()
            }
            FileManagerMsg::Work(event) => self.took_event(event, wrap),
            FileManagerMsg::Stop => match &self.work {
                Some(work) => Command::cancel_task(work.id),
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
            FileManagerMsg::Detail(keys) => self.read_details(keys, wrap),
            FileManagerMsg::Detailed(read) => {
                for (key, details) in read {
                    self.reading.remove(&key);
                    self.details.insert(key, details);
                }
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
        for (key, result) in results {
            match result {
                // The trash could not take it, so the person is asked about that one entry instead
                // of being told off: it is a limit of the trash, not a mistake of theirs.
                Err(FileError::NoTrash) => without_trash.push(key),
                Err(error) => refused.push((key, error)),
                Ok(FileChange::Copied(key)) => {
                    touched.push(parent_key(&key).to_owned());
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
                    self.forget(&key);
                    touched.push(parent_key(&key).to_owned());
                }
            }
        }
        if let Some(first) = arrived.first() {
            self.selected = Some(first.clone());
            self.chosen = arrived;
        }
        touched.sort();
        touched.dedup();
        // An entry the trash could not take is asked about, not counted among the refusals.
        let total = total - without_trash.len();
        let asked = self.ask_delete_forever(without_trash, wrap);
        Command::batch([refusal(total, &refused), asked, self.reread(touched, wrap)])
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
