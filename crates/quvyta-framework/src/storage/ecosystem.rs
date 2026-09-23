//! One settings folder for an ecosystem of applications.
//!
//! Applications made to be used together keep their settings side by side, so a user finds all
//! of them in one place and a setting they share is written once:
//!
//! ```text
//! ~/.config/quvyta/
//!     quvyta.conf     the ecosystem's shared settings
//!     code.conf       the settings of the application `code`
//!     code/           its other configuration files
//! ```
//!
//! What a member remembers between runs and what it can rebuild sit in the ecosystem's folder under
//! the platform's state and cache folders, one folder per member: `~/.local/state/quvyta/code`,
//! `~/.cache/quvyta/code`.
//!
//! The work the user makes with an application lives in the Documents folder, under the ecosystem's
//! and the application's titles: `~/Documents/Quvyta/Code`.

use std::path::{Path, PathBuf};

use super::dirs::{cache_root, config_root, env_lookup, state_root};
use super::documents::documents_dir;
use super::migrate::{self, Migration};

/// The extension every settings file of an ecosystem carries.
const EXTENSION: &str = "conf";

/// An ecosystem of applications that share one settings folder.
///
/// The `id` names the folder and the shared file where names are lowercase by custom, on Linux
/// and other Unix systems; the `title` names the folder where a user sees it written like a
/// name, on macOS and Windows, and always in the Documents folder. File names are always the
/// lowercase id: `quvyta.conf`, `code.conf`.
///
/// ```
/// use qframe::storage::Ecosystem;
///
/// let ecosystem = Ecosystem::QUVYTA;
/// if let (Some(folder), Some(file)) = (ecosystem.config_dir(), ecosystem.app_file("code")) {
///     assert_eq!(file, folder.join("code.conf"));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ecosystem {
    id: &'static str,
    title: &'static str,
}

impl Ecosystem {
    /// The Quvyta ecosystem: `~/.config/quvyta` on Linux and other Unix systems,
    /// `~/Library/Application Support/Quvyta` on macOS, `%APPDATA%\Quvyta` on Windows.
    pub const QUVYTA: Ecosystem = Ecosystem::new("quvyta", "Quvyta");

    /// An ecosystem with a lowercase `id` for folder and file names, such as `quvyta`, and a `title`
    /// for the places a user reads it as a name, such as `Quvyta`.
    #[must_use]
    pub const fn new(id: &'static str, title: &'static str) -> Self {
        Self { id, title }
    }

    /// The lowercase name of the ecosystem's folder on Linux and other Unix systems and of its
    /// shared file.
    #[must_use]
    pub fn id(&self) -> &'static str {
        self.id
    }

    /// The ecosystem's name as a user reads it.
    #[must_use]
    pub fn title(&self) -> &'static str {
        self.title
    }

    /// The folder every settings file of the ecosystem lives in:
    ///
    /// - Linux and other Unix systems: `$XDG_CONFIG_HOME/<id>` when `XDG_CONFIG_HOME` is an
    ///   absolute path, else `$HOME/.config/<id>`.
    /// - macOS: `$HOME/Library/Application Support/<title>`.
    /// - Windows: `%APPDATA%\<title>`, the roaming folder.
    ///
    /// `None` when there is no home folder, as for [`config_dir`](super::config_dir). The folder
    /// is not created.
    #[must_use]
    pub fn config_dir(&self) -> Option<PathBuf> {
        self.config_under(config_root(env_lookup))
    }

    /// The settings every application of the ecosystem shares: `<config_dir>/<id>.conf`, such as
    /// `quvyta.conf`.
    #[must_use]
    pub fn shared_file(&self) -> Option<PathBuf> {
        self.config_dir().map(|dir| dir.join(file_name(self.id)))
    }

    /// The settings file of application `app`: `<config_dir>/<app>.conf`, such as `code.conf`.
    /// `app` is the application's lowercase id. An application whose id is the ecosystem's own
    /// would get the [shared file](Self::shared_file), so give it another id.
    #[must_use]
    pub fn app_file(&self, app: &str) -> Option<PathBuf> {
        self.config_dir().map(|dir| dir.join(file_name(app)))
    }

    /// The folder for the other configuration files of application `app`, next to its settings
    /// file: `<config_dir>/<app>`. Not created.
    #[must_use]
    pub fn app_dir(&self, app: &str) -> Option<PathBuf> {
        self.config_dir().map(|dir| dir.join(app))
    }

    /// Where application `app` of the ecosystem keeps its state, such as the result of its last
    /// background check: `<state folder>/<ecosystem>/<app>`.
    ///
    /// - Linux and other Unix systems: `$XDG_STATE_HOME/<id>/<app>` when `XDG_STATE_HOME` is an
    ///   absolute path, else `$HOME/.local/state/<id>/<app>`.
    /// - macOS: `$HOME/Library/Application Support/<title>/<app>`.
    /// - Windows: `%LOCALAPPDATA%\<title>\<app>`.
    ///
    /// `None` when there is no home folder, as for [`state_dir`](super::state_dir). Not created.
    #[must_use]
    pub fn state_dir(&self, app: &str) -> Option<PathBuf> {
        self.member_under(state_root(env_lookup), app)
    }

    /// Where application `app` of the ecosystem keeps files it can rebuild at any time:
    /// `<cache folder>/<ecosystem>/<app>`.
    ///
    /// - Linux and other Unix systems: `$XDG_CACHE_HOME/<id>/<app>` when `XDG_CACHE_HOME` is an
    ///   absolute path, else `$HOME/.cache/<id>/<app>`.
    /// - macOS: `$HOME/Library/Caches/<title>/<app>`.
    /// - Windows: `%LOCALAPPDATA%\<title>\<app>`.
    ///
    /// `None` when there is no home folder, as for [`cache_dir`](super::cache_dir). Not created.
    #[must_use]
    pub fn cache_dir(&self, app: &str) -> Option<PathBuf> {
        self.member_under(cache_root(env_lookup), app)
    }

    /// Where the work the user makes with an application is kept by default:
    /// `<documents>/<ecosystem title>/<app_title>`, such as `~/Documents/Quvyta/Code`, in the
    /// [Documents folder](super::documents_dir) under the name the user's desktop gave it.
    /// `app_title` is the application's name as the user reads it. `None` when there is no home
    /// folder. Not created.
    #[must_use]
    pub fn workspace_dir(&self, app_title: &str) -> Option<PathBuf> {
        documents_dir().map(|documents| self.workspace_under(&documents, app_title))
    }

    /// Moves the settings of application `app` from the folder it used before it joined the
    /// ecosystem into the ecosystem's layout, once, without losing anything.
    ///
    /// `legacy_dir/settings.toml` becomes [`app_file`](Self::app_file); every other file under
    /// `legacy_dir`, at any depth, goes to the same place under [`app_dir`](Self::app_dir). When
    /// `legacy_dir` already is the application's folder, only `settings.toml` moves and the other
    /// files stay where they are. Call it at start, before
    /// [`Settings::load_member`](super::Settings::load_member).
    ///
    /// Every file is copied first, with its permissions, then read back and compared, and only
    /// then removed from the old place. A file whose new place is already taken stays where it
    /// is, and so do symbolic links, which are never followed; nothing is overwritten or merged.
    /// Old folders left empty are removed, from the deepest up; a folder with anything left in it
    /// is kept. A missing `legacy_dir` is nothing to do, so calling it again after a finished move
    /// changes nothing. Whatever stayed behind is in the report, with the reason.
    ///
    /// A crash in the middle of a move can leave the new file empty next to the old one; the old
    /// one is still whole, and the next call reports the pair instead of choosing between them.
    #[must_use]
    pub fn adopt(&self, app: &str, legacy_dir: &Path) -> Migration {
        match self.config_dir() {
            Some(config_dir) => self.adopt_in(&config_dir, app, legacy_dir),
            None => Migration::without_folder(),
        }
    }

    /// [`adopt`](Self::adopt) into `config_dir` as the ecosystem's folder instead of this
    /// platform's, for a test or a demo that must leave the user's own settings alone: the
    /// settings become `<config_dir>/<app>.conf` and the other files move under
    /// `<config_dir>/<app>`.
    #[must_use]
    pub fn adopt_in(&self, config_dir: &Path, app: &str, legacy_dir: &Path) -> Migration {
        migrate::adopt(legacy_dir, &config_dir.join(file_name(app)), &config_dir.join(app))
    }

    /// The folder of member `app` in the ecosystem's folder under `root`.
    fn member_under(&self, root: Option<PathBuf>, app: &str) -> Option<PathBuf> {
        self.config_under(root).map(|ecosystem| ecosystem.join(app))
    }

    /// The ecosystem's folder under the `root` of this platform, config or any other.
    fn config_under(&self, root: Option<PathBuf>) -> Option<PathBuf> {
        // macOS and Windows show these folders by their names, so they are written as names are.
        let name = if cfg!(any(windows, target_os = "macos")) { self.title } else { self.id };
        root.map(|root| root.join(name))
    }

    /// The workspace of `app_title` under the Documents folder `documents`.
    fn workspace_under(&self, documents: &Path, app_title: &str) -> PathBuf {
        documents.join(self.title).join(app_title)
    }
}

/// The settings file name of `id`.
pub(super) fn file_name(id: &str) -> String {
    format!("{id}.{EXTENSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lookup over a fixed list of variables, so no test reads the developer's own environment.
    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<PathBuf> {
        move |name: &str| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| PathBuf::from(value))
    }

    #[test]
    fn the_ecosystem_folder_is_lowercase_on_unix() {
        if !cfg!(all(unix, not(target_os = "macos"))) {
            return;
        }
        let ecosystem = Ecosystem::QUVYTA;
        let home = config_root(env(&[("HOME", "/home/ada")]));
        assert_eq!(ecosystem.config_under(home), Some(PathBuf::from("/home/ada/.config/quvyta")));
        let xdg = config_root(env(&[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "/cfg")]));
        assert_eq!(ecosystem.config_under(xdg), Some(PathBuf::from("/cfg/quvyta")));
    }

    #[test]
    fn the_ecosystem_folder_carries_the_title_on_macos_and_windows() {
        if cfg!(target_os = "macos") {
            let root = config_root(env(&[("HOME", "/Users/ada")]));
            let expected = PathBuf::from("/Users/ada/Library/Application Support/Quvyta");
            assert_eq!(Ecosystem::QUVYTA.config_under(root), Some(expected));
        }
        if cfg!(windows) {
            let root = config_root(env(&[("APPDATA", r"C:\Users\ada\AppData\Roaming")]));
            let expected = PathBuf::from(r"C:\Users\ada\AppData\Roaming\Quvyta");
            assert_eq!(Ecosystem::QUVYTA.config_under(root), Some(expected));
        }
    }

    #[test]
    fn a_member_keeps_its_state_and_cache_under_the_ecosystem_folder() {
        if !cfg!(all(unix, not(target_os = "macos"))) {
            return;
        }
        let ecosystem = Ecosystem::QUVYTA;
        let home = env(&[("HOME", "/home/ada")]);
        assert_eq!(
            ecosystem.member_under(state_root(&home), "packages"),
            Some(PathBuf::from("/home/ada/.local/state/quvyta/packages"))
        );
        assert_eq!(
            ecosystem.member_under(cache_root(&home), "packages"),
            Some(PathBuf::from("/home/ada/.cache/quvyta/packages"))
        );
        let xdg = env(&[("HOME", "/home/ada"), ("XDG_STATE_HOME", "/st"), ("XDG_CACHE_HOME", "/ca")]);
        assert_eq!(ecosystem.member_under(state_root(&xdg), "packages"), Some(PathBuf::from("/st/quvyta/packages")));
        assert_eq!(ecosystem.member_under(cache_root(&xdg), "packages"), Some(PathBuf::from("/ca/quvyta/packages")));
        let relative = env(&[("HOME", "/home/ada"), ("XDG_CACHE_HOME", "ca")]);
        assert_eq!(
            ecosystem.member_under(cache_root(&relative), "packages"),
            Some(PathBuf::from("/home/ada/.cache/quvyta/packages"))
        );
        assert_eq!(ecosystem.member_under(cache_root(env(&[])), "packages"), None);
    }

    #[test]
    fn the_public_state_and_cache_folders_end_in_ecosystem_and_member() {
        let ecosystem = Ecosystem::QUVYTA;
        for dir in [ecosystem.state_dir("packages"), ecosystem.cache_dir("packages")].into_iter().flatten() {
            assert!(
                dir.ends_with(Path::new(ecosystem.id()).join("packages"))
                    || dir.ends_with(Path::new(ecosystem.title()).join("packages")),
                "{}",
                dir.display()
            );
        }
    }

    #[test]
    fn no_home_means_no_ecosystem_folder() {
        assert_eq!(Ecosystem::QUVYTA.config_under(config_root(env(&[]))), None);
    }

    #[test]
    fn files_are_lowercase_ids_in_the_ecosystem_folder() {
        let ecosystem = Ecosystem::new("tools", "Tools");
        let Some(dir) = ecosystem.config_dir() else { return };
        assert_eq!(ecosystem.shared_file(), Some(dir.join("tools.conf")));
        assert_eq!(ecosystem.app_file("code"), Some(dir.join("code.conf")));
        assert_eq!(ecosystem.app_dir("code"), Some(dir.join("code")));
        assert_eq!((ecosystem.id(), ecosystem.title()), ("tools", "Tools"));
    }

    #[test]
    fn the_workspace_is_the_ecosystem_and_app_titles_under_documents() {
        let documents = Path::new("/home/ada/Belgeler");
        let expected = PathBuf::from("/home/ada/Belgeler/Quvyta/Code");
        assert_eq!(Ecosystem::QUVYTA.workspace_under(documents, "Code"), expected);
        if let (Some(documents), Some(workspace)) = (documents_dir(), Ecosystem::QUVYTA.workspace_dir("Code")) {
            assert_eq!(workspace, documents.join("Quvyta").join("Code"));
        }
    }
}
