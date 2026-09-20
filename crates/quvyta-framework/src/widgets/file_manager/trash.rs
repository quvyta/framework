//! Putting an entry in the trash, the way the freedesktop trash specification says.
//!
//! A trash folder holds two folders: `files`, where the entry itself goes, and `info`, where a
//! small `.trashinfo` file says where it came from and when it went. Both are written before the
//! entry moves, so a trash is never left with a file nothing knows the way back for.
//!
//! An entry goes to the trash by being renamed into it, which only works on the file system the
//! trash is on. The specification has a second kind of trash for the other file systems, at the
//! top of each one; a file manager cannot make that folder without the rights to write at the root
//! of the disk, so this one does not try: the entry is refused with
//! [`FileError::NoTrash`](super::FileError::NoTrash) and the manager offers to delete it for good
//! instead, behind a question of its own.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use super::ops::FileError;

/// How many names are tried before giving up on finding a free one in the trash.
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
    let (files, info) = (trash.join("files"), trash.join("info"));
    for folder in [&files, &info] {
        fs::create_dir_all(folder).map_err(|_| FileError::NoTrash)?;
    }
    let name = path.file_name().ok_or(FileError::NoTrash)?.to_string_lossy().into_owned();
    // The info file is made first and with `create_new`, so writing it is what reserves the name:
    // two managers trashing the same name at the same moment cannot both win it.
    let (taken, note) = reserve(&info, &name)?;
    if let Err(problem) = write_info(&note, path) {
        // The note is this call's own, made moments ago with `create_new`, so removing it can
        // only fail because it is already gone. The error that is reported is the one the person
        // asked about; a note left behind would cost the next trashing one name, nothing more.
        let _ = fs::remove_file(&note);
        return Err(problem);
    }
    match fs::rename(path, files.join(&taken)) {
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

/// The path as the specification writes it: the bytes of a URL path, with everything outside the
/// unreserved set escaped, and the separators left as they are.
fn encode_path(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;

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
