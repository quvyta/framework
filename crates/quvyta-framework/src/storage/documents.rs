//! Where the user keeps their own documents, the folder a file manager calls Documents.
//!
//! The name of that folder is the user's language: `~/Belgeler` on a Turkish desktop,
//! `~/Dokumente` on a German one. Linux desktops write the real name into `user-dirs.dirs`, so
//! it is read from there rather than guessed.

use std::fs;
use std::path::{Path, PathBuf};

use super::dirs::{absolute, env_lookup};

/// The file Linux desktops name the user's folders in, under the config root.
const USER_DIRS: &str = "user-dirs.dirs";

/// The line of [`USER_DIRS`] that names the Documents folder.
const DOCUMENTS_KEY: &str = "XDG_DOCUMENTS_DIR";

/// The user's Documents folder, where an application puts files the user made and wants to find
/// again in a file manager: projects, exports, notes.
///
/// - Linux and other Unix systems: the `XDG_DOCUMENTS_DIR` line of `user-dirs.dirs` in the
///   config root (`$XDG_CONFIG_HOME` when it is an absolute path, else `$HOME/.config`), which
///   is where a desktop records the folder's name in the user's language, such as
///   `$HOME/Belgeler`. A missing or unreadable file, a line that does not follow the format, and
///   a line that names the home folder itself (the way that file turns a folder off) all give
///   `$HOME/Documents`.
/// - macOS: `$HOME/Documents`.
/// - Windows: the Documents Known Folder, which the user can move to another drive; when the
///   system does not answer, `%USERPROFILE%\Documents`.
///
/// `None` when there is no home folder to put it in. A `HOME` that is not an absolute path
/// counts as missing, as it does for [`config_dir`](super::config_dir). The folder is not created
/// and does not have to exist.
#[must_use]
pub fn documents_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(known) = dirs::document_dir() {
        return Some(known);
    }
    documents_root(env_lookup, |path| fs::read_to_string(path).ok())
}

/// The Documents folder from variables read through `lookup` and the text of `user-dirs.dirs`
/// read through `read`, so no test depends on the developer's own desktop.
fn documents_root(lookup: impl Fn(&str) -> Option<PathBuf>, read: impl Fn(&Path) -> Option<String>) -> Option<PathBuf> {
    let non_empty = |name: &str| lookup(name).filter(|path| !path.as_os_str().is_empty());
    if cfg!(windows) {
        return absolute(non_empty("USERPROFILE")).map(|home| home.join("Documents"));
    }
    let home = absolute(non_empty("HOME"));
    if cfg!(target_os = "macos") {
        return home.map(|home| home.join("Documents"));
    }
    let config = absolute(non_empty("XDG_CONFIG_HOME")).or_else(|| home.as_ref().map(|home| home.join(".config")));
    let named =
        config.and_then(|config| read(&config.join(USER_DIRS))).and_then(|text| documents_line(&text, home.as_deref()));
    named.or_else(|| home.map(|home| home.join("Documents")))
}

/// The Documents folder `text`, the contents of a `user-dirs.dirs`, names; `None` when it names
/// none, names it in a way the format does not allow, or turns it off by naming the home folder.
///
/// The format is a shell file with one `XDG_<NAME>_DIR="<value>"` line per folder, where the
/// value is `$HOME` followed by a path, or an absolute path. `#` starts a comment line, a
/// backslash takes the next character as it is, and nothing else is expanded. As in a shell, the
/// last valid line wins.
fn documents_line(text: &str, home: Option<&Path>) -> Option<PathBuf> {
    let named = text.lines().rev().find_map(|line| documents_value(line, home))?;
    // A folder set to the home folder itself is how the format says the folder is turned off.
    (Some(named.as_path()) != home).then_some(named)
}

/// The path one line of `user-dirs.dirs` gives the Documents folder, if it is that line and it
/// is valid.
fn documents_value(line: &str, home: Option<&Path>) -> Option<PathBuf> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    if key.trim() != DOCUMENTS_KEY {
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
        assert_eq!(documents_root(env(HOME), read), Some(PathBuf::from("/home/ada/Belgeler")));
    }

    #[test]
    fn the_file_is_read_from_the_xdg_config_root() {
        if !linux() {
            return;
        }
        let read = file("/cfg/user-dirs.dirs", "XDG_DOCUMENTS_DIR=\"/data/docs\"\n");
        let vars = env(&[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "/cfg")]);
        assert_eq!(documents_root(vars, read), Some(PathBuf::from("/data/docs")), "an absolute value is used as it is");
        // A relative XDG_CONFIG_HOME is ignored, as it is for the settings folder.
        let read = file("/home/ada/.config/user-dirs.dirs", "XDG_DOCUMENTS_DIR=\"$HOME/Docs\"\n");
        let vars = env(&[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "cfg")]);
        assert_eq!(documents_root(vars, read), Some(PathBuf::from("/home/ada/Docs")));
    }

    #[test]
    fn a_missing_or_broken_file_falls_back_to_documents() {
        if !linux() {
            return;
        }
        let fallback = Some(PathBuf::from("/home/ada/Documents"));
        assert_eq!(documents_root(env(HOME), none), fallback, "no file");
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
            assert_eq!(documents_line(text, Some(Path::new("/home/ada"))), None, "{text:?}");
        }
    }

    #[test]
    fn naming_the_home_folder_turns_the_folder_off() {
        if !linux() {
            return;
        }
        let home = Some(Path::new("/home/ada"));
        assert_eq!(documents_line("XDG_DOCUMENTS_DIR=\"$HOME\"\n", home), None);
        assert_eq!(documents_line("XDG_DOCUMENTS_DIR=\"$HOME/\"\n", home), None);
        assert_eq!(documents_line("XDG_DOCUMENTS_DIR=\"/home/ada\"\n", home), None);
        let read = file("/home/ada/.config/user-dirs.dirs", "XDG_DOCUMENTS_DIR=\"$HOME/\"\n");
        assert_eq!(documents_root(env(HOME), read), Some(PathBuf::from("/home/ada/Documents")));
    }

    #[test]
    fn the_format_is_read_as_a_shell_reads_it() {
        if !linux() {
            return;
        }
        let home = Some(Path::new("/home/ada"));
        let text = "  XDG_DOCUMENTS_DIR = \"$HOME/My\\ Files\\\\old\"  \nXDG_DOCUMENTS_DIR=\"$HOME/New\"\nXDG_DOCUMENTS_DIR=broken\n";
        assert_eq!(documents_line(text, home), Some(PathBuf::from("/home/ada/New")), "the last valid line wins");
        let escaped = "XDG_DOCUMENTS_DIR=\"$HOME/My\\ Files\\\\old \\\"x\\\"\"\n";
        assert_eq!(documents_line(escaped, home), Some(PathBuf::from("/home/ada/My Files\\old \"x\"")));
        // Without a home folder only an absolute value can name the folder.
        assert_eq!(documents_line("XDG_DOCUMENTS_DIR=\"$HOME/Docs\"\n", None), None);
        assert_eq!(documents_line("XDG_DOCUMENTS_DIR=\"/srv/docs\"\n", None), Some(PathBuf::from("/srv/docs")));
    }

    #[test]
    fn macos_and_windows_use_the_documents_folder_in_the_home() {
        if cfg!(target_os = "macos") {
            assert_eq!(
                documents_root(env(&[("HOME", "/Users/ada")]), none),
                Some(PathBuf::from("/Users/ada/Documents"))
            );
        }
        if cfg!(windows) {
            let vars = env(&[("USERPROFILE", r"C:\Users\ada")]);
            assert_eq!(documents_root(vars, none), Some(PathBuf::from(r"C:\Users\ada\Documents")));
        }
    }

    #[test]
    fn no_home_means_no_folder() {
        assert_eq!(documents_root(env(&[]), none), None);
        if !cfg!(windows) {
            assert_eq!(documents_root(env(&[("HOME", "ada")]), none), None, "a relative home counts as missing");
        }
    }
}
