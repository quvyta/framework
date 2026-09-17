//! Built-in theme, icon, locale and keymap files, compiled into the binary, and reading of
//! user-provided directories.

use std::fs;
use std::io;
use std::path::Path;

use crate::diagnostics::{Diagnostic, Location};

/// Built-in themes as `(id, toml)`. `monochrome` is the default and the root every other
/// built-in theme extends.
pub(crate) const THEMES: [(&str, &str); 4] = [
    ("monochrome", include_str!("../assets/themes/monochrome.toml")),
    ("iris", include_str!("../assets/themes/iris.toml")),
    ("nordic", include_str!("../assets/themes/nordic.toml")),
    ("amber", include_str!("../assets/themes/amber.toml")),
];

/// Built-in icon sets as `(id, toml)`.
pub(crate) const ICON_SETS: [(&str, &str); 1] = [("default", include_str!("../assets/icons/default.toml"))];

/// Built-in locales as `(code, toml)`. `en` is the final fallback.
pub(crate) const LOCALES: [(&str, &str); 2] =
    [("en", include_str!("../assets/locales/en.toml")), ("tr", include_str!("../assets/locales/tr.toml"))];

/// The built-in keymap.
pub(crate) const KEYMAP: &str = include_str!("../assets/keymaps/default.toml");

/// The `*.toml` files directly inside a directory, and the ones that could not be read.
pub(crate) struct TomlDir {
    /// `(stem, file name, text)` of every readable file, sorted by file name so loading order is
    /// stable.
    pub(crate) files: Vec<(String, String, String)>,
    /// One error per file that could not be read (unreadable or not UTF-8); loading skips it.
    pub(crate) skipped: Vec<Diagnostic>,
}

/// Reads every `*.toml` file directly inside `dir`. A file that cannot be read is skipped with a
/// diagnostic, so one broken file never hides the others.
pub(crate) fn read_toml_dir(dir: &Path) -> io::Result<TomlDir> {
    let mut found = TomlDir { files: Vec::new(), skipped: Vec::new() };
    for entry in fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let (Some(stem), Some(name)) =
            (path.file_stem().and_then(|s| s.to_str()), path.file_name().and_then(|s| s.to_str()))
        else {
            continue;
        };
        match fs::read_to_string(&path) {
            Ok(text) => found.files.push((stem.to_owned(), name.to_owned(), text)),
            Err(error) => found.skipped.push(Diagnostic::error(
                Some(Location::from_offset(name, "", 0)),
                format!("cannot read the file, it is skipped: {error}"),
            )),
        }
    }
    found.files.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_toml_files_in_name_order() {
        let dir = std::env::temp_dir().join(format!("quvyta-assets-{}", std::process::id()));
        fs::create_dir_all(dir.join("nested.toml")).expect("create dirs");
        fs::write(dir.join("b.toml"), "b").expect("write");
        fs::write(dir.join("a.toml"), "a").expect("write");
        fs::write(dir.join("notes.txt"), "x").expect("write");
        let files = read_toml_dir(&dir).expect("readable").files;
        fs::remove_dir_all(&dir).expect("clean");
        let names: Vec<&str> = files.iter().map(|f| f.1.as_str()).collect();
        assert_eq!(names, vec!["a.toml", "b.toml"]);
        assert_eq!(files[0].0, "a");
        assert_eq!(files[0].2, "a");
    }

    #[test]
    fn a_file_that_is_not_utf8_is_skipped_with_a_diagnostic_and_the_rest_load() {
        let dir = std::env::temp_dir().join(format!("quvyta-assets-bad-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join("broken.toml"), [0xff, 0xfe, 0x00]).expect("write");
        fs::write(dir.join("good.toml"), "ok").expect("write");
        let found = read_toml_dir(&dir).expect("readable");
        fs::remove_dir_all(&dir).expect("clean");
        let names: Vec<&str> = found.files.iter().map(|f| f.1.as_str()).collect();
        assert_eq!(names, vec!["good.toml"]);
        assert_eq!(found.skipped.len(), 1);
        assert!(found.skipped[0].to_string().contains("broken.toml"), "{}", found.skipped[0]);
    }
}
