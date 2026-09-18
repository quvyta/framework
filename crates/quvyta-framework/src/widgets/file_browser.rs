//! The state behind [`FilePicker`](super::FilePicker): where it is, what the folder holds,
//! filters and the selection, with folder reading done off the render path.

use std::cmp::Ordering;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::runtime::Command;

/// Whether a picker chooses files or folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickMode {
    /// Choose a file; folders are opened.
    #[default]
    Files,
    /// Choose a folder; files are shown faint.
    Folders,
}

/// One entry of a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    name: String,
    folder: bool,
    size: Option<u64>,
}

impl FileEntry {
    /// The file or folder name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the entry is a folder (links to folders count).
    #[must_use]
    pub fn is_folder(&self) -> bool {
        self.folder
    }

    /// The size of a file in bytes, when it could be read.
    #[must_use]
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// Whether the name starts with a dot.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }
}

/// Why a folder could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingError {
    /// The user may not read it.
    PermissionDenied,
    /// It does not exist (any more).
    NotFound,
    /// The path is not a folder.
    NotAFolder,
    /// Anything else, with the system's message.
    Other(String),
}

impl ListingError {
    fn from_io(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::NotADirectory => Self::NotAFolder,
            _ => Self::Other(error.to_string()),
        }
    }
}

/// What reading a folder gave.
pub type Listing = Result<Arc<[FileEntry]>, ListingError>;

/// Reads the entries of `folder`, folders first, then by name ignoring case. Blocks on the file
/// system: call it from [`Command::perform`], never from `view`.
///
/// # Errors
///
/// Returns why the folder could not be read.
pub fn read_folder(folder: &Path) -> Listing {
    let entries = std::fs::read_dir(folder).map_err(|error| ListingError::from_io(&error))?;
    let mut out: Vec<FileEntry> = entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            // `fs::metadata` follows links, so a link to a folder opens like a folder.
            let metadata = std::fs::metadata(&path).or_else(|_| entry.metadata()).ok();
            let folder = metadata.as_ref().is_some_and(std::fs::Metadata::is_dir);
            FileEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                folder,
                size: metadata.filter(|_| !folder).map(|m| m.len()),
            }
        })
        .collect();
    out.sort_by(|a, b| match (a.folder, b.folder) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name)),
    });
    Ok(out.into())
}

/// Something that happened in a [`FilePicker`](super::FilePicker). Hand every message to
/// [`FileBrowser::update`], except [`FilePickerMsg::Chosen`], which is the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePickerMsg {
    /// Go to a folder.
    Open(PathBuf),
    /// A folder was read.
    Loaded(PathBuf, Listing),
    /// The selection moved to the entry with this name; `None` is the parent row.
    Select(Option<String>),
    /// The filter text changed.
    Filter(String),
    /// Hidden entries were shown or hidden.
    ShowHidden(bool),
    /// Read the current folder again.
    Refresh,
    /// The user chose this path.
    Chosen(PathBuf),
}

/// What the shown folder holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FolderState {
    /// No read has answered yet.
    Unread,
    Ready(Arc<[FileEntry]>),
    Failed(ListingError),
}

/// The application-owned state of a [`FilePicker`](super::FilePicker).
///
/// Create it with a start folder, start reading with [`FileBrowser::open`] and pass every
/// [`FilePickerMsg`] to [`FileBrowser::update`]; both return the command that reads folders on
/// a background thread.
///
/// While a folder is being read, everything shown stays as it was: the folder, its entries, the
/// filter and the selection change together only when the answer arrives, so quick reads never
/// flash an empty or loading state. [`FileBrowser::loading`] tells which folder is on its way.
#[derive(Debug, Clone)]
pub struct FileBrowser {
    pub(crate) folder: PathBuf,
    pub(crate) state: FolderState,
    /// The folder being read, until its answer arrives.
    pub(crate) loading: Option<PathBuf>,
    pub(crate) mode: PickMode,
    pub(crate) extensions: Vec<String>,
    pub(crate) show_hidden: bool,
    pub(crate) filter: String,
    pub(crate) selected: Option<String>,
}

impl FileBrowser {
    /// A browser at `folder` choosing in `mode`. Nothing is read until [`FileBrowser::open`].
    #[must_use]
    pub fn new(folder: impl Into<PathBuf>, mode: PickMode) -> Self {
        Self {
            folder: folder.into(),
            state: FolderState::Unread,
            loading: None,
            mode,
            extensions: Vec::new(),
            show_hidden: false,
            filter: String::new(),
            selected: None,
        }
    }

    /// Shows only files with these extensions (without the dot, any case); folders always show.
    #[must_use]
    pub fn extensions(mut self, extensions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.extensions = extensions.into_iter().map(|e| e.into().to_lowercase()).collect();
        self
    }

    /// The folder shown. While another folder is read it stays the one shown until the answer
    /// arrives.
    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The folder being read, from [`FileBrowser::open`] until its answer arrives.
    #[must_use]
    pub fn loading(&self) -> Option<&Path> {
        self.loading.as_deref()
    }

    /// Whether hidden entries are shown.
    #[must_use]
    pub fn shows_hidden(&self) -> bool {
        self.show_hidden
    }

    /// The path of the selected entry, if any.
    #[must_use]
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected.as_ref().map(|name| self.folder.join(name))
    }

    /// Reads `folder` in the background and goes there when it has been read; `wrap` turns
    /// picker messages into yours. The folder shown stays until then, and a later `open` makes
    /// the answer of this one stale.
    ///
    /// `wrap` is a function such as `Msg::Picker` or a closure that captures what it needs. It
    /// runs once, on the background thread that read the folder, so it must be `Send`; it is
    /// never copied, so it need not be `Clone`.
    pub fn open<Msg: Send + 'static>(
        &mut self,
        folder: impl Into<PathBuf>,
        wrap: impl FnOnce(FilePickerMsg) -> Msg + Send + 'static,
    ) -> Command<Msg> {
        let folder = folder.into();
        self.loading = Some(folder.clone());
        Command::perform(move || {
            let listing = read_folder(&folder);
            wrap(FilePickerMsg::Loaded(folder, listing))
        })
    }

    /// Applies a picker message. [`FilePickerMsg::Chosen`] changes nothing; act on it yourself.
    /// `wrap` is used as by [`open`](Self::open), for the folder a message asks to read.
    pub fn update<Msg: Send + 'static>(
        &mut self,
        message: FilePickerMsg,
        wrap: impl FnOnce(FilePickerMsg) -> Msg + Send + 'static,
    ) -> Command<Msg> {
        match message {
            FilePickerMsg::Open(folder) => return self.open(folder, wrap),
            FilePickerMsg::Refresh => {
                let folder = self.loading.clone().unwrap_or_else(|| self.folder.clone());
                return self.open(folder, wrap);
            }
            // The answer awaited, or an answer for the folder shown when nothing else is awaited
            // (a folder read before the first view, for example).
            FilePickerMsg::Loaded(folder, listing)
                if self.loading.as_ref().map_or(folder == self.folder, |awaited| *awaited == folder) =>
            {
                self.loading = None;
                if folder != self.folder {
                    self.selected = self.child_name_towards(&folder);
                    self.filter.clear();
                    self.folder = folder;
                }
                self.state = match listing {
                    Ok(entries) => FolderState::Ready(entries),
                    Err(error) => FolderState::Failed(error),
                };
                if self.selected.is_none() {
                    self.selected = self.visible().first().map(|entry| entry.name.clone());
                }
            }
            // An answer for a folder the user no longer waits for.
            FilePickerMsg::Loaded(..) | FilePickerMsg::Chosen(_) => {}
            FilePickerMsg::Select(name) => self.selected = name,
            FilePickerMsg::Filter(text) => {
                self.filter = text;
                let visible = self.visible();
                if !visible.iter().any(|entry| Some(&entry.name) == self.selected.as_ref()) {
                    self.selected = visible.first().map(|entry| entry.name.clone());
                }
            }
            FilePickerMsg::ShowHidden(on) => self.show_hidden = on,
        }
        Command::none()
    }

    /// The entries that pass the hidden, extension and name filters.
    pub(crate) fn visible(&self) -> Vec<&FileEntry> {
        let FolderState::Ready(entries) = &self.state else {
            return Vec::new();
        };
        let query = self.filter.to_lowercase();
        entries
            .iter()
            .filter(|entry| self.show_hidden || !entry.is_hidden())
            .filter(|entry| entry.folder || self.extensions.is_empty() || self.extension_matches(&entry.name))
            .filter(|entry| query.is_empty() || entry.name.to_lowercase().contains(&query))
            .collect()
    }

    fn extension_matches(&self, name: &str) -> bool {
        Path::new(name)
            .extension()
            .is_some_and(|ext| self.extensions.iter().any(|wanted| ext.to_string_lossy().to_lowercase() == *wanted))
    }

    /// When going up, the folder we came from stays selected.
    fn child_name_towards(&self, target: &Path) -> Option<String> {
        let rest = self.folder.strip_prefix(target).ok()?;
        rest.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-a8-browser-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).expect("scratch folder");
        for file in ["Cargo.toml", "README.md", ".env", "build.rs"] {
            std::fs::write(dir.join(file), "x").expect("scratch file");
        }
        dir
    }

    #[test]
    fn reads_folders_first_and_filters() {
        let dir = scratch("filters");
        let mut browser = FileBrowser::new(&dir, PickMode::Files).extensions(["RS", "toml"]);
        let command: Command<FilePickerMsg> = browser.open(dir.clone(), |m| m);
        assert_eq!(command.actions.len(), 1);
        let listing = read_folder(&dir);
        let _ = browser.update(FilePickerMsg::Loaded(dir.clone(), listing), |m| m);
        let names: Vec<&str> = browser.visible().iter().map(|e| e.name()).collect();
        assert_eq!(names, ["src", "build.rs", "Cargo.toml"]);
        assert_eq!(browser.selected.as_deref(), Some("src"));
        let _ = browser.update(FilePickerMsg::Filter("CARGO".into()), |m| m);
        assert_eq!(browser.selected.as_deref(), Some("Cargo.toml"));
        let mut all = FileBrowser::new(&dir, PickMode::Files);
        all.state = FolderState::Ready(read_folder(&dir).expect("readable"));
        assert_eq!(all.visible().len(), 4);
        let _ = all.update(FilePickerMsg::ShowHidden(true), |m| m);
        assert_eq!(all.visible().len(), 5);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn errors_become_states_and_stale_answers_are_ignored() {
        let missing = std::env::temp_dir().join("quvyta-a8-no-such-folder");
        assert_eq!(read_folder(&missing), Err(ListingError::NotFound));
        let mut browser = FileBrowser::new("/", PickMode::Folders);
        let _ = browser.update(FilePickerMsg::Open(missing.clone()), |m| m);
        let _ = browser.update(FilePickerMsg::Loaded(PathBuf::from("/elsewhere"), Ok(Arc::from(Vec::new()))), |m| m);
        assert_eq!(browser.state, FolderState::Unread);
        assert_eq!(browser.loading(), Some(missing.as_path()));
        let _ = browser.update(FilePickerMsg::Loaded(missing.clone(), read_folder(&missing)), |m| m);
        assert_eq!(browser.state, FolderState::Failed(ListingError::NotFound));
        assert_eq!((browser.folder(), browser.loading()), (missing.as_path(), None));
    }

    #[test]
    fn going_up_keeps_the_folder_we_left_selected() {
        let mut browser = FileBrowser::new("/home/ada/projects", PickMode::Files);
        let _ = browser.update(FilePickerMsg::Open(PathBuf::from("/home")), |m| m);
        let entries = ["ada", "guest"].map(|name| FileEntry { name: name.to_owned(), folder: true, size: None });
        let _ = browser.update(FilePickerMsg::Loaded(PathBuf::from("/home"), Ok(Arc::from(entries))), |m| m);
        assert_eq!(browser.selected.as_deref(), Some("ada"));
    }

    #[test]
    fn the_shown_folder_stays_until_the_new_one_is_read() {
        let dir = scratch("stays");
        let mut browser = FileBrowser::new(&dir, PickMode::Files);
        let _ = browser.update(FilePickerMsg::Loaded(dir.clone(), read_folder(&dir)), |m| m);
        let _ = browser.update(FilePickerMsg::Filter("cargo".into()), |m| m);
        let src = dir.join("src");
        let _ = browser.update(FilePickerMsg::Open(src.clone()), |m| m);
        // Nothing shown changes while `src` is read: folder, entries, filter and selection.
        assert_eq!(browser.folder(), dir.as_path());
        assert_eq!(browser.loading(), Some(src.as_path()));
        assert_eq!(browser.filter, "cargo");
        assert_eq!(browser.selected.as_deref(), Some("Cargo.toml"));
        assert_eq!(browser.visible().len(), 1);
        // A newer open makes the older answer stale.
        let _ = browser.update(FilePickerMsg::Open(dir.clone()), |m| m);
        let _ = browser.update(FilePickerMsg::Loaded(src.clone(), read_folder(&src)), |m| m);
        assert_eq!(browser.folder(), dir.as_path());
        let _ = browser.update(FilePickerMsg::Open(src.clone()), |m| m);
        let _ = browser.update(FilePickerMsg::Loaded(src.clone(), read_folder(&src)), |m| m);
        assert_eq!((browser.folder(), browser.loading()), (src.as_path(), None));
        assert_eq!(browser.filter, "");
        std::fs::remove_dir_all(dir).ok();
    }
}
