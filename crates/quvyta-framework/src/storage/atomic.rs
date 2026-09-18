//! Writing a file so that a crash or a power cut never leaves half of it on disk.
//!
//! The order is the whole point: write a temporary file next to the real one, flush it to the
//! disk, rename it over the real name, then flush the directory itself. Without the last step
//! the rename can still be lost after a power cut, and then both names are gone: the old one was
//! replaced and the new one never reached the disk.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Tells two temporary files apart when two threads write the same path at the same time; the
/// process id alone would give them the same name.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// How many symbolic links are followed before a path is taken for a loop, as the kernel does.
const MAX_LINKS: usize = 40;

/// A step [`atomic_write`] has finished, in the order they happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteStep {
    /// The contents are in a temporary file next to the real one, which the path names.
    Wrote(PathBuf),
    /// The temporary file is on the disk, not only in the operating system's cache.
    SyncedFile,
    /// The temporary file now carries the real name, replacing what was there.
    Renamed,
    /// The directory entry is on the disk, so the rename survives a power cut. Not reported on
    /// platforms where a directory cannot be flushed; see [`atomic_write`].
    SyncedDirectory,
}

/// Writes `contents` to `path` so that a crash or a power cut leaves either the file that was
/// there before or the new one, never half of either.
///
/// The parent directory has to exist; it is not created. The temporary file is written in that
/// same directory, because a rename cannot cross a file system. On Unix systems the new file
/// keeps the permissions of the file it replaces, so a private file stays private.
///
/// A `path` that is a symbolic link is written through: the file it points at is replaced and
/// the link stays a link, so a settings file linked from a dotfiles repository keeps being
/// that repository's file. The temporary file is then written next to the file the link points
/// at, and the new file keeps that file's permissions. A link to a file that does not exist
/// yet creates that file. Links pointing at links are followed to the end.
///
/// On Unix systems the directory is flushed after the rename, which is what makes the new name
/// survive a power cut. On other platforms, Windows among them, the standard library cannot open
/// a directory to flush it, so that step is skipped: the file's own contents are still flushed
/// before the rename, so no half-written file appears, but a power cut in the moment after the
/// rename can leave the old file. Doing better needs calls the framework cannot make without
/// `unsafe`, which it forbids.
///
/// # Errors
///
/// Returns the I/O error of the first step that failed. The temporary file is removed when a
/// step after it fails, so a failed write leaves nothing behind. Links that point at each
/// other in a loop are an error, and nothing is written.
pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    atomic_write_reporting(path, contents, |_| {})
}

/// [`atomic_write`], reporting each step to `on_step` as it finishes, for a log or a screen that
/// shows what writing a file safely actually does.
///
/// # Errors
///
/// The same as [`atomic_write`]: the error of the first step that failed. `on_step` is called
/// only for the steps that succeeded.
pub fn atomic_write_reporting(path: &Path, contents: &[u8], mut on_step: impl FnMut(WriteStep)) -> io::Result<()> {
    // Renaming over a link would replace the link itself with a plain file.
    let target = link_target(path)?;
    let path = target.as_path();
    let directory = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let temporary = directory.join(temporary_name(path));
    let written = (|| {
        let mut file = fs::File::create(&temporary)?;
        // The new file replaces the old one, so it keeps the old one's permissions: a file the
        // user made private stays private. Set before anything is written into it.
        #[cfg(unix)]
        if let Some(existing) = fs::metadata(path).ok().filter(fs::Metadata::is_file) {
            file.set_permissions(existing.permissions())?;
        }
        file.write_all(contents)?;
        on_step(WriteStep::Wrote(temporary.clone()));
        file.sync_all()?;
        on_step(WriteStep::SyncedFile);
        fs::rename(&temporary, path)?;
        on_step(WriteStep::Renamed);
        if sync_directory(directory)? {
            on_step(WriteStep::SyncedDirectory);
        }
        Ok(())
    })();
    if written.is_err() {
        // The temporary file is ours alone, so removing it can only fail because it is already
        // gone, which is the state we want.
        let _ = fs::remove_file(&temporary);
    }
    written
}

/// The file `path` names once every symbolic link on its last component is followed; `path`
/// itself when it is no link. A relative link is read from the directory the link stands in.
fn link_target(path: &Path) -> io::Result<PathBuf> {
    let mut target = path.to_path_buf();
    for _ in 0..MAX_LINKS {
        let is_link = fs::symlink_metadata(&target).is_ok_and(|meta| meta.file_type().is_symlink());
        if !is_link {
            return Ok(target);
        }
        let next = fs::read_link(&target)?;
        target = match target.parent() {
            Some(parent) if next.is_relative() => parent.join(next),
            _ => next,
        };
    }
    Err(io::Error::other(format!("{}: too many levels of symbolic links", path.display())))
}

/// A name no other writer of the same file uses, in the same directory as the real file.
fn temporary_name(path: &Path) -> String {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("file");
    let ticket = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{name}.tmp-{}-{ticket}", std::process::id())
}

/// Flushes the directory entry itself, so a rename in it survives a power cut. `Ok(false)` on a
/// platform where the standard library cannot open a directory; see [`atomic_write`].
#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<bool> {
    // Opening a directory read-only is enough to flush it; no write handle exists for one.
    fs::File::open(directory)?.sync_all()?;
    Ok(true)
}

/// Flushes the directory entry itself. Always `Ok(false)` here: the standard library cannot open
/// a directory on this platform.
#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> io::Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty directory of this test's own.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-atomic-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("test directory");
        dir
    }

    #[test]
    fn the_steps_happen_in_the_order_that_makes_the_write_safe() {
        let dir = temp_dir("steps");
        let path = dir.join("tree.toml");
        let mut steps = Vec::new();
        atomic_write(&path, b"first\n").expect("first write");
        atomic_write_reporting(&path, b"second\n", |step| steps.push(step)).expect("second write");

        let temporary = match steps.first() {
            Some(WriteStep::Wrote(temporary)) => temporary.clone(),
            other => panic!("the first step writes a temporary file, not {other:?}"),
        };
        assert_eq!(temporary.parent(), path.parent(), "the temporary file shares the directory");
        assert_ne!(temporary, path);
        let rest: Vec<WriteStep> = steps[1..].to_vec();
        if cfg!(unix) {
            assert_eq!(
                rest,
                [WriteStep::SyncedFile, WriteStep::Renamed, WriteStep::SyncedDirectory],
                "the directory is flushed, and only after the rename"
            );
        } else {
            assert_eq!(rest, [WriteStep::SyncedFile, WriteStep::Renamed]);
        }
        assert_eq!(fs::read_to_string(&path).expect("read"), "second\n");
        let leftovers = fs::read_dir(&dir).expect("list").count();
        assert_eq!(leftovers, 1, "nothing but the file itself stays behind");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_failed_write_keeps_the_old_file_and_cleans_up_after_itself() {
        let dir = temp_dir("failure");
        // A directory cannot be renamed over, so the rename step fails with the temporary file
        // already written: exactly the moment the clean-up is for.
        let path = dir.join("occupied");
        fs::create_dir(&path).expect("directory in the way");
        fs::write(path.join("inside.toml"), "kept\n").expect("content under it");

        let mut steps = Vec::new();
        let error = atomic_write_reporting(&path, b"new\n", |step| steps.push(step)).expect_err("rename fails");
        assert!(matches!(steps.first(), Some(WriteStep::Wrote(_))), "{steps:?}");
        assert!(!steps.contains(&WriteStep::Renamed), "{steps:?} after {error}");
        assert_eq!(fs::read_to_string(path.join("inside.toml")).expect("read"), "kept\n", "the old state is intact");
        let names: Vec<String> = fs::read_dir(&dir)
            .expect("list")
            .map(|entry| entry.expect("entry").file_name().display().to_string())
            .collect();
        assert_eq!(names, ["occupied"], "the temporary file was removed");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[cfg(unix)]
    #[test]
    fn the_file_keeps_the_permissions_it_had() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = temp_dir("mode");
        let path = dir.join("secrets.toml");
        fs::write(&path, "token = \"old\"\n").expect("first version");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("private");
        atomic_write(&path, b"token = \"new\"\n").expect("replace");
        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "a private file stays private after it is replaced");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_is_written_through_and_stays_a_link() {
        use std::os::unix::fs::{PermissionsExt as _, symlink};

        let dir = temp_dir("link");
        let dotfiles = dir.join("dotfiles");
        let config = dir.join("config");
        fs::create_dir_all(&dotfiles).expect("dotfiles");
        fs::create_dir_all(&config).expect("config");
        let target = dotfiles.join("settings.toml");
        fs::write(&target, "theme = \"old\"\n").expect("the linked file");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).expect("private");
        // A relative link, as a dotfiles manager makes them, and a link to that link.
        let link = config.join("settings.toml");
        symlink("../dotfiles/settings.toml", &link).expect("link");
        let second = config.join("again.toml");
        symlink(&link, &second).expect("link to the link");

        let mut steps = Vec::new();
        atomic_write_reporting(&second, b"theme = \"new\"\n", |step| steps.push(step)).expect("write");
        assert!(fs::symlink_metadata(&link).expect("link").file_type().is_symlink(), "the link is still a link");
        assert!(fs::symlink_metadata(&second).expect("link").file_type().is_symlink());
        assert_eq!(fs::read_to_string(&target).expect("read"), "theme = \"new\"\n", "the file it points at changed");
        let mode = fs::metadata(&target).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the linked file keeps its permissions");
        let Some(WriteStep::Wrote(temporary)) = steps.first() else { panic!("{steps:?}") };
        let folder = fs::canonicalize(temporary.parent().expect("a folder")).expect("folder");
        assert_eq!(folder, fs::canonicalize(&dotfiles).expect("dotfiles"), "renamed within the linked file's folder");
        assert_eq!(fs::read_dir(&dotfiles).expect("list").count(), 1, "nothing left next to the file");
        assert_eq!(fs::read_dir(&config).expect("list").count(), 2, "only the two links");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_missing_file_creates_it_and_a_loop_writes_nothing() {
        use std::os::unix::fs::symlink;

        let dir = temp_dir("dangling");
        let link = dir.join("settings.toml");
        symlink("real.toml", &link).expect("link");
        atomic_write(&link, b"x = 1\n").expect("write");
        assert!(fs::symlink_metadata(&link).expect("link").file_type().is_symlink());
        assert_eq!(fs::read_to_string(dir.join("real.toml")).expect("created"), "x = 1\n");

        let a = dir.join("a.toml");
        let b = dir.join("b.toml");
        symlink("b.toml", &a).expect("a");
        symlink("a.toml", &b).expect("b");
        let error = atomic_write(&a, b"y = 2\n").expect_err("a loop has no file at its end");
        assert!(error.to_string().contains("symbolic links"), "{error}");
        assert_eq!(fs::read_dir(&dir).expect("list").count(), 4, "nothing was written");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_missing_directory_is_an_error_and_writes_nothing() {
        let dir = temp_dir("missing");
        let path = dir.join("absent").join("tree.toml");
        let error = atomic_write(&path, b"x\n").expect_err("no directory to write in");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(!path.exists());
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn two_writes_at_once_use_two_temporary_files() {
        let dir = temp_dir("parallel");
        let path = dir.join("tree.toml");
        let mut first = None;
        atomic_write_reporting(&path, b"a\n", |step| {
            if let WriteStep::Wrote(temporary) = step {
                first = Some(temporary);
            }
        })
        .expect("first");
        let mut second = None;
        atomic_write_reporting(&path, b"b\n", |step| {
            if let WriteStep::Wrote(temporary) = step {
                second = Some(temporary);
            }
        })
        .expect("second");
        assert_ne!(first.expect("first name"), second.expect("second name"));
        fs::remove_dir_all(&dir).expect("clean");
    }
}
