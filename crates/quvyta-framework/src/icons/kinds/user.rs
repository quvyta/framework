//! The folders of a person's home: Desktop, Downloads, Music and the rest, by the names the person
//! gave them.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::FileKind;
use crate::storage::UserDir;

/// Each user folder with its icon.
const XDG: &[(UserDir, &str)] = &[
    (UserDir::Desktop, "folder-desktop"),
    (UserDir::Documents, "folder-documents"),
    (UserDir::Downloads, "folder-downloads"),
    (UserDir::Music, "folder-music"),
    (UserDir::Pictures, "folder-pictures"),
    (UserDir::Public, "folder-public"),
    (UserDir::Templates, "folder-templates"),
    (UserDir::Videos, "folder-videos"),
];

/// The folders a person's home keeps for them, known by the names the person's language gives
/// them: `Downloads` in English, `İndirilenler` in Turkish, `Téléchargements` in French.
///
/// A folder called `Downloads` is only the downloads folder in the home; anywhere else it is a
/// folder like any other. So these are recognised by where they are, not by name alone, and
/// [`file_kind`](super::file_kind), which sees only a name, leaves them to this.
///
/// The names come from `user-dirs.dirs`, the file the desktop's XDG user directories are written
/// to. A home without that file uses the English names. A folder the file puts outside the home,
/// or at the home itself (which is how a folder is turned off there), is not recognised.
///
/// ```
/// use std::path::Path;
/// use qframe::icons::UserFolders;
///
/// let text = "XDG_DOWNLOAD_DIR=\"$HOME/İndirilenler\"\nXDG_MUSIC_DIR=\"$HOME/Müzik\"\n";
/// let folders = UserFolders::parse("/home/ada", text);
/// assert_eq!(folders.kind(Path::new("/home/ada/İndirilenler")).map(|k| k.icon()), Some("folder-downloads"));
/// assert_eq!(folders.kind(Path::new("/home/ada")).map(|k| k.icon()), Some("folder-home"));
/// assert_eq!(folders.kind(Path::new("/home/ada/work/Müzik")), None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFolders {
    home: PathBuf,
    /// Each folder's name inside the home, with its icon.
    names: Vec<(String, &'static str)>,
}

impl UserFolders {
    /// The folders of `home` by their English names, as a home without `user-dirs.dirs` has them.
    #[must_use]
    pub fn english(home: impl Into<PathBuf>) -> Self {
        let names = XDG.iter().map(|&(which, icon)| (which.english_name().to_owned(), icon)).collect();
        Self { home: home.into(), names }
    }

    /// The folders of `home` as the text of a `user-dirs.dirs` file names them.
    ///
    /// Lines such as `XDG_DOWNLOAD_DIR="$HOME/İndirilenler"` are read; comments, unknown names and
    /// folders outside the home are passed over.
    #[must_use]
    pub fn parse(home: impl Into<PathBuf>, text: &str) -> Self {
        let home = home.into();
        let mut names = Vec::new();
        for &(which, icon) in XDG {
            // The line is read by the same rules as `storage::user_dir`; only a folder right
            // inside the home is one of its folders, and `$HOME` itself turns the folder off.
            let Some(path) = crate::storage::user_dir_line(text, which, Some(&home)) else { continue };
            let inside = path.strip_prefix(&home).ok().and_then(Path::to_str).map(|name| name.trim_end_matches('/'));
            if let Some(name) = inside.filter(|name| is_one_name(name)) {
                names.push((name.to_owned(), icon));
            }
        }
        Self { home, names }
    }

    /// The folders of `home`, read from `user-dirs.dirs` in the folder `config` (the person's
    /// `$XDG_CONFIG_HOME`, `~/.config` by default), or by their English names when that file
    /// cannot be read.
    #[must_use]
    pub fn read(home: impl Into<PathBuf>, config: &Path) -> Self {
        let home = home.into();
        match std::fs::read_to_string(config.join("user-dirs.dirs")) {
            Ok(text) => Self::parse(home, &text),
            Err(_) => Self::english(home),
        }
    }

    /// The folders of the person running the application, read once from `$HOME` and
    /// `$XDG_CONFIG_HOME` the first time they are asked for; `None` without a `$HOME`.
    ///
    /// This reads a file the first time, so it is for the running application; a test gives the
    /// folders it means with [`parse`](Self::parse) or [`english`](Self::english) instead.
    #[must_use]
    pub fn current() -> Option<&'static Self> {
        static CURRENT: OnceLock<Option<UserFolders>> = OnceLock::new();
        CURRENT
            .get_or_init(|| {
                let home = PathBuf::from(std::env::var_os("HOME").filter(|home| !home.is_empty())?);
                let config = std::env::var_os("XDG_CONFIG_HOME")
                    .filter(|config| !config.is_empty())
                    .map_or_else(|| home.join(".config"), PathBuf::from);
                Some(Self::read(home, &config))
            })
            .as_ref()
    }

    /// The home the folders are in.
    #[must_use]
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The kind of the folder at `path` when it is the home or one of the home's folders:
    /// `folder-home`, `folder-downloads` and the rest; `None` for any other path.
    #[must_use]
    pub fn kind(&self, path: &Path) -> Option<FileKind> {
        if path == self.home {
            return Some(FileKind::of("folder-home"));
        }
        let name = path.strip_prefix(&self.home).ok()?.to_str()?;
        self.inside(name)
    }

    /// The kind of the folder called `name` right inside the home, when it is one of its folders.
    pub(crate) fn inside(&self, name: &str) -> Option<FileKind> {
        self.names.iter().find(|(own, _)| own == name).map(|&(_, icon)| FileKind::of(icon))
    }
}

/// Whether `name` is the name of one folder, not a path through several.
fn is_one_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/')
}
