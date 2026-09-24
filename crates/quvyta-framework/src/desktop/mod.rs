//! Which programs open a file, read from the desktop's own databases, and starting one of them.
//!
//! The desktop already knows what every file is and which programs open it: shared-mime-info says
//! what a file is, the desktop entries say which programs take which kinds, and `mimeapps.list`
//! holds the choices a person made in any other file manager. Reading those instead of keeping a
//! table of our own means a file opens here with the program it opens with everywhere else.
//!
//! - [`XdgDirs`] names the folders: [`XdgDirs::from_env`] with the process's variables in an
//!   application, a fake tree in a test, so no test ever reads the machine's own databases.
//! - [`MimeDb`] tells what a file is: from its name ([`MimeDb::guess`]), from its first 4 KiB when
//!   the name says nothing ([`MimeDb::sniff`]), and what else it is a case of
//!   ([`MimeDb::ancestors`]: Rust source is plain text).
//! - [`Apps`] lists the programs for a kind and the one to use; [`Openers::for_file`] answers both
//!   for one file as [`Choices`].
//! - [`DesktopApp::command`] gives the command line, never handed to a shell;
//!   [`DesktopApp::launch`] starts it the right way for its kind: a terminal program through a
//!   [`Handoff`](crate::runtime::Handoff), a graphical one through
//!   [`Open::program`](crate::runtime::Open::program).
//!
//! Reading never panics: a missing file is normal, and a line or a file that cannot be used is
//! skipped with a [`Diagnostic`] that says where it is. Nothing is ever written: the person's
//! defaults are their desktop's setting.
//!
//! Not read, on purpose: the binary `magic` rules of shared-mime-info (a name no pattern knows is
//! told apart only as text or bytes) and its XML descriptions, so there is no human-readable
//! comment for a kind: that would take an XML parser for one sentence.

mod apps;
mod exec;
mod keyfile;
mod launch;
mod mime;
mod program;

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;

use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub use apps::{Apps, DesktopApp};
pub use launch::{LaunchError, Launched, graphical_session};
pub use mime::MimeDb;

use crate::diagnostics::{Diagnostic, Location};

/// The folders the desktop's databases are read from, and the desktops the person is running.
///
/// Folders come in the order they are searched: the person's own before the system's, since what a
/// person installs or changes for themselves overrides what came with the system.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XdgDirs {
    /// The person's own data folder (`XDG_DATA_HOME`), searched first.
    pub data_home: Option<PathBuf>,
    /// The system's data folders (`XDG_DATA_DIRS`), most important first.
    pub data_dirs: Vec<PathBuf>,
    /// The person's own configuration folder (`XDG_CONFIG_HOME`), searched first.
    pub config_home: Option<PathBuf>,
    /// The system's configuration folders (`XDG_CONFIG_DIRS`), most important first.
    pub config_dirs: Vec<PathBuf>,
    /// The desktops the person is running (`XDG_CURRENT_DESKTOP`), lowercased, most specific
    /// first. A desktop may keep its own `mimeapps.list` beside the shared one.
    pub desktops: Vec<String>,
}

impl XdgDirs {
    /// The folders named by the environment, with the defaults the base directory specification
    /// gives for anything unset.
    ///
    /// `lookup` returns a variable's value: `XDG_DATA_HOME` (default `$HOME/.local/share`),
    /// `XDG_DATA_DIRS` (default `/usr/local/share:/usr/share`), `XDG_CONFIG_HOME` (default
    /// `$HOME/.config`), `XDG_CONFIG_DIRS` (default `/etc/xdg`) and `XDG_CURRENT_DESKTOP` (a
    /// colon-separated list). An empty value counts as unset. A relative folder is ignored, as the
    /// specification asks: it would mean something different in every folder the program is
    /// started from.
    pub fn from_env(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let var = |name: &str| lookup(name).filter(|value| !value.is_empty());
        let home = var("HOME").map(PathBuf::from).filter(|path| path.is_absolute());
        let own = |name: &str, below_home: &str| {
            var(name)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .or_else(|| home.as_ref().map(|home| home.join(below_home)))
        };
        let system = |name: &str, default: &[&str]| {
            let named: Vec<PathBuf> = var(name).map(|value| absolute_folders(&value)).unwrap_or_default();
            if named.is_empty() { default.iter().map(PathBuf::from).collect() } else { named }
        };
        Self {
            data_home: own("XDG_DATA_HOME", ".local/share"),
            data_dirs: system("XDG_DATA_DIRS", &["/usr/local/share", "/usr/share"]),
            config_home: own("XDG_CONFIG_HOME", ".config"),
            config_dirs: system("XDG_CONFIG_DIRS", &["/etc/xdg"]),
            desktops: var("XDG_CURRENT_DESKTOP")
                .map(|value| value.split(':').filter(|name| !name.is_empty()).map(str::to_lowercase).collect())
                .unwrap_or_default(),
        }
    }

    /// The data folders, the person's own first.
    fn data(&self) -> impl Iterator<Item = &PathBuf> {
        self.data_home.iter().chain(&self.data_dirs)
    }

    /// The configuration folders, the person's own first.
    fn config(&self) -> impl Iterator<Item = &PathBuf> {
        self.config_home.iter().chain(&self.config_dirs)
    }
}

/// The absolute folders in a colon-separated list.
fn absolute_folders(list: &str) -> Vec<PathBuf> {
    list.split(':').map(PathBuf::from).filter(|path| path.is_absolute()).collect()
}

/// Everything needed to tell which programs open a file, read once and asked many times.
#[derive(Debug, Clone)]
pub struct Openers {
    /// What kind each file is.
    pub mime: MimeDb,
    /// The programs and the associations between kinds and programs.
    pub apps: Apps,
}

impl Openers {
    /// Reads the kinds and the programs from the given folders.
    ///
    /// `lang` is the person's language as in `LANG` (`tr_TR.UTF-8`), used for program names;
    /// `path_var` is the program search path as in `PATH`, used to drop programs that are not
    /// installed.
    #[must_use]
    pub fn load(dirs: &XdgDirs, lang: &str, path_var: Option<&OsStr>) -> Self {
        Self { mime: MimeDb::load(dirs), apps: Apps::load(dirs, lang, path_var) }
    }

    /// Every problem found while reading, the kinds' files first.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<&Diagnostic> {
        self.mime.diagnostics().iter().chain(self.apps.diagnostics()).collect()
    }

    /// The kind of the file at `path` and the programs that open it.
    #[must_use]
    pub fn for_file(&self, path: &Path) -> Choices {
        let mime = self.mime.canonical(&self.mime.sniff(path));
        let apps: Vec<DesktopApp> = self.apps.for_mime(&self.mime, &mime).into_iter().cloned().collect();
        let default =
            self.apps.default_for(&self.mime, &mime).and_then(|chosen| apps.iter().position(|app| app.id == chosen.id));
        Choices { mime, apps, default }
    }
}

/// The programs that open one file.
#[derive(Debug, Clone, PartialEq)]
pub struct Choices {
    /// The file's kind, as a MIME type (`text/x-rust`).
    pub mime: String,
    /// The programs that open it, the most fitting first.
    pub apps: Vec<DesktopApp>,
    /// Which of [`apps`](Self::apps) opens the file when nothing else is asked for; `None` only
    /// when there is no program at all.
    pub default: Option<usize>,
}

/// The largest database file read. The real ones are a few hundred KiB at most; the limit keeps a
/// stray huge file from being pulled whole into memory.
const LARGEST_FILE: u64 = 16 << 20;

/// The bytes of a small regular file, or `None` when there is none to read.
///
/// A missing file is normal and says nothing. Anything but a regular file is refused before it is
/// opened, since reading a named pipe that happens to carry a database's name would wait forever;
/// that, a file too large and one that cannot be read are reported.
fn read_small(path: &Path, diagnostics: &mut Vec<Diagnostic>) -> Option<Vec<u8>> {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(error) => {
            diagnostics.push(Diagnostic::warning(None, format!("{}: {error}", path.display())));
            return None;
        }
    };
    let problem = if !meta.is_file() {
        "not a regular file; it is not read"
    } else if meta.len() > LARGEST_FILE {
        "larger than 16 MiB; it is not read"
    } else {
        return match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                diagnostics.push(Diagnostic::warning(None, format!("{}: {error}", path.display())));
                None
            }
        };
    };
    diagnostics.push(Diagnostic::warning(None, format!("{}: {problem}", path.display())));
    None
}

/// The lines of a text file with their numbers, starting at 1. A line that is not valid UTF-8 is
/// reported and skipped, so one broken line never costs the rest of the file.
fn lines<'a>(bytes: &'a [u8], path: &Path, diagnostics: &mut Vec<Diagnostic>) -> Vec<(usize, &'a str)> {
    let mut out = Vec::new();
    for (index, line) in bytes.split(|&byte| byte == b'\n').enumerate() {
        match std::str::from_utf8(line) {
            Ok(line) => out.push((index + 1, line.strip_suffix('\r').unwrap_or(line))),
            Err(_) => warn(diagnostics, path, index + 1, "the line is not UTF-8; it is skipped"),
        }
    }
    out
}

/// Reports a problem on line `line` of `path`.
fn warn(diagnostics: &mut Vec<Diagnostic>, path: &Path, line: usize, message: &str) {
    let location = Location { file: path.display().to_string(), line, column: 1 };
    diagnostics.push(Diagnostic::warning(Some(location), message));
}
