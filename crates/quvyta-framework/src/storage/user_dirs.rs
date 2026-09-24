//! The folders a person keeps their own things in: Desktop, Documents, Downloads, Music, Pictures,
//! Videos and the rest, the folders a file manager lists under the home.
//!
//! Their names are the person's language: `~/Masaüstü` and `~/Belgeler` on a Turkish desktop,
//! `~/Schreibtisch` and `~/Dokumente` on a German one. Linux desktops write the real names into
//! `user-dirs.dirs`, so they are read from there rather than guessed.

use std::fs;
use std::path::{Path, PathBuf};

use super::dirs::{absolute, env_lookup};

/// The file Linux desktops name the user's folders in, under the config root.
const USER_DIRS: &str = "user-dirs.dirs";

/// One of the folders the XDG user directories name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UserDir {
    /// The desktop, whose files a desktop environment shows on its background.
    Desktop,
    /// Documents: files the person made and wants to find again.
    Documents,
    /// Downloads, where a browser saves what it fetches.
    Downloads,
    /// Music.
    Music,
    /// Pictures.
    Pictures,
    /// Files shared with others on the same machine or network.
    Public,
    /// Templates a file manager offers for a new file.
    Templates,
    /// Videos.
    Videos,
}

impl UserDir {
    /// Every user folder, in the order a file manager usually lists them.
    pub const ALL: [Self; 8] = [
        Self::Desktop,
        Self::Documents,
        Self::Downloads,
        Self::Music,
        Self::Pictures,
        Self::Videos,
        Self::Public,
        Self::Templates,
    ];

    /// The word in its `user-dirs.dirs` line: `XDG_<word>_DIR`.
    #[must_use]
    pub fn xdg_name(self) -> &'static str {
        match self {
            Self::Desktop => "DESKTOP",
            Self::Documents => "DOCUMENTS",
            Self::Downloads => "DOWNLOAD",
            Self::Music => "MUSIC",
            Self::Pictures => "PICTURES",
            Self::Public => "PUBLICSHARE",
            Self::Templates => "TEMPLATES",
            Self::Videos => "VIDEOS",
        }
    }

    /// The folder's name in a home without `user-dirs.dirs`, and on macOS and Windows.
    #[must_use]
    pub fn english_name(self) -> &'static str {
        match self {
            Self::Desktop => "Desktop",
            Self::Documents => "Documents",
            Self::Downloads => "Downloads",
            Self::Music => "Music",
            Self::Pictures => "Pictures",
            Self::Public => "Public",
            Self::Templates => "Templates",
            Self::Videos => "Videos",
        }
    }
}

/// The person's folder `which`, such as the desktop: on a Turkish desktop `user_dir(UserDir::Desktop)`
/// is `$HOME/Masaüstü`.
///
/// - Linux and other Unix systems: the `XDG_<NAME>_DIR` line of `user-dirs.dirs` in the config
///   root (`$XDG_CONFIG_HOME` when it is an absolute path, else `$HOME/.config`), which is where a
///   desktop records the folder's name in the user's language. A missing or unreadable file, a line
///   that does not follow the format, and a line that names the home folder itself (the way that
///   file turns a folder off) all give the English name in the home, such as `$HOME/Desktop`.
/// - macOS: the English name in the home.
/// - Windows: the Known Folder, which the user can move to another drive; when the system does not
///   answer, the English name in `%USERPROFILE%`.
///
/// `None` when there is no home folder to put it in. A `HOME` that is not an absolute path counts
/// as missing, as it does for [`config_dir`](super::config_dir). The folder is not created and does
/// not have to exist. A test uses [`user_dir_in`], which reads no environment.
#[must_use]
pub fn user_dir(which: UserDir) -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(known) = known_folder(which) {
        return Some(known);
    }
    user_dir_root(which, env_lookup, |path| fs::read_to_string(path).ok())
}

/// The folder `which` of the home `home`, as the `user-dirs.dirs` in the config folder `config`
/// names it, or its English name in `home` when that file does not name it.
///
/// For a test, and for an application that already knows the home and config folders it means.
/// Only the file is read; the environment is not.
#[must_use]
pub fn user_dir_in(which: UserDir, home: &Path, config: &Path) -> PathBuf {
    fs::read_to_string(config.join(USER_DIRS))
        .ok()
        .and_then(|text| user_dir_line(&text, which, Some(home)))
        .unwrap_or_else(|| home.join(which.english_name()))
}

/// The user's Documents folder, where an application puts files the user made and wants to find
/// again in a file manager: projects, exports, notes. The same as
/// [`user_dir(UserDir::Documents)`](user_dir).
#[must_use]
pub fn documents_dir() -> Option<PathBuf> {
    user_dir(UserDir::Documents)
}

/// The system's own answer for `which` on Windows.
#[cfg(windows)]
fn known_folder(which: UserDir) -> Option<PathBuf> {
    match which {
        UserDir::Desktop => dirs::desktop_dir(),
        UserDir::Documents => dirs::document_dir(),
        UserDir::Downloads => dirs::download_dir(),
        UserDir::Music => dirs::audio_dir(),
        UserDir::Pictures => dirs::picture_dir(),
        UserDir::Public => dirs::public_dir(),
        UserDir::Templates => dirs::template_dir(),
        UserDir::Videos => dirs::video_dir(),
    }
}

/// The folder `which` from variables read through `lookup` and the text of `user-dirs.dirs` read
/// through `read`, so no test depends on the developer's own desktop.
fn user_dir_root(
    which: UserDir,
    lookup: impl Fn(&str) -> Option<PathBuf>,
    read: impl Fn(&Path) -> Option<String>,
) -> Option<PathBuf> {
    let non_empty = |name: &str| lookup(name).filter(|path| !path.as_os_str().is_empty());
    if cfg!(windows) {
        return absolute(non_empty("USERPROFILE")).map(|home| home.join(which.english_name()));
    }
    let home = absolute(non_empty("HOME"));
    if cfg!(target_os = "macos") {
        return home.map(|home| home.join(which.english_name()));
    }
    let config = absolute(non_empty("XDG_CONFIG_HOME")).or_else(|| home.as_ref().map(|home| home.join(".config")));
    let named = config
        .and_then(|config| read(&config.join(USER_DIRS)))
        .and_then(|text| user_dir_line(&text, which, home.as_deref()));
    named.or_else(|| home.map(|home| home.join(which.english_name())))
}

/// The folder `which` that `text`, the contents of a `user-dirs.dirs`, names; `None` when it names
/// none, names it in a way the format does not allow, or turns it off by naming the home folder.
///
/// The format is a shell file with one `XDG_<NAME>_DIR="<value>"` line per folder, where the
/// value is `$HOME` followed by a path, or an absolute path. `#` starts a comment line, a
/// backslash takes the next character as it is, and nothing else is expanded. As in a shell, the
/// last valid line wins.
pub(crate) fn user_dir_line(text: &str, which: UserDir, home: Option<&Path>) -> Option<PathBuf> {
    let key = format!("XDG_{}_DIR", which.xdg_name());
    let named = text.lines().rev().find_map(|line| user_dir_value(line, &key, home))?;
    // A folder set to the home folder itself is how the format says the folder is turned off.
    (Some(named.as_path()) != home).then_some(named)
}

/// The path one line of `user-dirs.dirs` gives the folder of `key`, if it is that line and it is
/// valid.
fn user_dir_value(line: &str, key: &str, home: Option<&Path>) -> Option<PathBuf> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (name, value) = line.split_once('=')?;
    if name.trim() != key {
        return None;
    }
    let quoted = value.trim().strip_prefix('"')?.strip_suffix('"')?;
    let value = unescape(quoted)?;
    match value.strip_prefix("$HOME") {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => Some(home?.join(rest.trim_start_matches('/'))),
        // `$HOMEWORK` is not the home folder, and no other variable is expanded.
        Some(_) => None,
        None => Some(PathBuf::from(value)).filter(|path| path.is_absolute()),
    }
}

/// `text` with every backslash escape replaced by the character it escapes. A bare `"` inside the
/// quotes would end the shell word, so such a line is invalid; so is a backslash at the very end.
fn unescape(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            '"' => return None,
            c => out.push(c),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lookup over a fixed list of variables, so no test reads the developer's own environment.
    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<PathBuf> {
        move |name: &str| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| PathBuf::from(value))
    }

    /// A reader that finds `text` at `at` and nothing anywhere else.
    fn file(at: &'static str, text: &'static str) -> impl Fn(&Path) -> Option<String> {
        move |path: &Path| (path == Path::new(at)).then(|| text.to_owned())
    }

    fn none(_: &Path) -> Option<String> {
        None
    }

    const HOME: &[(&str, &str)] = &[("HOME", "/home/ada")];

    fn linux() -> bool {
        cfg!(all(unix, not(target_os = "macos")))
    }

    #[test]
    fn the_desktop_names_the_folder_in_the_users_language() {
        if !linux() {
            return;
        }
        let text = "# This file is written by xdg-user-dirs-update\n\
                    XDG_DESKTOP_DIR=\"$HOME/Masaüstü\"\n\
                    XDG_DOCUMENTS_DIR=\"$HOME/Belgeler\"\n";
        let read = file("/home/ada/.config/user-dirs.dirs", text);
        assert_eq!(user_dir_root(UserDir::Documents, env(HOME), read), Some(PathBuf::from("/home/ada/Belgeler")));
    }

    #[test]
    fn the_file_is_read_from_the_xdg_config_root() {
        if !linux() {
            return;
        }
        let read = file("/cfg/user-dirs.dirs", "XDG_DOCUMENTS_DIR=\"/data/docs\"\n");
        let vars = env(&[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "/cfg")]);
        assert_eq!(
            user_dir_root(UserDir::Documents, vars, read),
            Some(PathBuf::from("/data/docs")),
            "an absolute value is used as it is"
        );
        // A relative XDG_CONFIG_HOME is ignored, as it is for the settings folder.
        let read = file("/home/ada/.config/user-dirs.dirs", "XDG_DOCUMENTS_DIR=\"$HOME/Docs\"\n");
        let vars = env(&[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "cfg")]);
        assert_eq!(user_dir_root(UserDir::Documents, vars, read), Some(PathBuf::from("/home/ada/Docs")));
    }

    #[test]
    fn a_missing_or_broken_file_falls_back_to_documents() {
        if !linux() {
            return;
        }
        let fallback = Some(PathBuf::from("/home/ada/Documents"));
        assert_eq!(user_dir_root(UserDir::Documents, env(HOME), none), fallback, "no file");
        let broken = [
            "",
            "XDG_DOCUMENTS_DIR=$HOME/Docs\n",
            "XDG_DOCUMENTS_DIR=\"$HOME/Docs\n",
            "XDG_DOCUMENTS_DIR=\"Docs\"\n",
            "XDG_DOCUMENTS_DIR=\"$HOMEWORK/Docs\"\n",
            "XDG_DOCUMENTS_DIR=\"${XDG_DATA_HOME}/Docs\"\n",
            "XDG_DOCUMENTS_DIR=\"$HOME/a\"b\"\n",
            "XDG_DOCUMENTS_DIR=\"$HOME/Docs\\\"\n",
            "# XDG_DOCUMENTS_DIR=\"$HOME/Docs\"\n",
            "XDG_DOWNLOAD_DIR=\"$HOME/Docs\"\n",
            "\u{0}\u{ffff}=== not a shell file",
        ];
        for text in broken {
            assert_eq!(user_dir_line(text, UserDir::Documents, Some(Path::new("/home/ada"))), None, "{text:?}");
        }
    }

    #[test]
    fn naming_the_home_folder_turns_the_folder_off() {
        if !linux() {
            return;
        }
        let home = Some(Path::new("/home/ada"));
        assert_eq!(user_dir_line("XDG_DOCUMENTS_DIR=\"$HOME\"\n", UserDir::Documents, home), None);
        assert_eq!(user_dir_line("XDG_DOCUMENTS_DIR=\"$HOME/\"\n", UserDir::Documents, home), None);
        assert_eq!(user_dir_line("XDG_DOCUMENTS_DIR=\"/home/ada\"\n", UserDir::Documents, home), None);
        let read = file("/home/ada/.config/user-dirs.dirs", "XDG_DOCUMENTS_DIR=\"$HOME/\"\n");
        assert_eq!(user_dir_root(UserDir::Documents, env(HOME), read), Some(PathBuf::from("/home/ada/Documents")));
    }

    #[test]
    fn the_format_is_read_as_a_shell_reads_it() {
        if !linux() {
            return;
        }
        let home = Some(Path::new("/home/ada"));
        let text = "  XDG_DOCUMENTS_DIR = \"$HOME/My\\ Files\\\\old\"  \nXDG_DOCUMENTS_DIR=\"$HOME/New\"\nXDG_DOCUMENTS_DIR=broken\n";
        assert_eq!(
            user_dir_line(text, UserDir::Documents, home),
            Some(PathBuf::from("/home/ada/New")),
            "the last valid line wins"
        );
        let escaped = "XDG_DOCUMENTS_DIR=\"$HOME/My\\ Files\\\\old \\\"x\\\"\"\n";
        assert_eq!(
            user_dir_line(escaped, UserDir::Documents, home),
            Some(PathBuf::from("/home/ada/My Files\\old \"x\""))
        );
        // Without a home folder only an absolute value can name the folder.
        assert_eq!(user_dir_line("XDG_DOCUMENTS_DIR=\"$HOME/Docs\"\n", UserDir::Documents, None), None);
        assert_eq!(
            user_dir_line("XDG_DOCUMENTS_DIR=\"/srv/docs\"\n", UserDir::Documents, None),
            Some(PathBuf::from("/srv/docs"))
        );
    }

    #[test]
    fn macos_and_windows_use_the_documents_folder_in_the_home() {
        if cfg!(target_os = "macos") {
            assert_eq!(
                user_dir_root(UserDir::Documents, env(&[("HOME", "/Users/ada")]), none),
                Some(PathBuf::from("/Users/ada/Documents"))
            );
        }
        if cfg!(windows) {
            let vars = env(&[("USERPROFILE", r"C:\Users\ada")]);
            assert_eq!(user_dir_root(UserDir::Documents, vars, none), Some(PathBuf::from(r"C:\Users\ada\Documents")));
        }
    }

    #[test]
    fn no_home_means_no_folder() {
        assert_eq!(user_dir_root(UserDir::Documents, env(&[]), none), None);
        if !cfg!(windows) {
            assert_eq!(
                user_dir_root(UserDir::Documents, env(&[("HOME", "ada")]), none),
                None,
                "a relative home counts as missing"
            );
        }
    }

    #[test]
    fn the_desktop_and_every_other_folder_are_found_by_their_own_line() {
        if !linux() {
            return;
        }
        let text = "XDG_DESKTOP_DIR=\"$HOME/Masaüstü\"\n\
                    XDG_DOWNLOAD_DIR=\"$HOME/İndirilenler\"\n\
                    XDG_DOCUMENTS_DIR=\"$HOME/Belgeler\"\n";
        let at = |which| user_dir_root(which, env(HOME), file("/home/ada/.config/user-dirs.dirs", text));
        assert_eq!(at(UserDir::Desktop), Some(PathBuf::from("/home/ada/Masaüstü")), "the desktop by its Turkish name");
        assert_eq!(at(UserDir::Downloads), Some(PathBuf::from("/home/ada/İndirilenler")));
        assert_eq!(at(UserDir::Documents), Some(PathBuf::from("/home/ada/Belgeler")));
        assert_eq!(at(UserDir::Music), Some(PathBuf::from("/home/ada/Music")), "a folder the file does not name");
    }

    #[test]
    fn a_folder_is_read_from_the_config_folder_a_test_names() {
        let root = std::env::temp_dir().join(format!("qframe-user-dirs-{}", std::process::id()));
        let (home, config) = (root.join("home"), root.join("config"));
        std::fs::create_dir_all(&config).expect("a config folder");
        assert_eq!(user_dir_in(UserDir::Desktop, &home, &config), home.join("Desktop"), "no file: the English name");
        std::fs::write(config.join("user-dirs.dirs"), "XDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\n").expect("the file");
        assert_eq!(user_dir_in(UserDir::Desktop, &home, &config), home.join("Schreibtisch"));
        assert_eq!(user_dir_in(UserDir::Videos, &home, &config), home.join("Videos"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn every_folder_has_its_own_line_and_name() {
        let words: std::collections::BTreeSet<_> = UserDir::ALL.iter().map(|which| which.xdg_name()).collect();
        let names: std::collections::BTreeSet<_> = UserDir::ALL.iter().map(|which| which.english_name()).collect();
        assert_eq!((words.len(), names.len()), (UserDir::ALL.len(), UserDir::ALL.len()));
    }
}
