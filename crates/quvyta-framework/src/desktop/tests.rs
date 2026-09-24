use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::fixture::Tree;
use super::*;

fn env(vars: &[(&str, &str)]) -> XdgDirs {
    let vars: HashMap<String, String> =
        vars.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect();
    XdgDirs::from_env(|name| vars.get(name).cloned())
}

#[test]
fn unset_folders_take_the_standard_defaults() {
    let dirs = env(&[("HOME", "/home/ada")]);
    assert_eq!(
        dirs,
        XdgDirs {
            data_home: Some(PathBuf::from("/home/ada/.local/share")),
            data_dirs: vec![PathBuf::from("/usr/local/share"), PathBuf::from("/usr/share")],
            config_home: Some(PathBuf::from("/home/ada/.config")),
            config_dirs: vec![PathBuf::from("/etc/xdg")],
            desktops: Vec::new(),
        }
    );
}

#[test]
fn set_folders_are_used_and_empty_ones_count_as_unset() {
    let dirs = env(&[
        ("HOME", "/home/ada"),
        ("XDG_DATA_HOME", "/data/ada"),
        ("XDG_DATA_DIRS", "/opt/share::relative:/usr/share"),
        ("XDG_CONFIG_HOME", ""),
        ("XDG_CONFIG_DIRS", "relative"),
        ("XDG_CURRENT_DESKTOP", "ubuntu:GNOME:"),
    ]);
    assert_eq!(dirs.data_home, Some(PathBuf::from("/data/ada")));
    assert_eq!(
        dirs.data_dirs,
        [PathBuf::from("/opt/share"), PathBuf::from("/usr/share")],
        "a relative folder in a list is left out"
    );
    assert_eq!(dirs.config_home, Some(PathBuf::from("/home/ada/.config")));
    assert_eq!(dirs.config_dirs, [PathBuf::from("/etc/xdg")], "a list with nothing usable falls back to the default");
    assert_eq!(dirs.desktops, ["ubuntu", "gnome"]);
}

#[test]
fn with_no_home_the_persons_folders_are_unknown() {
    let dirs = env(&[("HOME", ""), ("XDG_DATA_HOME", "relative/data")]);
    assert_eq!(dirs.data_home, None);
    assert_eq!(dirs.config_home, None);
    assert_eq!(dirs.data_dirs.len(), 2);
}

#[test]
fn a_rust_file_opens_with_its_own_program_and_the_plain_text_default() {
    let tree = Tree::new();
    tree.write("usr/mime/globs2", "50:text/x-rust:*.rs\n50:image/png:*.png\n");
    tree.write("usr/mime/aliases", "text/rust text/x-rust\n");
    tree.write("usr/mime/subclasses", "text/x-rust text/plain\n");
    tree.write(
        "usr/applications/ide.desktop",
        "[Desktop Entry]\nType=Application\nName=IDE\nExec=ide %f\nMimeType=text/rust;\n",
    );
    tree.write(
        "usr/applications/editor.desktop",
        "[Desktop Entry]\nType=Application\nName=Editor\nName[tr]=Düzenleyici\nExec=editor %F\n\
         MimeType=text/plain;\nTerminal=true\n",
    );
    tree.write(
        "usr/applications/files.desktop",
        "[Desktop Entry]\nType=Application\nName=Files\nExec=files %u\nMimeType=inode/directory;\n",
    );
    tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/plain=editor.desktop\n");
    let openers = Openers::load(&tree.dirs(), "tr_TR.UTF-8", None);

    let source = tree.write("project/main.rs", "fn main() {}\n");
    let choices = openers.for_file(&source);
    assert_eq!(choices.mime, "text/x-rust");
    let names: Vec<&str> = choices.apps.iter().map(|app| app.name.as_str()).collect();
    assert_eq!(names, ["IDE", "Düzenleyici"]);
    assert_eq!(choices.default, Some(1));
    assert!(choices.apps[1].terminal);

    let notes = tree.write("project/NOTES", "remember\n");
    let choices = openers.for_file(&notes);
    assert_eq!(choices.mime, "text/plain");
    assert_eq!(choices.default, Some(0));

    let choices = openers.for_file(&tree.path("project"));
    assert_eq!(choices.mime, "inode/directory");
    assert_eq!(choices.apps.len(), 1);
    assert_eq!(choices.default, Some(0), "the only program is the default");

    let picture = tree.write("project/logo.png", [0x89, b'P', b'N', b'G', 0]);
    let choices = openers.for_file(&picture);
    assert_eq!(choices, Choices { mime: "image/png".to_owned(), apps: Vec::new(), default: None });
    fs::remove_file(&picture).expect("removed");
}

/// Where each diagnostic points, as `file:line`, with the file's name only.
fn places(diagnostics: &[&crate::diagnostics::Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.location.as_ref())
        .map(|at| {
            let name = std::path::Path::new(&at.file).file_name().unwrap_or_default().to_string_lossy().into_owned();
            format!("{name}:{}", at.line)
        })
        .collect()
}

#[test]
fn broken_lines_and_programs_are_reported_where_they_are() {
    let tree = Tree::new();
    tree.write("usr/mime/globs2", "# comment\n50:text/plain:*.txt\nnot a glob\n\n50:text/x-rust:*.rs\n");
    tree.write("usr/mime/subclasses", "text/x-rust text/plain\nlonely\n");
    tree.write("usr/mime/aliases", b"a/b c/d\n\xff\xfe\n");
    let entry = |body: &str| format!("[Desktop Entry]\nType=Application\n{body}");
    tree.write("usr/applications/a-no-name.desktop", entry("Exec=a %f\n"));
    tree.write("usr/applications/b-no-exec.desktop", entry("Name=B\n"));
    tree.write("usr/applications/c-bad-exec.desktop", entry("Name=C\nExec=c \"unclosed %f\n"));
    tree.write("usr/applications/d-no-group.desktop", "Name=D\nExec=d\n");
    tree.write("usr/applications/e-good.desktop", entry("Name=E\nnot a line\nExec=e %f\nMimeType=text/plain;\n"));
    tree.write("home/config/mimeapps.list", "[Default Applications]\nbroken\ntext/plain=e-good.desktop\n");
    let openers = Openers::load(&tree.dirs(), "C", None);
    assert_eq!(
        places(&openers.diagnostics()),
        [
            "globs2:3",
            "aliases:2",
            "subclasses:2",
            "a-no-name.desktop:1",
            "b-no-exec.desktop:1",
            "c-bad-exec.desktop:4",
            "d-no-group.desktop:1",
            "d-no-group.desktop:2",
            "d-no-group.desktop:1",
            "e-good.desktop:4",
            "mimeapps.list:2",
        ]
    );
    let ids: Vec<&str> = openers.apps.all().iter().map(|app| app.id.as_str()).collect();
    assert_eq!(ids, ["e-good.desktop"], "only the usable program is offered");
    assert_eq!(openers.mime.guess("a.rs").as_deref(), Some("text/x-rust"), "the line after a broken one counts");
    let choices = openers.for_file(&tree.write("notes.txt", "x"));
    assert_eq!(choices.default, Some(0), "the list after a broken line still counts");
}

#[test]
fn a_case_sensitive_pattern_wins_a_tie_whatever_the_order() {
    let tree = Tree::new();
    tree.write("usr/mime/globs2", "50:text/x-csrc:*.c\n50:text/x-c++src:*.C:cs\n");
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(db.guess("main.C").as_deref(), Some("text/x-c++src"));
    assert_eq!(db.guess("main.c").as_deref(), Some("text/x-csrc"));
}
