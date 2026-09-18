//! Where an application keeps its settings and its own data.
//!
//! Settings and data are two different folders. A theme choice is a setting; a recorded session
//! is data. On Linux and other Unix systems they are two separate XDG folders, on Windows the
//! roaming and the local folder, and on macOS the same folder, because macOS has no split
//! between the two.

use std::path::PathBuf;

/// Where application `app` keeps its settings.
///
/// - Linux and other Unix systems: `$XDG_CONFIG_HOME/<app>` when `XDG_CONFIG_HOME` is an
///   absolute path, else `$HOME/.config/<app>`.
/// - macOS: `$HOME/Library/Application Support/<app>`.
/// - Windows: `%APPDATA%\<app>`, the roaming folder, so the settings follow the user between
///   machines.
///
/// `None` when the platform's variables say nothing: there is no home directory to write in,
/// and the application has to keep its settings in memory. A `HOME` that is not an absolute
/// path counts as missing, as a relative XDG variable does: it would put the files under
/// whatever folder the application happened to be started in. The folder is not created and does
/// not have to exist.
#[must_use]
pub fn config_dir(app: &str) -> Option<PathBuf> {
    config_root(env_lookup).map(|root| root.join(app))
}

/// Where application `app` keeps its own data: records, caches it wants to survive, files it
/// wrote itself. Records are not settings, so they do not live next to them.
///
/// - Linux and other Unix systems: `$XDG_DATA_HOME/<app>` when `XDG_DATA_HOME` is an absolute
///   path, else `$HOME/.local/share/<app>`.
/// - macOS: `$HOME/Library/Application Support/<app>`, the same folder as the settings. macOS
///   has no separate data folder for a command line application, and the framework does not
///   invent one.
/// - Windows: `%LOCALAPPDATA%\<app>`, the local folder, so records are not copied between
///   machines by a roaming profile.
///
/// `None` when the platform's variables say nothing, and when `HOME` is not an absolute path.
/// The folder is not created and does not have to exist.
#[must_use]
pub fn data_dir(app: &str) -> Option<PathBuf> {
    data_root(env_lookup).map(|root| root.join(app))
}

/// Reads one environment variable as a path.
fn env_lookup(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

/// The folder settings of every application live under, from variables read through `lookup`.
fn config_root(lookup: impl Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    let non_empty = |name: &str| lookup(name).filter(|path| !path.as_os_str().is_empty());
    if cfg!(windows) {
        return non_empty("APPDATA");
    }
    if cfg!(target_os = "macos") {
        return absolute(non_empty("HOME")).map(|home| home.join("Library").join("Application Support"));
    }
    absolute(non_empty("XDG_CONFIG_HOME")).or_else(|| absolute(non_empty("HOME")).map(|home| home.join(".config")))
}

/// The folder data of every application lives under, from variables read through `lookup`.
fn data_root(lookup: impl Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    let non_empty = |name: &str| lookup(name).filter(|path| !path.as_os_str().is_empty());
    if cfg!(windows) {
        return non_empty("LOCALAPPDATA");
    }
    if cfg!(target_os = "macos") {
        return absolute(non_empty("HOME")).map(|home| home.join("Library").join("Application Support"));
    }
    absolute(non_empty("XDG_DATA_HOME"))
        .or_else(|| absolute(non_empty("HOME")).map(|home| home.join(".local").join("share")))
}

/// A variable counts only when it holds an absolute path. The XDG specification says a relative
/// one is invalid and must be ignored; a relative `HOME` is no better, since it would put the
/// files under the working directory.
fn absolute(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|path| path.is_absolute())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lookup over a fixed list of variables, so no test reads the developer's own environment.
    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<PathBuf> {
        move |name: &str| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| PathBuf::from(value))
    }

    #[test]
    fn settings_and_data_are_two_folders_on_unix() {
        if !cfg!(all(unix, not(target_os = "macos"))) {
            return;
        }
        let home = env(&[("HOME", "/home/ada")]);
        assert_eq!(config_root(&home), Some(PathBuf::from("/home/ada/.config")));
        assert_eq!(data_root(&home), Some(PathBuf::from("/home/ada/.local/share")));

        let xdg = env(&[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "/cfg"), ("XDG_DATA_HOME", "/dat")]);
        assert_eq!(config_root(&xdg), Some(PathBuf::from("/cfg")));
        assert_eq!(data_root(&xdg), Some(PathBuf::from("/dat")));

        // A relative XDG path is invalid and ignored, in both folders.
        let relative = env(&[("HOME", "/home/ada"), ("XDG_CONFIG_HOME", "cfg"), ("XDG_DATA_HOME", "dat")]);
        assert_eq!(config_root(&relative), Some(PathBuf::from("/home/ada/.config")));
        assert_eq!(data_root(&relative), Some(PathBuf::from("/home/ada/.local/share")));

        // An empty variable is as good as unset.
        let empty = env(&[("HOME", "/home/ada"), ("XDG_DATA_HOME", "")]);
        assert_eq!(data_root(&empty), Some(PathBuf::from("/home/ada/.local/share")));
    }

    #[test]
    fn macos_keeps_both_in_application_support() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let home = env(&[("HOME", "/Users/ada")]);
        let support = PathBuf::from("/Users/ada/Library/Application Support");
        assert_eq!(config_root(&home), Some(support.clone()));
        assert_eq!(data_root(&home), Some(support));
    }

    #[test]
    fn windows_roams_settings_and_keeps_data_local() {
        if !cfg!(windows) {
            return;
        }
        let both =
            env(&[("APPDATA", r"C:\Users\ada\AppData\Roaming"), ("LOCALAPPDATA", r"C:\Users\ada\AppData\Local")]);
        assert_eq!(config_root(&both), Some(PathBuf::from(r"C:\Users\ada\AppData\Roaming")));
        assert_eq!(data_root(&both), Some(PathBuf::from(r"C:\Users\ada\AppData\Local")));
    }

    #[test]
    fn a_relative_home_counts_as_missing() {
        if cfg!(windows) {
            return;
        }
        let relative = env(&[("HOME", "ada")]);
        assert_eq!(config_root(&relative), None, "not a folder under the working directory");
        assert_eq!(data_root(&relative), None);
        // An absolute XDG folder still stands on its own.
        if cfg!(not(target_os = "macos")) {
            let xdg = env(&[("HOME", "ada"), ("XDG_CONFIG_HOME", "/cfg"), ("XDG_DATA_HOME", "/dat")]);
            assert_eq!(config_root(&xdg), Some(PathBuf::from("/cfg")));
            assert_eq!(data_root(&xdg), Some(PathBuf::from("/dat")));
        }
    }

    #[test]
    fn nothing_in_the_environment_means_no_folder() {
        assert_eq!(config_root(env(&[])), None);
        assert_eq!(data_root(env(&[])), None);
    }

    #[test]
    fn the_application_name_is_the_last_segment() {
        // Whatever the platform, both public functions end in the application's own folder.
        for dir in [config_dir("qfocus"), data_dir("qfocus")].into_iter().flatten() {
            assert_eq!(dir.file_name().and_then(|name| name.to_str()), Some("qfocus"), "{}", dir.display());
            assert!(dir.is_absolute(), "{}", dir.display());
        }
    }
}
