//! Moving an application's settings from the folder it used on its own into its ecosystem's
//! layout, once, so that no file is ever lost on the way.
//!
//! Each file is copied before anything is removed: the new file is claimed under its name, filled
//! with [`atomic_write`], given the old file's permissions and read back, and only a copy that
//! reads back the same lets the old file go. Whatever cannot be moved that way stays where it is
//! and is reported.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::FILE_NAME;
use super::atomic::atomic_write;
use crate::diagnostics::Diagnostic;

/// What [`Ecosystem::adopt`](super::Ecosystem::adopt) did: the files it moved and, for everything it
/// left behind, a diagnostic that names the path and says why.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Migration {
    moved: Vec<(PathBuf, PathBuf)>,
    diagnostics: Vec<Diagnostic>,
}

impl Migration {
    /// The files that were moved, each as `(from, to)`, in the order they were moved.
    #[must_use]
    pub fn moved(&self) -> &[(PathBuf, PathBuf)] {
        &self.moved
    }

    /// Everything that was left behind, and why: a file whose new place was taken, a symbolic
    /// link, a file that could not be read or copied, a folder that could not be listed.
    /// Warnings for what was left on purpose, errors for what failed.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether nothing was left behind: every file found was moved, or there was nothing to move.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// The report when the platform gives the ecosystem no folder to move into.
    pub(super) fn without_folder() -> Self {
        let mut report = Self::default();
        report.diagnostics.push(Diagnostic::warning(None, "no config directory found; nothing is adopted"));
        report
    }

    fn warn(&mut self, path: &Path, message: impl std::fmt::Display) {
        self.diagnostics.push(Diagnostic::warning(None, format!("{}: {message}", path.display())));
    }

    fn fail(&mut self, path: &Path, message: impl std::fmt::Display) {
        self.diagnostics.push(Diagnostic::error(None, format!("{}: {message}", path.display())));
    }
}

/// Moves `legacy/settings.toml` to `app_file` and every other file under `legacy` to the same
/// place under `app_dir`, then removes the old folders left empty.
pub(super) fn adopt(legacy: &Path, app_file: &Path, app_dir: &Path) -> Migration {
    let mut report = Migration::default();
    match fs::symlink_metadata(legacy) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return report,
        Err(error) => {
            report.fail(legacy, format_args!("could not be read ({error}); nothing is adopted"));
            return report;
        }
        Ok(meta) if meta.file_type().is_symlink() => {
            report.warn(legacy, "a symbolic link is not followed; nothing is adopted from it");
            return report;
        }
        Ok(meta) if !meta.is_dir() => {
            report.warn(legacy, "not a folder; nothing is adopted from it");
            return report;
        }
        Ok(_) => {}
    }
    let (old, new) = (real(legacy), real(app_dir));
    if old == new {
        // The application's folder already: its other files are in place, only the settings move.
        let settings = legacy.join(FILE_NAME);
        match fs::symlink_metadata(&settings) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => report.fail(&settings, format_args!("could not be read ({error}); it stays where it is")),
            Ok(meta) => adopt_entry(&settings, meta.file_type(), app_file, &mut report),
        }
        return report;
    }
    if new.starts_with(&old) || old.starts_with(&new) {
        // Moving a folder into itself, or out of itself, would walk files it has just moved.
        report.warn(legacy, format_args!("overlaps {}; nothing is adopted", app_dir.display()));
        return report;
    }
    adopt_folder(legacy, Path::new(""), app_file, app_dir, &mut report);
    remove_if_empty(legacy, &mut report);
    report
}

/// `path` with every link on the way resolved when it exists, so two names of one folder compare
/// equal; `path` itself otherwise.
fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Moves every file under `folder`, which sits at `relative` under the legacy folder.
fn adopt_folder(folder: &Path, relative: &Path, app_file: &Path, app_dir: &Path, report: &mut Migration) {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) => {
            report.fail(folder, format_args!("could not be listed ({error}); what is in it stays"));
            return;
        }
    };
    let mut entries: Vec<_> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(error) => {
                report.fail(folder, format_args!("could not be listed completely ({error}); the rest stays"));
                None
            }
        })
        .collect();
    // The same order on every system, so the report reads the same.
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let kind = match entry.file_type() {
            Ok(kind) => kind,
            Err(error) => {
                report.fail(&path, format_args!("could not be read ({error}); it stays where it is"));
                continue;
            }
        };
        if kind.is_dir() {
            adopt_folder(&path, &relative.join(&name), app_file, app_dir, report);
            remove_if_empty(&path, report);
            continue;
        }
        let target = if relative.as_os_str().is_empty() && name == FILE_NAME {
            app_file.to_path_buf()
        } else {
            app_dir.join(relative).join(&name)
        };
        adopt_entry(&path, kind, &target, report);
    }
}

/// Moves the entry at `from`, of type `kind`, to `to` when it is a plain file; reports it otherwise.
fn adopt_entry(from: &Path, kind: fs::FileType, to: &Path, report: &mut Migration) {
    if kind.is_symlink() {
        report.warn(from, "a symbolic link is not moved; it stays where it is");
    } else if !kind.is_file() {
        report.warn(from, "not a regular file; it stays where it is");
    } else if move_file(from, to, report) {
        report.moved.push((from.to_path_buf(), to.to_path_buf()));
    }
}

/// Moves one plain file, copying before removing. Returns whether it moved; when it did not, the
/// reason is in `report` and the old file is where it was.
fn move_file(from: &Path, to: &Path, report: &mut Migration) -> bool {
    if fs::symlink_metadata(to).is_ok() {
        report.warn(from, format_args!("{} already exists; this file stays and nothing is merged", to.display()));
        return false;
    }
    let (contents, permissions) = match fs::read(from).and_then(|contents| Ok((contents, fs::metadata(from)?))) {
        Ok((contents, meta)) => (contents, meta.permissions()),
        Err(error) => {
            report.fail(from, format_args!("could not be read ({error}); it stays where it is"));
            return false;
        }
    };
    match claim(to, &permissions) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            report.warn(from, format_args!("{} already exists; this file stays and nothing is merged", to.display()));
            return false;
        }
        Err(error) => {
            report.fail(from, format_args!("could not be copied to {} ({error}); it stays where it is", to.display()));
            return false;
        }
    }
    let copied = (|| {
        atomic_write(to, &contents)?;
        fs::set_permissions(to, permissions)?;
        if fs::read(to)? != contents {
            return Err(io::Error::other("the copy reads back different"));
        }
        Ok(())
    })();
    if let Err(error) = copied {
        // The new name was claimed by this call, so what is there is this call's own copy.
        let _ = fs::remove_file(to);
        report.fail(from, format_args!("could not be copied to {} ({error}); it stays where it is", to.display()));
        return false;
    }
    if let Err(error) = fs::remove_file(from) {
        report.fail(from, format_args!("copied to {} but could not be removed ({error}); both stay", to.display()));
        return false;
    }
    true
}

/// Creates the empty file `to`, failing when anything is already there, so no other file can
/// take the name between the check and the write and nothing is ever overwritten. On Unix it is
/// created with the permissions of the file it is a copy of, so a private file is never readable
/// by others, not even for the moment before they are set.
fn claim(to: &Path, permissions: &fs::Permissions) -> io::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
        options.mode(permissions.mode());
    }
    // No error is being dropped here: where a file has no mode there is nothing to copy from
    // the old permissions, and the parameter would otherwise be an unused one.
    #[cfg(not(unix))]
    let _ = permissions;
    options.open(to).map(drop)
}

/// Removes `folder` when nothing is left in it.
fn remove_if_empty(folder: &Path, report: &mut Migration) {
    let empty = fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_none());
    if empty && let Err(error) = fs::remove_dir(folder) {
        report.warn(folder, format_args!("is empty but could not be removed ({error})"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder of this test's own under the temporary folder.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-migrate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a temporary folder");
        dir
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("a folder")).expect("folders");
        fs::write(path, text).expect("written");
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    }

    /// The legacy folder and the ecosystem's places for the application `packages`.
    struct Layout {
        root: PathBuf,
        legacy: PathBuf,
        file: PathBuf,
        dir: PathBuf,
    }

    impl Layout {
        fn new(name: &str) -> Self {
            let root = temp_dir(name);
            let legacy = root.join("quvyta-packages");
            let ecosystem = root.join("quvyta");
            Self { file: ecosystem.join("packages.conf"), dir: ecosystem.join("packages"), legacy, root }
        }

        fn adopt(&self) -> Migration {
            adopt(&self.legacy, &self.file, &self.dir)
        }
    }

    impl Drop for Layout {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn settings_become_the_app_file_and_the_rest_moves_under_the_app_folder() {
        let layout = Layout::new("plain");
        write(&layout.legacy.join("settings.toml"), "theme = \"iris\"\n");
        write(&layout.legacy.join("history.toml"), "one\n");
        write(&layout.legacy.join("profiles/work/settings.toml"), "nested\n");

        let report = layout.adopt();
        assert!(report.is_clean(), "{:?}", report.diagnostics());
        assert_eq!(read(&layout.file), "theme = \"iris\"\n");
        assert_eq!(read(&layout.dir.join("history.toml")), "one\n");
        assert_eq!(read(&layout.dir.join("profiles/work/settings.toml")), "nested\n", "only the top file is special");
        assert_eq!(report.moved().len(), 3);
        assert!(report.moved().contains(&(layout.legacy.join("settings.toml"), layout.file.clone())));
        assert!(!layout.legacy.exists(), "the emptied old folders are gone");
    }

    #[test]
    fn a_second_run_finds_nothing_and_changes_nothing() {
        let layout = Layout::new("twice");
        write(&layout.legacy.join("settings.toml"), "a = 1\n");
        assert_eq!(layout.adopt().moved().len(), 1);
        let again = layout.adopt();
        assert_eq!(again, Migration::default());
        assert_eq!(read(&layout.file), "a = 1\n");

        let missing = adopt(&layout.root.join("never-there"), &layout.file, &layout.dir);
        assert_eq!(missing, Migration::default(), "a missing old folder is no problem");
    }

    #[test]
    fn when_the_old_folder_is_the_app_folder_only_the_settings_move() {
        let root = temp_dir("in-place");
        let dir = root.join("quvyta").join("focus");
        let file = root.join("quvyta").join("focus.conf");
        write(&dir.join("settings.toml"), "slide = true\n");
        write(&dir.join("blocks.toml"), "blocks\n");

        let report = adopt(&dir, &file, &dir);
        assert!(report.is_clean(), "{:?}", report.diagnostics());
        assert_eq!(report.moved(), &[(dir.join("settings.toml"), file.clone())]);
        assert_eq!(read(&file), "slide = true\n");
        assert_eq!(read(&dir.join("blocks.toml")), "blocks\n", "the other files are already in place");
        assert!(dir.is_dir(), "the application's folder stays");

        // Once the settings are gone the folder is the application's own, and nothing moves.
        assert_eq!(adopt(&dir, &file, &dir), Migration::default());
        // An empty application folder is not removed either.
        fs::remove_file(dir.join("blocks.toml")).expect("remove");
        assert_eq!(adopt(&dir, &file, &dir), Migration::default());
        assert!(dir.is_dir());
        fs::remove_dir_all(&root).expect("clean");
    }

    #[test]
    fn a_taken_place_keeps_both_files() {
        let layout = Layout::new("conflict");
        write(&layout.legacy.join("settings.toml"), "old\n");
        write(&layout.legacy.join("notes.txt"), "moves\n");
        write(&layout.file, "new\n");

        let report = layout.adopt();
        assert_eq!(read(&layout.file), "new\n", "never overwritten");
        assert_eq!(read(&layout.legacy.join("settings.toml")), "old\n", "never lost");
        assert_eq!(read(&layout.dir.join("notes.txt")), "moves\n", "the rest still moves");
        assert_eq!(report.diagnostics().len(), 1, "{:?}", report.diagnostics());
        let message = &report.diagnostics()[0].message;
        assert!(message.contains("settings.toml") && message.contains("already exists"), "{message}");
        assert!(layout.legacy.is_dir(), "a folder with a file left in it stays");

        // Nothing changes on the next run either: the same conflict, reported again.
        let again = layout.adopt();
        assert!(again.moved().is_empty());
        assert_eq!(again.diagnostics(), report.diagnostics());
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_is_left_alone() {
        let layout = Layout::new("link");
        write(&layout.root.join("dotfiles/settings.toml"), "linked\n");
        fs::create_dir_all(&layout.legacy).expect("folder");
        std::os::unix::fs::symlink(layout.root.join("dotfiles/settings.toml"), layout.legacy.join("settings.toml"))
            .expect("link");

        let report = layout.adopt();
        assert!(report.moved().is_empty());
        assert!(report.diagnostics()[0].message.contains("symbolic link"), "{:?}", report.diagnostics());
        assert!(fs::symlink_metadata(layout.legacy.join("settings.toml")).expect("still there").is_symlink());
        assert!(!layout.file.exists());
        assert_eq!(read(&layout.root.join("dotfiles/settings.toml")), "linked\n");

        // A linked old folder is not followed either.
        let linked = layout.root.join("linked-folder");
        std::os::unix::fs::symlink(&layout.legacy, &linked).expect("link");
        let report = adopt(&linked, &layout.file, &layout.dir);
        assert!(report.moved().is_empty() && !report.is_clean());
    }

    #[cfg(unix)]
    #[test]
    fn permissions_come_along() {
        use std::os::unix::fs::PermissionsExt as _;
        let layout = Layout::new("mode");
        let secret = layout.legacy.join("tokens/api.toml");
        write(&secret, "token = \"s\"\n");
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o600)).expect("private");
        write(&layout.legacy.join("settings.toml"), "a = 1\n");
        fs::set_permissions(layout.legacy.join("settings.toml"), fs::Permissions::from_mode(0o640)).expect("mode");

        assert!(layout.adopt().is_clean());
        let mode = |path: &Path| fs::metadata(path).expect("moved").permissions().mode() & 0o777;
        assert_eq!(mode(&layout.dir.join("tokens/api.toml")), 0o600);
        assert_eq!(mode(&layout.file), 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_stays_and_is_reported() {
        use std::os::unix::fs::PermissionsExt as _;
        let layout = Layout::new("unreadable");
        let locked = layout.legacy.join("locked.toml");
        write(&locked, "x\n");
        write(&layout.legacy.join("settings.toml"), "a = 1\n");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("lock");
        if fs::read(&locked).is_ok() {
            // Running as root: every file is readable, so there is nothing to see here.
            return;
        }

        let report = layout.adopt();
        assert_eq!(report.moved().len(), 1, "the readable file still moves");
        assert_eq!(report.diagnostics().len(), 1, "{:?}", report.diagnostics());
        assert!(report.diagnostics()[0].message.contains("could not be read"), "{:?}", report.diagnostics());
        assert!(locked.exists() && !layout.dir.join("locked.toml").exists(), "the file stays, no copy is left");
        assert!(layout.legacy.is_dir());
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o600)).expect("unlock for cleanup");
    }

    #[test]
    fn folders_that_hold_each_other_are_refused() {
        let root = temp_dir("overlap");
        let ecosystem = root.join("quvyta");
        write(&ecosystem.join("settings.toml"), "a = 1\n");
        let report = adopt(&ecosystem, &ecosystem.join("code.conf"), &ecosystem.join("code"));
        assert!(report.moved().is_empty());
        assert!(report.diagnostics()[0].message.contains("overlaps"), "{:?}", report.diagnostics());
        assert_eq!(read(&ecosystem.join("settings.toml")), "a = 1\n");
        fs::remove_dir_all(&root).expect("clean");
    }

    #[test]
    fn an_old_file_in_place_of_a_folder_is_reported() {
        let root = temp_dir("not-a-folder");
        write(&root.join("old"), "a file\n");
        let report = adopt(&root.join("old"), &root.join("new.conf"), &root.join("new"));
        assert!(report.diagnostics()[0].message.contains("not a folder"), "{:?}", report.diagnostics());
        assert_eq!(read(&root.join("old")), "a file\n");
        fs::remove_dir_all(&root).expect("clean");
    }
}
