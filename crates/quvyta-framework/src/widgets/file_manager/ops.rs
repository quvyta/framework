//! The file operations of a [`FileManager`](super::FileManager): reading a folder, and making,
//! renaming, moving and deleting entries under its root.
//!
//! Every operation takes the root folder and keys, never a path, and turns a key into a path
//! itself, so nothing handed over can reach past the root: a key part that is empty, `.`, `..` or
//! holds a NUL is refused. A confined manager also refuses a key that goes through a symbolic
//! link, because a link can point anywhere; without that option a link on the way is followed
//! like any other folder. A link is an entry like any other, so renaming, moving or deleting one
//! acts on the link and leaves what it points at alone.
//!
//! These touch the disk, so they run on a background thread.

use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::state::{ROOT, child_key};

/// Why a name cannot be used, as the person types it.
///
/// More reasons may be added, so match with a `_` arm and lean on
/// [`message`](Self::message) for the words.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NameProblem {
    /// Nothing, or only spaces, was typed.
    Empty,
    /// It holds a `/`, which would make it a path rather than a name.
    Slash,
    /// It holds a character no file name can hold.
    Nul,
    /// It is `.` or `..`, which already mean this folder and the one above it.
    Dots,
    /// An entry of the folder already has it.
    Taken,
}

impl NameProblem {
    /// What is wrong, in the person's language.
    #[must_use]
    pub fn message(&self) -> String {
        let key = match self {
            Self::Empty => "quvyta.file-manager.name-empty",
            Self::Slash => "quvyta.file-manager.name-slash",
            Self::Nul => "quvyta.file-manager.name-nul",
            Self::Dots => "quvyta.file-manager.name-dots",
            Self::Taken => "quvyta.file-manager.name-taken",
        };
        crate::t!(key)
    }
}

/// Checks `name` as the name of a new entry among `siblings`.
///
/// `current` is the entry's own name when it is being renamed, which does not count as taken.
///
/// # Errors
///
/// The first thing wrong with the name.
pub fn check_name<'a>(
    name: &str,
    mut siblings: impl Iterator<Item = &'a str>,
    current: Option<&str>,
) -> Result<(), NameProblem> {
    if name.trim().is_empty() {
        return Err(NameProblem::Empty);
    }
    if name.contains('/') {
        return Err(NameProblem::Slash);
    }
    if name.contains('\0') {
        return Err(NameProblem::Nul);
    }
    if name == "." || name == ".." {
        return Err(NameProblem::Dots);
    }
    if current != Some(name) && siblings.any(|sibling| sibling == name) {
        return Err(NameProblem::Taken);
    }
    Ok(())
}

/// Why a file operation was not done.
///
/// More reasons may be added as more operations arrive, so match with a `_` arm and lean on
/// [`message`](Self::message) for the words.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileError {
    /// The name cannot be used.
    Name(NameProblem),
    /// The entry would be reached, or land, outside the root folder.
    Outside,
    /// A folder was to go into itself or into a folder below it.
    IntoItself,
    /// The target folder already has an entry of this name.
    Taken(String),
    /// Moving would cross to another file system, which a rename cannot do; files are not copied.
    CrossDevice,
    /// The user may not read or change this.
    Denied,
    /// The user may not see what is in this folder.
    NotReadable,
    /// It is not there any more: another program took it away while the rows still showed it.
    Missing,
    /// There is no trash this entry can go to: none on this file system, or none at all.
    NoTrash,
    /// The disk is full.
    NoRoom,
    /// The person said to stop, and what had been written was taken away again.
    Stopped,
    /// What the operating system said.
    System(String),
}

impl FileError {
    /// What went wrong, in the person's language.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Name(problem) => problem.message(),
            Self::Outside => crate::t!("quvyta.file-manager.outside"),
            Self::IntoItself => crate::t!("quvyta.file-manager.into-itself"),
            Self::Taken(name) => crate::t!("quvyta.file-manager.taken", name = name.as_str()),
            Self::CrossDevice => crate::t!("quvyta.file-manager.cross-device"),
            Self::Denied => crate::t!("quvyta.file-manager.denied"),
            Self::NotReadable => crate::t!("quvyta.file-manager.not-readable"),
            Self::Missing => crate::t!("quvyta.file-manager.missing"),
            Self::NoTrash => crate::t!("quvyta.file-manager.no-trash"),
            Self::NoRoom => crate::t!("quvyta.file-manager.no-room"),
            Self::Stopped => crate::t!("quvyta.file-manager.stopped"),
            Self::System(said) => said.clone(),
        }
    }
}

impl From<std::io::Error> for FileError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            // A move is a rename; copying a whole tree is another operation, not a fallback.
            ErrorKind::CrossesDevices => Self::CrossDevice,
            ErrorKind::PermissionDenied => Self::Denied,
            ErrorKind::NotFound => Self::Missing,
            ErrorKind::StorageFull => Self::NoRoom,
            _ => Self::System(error.to_string()),
        }
    }
}

/// Why a folder could not be read, in the person's language rather than the system's.
///
/// A folder is read for its rows, so a refusal is about seeing rather than about changing: the
/// person is told they may not look inside, not that they may not change something.
pub(super) fn read_error(error: std::io::Error) -> FileError {
    if error.kind() == ErrorKind::PermissionDenied { FileError::NotReadable } else { error.into() }
}

/// What an operation changed, by key.
///
/// More kinds of change may be added as more operations arrive, so match with a `_` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileChange {
    /// An entry was made.
    Created(String),
    /// An entry moved from the first key to the second, by a rename or a move.
    Moved(String, String),
    /// An entry was deleted, with everything in it.
    Deleted(String),
    /// An entry was copied; the key is the copy, and the entry it was made from stays where it is.
    Copied(String),
    /// An entry went to the trash, from where the person can still get it back.
    Trashed(String),
}

/// The key of the folder an entry is in, the root for an entry directly in it.
#[must_use]
pub fn parent_key(key: &str) -> &str {
    key.rsplit_once('/').map_or(ROOT, |(parent, _)| parent)
}

/// The name of the entry `key`, without the folders before it.
#[must_use]
pub fn name_of(key: &str) -> &str {
    key.rsplit_once('/').map_or(key, |(_, name)| name)
}

/// The characters of `name` a rename starts with selected: the name before its extension, so
/// typing replaces `report.final` and keeps `.md`.
///
/// A folder has no extension, and neither has a name whose only dot starts it (`.gitignore`):
/// both are selected whole. The range counts characters, the way the field counts them.
#[must_use]
pub fn stem(name: &str, folder: bool) -> Range<usize> {
    let length = name.chars().count();
    if folder {
        return 0..length;
    }
    match name.chars().rev().position(|c| c == '.').map(|from_end| length - 1 - from_end) {
        Some(dot) if dot > 0 => 0..dot,
        _ => 0..length,
    }
}

/// Whether `key` is `folder` or somewhere below it.
#[must_use]
pub fn is_within(key: &str, folder: &str) -> bool {
    key == folder || key.strip_prefix(folder).is_some_and(|rest| rest.starts_with('/'))
}

/// Whether `key` names an entry under a root: a path of plain names, none of them empty, `.` or
/// `..`, and not starting at `/`.
///
/// Keys come from the manager itself, which only ever joins the names a folder listed, but an
/// application also keeps them in files anyone can edit. A key that climbs out of the root would
/// reach a file the manager never showed, so it is refused wherever a key becomes a path.
#[must_use]
pub fn is_inside(key: &str) -> bool {
    !key.is_empty() && !key.contains('\0') && key.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The path of the entry `key`, which is not followed if it is a link.
///
/// While `confined`, every folder on the way must be a real folder under the root, not a link, so
/// the path cannot lead out of it.
fn entry_path(root: &Path, key: &str, confined: bool) -> Result<PathBuf, FileError> {
    let parts: Vec<&str> = key.split('/').collect();
    let mut path = root.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || *part == "." || *part == ".." || part.contains('\0') {
            return Err(FileError::Outside);
        }
        path.push(part);
        if confined && index + 1 < parts.len() {
            real_folder(&path)?;
        }
    }
    Ok(path)
}

/// The path of the folder `key`, which must be a folder under the root.
fn folder_path(root: &Path, key: &str, confined: bool) -> Result<PathBuf, FileError> {
    if key == ROOT {
        return Ok(root.to_path_buf());
    }
    let path = entry_path(root, key, confined)?;
    if confined {
        real_folder(&path)?;
    }
    Ok(path)
}

/// Refuses a path that is a link or not a folder.
fn real_folder(path: &Path) -> Result<(), FileError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(FileError::Outside);
    }
    Ok(())
}

/// Whether anything, a dangling link included, is at `path`.
fn occupied(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Checks a name the operations are handed; the dialog checked it already, so this only guards
/// against a key or a name that did not come through it.
fn usable(name: &str) -> Result<(), FileError> {
    check_name(name, std::iter::empty(), None).map_err(FileError::Name)
}

/// Makes an empty file `name` in the folder `folder`.
///
/// # Errors
///
/// A name that cannot be used, a folder outside the root, a name already there, or what the
/// system said.
pub fn create_file(root: &Path, folder: &str, name: &str, confined: bool) -> Result<FileChange, FileError> {
    usable(name)?;
    let path = folder_path(root, folder, confined)?.join(name);
    // `create_new` refuses an existing entry in the same step that makes the file, so nothing
    // already there is ever truncated.
    OpenOptions::new().write(true).create_new(true).open(&path).map_err(|error| taken_or(error, name))?;
    Ok(FileChange::Created(child_key(folder, name)))
}

/// Makes an empty folder `name` in the folder `folder`.
///
/// # Errors
///
/// As [`create_file`].
pub fn create_folder(root: &Path, folder: &str, name: &str, confined: bool) -> Result<FileChange, FileError> {
    usable(name)?;
    let path = folder_path(root, folder, confined)?.join(name);
    fs::create_dir(&path).map_err(|error| taken_or(error, name))?;
    Ok(FileChange::Created(child_key(folder, name)))
}

/// Gives the entry `key` the name `name`, in the same folder.
///
/// # Errors
///
/// A name that cannot be used, an entry outside the root, a name already there, or what the
/// system said.
pub fn rename(root: &Path, key: &str, name: &str, confined: bool) -> Result<FileChange, FileError> {
    usable(name)?;
    let from = entry_path(root, key, confined)?;
    let folder = parent_key(key);
    if name_of(key) == name {
        return Ok(FileChange::Moved(key.to_owned(), key.to_owned()));
    }
    let to = folder_path(root, folder, confined)?.join(name);
    if occupied(&to) {
        return Err(FileError::Taken(name.to_owned()));
    }
    fs::rename(&from, &to)?;
    Ok(FileChange::Moved(key.to_owned(), child_key(folder, name)))
}

/// Moves the entry `key` into the folder `into`, keeping its name.
///
/// A move is a rename on the same file system; files are never copied, so a move to another file
/// system is refused and said so.
///
/// # Errors
///
/// An entry or folder outside the root, a folder into itself, a name already there, another file
/// system, or what the system said.
pub fn move_into(root: &Path, key: &str, into: &str, confined: bool) -> Result<FileChange, FileError> {
    if is_within(into, key) {
        return Err(FileError::IntoItself);
    }
    let from = entry_path(root, key, confined)?;
    let name = name_of(key);
    if parent_key(key) == into {
        return Ok(FileChange::Moved(key.to_owned(), key.to_owned()));
    }
    let to = folder_path(root, into, confined)?.join(name);
    if occupied(&to) {
        return Err(FileError::Taken(name.to_owned()));
    }
    fs::rename(&from, &to)?;
    Ok(FileChange::Moved(key.to_owned(), child_key(into, name)))
}

/// Deletes the entry `key`, a folder with everything in it. A link is removed as a link; what it
/// points at stays.
///
/// # Errors
///
/// An entry outside the root, or what the system said.
pub fn delete(root: &Path, key: &str, confined: bool) -> Result<FileChange, FileError> {
    let path = entry_path(root, key, confined)?;
    let meta = fs::symlink_metadata(&path)?;
    // The standard library's `remove_dir_all` does not follow links inside the folder either, so
    // a link in there is removed without touching where it leads.
    if meta.is_dir() {
        fs::remove_dir_all(&path)?;
    } else {
        fs::remove_file(&path)?;
    }
    Ok(FileChange::Deleted(key.to_owned()))
}

/// Puts the entry `key` into the trash folder `trash`.
///
/// # Errors
///
/// An entry outside the root, [`FileError::NoTrash`] when there is no trash the entry can be
/// renamed into, and what the system said otherwise.
pub(super) fn to_trash(trash: &Path, root: &Path, key: &str, confined: bool) -> Result<FileChange, FileError> {
    let path = entry_path(root, key, confined)?;
    // An entry that is already gone is said so plainly rather than as a failed rename.
    fs::symlink_metadata(&path)?;
    super::trash::move_to_trash(trash, &path)?;
    Ok(FileChange::Trashed(key.to_owned()))
}

/// An error of making `name`, where an entry already there is said by name.
fn taken_or(error: std::io::Error, name: &str) -> FileError {
    if error.kind() == ErrorKind::AlreadyExists { FileError::Taken(name.to_owned()) } else { error.into() }
}

/// What a long operation tells the outside while it runs, and what it asks it.
///
/// A copy of one small file needs none of this; a copy of a folder of photographs needs both, so
/// the same code does the work either way and a plain copy hands it a watch that says nothing.
pub(super) struct Watch<'a> {
    /// Told how many bytes have been written of how many there are to write.
    pub(super) progress: &'a dyn Fn(u64, u64),
    /// Asked, between files and between blocks of a large file, whether to stop.
    pub(super) stopped: &'a dyn Fn() -> bool,
}

impl Watch<'_> {
    /// A watch that reports nothing and never stops the work.
    pub(super) fn none() -> Self {
        Self { progress: &|_, _| {}, stopped: &|| false }
    }
}

/// How much of a large file is written between two looks at whether to stop: big enough that the
/// check costs nothing, small enough that stopping feels immediate on a slow disk.
const BLOCK: usize = 256 * 1024;

/// How many bytes the entry at `path` holds, itself and everything below it. A link counts as its
/// own size and is never followed.
fn weight(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 0 };
    if !meta.is_dir() {
        return meta.len();
    }
    let Ok(entries) = fs::read_dir(path) else { return 0 };
    entries.flatten().map(|entry| weight(&entry.path())).sum()
}

/// Copies the entry `key` into the folder `into`, keeping its name; a folder is copied with
/// everything in it.
///
/// Nothing already there is overwritten: a name the target folder has is refused. A folder cannot
/// be copied into itself or into a folder below it, which would never end. A symbolic link is
/// copied as a link, so what it points at is not duplicated.
///
/// # Errors
///
/// An entry or folder outside the root, a folder into itself, a name already there, or what the
/// system said.
pub fn copy_into(root: &Path, key: &str, into: &str, confined: bool) -> Result<FileChange, FileError> {
    copy_watched(root, key, into, confined, &Watch::none())
}

/// [`copy_into`], telling `watch` how far it has come and asking it whether to stop.
pub(super) fn copy_watched(
    root: &Path,
    key: &str,
    into: &str,
    confined: bool,
    watch: &Watch<'_>,
) -> Result<FileChange, FileError> {
    if is_within(into, key) {
        return Err(FileError::IntoItself);
    }
    let from = entry_path(root, key, confined)?;
    let name = name_of(key);
    let to = folder_path(root, into, confined)?.join(name);
    if occupied(&to) {
        return Err(FileError::Taken(name.to_owned()));
    }
    let total = weight(&from);
    let mut written = 0;
    (watch.progress)(0, total);
    match copy_tree(&from, &to, total, &mut written, watch) {
        Ok(()) => Ok(FileChange::Copied(child_key(into, name))),
        Err(problem) => {
            // A copy that stopped leaves nothing behind: half a file is worse than no file, and
            // the person who cancelled did not ask for one.
            let _ = fs::symlink_metadata(&to)
                .map(|meta| if meta.is_dir() { fs::remove_dir_all(&to) } else { fs::remove_file(&to) });
            Err(problem)
        }
    }
}

/// Copies `from` to `to`, a folder with everything in it, counting the bytes written into
/// `written` out of `total`.
fn copy_tree(from: &Path, to: &Path, total: u64, written: &mut u64, watch: &Watch<'_>) -> Result<(), FileError> {
    if (watch.stopped)() {
        return Err(FileError::Stopped);
    }
    let meta = fs::symlink_metadata(from)?;
    if meta.file_type().is_symlink() {
        // A link is copied as a link: following it would duplicate whatever it points at, which
        // is not what the folder holds.
        let target = fs::read_link(from)?;
        std::os::unix::fs::symlink(target, to)?;
        return Ok(());
    }
    if !meta.is_dir() {
        copy_file(from, to, total, written, watch)?;
        return Ok(());
    }
    fs::create_dir(to)?;
    let mut entries: Vec<PathBuf> = fs::read_dir(from)?.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for entry in entries {
        let Some(name) = entry.file_name() else { continue };
        copy_tree(&entry, &to.join(name), total, written, watch)?;
    }
    // The folder's own permissions come last, so a folder the user may not write into is still
    // filled first and then made what it was.
    fs::set_permissions(to, meta.permissions())?;
    Ok(())
}

/// Copies one file block by block, so a large one can be stopped in the middle and reports its
/// progress on the way.
fn copy_file(from: &Path, to: &Path, total: u64, written: &mut u64, watch: &Watch<'_>) -> Result<(), FileError> {
    use std::io::{Read, Write};

    let mut source = fs::File::open(from)?;
    let mut target = OpenOptions::new().write(true).create_new(true).open(to)?;
    let mut block = vec![0u8; BLOCK];
    loop {
        if (watch.stopped)() {
            return Err(FileError::Stopped);
        }
        let read = source.read(&mut block)?;
        if read == 0 {
            break;
        }
        target.write_all(&block[..read])?;
        *written += read as u64;
        (watch.progress)(*written, total);
    }
    target.flush()?;
    // The mode is what the file is, so a program stays a program and a key stays unreadable.
    fs::set_permissions(to, fs::metadata(from)?.permissions())?;
    Ok(())
}
