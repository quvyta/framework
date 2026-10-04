//! The trash, the way the freedesktop trash specification describes it: an entry going in, what is
//! in it, putting one back and emptying it.
//!
//! A trash folder holds two folders: `files`, where the entry itself goes, and `info`, where a
//! small `.trashinfo` file says where it came from and when it went. Both are written before the
//! entry moves, so a trash is never left with a file nothing knows the way back for.
//!
//! A note is read back whatever is in it. An entry whose note is missing, unreadable or says
//! nothing the specification describes is listed all the same, under the name it has in the trash
//! and with where it came from unknown, because one note nobody can read is no reason to leave out
//! an entry that is still there.
//!
//! An entry goes to the trash by being renamed into it, which only works on the file system the
//! trash is on. The specification has a second kind of trash for the other file systems, at the
//! top of each one; a file manager cannot make that folder without the rights to write at the root
//! of the disk, so this one does not try: the entry is refused with
//! [`FileError::NoTrash`](super::FileError::NoTrash) and the manager offers to delete it for good
//! instead, behind a question of its own.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use super::ops::{self, FileChange, FileError};

/// How many names are tried before giving up on finding a free one, in the trash and in the folder
/// an entry is put back into.
const TRIES: u32 = 1_000;

/// The trash of the person's own home, where the freedesktop specification puts it:
/// `$XDG_DATA_HOME/Trash`, which is `~/.local/share/Trash` when that variable says nothing.
///
/// `None` where the platform gives no such folder: Windows and macOS have a trash of their own
/// that this specification does not describe, and a run with no home folder has none at all.
#[must_use]
pub(super) fn home_trash() -> Option<PathBuf> {
    if cfg!(windows) || cfg!(target_os = "macos") {
        return None;
    }
    crate::storage::data_dir("Trash")
}

/// The folder the entries themselves are in.
fn files(trash: &Path) -> PathBuf {
    trash.join("files")
}

/// The folder the notes are in, one per entry, each named after the name the entry has in the trash.
fn info(trash: &Path) -> PathBuf {
    trash.join("info")
}

/// The note of the entry `name`: where it came from, and when it went.
///
/// A note that cannot be read, or that says neither key, is no reason to leave the entry out: it is
/// listed under the name it has here with where it came from unknown.
fn note(trash: &Path, name: &str) -> Note {
    let written = fs::read_to_string(info(trash).join(format!("{name}.trashinfo"))).unwrap_or_default();
    let mut origin = None;
    let mut deleted = None;
    for line in written.lines() {
        if let Some(written) = line.strip_prefix("Path=") {
            origin = decode_path(written).filter(|path| path.is_absolute());
        } else if let Some(stamp) = line.strip_prefix("DeletionDate=").map(str::trim).filter(|it| !it.is_empty()) {
            deleted = Some(stamp.to_owned());
        }
    }
    Note { origin, deleted }
}

/// What a note says about the entry it belongs to.
struct Note {
    /// Where the entry came from, where the note says it and the path is one this trash could put
    /// it back to. A note that writes a relative path belongs to a trash at the top of another file
    /// system, whose top this one does not know, and answers nothing.
    origin: Option<PathBuf>,
    /// When it went, as the note writes it.
    deleted: Option<String>,
}

/// One entry of a trash folder, as the manager reads it: the name it has there, the name it has
/// where it came from, where that was and when it went.
///
/// The two names are two names and not one: an entry trashed a second time takes a number in the
/// trash and keeps its own name where it came from, and only [`name`](Self::name) is what an
/// operation on the entry is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trashed {
    /// The name it has inside the trash, which is its own name unless that one was taken already,
    /// in which case the specification adds a number to it. This is what every operation on the
    /// entry is given, as a row of a manager is keyed by the name the entry has there.
    pub name: String,
    /// What a row of the entry says: the name it has where it came from, which is what a person
    /// knows it by, and the name in the trash where nothing says where it came from.
    pub label: String,
    /// Where the entry came from, the note's `Path=`, and `None` where the note is missing or says
    /// nothing about it.
    pub origin: Option<PathBuf>,
    /// When it went, the note's `DeletionDate=` as it is written, and `None` where the note says
    /// nothing about it.
    pub deleted: Option<String>,
    /// Whether it is a folder, and so is put back or deleted with everything in it.
    pub folder: bool,
    /// Whether it is a file that may be run, which gives it a program's icon.
    pub executable: bool,
}

impl Trashed {
    /// Where the entry came from, as a column of it says: the folder it was in, since its own name
    /// is the name of its row, and that a person reads where nothing says where it came from.
    ///
    /// Draws in the person's language, so it belongs to the view and not to a background thread.
    #[must_use]
    pub fn origin_text(&self) -> String {
        let Some(origin) = &self.origin else { return crate::t!("quvyta.file-manager.origin-unknown") };
        origin.parent().unwrap_or(origin).to_string_lossy().into_owned()
    }

    /// When the entry went, as a column of it says: `2026-09-20 14:32`, the year, the month, the
    /// day and the clock the machine stood by, which is the shape a folder's changed date has. The
    /// note writes the local clock with a `T` between the date and the time and seconds a column of
    /// that width has no room for; a note that says something else is shown as it was written.
    #[must_use]
    pub fn deleted_text(&self) -> String {
        let Some(deleted) = &self.deleted else { return String::new() };
        match deleted.split_once('T') {
            Some((date, time)) if time.get(..5).is_some() => format!("{date} {}", &time[..5]),
            _ => deleted.clone(),
        }
    }

    /// When the entry went, in seconds since 1970-01-01 UTC, so a list can put entries in order
    /// of it the way it puts a folder's entries in order of when they changed.
    ///
    /// The note writes the local clock, which is what a person reads, and it is read back with the
    /// machine's own offset to become an instant; a note that says something this does not
    /// understand is nothing, and such an entry stands after the ones whose date is known.
    #[must_use]
    pub fn deleted_at(&self) -> Option<i64> {
        let (date, time) = self.deleted.as_deref()?.split_once('T')?;
        let moment = crate::date::DateTime {
            date: crate::date::Date::parse(date).ok()?,
            time: crate::date::TimeOfDay::parse(time)?,
            offset_minutes: crate::date::local_offset_minutes(),
        };
        Some(moment.to_unix())
    }
}

/// Reads the entries of the trash folder `trash`: what is in `files`, each with what its note in
/// `info` says about it, in the order the folder lists them.
///
/// A trash nobody has put anything into yet has no `files` folder at all, which is empty rather
/// than broken: there is nothing in it, and that is not a refusal.
///
/// # Errors
///
/// Why the folder could not be read, in the person's language; see
/// [`FileError::NotReadable`](super::FileError::NotReadable).
pub(super) fn entries(trash: &Path) -> Result<Vec<Trashed>, FileError> {
    if !files(trash).exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(files(trash)).map_err(ops::read_error)? {
        let entry = entry.map_err(ops::read_error)?;
        let own = entry.file_name();
        let name = own.to_string_lossy().into_owned();
        let folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
        let executable = !folder && super::state::runs(&entry, &name);
        let note = note(trash, &name);
        let label = note.origin.as_deref().and_then(Path::file_name).unwrap_or(own.as_os_str());
        entries.push(Trashed {
            name,
            label: label.to_string_lossy().into_owned(),
            origin: note.origin,
            deleted: note.deleted,
            folder,
            executable,
        });
    }
    Ok(entries)
}

/// Puts the entry at `path` into the trash folder `trash`, and answers with the name it took
/// inside it.
///
/// `path` is the entry's real path; the caller has already turned a key into one and checked it
/// against the root.
///
/// # Errors
///
/// [`FileError::NoTrash`](super::FileError::NoTrash) when the trash cannot be made or is on
/// another file system than the entry, and what the system said otherwise.
pub(super) fn move_to_trash(trash: &Path, path: &Path) -> Result<String, FileError> {
    for folder in [files(trash), info(trash)] {
        fs::create_dir_all(&folder).map_err(|_| FileError::NoTrash)?;
    }
    let name = path.file_name().ok_or(FileError::NoTrash)?.to_string_lossy().into_owned();
    // The info file is made first and with `create_new`, so writing it is what reserves the name:
    // two managers trashing the same name at the same moment cannot both win it.
    let (taken, note) = reserve(&info(trash), &name)?;
    if let Err(problem) = write_info(&note, path) {
        // The note is this call's own, made moments ago with `create_new`, so removing it can
        // only fail because it is already gone. The error that is reported is the one the person
        // asked about; a note left behind would cost the next trashing one name, nothing more.
        let _ = fs::remove_file(&note);
        return Err(problem);
    }
    match fs::rename(path, files(trash).join(&taken)) {
        Ok(()) => Ok(taken),
        Err(problem) => {
            // As above: the note is this call's own and the entry never moved, so the person is
            // told why the trashing failed and nothing of theirs is touched.
            let _ = fs::remove_file(&note);
            // A rename across file systems is the specification's own limit, not the user's fault.
            Err(if problem.kind() == ErrorKind::CrossesDevices { FileError::NoTrash } else { problem.into() })
        }
    }
}

/// Takes a free name for `name` in the trash and answers with it and the info file that holds it.
///
/// The specification's own way: the name itself, and then the name with a number added, until one
/// is free.
fn reserve(info: &Path, name: &str) -> Result<(String, PathBuf), FileError> {
    for attempt in 0..TRIES {
        let taken = if attempt == 0 { name.to_owned() } else { format!("{name}.{attempt}") };
        let note = info.join(format!("{taken}.trashinfo"));
        match OpenOptions::new().write(true).create_new(true).open(&note) {
            Ok(_) => return Ok((taken, note)),
            Err(problem) if problem.kind() == ErrorKind::AlreadyExists => {}
            Err(problem) => return Err(problem.into()),
        }
    }
    Err(FileError::NoTrash)
}

/// Writes the note that says where the entry came from and when it went.
fn write_info(note: &Path, path: &Path) -> Result<(), FileError> {
    let now = crate::date::DateTime::now_local();
    let date = now.date;
    let stamp = format!("{:04}-{:02}-{:02}T{}", date.year(), date.month(), date.day(), now.time);
    let text = format!("[Trash Info]\nPath={}\nDeletionDate={stamp}\n", encode_path(path));
    let mut file = OpenOptions::new().write(true).truncate(true).open(note)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Puts the entry `name` of the trash folder `trash` back where its note says it came from, and
/// takes the note away with it.
///
/// The name it takes there is its own, or `wanted` for the name the person is offered once its own
/// was taken again. Nothing that is already there is ever overwritten: a name that is taken is
/// said so with one that is free there, so the question about it has something to offer.
///
/// # Errors
///
/// [`FileError::NoOrigin`](super::FileError::NoOrigin) when the note says nothing about where the
/// entry came from, so there is nowhere to put it back,
/// [`FileError::TakenAs`](super::FileError::TakenAs) with a name that is free there when the name
/// it would take is taken again, and what the system said otherwise.
pub(super) fn restore(trash: &Path, name: &str, wanted: Option<&str>) -> Result<FileChange, FileError> {
    // A name offered after a question came through that question, but a key can be written in a
    // file anyone edits, so a name that would land outside the folder is refused as everywhere.
    if let Some(wanted) = wanted {
        ops::check_name(wanted, std::iter::empty(), None).map_err(FileError::Name)?;
    }
    one_entry(name)?;
    let note = note(trash, name);
    let Some(origin) = note.origin else { return Err(FileError::NoOrigin) };
    let own = origin.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let to = origin.parent().unwrap_or(&origin).join(wanted.unwrap_or(&own));
    if fs::symlink_metadata(&to).is_ok() {
        return Err(FileError::TakenAs(free_name(&to)));
    }
    // The entry moves before its note goes: a restore that stops halfway leaves the entry in the
    // trash, where it can be put back again, rather than an entry nobody knows the way to.
    fs::rename(files(trash).join(name), &to)?;
    // The note is this trash's own, so its removal can only fail because it is already gone, and
    // the entry is back where it came from either way.
    let _ = fs::remove_file(info(trash).join(format!("{name}.trashinfo")));
    Ok(FileChange::Restored(name.to_owned()))
}

/// Deletes the entry `name` of the trash folder `trash` for good, a folder with everything in it,
/// and its note.
///
/// The note goes after the entry, so an entry that cannot be deleted keeps the note that says where
/// it came from and can still be put back.
///
/// # Errors
///
/// What the system said about the entry itself, which is the one thing that has to go.
pub(super) fn purge(trash: &Path, name: &str) -> Result<FileChange, FileError> {
    one_entry(name)?;
    let entry = files(trash).join(name);
    // A link in the trash goes as a link, as anywhere else; what it points at stays.
    if fs::symlink_metadata(&entry)?.is_dir() {
        fs::remove_dir_all(&entry)?;
    } else {
        fs::remove_file(&entry)?;
    }
    let _ = fs::remove_file(info(trash).join(format!("{name}.trashinfo")));
    Ok(FileChange::Deleted(name.to_owned()))
}

/// Deletes everything in the trash folder `trash` for good, every entry with its note, and answers
/// with the names that went and, if one could not go, why.
///
/// Each entry goes on its own and the answer is taken as far as it came, so one that cannot be
/// deleted leaves the rest gone, which is what the person who emptied the trash asked for, and both
/// what went and the refusal are said: an entry that is gone must not be left in the list as though
/// it were there.
///
/// The names are tried in name order rather than in the order the folder lists them, so that the
/// answer is the same every time a trash is emptied.
pub(super) fn empty(trash: &Path) -> (Vec<String>, Option<FileError>) {
    if !files(trash).exists() {
        return (Vec::new(), None);
    }
    let mut names: Vec<String> = match fs::read_dir(files(trash)) {
        // An entry the folder will not name is one nobody could have opened the trash to see, so
        // it is left out rather than stopping the emptying.
        Ok(entries) => entries.flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect(),
        Err(problem) => return (Vec::new(), Some(ops::read_error(problem))),
    };
    names.sort();
    let mut deleted = Vec::new();
    for name in &names {
        match purge(trash, name) {
            Ok(_) => deleted.push(name.clone()),
            // What is left is what could not go; it stays in the trash with its note, so it can
            // still be put back, and the reason is said rather than the whole emptying refused.
            Err(problem) => return (deleted, Some(problem)),
        }
    }
    (deleted, None)
}

/// Refuses a name that is not the name of one entry of a trash's `files` folder.
///
/// An operation is given an entry by its name alone, and a name with a separator in it, or one
/// that is empty or a dot, is joined to `files` as a path: `..` is the trash itself and
/// `../../x` is anywhere at all. The names a trash lists are never such names, so one that is has
/// been written by hand, and deleting or moving it would reach outside the trash.
fn one_entry(name: &str) -> Result<(), FileError> {
    ops::check_name(name, std::iter::empty(), None).map_err(|_| FileError::Outside)
}

/// The name the entry at `to` would take there if it were free: its own, then its own with a
/// number added, which is the way the trash takes a name for itself.
fn free_name(to: &Path) -> String {
    let Some(folder) = to.parent() else { return to.to_string_lossy().into_owned() };
    let first = to.file_name().unwrap_or_default().to_string_lossy().into_owned();
    (0..TRIES)
        .map(|attempt| if attempt == 0 { first.clone() } else { format!("{first}.{attempt}") })
        .find(|name| fs::symlink_metadata(folder.join(name)).is_err())
        .unwrap_or(first)
}

/// The path as the specification writes it: the bytes of a URL path, with everything outside the
/// unreserved set escaped, and the separators left as they are.
fn encode_path(path: &Path) -> String {
    let mut encoded = String::new();
    for byte in path.as_os_str().as_bytes() {
        let plain = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/');
        if plain {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// A path as a note writes it, read back: the escaped bytes of a URL path, with everything outside
/// the unreserved set spelled `%XX`.
///
/// The bytes are the path's own, so a name that is not text is put back under the same bytes it
/// left with rather than not at all.
fn decode_path(written: &str) -> Option<PathBuf> {
    let bytes = written.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let escape = written.get(at + 1..at + 3)?;
                decoded.push(u8::from_str_radix(escape, 16).ok()?);
                at += 3;
            }
            byte => {
                decoded.push(byte);
                at += 1;
            }
        }
    }
    Some(PathBuf::from(OsString::from_vec(decoded)))
}
