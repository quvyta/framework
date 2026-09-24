use std::collections::BTreeMap;
use std::path::Path;

use super::table::{DOUBLES, EXTENSIONS, FOLDERS, KINDS, NAMES};
use super::*;
use crate::icons::{GlyphMode, IconSetRegistry};

/// The icon `name` is drawn with, as a file that may not be run.
fn icon(name: &str) -> &'static str {
    file_kind(name, false, false).icon()
}

#[test]
fn a_name_from_every_family_takes_its_kind() {
    let cases = [
        ("main.rs", "file-rust", KindFamily::Code),
        ("notes.txt", "file-text", KindFamily::Text),
        ("paper.pdf", "file-pdf", KindFamily::Document),
        ("budget.ods", "file-sheet", KindFamily::Sheet),
        ("settings.json", "file-json", KindFamily::Data),
        ("photo.jpg", "file-image", KindFamily::Image),
        ("song.flac", "file-audio", KindFamily::Audio),
        ("film.mkv", "file-video", KindFamily::Video),
        ("photos.zip", "file-archive", KindFamily::Archive),
        ("debian.iso", "file-disk", KindFamily::Package),
        ("libssl.so", "file-library", KindFamily::Executable),
        ("server.pem", "file-key", KindFamily::Key),
        ("Inter.ttf", "file-font", KindFamily::Font),
        ("mystery.qqq", "file", KindFamily::File),
    ];
    for (name, key, family) in cases {
        let kind = file_kind(name, false, false);
        assert_eq!((kind.icon(), kind.family()), (key, family), "{name}");
    }
    let folder = file_kind(".git", true, false);
    assert_eq!((folder.icon(), folder.family()), ("folder-git", KindFamily::Folder));
}

#[test]
fn letter_case_never_matters() {
    for name in ["MAIN.RS", "Main.Rs", "main.rs"] {
        assert_eq!(icon(name), "file-rust", "{name}");
    }
    assert_eq!(icon("dockerfile"), "file-docker");
    assert_eq!(icon("DOCKERFILE"), "file-docker");
    assert_eq!(file_kind("Node_Modules", true, false).icon(), "folder-packages");
}

#[test]
fn an_ending_of_several_parts_wins_over_the_last_one() {
    assert_eq!(icon("x.tar.gz"), "file-archive");
    assert_eq!(icon("firefox-130.0-1-x86_64.pkg.tar.zst"), "file-arch", "an Arch package before a .tar.zst archive");
    assert_eq!(icon("notes.txt.gz"), "file-archive", "a lone compressed file is an archive too");
    assert_eq!(icon(".tar.gz"), "file-archive", "a hidden name is read by its last ending");
    assert_eq!(icon("x.tar.zst"), "file-archive");
    assert_eq!(icon("x.zst"), "file-archive");
    assert_eq!(icon("index.d.ts"), "file-typescript");
    assert_eq!(icon("backup.2026.tar.xz"), "file-archive", "more dots before the ending change nothing");
}

#[test]
fn the_executable_bit_changes_only_a_file_nothing_else_recognised() {
    assert_eq!(file_kind("configure", false, true).icon(), "file-executable");
    assert_eq!(file_kind("configure", false, false).icon(), "file");
    assert_eq!(file_kind("build.sh", false, true).icon(), "file-shell", "a script stays a script");
    assert_eq!(file_kind("PKGBUILD", false, true).icon(), "file-arch", "a whole name wins too");
    assert_eq!(file_kind("bin", true, true).icon(), "folder", "a folder is never a program");
}

#[test]
fn whole_names_say_what_the_file_is() {
    assert_eq!(icon("Cargo.toml"), "file-cargo");
    assert_eq!(icon("Cargo.lock"), "file-lock");
    assert_eq!(icon("Dockerfile"), "file-docker");
    assert_eq!(icon("Makefile"), "file-makefile");
    assert_eq!(icon("CMakeLists.txt"), "file-cmake", "not the text file its extension says");
    assert_eq!(icon("PKGBUILD"), "file-arch");
    assert_eq!(icon(".gitignore"), "file-git");
    assert_eq!(icon(".bashrc"), "file-shell");
    assert_eq!(icon("package.json"), "file-node");
    assert_eq!(icon(".npmrc"), "file-npm");
    assert_eq!(icon("flake.nix"), "file-nix");
}

#[test]
fn a_readme_or_a_licence_is_one_however_its_name_goes_on() {
    for name in ["README", "README.md", "readme.txt", "README.de.md"] {
        assert_eq!(icon(name), "file-readme", "{name}");
    }
    for name in ["LICENSE", "LICENSE-MIT", "LICENSE_APACHE.txt", "LICENCE", "COPYING"] {
        assert_eq!(icon(name), "file-license", "{name}");
    }
    assert_eq!(icon("license.rs"), "file-rust", "a source file that starts with the word");
    assert_eq!(icon("readme_test.py"), "file-python");
    assert_eq!(icon("READMEFIRST"), "file");
}

#[test]
fn a_hidden_name_has_no_ending_in_its_first_dot() {
    assert_eq!(icon(".zst"), "file", "the dot of a hidden name starts no extension");
    assert_eq!(icon(".profile"), "file-shell", "known by its whole name");
    assert_eq!(file_kind("Cargo.toml", true, false).icon(), "folder", "a folder named like a file is a folder");
    assert_eq!(icon(".config.toml"), "file-toml");
    assert_eq!(icon("trailing."), "file");
}

#[test]
fn folders_that_say_what_they_hold() {
    let cases = [
        (".git", "folder-git"),
        (".github", "folder-github"),
        (".config", "folder-config"),
        ("node_modules", "folder-packages"),
        (".trash", "folder-trash"),
        (".mozilla", "folder-hidden"),
        ("target", "folder-build"),
        ("src", "folder-source"),
        ("docs", "folder-docs"),
        ("tests", "folder-tests"),
        ("Downloads", "folder"),
    ];
    for (name, key) in cases {
        assert_eq!(file_kind(name, true, false).icon(), key, "{name}");
    }
    assert_eq!(icon("src"), "file", "a file called src is not a folder");
}

#[test]
fn every_table_is_sorted_lower_case_and_without_repeats() {
    let tables: [(&str, Vec<&str>); 5] = [
        ("kinds", KINDS.iter().map(|(key, _)| *key).collect()),
        ("names", NAMES.iter().map(|(key, _)| *key).collect()),
        ("doubles", DOUBLES.iter().map(|(key, _)| *key).collect()),
        ("extensions", EXTENSIONS.iter().map(|(key, _)| *key).collect()),
        ("folders", FOLDERS.iter().map(|(key, _)| *key).collect()),
    ];
    for (table, keys) in tables {
        for pair in keys.windows(2) {
            assert!(pair[0] < pair[1], "{table}: `{}` then `{}`", pair[0], pair[1]);
        }
        for key in keys {
            assert_eq!(key, key.to_ascii_lowercase(), "{table}: `{key}` is looked up in lower case");
        }
    }
    for (ending, _) in DOUBLES {
        assert!((1..=2).contains(&ending.matches('.').count()), "{ending}: two or three parts");
    }
}

#[test]
fn the_tables_reach_the_scope_a_file_list_needs() {
    assert!(NAMES.len() >= 60, "{} whole names", NAMES.len());
    assert!(EXTENSIONS.len() >= 300, "{} extensions", EXTENSIONS.len());
    assert!(FOLDERS.len() >= 20, "{} folders", FOLDERS.len());
}

/// Every icon key the kinds can answer with.
fn every_key() -> Vec<&'static str> {
    let mut keys: Vec<&str> = KINDS.iter().map(|(key, _)| *key).collect();
    keys.extend(["file", "folder"]);
    keys
}

#[test]
fn every_key_the_tables_give_is_a_kind_of_its_own_family() {
    let kinds: BTreeMap<&str, KindFamily> = KINDS.iter().copied().collect();
    for table in [NAMES, DOUBLES, EXTENSIONS, FOLDERS] {
        for (name, key) in table {
            assert!(kinds.contains_key(key), "`{name}` gives `{key}`, which is not a kind");
        }
    }
    for (name, key) in FOLDERS {
        assert_eq!(kinds[key], KindFamily::Folder, "{name}");
    }
    for (key, family) in KINDS {
        assert_eq!(key.starts_with("folder-"), *family == KindFamily::Folder, "{key}");
    }
}

#[test]
fn every_kind_is_one_cell_in_all_three_modes_and_ascii_never_wraps() {
    let registry = IconSetRegistry::builtin();
    assert!(registry.diagnostics().is_empty(), "{:?}", registry.diagnostics());
    let mut icons = registry.icons("default", &BTreeMap::new(), GlyphMode::Nerd);
    for key in every_key() {
        let glyphs = icons.glyphs(key).unwrap_or_else(|| panic!("`{key}` is not in the icon set")).clone();
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            icons.set_mode(mode);
            let glyph = icons.glyph(key);
            assert_eq!(glyph.chars().count(), 1, "{key} in {mode:?}: `{glyph}`");
            assert_eq!(crate::text::width(&glyph), 1, "{key} in {mode:?}: `{glyph}`");
        }
        assert!(!glyphs.ascii.contains(['[', ']', '(', ')', '{', '}', '<', '|']), "{key}: `{}`", glyphs.ascii);
        let nerd = glyphs.nerd.chars().next().map_or(0, u32::from);
        let private = (0xE000..=0xF8FF).contains(&nerd) || (0xF0000..=0xFFFFD).contains(&nerd);
        assert!(private, "{key}: a Nerd Font glyph is a private-use code point, not U+{nerd:04X}");
    }
}

#[test]
fn outside_a_nerd_font_the_shape_says_the_family() {
    let icons = IconSetRegistry::builtin().icons("default", &BTreeMap::new(), GlyphMode::Unicode);
    let mut shapes: BTreeMap<String, KindFamily> = BTreeMap::new();
    for (key, family) in KINDS {
        let shape = icons.glyph(key).into_owned();
        let known = *shapes.entry(shape.clone()).or_insert(*family);
        assert_eq!(known, *family, "{key} shares `{shape}` with another family");
    }
    assert_eq!(icons.glyph("file-rust"), "◇");
    assert_eq!(icons.glyph("file-image"), "◩");
    assert_eq!(icons.glyph("file-archive"), "▣");
}

#[test]
fn user_folders_are_their_own_only_in_the_home() {
    let text = "# written by xdg-user-dirs-update\nXDG_DESKTOP_DIR=\"$HOME/Masaüstü\"\n\
                XDG_DOWNLOAD_DIR=\"$HOME/İndirilenler\"\nXDG_PICTURES_DIR=\"/home/ada/Resimler\"\n\
                XDG_MUSIC_DIR=\"$HOME\"\nXDG_VIDEOS_DIR=\"/srv/videos\"\nXDG_NOTHING_DIR=\"$HOME/x\"\n";
    let folders = UserFolders::parse("/home/ada", text);
    let kind = |path: &str| folders.kind(Path::new(path)).map(|kind| kind.icon());
    assert_eq!(kind("/home/ada"), Some("folder-home"));
    assert_eq!(kind("/home/ada/Masaüstü"), Some("folder-desktop"));
    assert_eq!(kind("/home/ada/İndirilenler"), Some("folder-downloads"));
    assert_eq!(kind("/home/ada/Resimler"), Some("folder-pictures"), "an absolute path inside the home");
    assert_eq!(kind("/home/ada/Downloads"), None, "the file names the downloads folder otherwise");
    assert_eq!(kind("/home/ada/work/İndirilenler"), None, "only in the home");
    assert_eq!(kind("/srv/videos"), None, "a folder outside the home is not one of its folders");
    assert_eq!(folders.kind(Path::new("/home/ada/Masaüstü")).map(|kind| kind.family()), Some(KindFamily::Folder));

    let english = UserFolders::english("/home/bo");
    assert_eq!(english.kind(Path::new("/home/bo/Downloads")).map(|kind| kind.icon()), Some("folder-downloads"));
    assert_eq!(english.kind(Path::new("/home/bo/Videos")).map(|kind| kind.icon()), Some("folder-videos"));
}

#[test]
fn user_folders_are_read_from_the_config_folder_and_fall_back_to_english() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let config = std::env::temp_dir().join(format!("qframe-user-folders-{stamp}"));
    std::fs::create_dir_all(&config).expect("the config folder");
    let missing = UserFolders::read("/home/ada", &config);
    assert_eq!(missing, UserFolders::english("/home/ada"));
    std::fs::write(config.join("user-dirs.dirs"), "XDG_MUSIC_DIR=\"$HOME/Müzik\"\n").expect("the file");
    let read = UserFolders::read("/home/ada", &config);
    assert_eq!(read.kind(Path::new("/home/ada/Müzik")).map(|kind| kind.icon()), Some("folder-music"));
    assert_eq!(read.kind(Path::new("/home/ada/Music")), None);
    let _ = std::fs::remove_dir_all(&config);
}

#[test]
fn tones_follow_the_families_and_a_file_of_no_kind_keeps_its_own() {
    assert_eq!(KindFamily::Folder.tone(), Some("accent"));
    assert_eq!(KindFamily::Code.tone(), KindFamily::Text.tone());
    assert_eq!(KindFamily::Image.tone(), KindFamily::Video.tone());
    assert_eq!(KindFamily::Archive.tone(), KindFamily::Package.tone());
    assert_eq!(KindFamily::Data.tone(), KindFamily::Key.tone());
    assert_ne!(KindFamily::Code.tone(), KindFamily::Image.tone());
    assert_eq!(KindFamily::File.tone(), None);
}
