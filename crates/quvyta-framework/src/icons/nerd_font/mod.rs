//! Installing the Nerd Font symbols, so a terminal can draw the Nerd glyphs of the icon sets.
//!
//! Only "Symbols Nerd Font Mono" is installed: a font of symbols and nothing else. The user keeps
//! the font their terminal draws text with; a terminal that takes the glyphs its font lacks from
//! other installed fonts (through fontconfig on Linux, the system's font fallback elsewhere)
//! finds them in this one. A terminal that does not do that still shows boxes or question marks,
//! which is why an application shows [`GlyphSample`](super::GlyphSample)s after the install and
//! says what to do then, see [`after_install_text`].
//!
//! The archive comes from a fixed Nerd Fonts release, [`RELEASE`], and its SHA-256 is written
//! here; a download that does not match is deleted. The system's own programs do the work, so
//! the framework carries no network or archive code: `curl` downloads, `sha256sum` or
//! `shasum -a 256` (`certutil` on Windows) checks, and `tar` unpacks. Everything runs in the
//! background as a [`Task`] that reports [`Progress`].
//!
//! The font goes into the user's own font folder, see [`target_dir`], so nothing needs an
//! administrator. Undoing the install is deleting one path, which [`Progress::Done`] names.
//!
//! ```no_run
//! use qframe::icons::nerd_font::{self, Progress};
//! use qframe::runtime::Command;
//!
//! enum Msg {
//!     Install,
//!     Font(Progress),
//! }
//!
//! fn update(msg: Msg) -> Command<Msg> {
//!     match msg {
//!         Msg::Install if !nerd_font::installed() => nerd_font::install(Msg::Font),
//!         // Show `progress.text()` while it runs; after `Progress::Done`, show sample
//!         // glyphs and `nerd_font::after_install_text()`.
//!         _ => Command::none(),
//!     }
//! }
//! ```

mod steps;

use std::path::{Path, PathBuf};

use super::detect::{contains_nerd_font, default_font_dirs};
use crate::i18n::translate_active;
use crate::runtime::{Command, Task};

/// The Nerd Fonts release the font is installed from.
pub const RELEASE: &str = "v3.5.1";

/// The family name of the installed font, as terminals and font pickers list it.
pub const FAMILY: &str = "Symbols Nerd Font Mono";

/// The one file taken from the archive and installed.
pub const FONT_FILE: &str = "SymbolsNerdFontMono-Regular.ttf";

/// The folder of its own the font goes into on Linux, inside the user's font folder.
const LINUX_FOLDER: &str = "QuvytaNerdFont";

/// Where the release archives are published.
const RELEASE_URL: &str = "https://github.com/ryanoasis/nerd-fonts/releases/download";

/// The `.tar.xz` archive: GNU tar on Linux and bsdtar on macOS open it.
const TAR_XZ: (&str, &str) =
    ("NerdFontsSymbolsOnly.tar.xz", "01172f37db8543edb102e5cb5c64101c9f4686630804d49b419aa07b23a69996");

/// The `.zip` archive: the `tar` of Windows opens zip files but not every build opens xz.
const ZIP: (&str, &str) =
    ("NerdFontsSymbolsOnly.zip", "fdca3682534f6f65e1ccb2345b0362ccf67d9b8eca7c8025330946e93e2473bc");

/// The operating system, for the choices that differ: folders, archive, checksum tool and
/// registration. Unix systems other than macOS follow Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Os {
    Linux,
    Mac,
    Windows,
}

impl Os {
    fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }
}

fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Whether a Nerd Font file lies in the font folders of this system: any file whose name
/// contains "nerd", the way glyph detection looks. A font on disk is not proof that the terminal
/// draws with it; the user's eye on a [`GlyphSample`](super::GlyphSample) is.
#[must_use]
pub fn installed() -> bool {
    installed_in(&default_font_dirs(process_env))
}

/// Whether a Nerd Font file lies in one of `dirs`, looking up to three folders deep.
#[must_use]
pub fn installed_in(dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| contains_nerd_font(dir, 3))
}

/// Where [`install`] puts the font, or `None` when the system names no folder for the user.
///
/// - Linux and other Unix systems: `$XDG_DATA_HOME/fonts/QuvytaNerdFont`, or
///   `~/.local/share/fonts/QuvytaNerdFont` when the variable is unset or not absolute.
/// - macOS: `~/Library/Fonts`.
/// - Windows: `%LOCALAPPDATA%\Microsoft\Windows\Fonts`.
///
/// Asking does not create the folder.
#[must_use]
pub fn target_dir() -> Option<PathBuf> {
    target_dir_for(Os::current(), process_env)
}

fn target_dir_for(os: Os, env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let absolute = |name: &str| env(name).map(PathBuf::from).filter(|path| path.is_absolute());
    match os {
        Os::Linux => absolute("XDG_DATA_HOME")
            .or_else(|| absolute("HOME").map(|home| home.join(".local").join("share")))
            .map(|data| data.join("fonts").join(LINUX_FOLDER)),
        Os::Mac => absolute("HOME").map(|home| home.join("Library").join("Fonts")),
        Os::Windows => absolute("LOCALAPPDATA").map(|local| local.join("Microsoft").join("Windows").join("Fonts")),
    }
}

/// An archive to install from: where to download it and the SHA-256 it must have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive {
    url: String,
    sha256: String,
}

impl Archive {
    /// The archive of [`RELEASE`] that this system's `tar` can open: `.tar.xz` on Linux and
    /// macOS, `.zip` on Windows.
    #[must_use]
    pub fn release() -> Self {
        release_for(Os::current())
    }

    /// An archive at `url`, which `curl` understands (`https://` or `file://`), expected to have
    /// the SHA-256 `sha256`, in hexadecimal. The archive must hold [`FONT_FILE`] at its top.
    #[must_use]
    pub fn new(url: impl Into<String>, sha256: &str) -> Self {
        Self { url: url.into(), sha256: sha256.trim().to_ascii_lowercase() }
    }

    /// Where the archive is downloaded from.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The SHA-256 the download must have, in lowercase hexadecimal.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

fn release_for(os: Os) -> Archive {
    let (name, sha256) = if os == Os::Windows { ZIP } else { TAR_XZ };
    Archive::new(format!("{RELEASE_URL}/{RELEASE}/{name}"), sha256)
}

/// How to install the font: the archive, the folder and whether the system is told.
///
/// [`Install::new`] is what [`install`] runs. The other methods are for installing elsewhere,
/// such as into a test's temporary folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    archive: Archive,
    target: Option<PathBuf>,
    register: bool,
    /// Where the download folder is made: the system's temporary folder.
    staging: PathBuf,
}

impl Default for Install {
    fn default() -> Self {
        Self::new()
    }
}

impl Install {
    /// The release archive into [`target_dir`], telling the system about the new font.
    #[must_use]
    pub fn new() -> Self {
        Self { archive: Archive::release(), target: target_dir(), register: true, staging: std::env::temp_dir() }
    }

    /// Installs from `archive` instead of the release.
    #[must_use]
    pub fn archive(mut self, archive: Archive) -> Self {
        self.archive = archive;
        self
    }

    /// Installs into `dir` instead of [`target_dir`]. The folder is created when missing.
    #[must_use]
    pub fn target(mut self, dir: impl Into<PathBuf>) -> Self {
        self.target = Some(dir.into());
        self
    }

    /// Whether the system is told about the new font, on by default: on Linux `fc-cache -f`
    /// reads the folder (skipped when fontconfig is not installed), on Windows the font is
    /// entered under `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts`, and macOS needs
    /// nothing. Turn it off for a folder the system does not look at.
    #[must_use]
    pub fn register(mut self, register: bool) -> Self {
        self.register = register;
        self
    }

    /// The folder the font goes into, if there is one.
    #[must_use]
    pub fn target_dir(&self) -> Option<&Path> {
        self.target.as_deref()
    }

    /// Downloads, verifies and installs, on the calling thread, handing every step to
    /// `on_progress`; the last one is [`Progress::Done`] or [`Progress::Failed`]. Returns the
    /// path [`Progress::Done`] names.
    ///
    /// `cancel` is asked between steps and while a program runs; when it turns true the program
    /// is stopped and nothing more is written. The download is kept in a folder of its own under
    /// the system's temporary folder and removed at the end, whatever the outcome.
    ///
    /// # Errors
    ///
    /// Every way the install can fail is an [`InstallError`], also handed to `on_progress` as
    /// [`Progress::Failed`] unless the install was cancelled.
    pub fn run(
        self,
        cancel: &dyn Fn() -> bool,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<PathBuf, InstallError> {
        let result = steps::run(&self, Os::current(), cancel, on_progress);
        match &result {
            Ok(path) => on_progress(Progress::Done { path: path.clone() }),
            Err(InstallError::Cancelled) => {}
            Err(error) => on_progress(Progress::Failed(error.clone())),
        }
        result
    }

    /// The install as a background [`Task`]. Every step arrives as `on_progress(step)`, and the
    /// task's own result is `on_progress(Progress::Done { .. })`. A failure arrives as
    /// [`Progress::Failed`] and also fails the task with the same text, so a
    /// [`TaskList`](crate::widgets::TaskList) shows it; the task's label and notes are
    /// translated when this is called, so call it where the application's language is active,
    /// such as in `update`.
    #[must_use]
    pub fn task<Msg: Send + 'static>(self, on_progress: impl Fn(Progress) -> Msg + Send + Sync + 'static) -> Task<Msg> {
        let texts = steps::Texts::capture();
        Task::new(translate_active("quvyta.nerd-font.task", &[]), move |cx| {
            let mut noted = false;
            let mut report = |progress: Progress| {
                match &progress {
                    Progress::Downloading { fraction } => {
                        if !noted {
                            noted = true;
                            cx.note(texts.get("quvyta.nerd-font.downloading", ""));
                        }
                        if let Some(fraction) = fraction {
                            // The download is most of the time the install takes.
                            cx.progress(fraction * 0.9);
                        }
                    }
                    Progress::Verifying => cx.note(texts.get("quvyta.nerd-font.verifying", "")),
                    Progress::Installing => cx.note(texts.get("quvyta.nerd-font.installing", "")),
                    Progress::Done { .. } | Progress::Failed(_) => return,
                }
                cx.send(on_progress(progress));
            };
            let result = self.run(&|| cx.is_cancelled(), &mut report);
            match result {
                Ok(path) => {
                    cx.progress(1.0);
                    Ok(on_progress(Progress::Done { path }))
                }
                Err(error) => {
                    let (key, detail) = error.key_and_detail();
                    let reason = texts.get(key, detail);
                    if error != InstallError::Cancelled {
                        cx.send(on_progress(Progress::Failed(error)));
                    }
                    Err(reason)
                }
            }
        })
    }
}

/// Installs the font in the background with [`Install::new`], delivering every step as
/// `on_progress(step)`. See [`Install::task`] for a task to cancel or to follow in a task list.
#[must_use]
pub fn install<Msg: Send + 'static>(on_progress: impl Fn(Progress) -> Msg + Send + Sync + 'static) -> Command<Msg> {
    Command::task(Install::new().task(on_progress))
}

/// One step of an install.
#[derive(Debug, Clone, PartialEq)]
pub enum Progress {
    /// The archive is downloading; `fraction` from 0 to 1 once `curl` knows the size.
    Downloading {
        /// The downloaded share, when known.
        fraction: Option<f32>,
    },
    /// The download is checked against its SHA-256.
    Verifying,
    /// The font is unpacked, copied into its folder and registered.
    Installing,
    /// The font is installed. `path` is what to delete to undo it: the folder of its own on
    /// Linux, the font file where the folder is shared with other fonts (macOS, Windows; there
    /// the entry under `HKCU\…\Fonts` stays and names a missing file, which Windows ignores).
    Done {
        /// What to delete to undo the install.
        path: PathBuf,
    },
    /// The install failed; nothing was left half copied.
    Failed(InstallError),
}

impl Progress {
    /// The step in the active language, such as "Checking the download".
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Downloading { fraction: None } => translate_active("quvyta.nerd-font.downloading", &[]),
            Self::Downloading { fraction: Some(fraction) } => {
                // A percentage of 0 to 100 always fits.
                #[allow(clippy::cast_possible_truncation)]
                let percent = (fraction.clamp(0.0, 1.0) * 100.0).round() as i32;
                translate_active("quvyta.nerd-font.downloading-share", &[("percent", percent.into())])
            }
            Self::Verifying => translate_active("quvyta.nerd-font.verifying", &[]),
            Self::Installing => translate_active("quvyta.nerd-font.installing", &[]),
            Self::Done { path } => {
                translate_active("quvyta.nerd-font.done", &[("path", path.display().to_string().into())])
            }
            Self::Failed(error) => error.text(),
        }
    }
}

/// Why an install failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    /// The system names no font folder for the user, see [`target_dir`].
    NoFolder,
    /// A program the install needs is missing; its name.
    MissingTool(String),
    /// `curl` could not download the archive; its message.
    Download(String),
    /// The checksum program failed; its message.
    Verify(String),
    /// The download does not have the SHA-256 written in the source, so it was deleted.
    Checksum {
        /// The SHA-256 the archive should have.
        expected: String,
        /// The SHA-256 the download has.
        actual: String,
    },
    /// `tar` could not unpack the font; its message.
    Extract(String),
    /// The font could not be copied into its folder; the system's message.
    Copy(String),
    /// The font is in its folder, but the system could not be told; the program's message.
    Register(String),
    /// The install was cancelled.
    Cancelled,
}

impl InstallError {
    /// The locale key of the message and what fills its `{detail}`.
    fn key_and_detail(&self) -> (&'static str, &str) {
        match self {
            Self::NoFolder => ("quvyta.nerd-font.error-folder", ""),
            Self::MissingTool(tool) => ("quvyta.nerd-font.error-tool", tool),
            Self::Download(detail) => ("quvyta.nerd-font.error-download", detail),
            Self::Verify(detail) => ("quvyta.nerd-font.error-verify", detail),
            Self::Checksum { .. } => ("quvyta.nerd-font.error-checksum", ""),
            Self::Extract(detail) => ("quvyta.nerd-font.error-extract", detail),
            Self::Copy(detail) => ("quvyta.nerd-font.error-copy", detail),
            Self::Register(detail) => ("quvyta.nerd-font.error-register", detail),
            Self::Cancelled => ("quvyta.nerd-font.cancelled", ""),
        }
    }

    /// The reason in the active language, with the program's own message where there is one.
    #[must_use]
    pub fn text(&self) -> String {
        let (key, detail) = self.key_and_detail();
        translate_active(key, &[("detail", detail.into())])
    }
}

/// What to say once the font is installed, in the active language: look at the sample glyphs,
/// and if they are still boxes, install JetBrainsMono Nerd Font and choose it in the terminal's
/// settings. Show it beside [`GlyphSample`](super::GlyphSample)s drawn again after the install.
#[must_use]
pub fn after_install_text() -> String {
    translate_active("quvyta.nerd-font.after-install", &[])
}

/// Whether a Nerd Font was found on this machine, as a sentence in the active language.
#[must_use]
pub fn status_text(installed: bool) -> String {
    let key = if installed { "quvyta.nerd-font.found" } else { "quvyta.nerd-font.missing" };
    translate_active(key, &[])
}

#[cfg(test)]
mod tests;
