use std::ffi::OsString;

use super::*;
use crate::desktop::fixture::Tree;

/// A desktop entry for a program that opens `mime_types`.
fn entry(name: &str, mime_types: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName={name}\nExec={} %f\nMimeType={mime_types}\n", name.to_lowercase())
}

fn ids(apps: &[&DesktopApp]) -> Vec<String> {
    apps.iter().map(|app| app.id.clone()).collect()
}

fn load(tree: &Tree) -> Apps {
    Apps::load(&tree.dirs(), "C", None)
}

/// A database that knows `text/x-rust` is a kind of plain text.
fn rust_db(tree: &Tree) -> MimeDb {
    tree.write("usr/mime/subclasses", "text/x-rust text/plain\n");
    MimeDb::load(&tree.dirs())
}

#[test]
fn the_persons_default_wins_over_the_systems() {
    let tree = Tree::new();
    tree.write("usr/applications/a.desktop", entry("A", "text/plain;"));
    tree.write("usr/applications/b.desktop", entry("B", "text/plain;"));
    tree.write("usr/applications/c.desktop", entry("C", "text/plain;"));
    tree.write("usr/applications/mimeapps.list", "[Default Applications]\ntext/plain=b.desktop\n");
    tree.write("etc/mimeapps.list", "[Default Applications]\ntext/plain=c.desktop\n");
    let db = MimeDb::default();
    let apps = load(&tree);
    assert_eq!(apps.default_for(&db, "text/plain").map(|a| a.id.as_str()), Some("c.desktop"));

    tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/plain=a.desktop\n");
    let apps = load(&tree);
    assert_eq!(apps.default_for(&db, "text/plain").map(|a| a.id.as_str()), Some("a.desktop"));
    assert_eq!(
        ids(&apps.for_mime(&db, "text/plain")),
        ["a.desktop", "c.desktop", "b.desktop"],
        "every file's default comes before the programs that merely declare the kind"
    );
}

#[test]
fn a_desktops_own_list_comes_before_the_shared_one() {
    let tree = Tree::new();
    tree.write("usr/applications/a.desktop", entry("A", ""));
    tree.write("usr/applications/k.desktop", entry("K", ""));
    tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/plain=a.desktop\n");
    tree.write("home/config/kde-mimeapps.list", "[Default Applications]\ntext/plain=k.desktop\n");
    let db = MimeDb::default();
    let mut dirs = tree.dirs();
    assert_eq!(
        Apps::load(&dirs, "C", None).default_for(&db, "text/plain").map(|a| a.id.clone()),
        Some("a.desktop".to_owned()),
        "a desktop that is not running has no say"
    );
    dirs.desktops = vec!["kde".to_owned()];
    assert_eq!(
        Apps::load(&dirs, "C", None).default_for(&db, "text/plain").map(|a| a.id.clone()),
        Some("k.desktop".to_owned())
    );
}

#[test]
fn a_default_that_is_not_installed_is_passed_over() {
    let tree = Tree::new();
    tree.write("usr/applications/b.desktop", entry("B", ""));
    tree.write("usr/applications/c.desktop", entry("C", ""));
    tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/plain=gone.desktop;b.desktop;\n");
    tree.write("etc/mimeapps.list", "[Default Applications]\ntext/plain=c.desktop\n");
    let db = MimeDb::default();
    let apps = load(&tree);
    assert_eq!(apps.default_for(&db, "text/plain").map(|a| a.id.as_str()), Some("b.desktop"));

    tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/plain=gone.desktop;\n");
    let apps = load(&tree);
    assert_eq!(
        apps.default_for(&db, "text/plain").map(|a| a.id.as_str()),
        Some("c.desktop"),
        "the next file's default is tried"
    );
}

#[test]
fn added_and_removed_associations() {
    let tree = Tree::new();
    tree.write("usr/applications/declares.desktop", entry("Declares", "text/plain;"));
    tree.write("usr/applications/unwanted.desktop", entry("Unwanted", "text/plain;"));
    tree.write("usr/applications/added.desktop", entry("Added", "image/png;"));
    tree.write(
        "home/config/mimeapps.list",
        "[Added Associations]\ntext/plain=added.desktop;\n\
         [Removed Associations]\ntext/plain=unwanted.desktop;\n",
    );
    let db = MimeDb::default();
    let apps = load(&tree);
    assert_eq!(ids(&apps.for_mime(&db, "text/plain")), ["added.desktop", "declares.desktop"]);
    assert_eq!(ids(&apps.for_mime(&db, "image/png")), ["added.desktop"], "a removal is for one kind only");
}

#[test]
fn a_removal_hides_what_less_important_files_add_but_not_what_more_important_ones_do() {
    let tree = Tree::new();
    tree.write("usr/applications/x.desktop", entry("X", ""));
    tree.write("usr/applications/y.desktop", entry("Y", ""));
    tree.write(
        "home/config/mimeapps.list",
        "[Added Associations]\ntext/plain=x.desktop;\n\
         [Removed Associations]\ntext/plain=x.desktop;y.desktop;\n",
    );
    tree.write(
        "usr/applications/mimeapps.list",
        "[Added Associations]\ntext/plain=y.desktop;x.desktop;\n\
         [Default Applications]\ntext/plain=y.desktop\n",
    );
    let db = MimeDb::default();
    let apps = load(&tree);
    assert_eq!(
        ids(&apps.for_mime(&db, "text/plain")),
        ["x.desktop"],
        "a file's own addition stands; the system's default and addition of a removed program do not"
    );
    assert_eq!(apps.default_for(&db, "text/plain").map(|a| a.id.as_str()), Some("x.desktop"));
}

#[test]
fn rust_source_opens_with_the_programs_for_plain_text() {
    let tree = Tree::new();
    let db = rust_db(&tree);
    tree.write("usr/applications/editor.desktop", entry("Editor", "text/plain;"));
    tree.write("usr/applications/ide.desktop", entry("Ide", "text/x-rust;"));
    tree.write("usr/applications/hex.desktop", entry("Hex", "application/octet-stream;"));
    tree.write("usr/applications/viewer.desktop", entry("Viewer", "image/png;"));
    tree.write("home/config/mimeapps.list", "[Default Applications]\ntext/plain=editor.desktop\n");
    let apps = load(&tree);
    assert_eq!(
        ids(&apps.for_mime(&db, "text/x-rust")),
        ["ide.desktop", "editor.desktop", "hex.desktop"],
        "a program for the exact kind comes first, then those for the kinds it is a case of"
    );
    assert_eq!(
        apps.default_for(&db, "text/x-rust").map(|a| a.id.as_str()),
        Some("editor.desktop"),
        "the default for plain text is the default for Rust too"
    );
    assert!(apps.for_mime(&db, "inode/directory").is_empty(), "a hex editor opens no folder");
}

#[test]
fn with_no_default_the_first_program_is_the_default() {
    let tree = Tree::new();
    let db = MimeDb::default();
    tree.write("usr/applications/b.desktop", entry("B", "image/png;"));
    tree.write("usr/applications/a.desktop", entry("A", "image/png;"));
    let apps = load(&tree);
    assert_eq!(apps.default_for(&db, "image/png").map(|a| a.id.as_str()), Some("a.desktop"));
    assert_eq!(apps.default_for(&db, "video/mp4"), None);
}

#[test]
fn a_program_declaring_an_alias_opens_the_kind() {
    let tree = Tree::new();
    tree.write("usr/mime/aliases", "application/x-pdf application/pdf\n");
    let db = MimeDb::load(&tree.dirs());
    tree.write("usr/applications/reader.desktop", entry("Reader", "application/x-pdf;"));
    let apps = load(&tree);
    assert_eq!(ids(&apps.for_mime(&db, "application/pdf")), ["reader.desktop"]);
}

#[test]
fn the_persons_copy_of_a_program_wins_and_hidden_hides_both() {
    let tree = Tree::new();
    tree.write("home/data/applications/editor.desktop", entry("Mine", "text/plain;"));
    tree.write("usr/local/applications/editor.desktop", entry("Local", "text/plain;"));
    tree.write("usr/applications/editor.desktop", entry("System", "text/plain;"));
    tree.write("usr/applications/viewer.desktop", entry("Viewer", "image/png;"));
    tree.write(
        "home/data/applications/viewer.desktop",
        "[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer\nHidden=true\n",
    );
    let apps = load(&tree);
    assert_eq!(apps.get("editor.desktop").map(|a| a.name.as_str()), Some("Mine"));
    assert_eq!(apps.get("viewer.desktop"), None);
    assert_eq!(apps.apps.len(), 1);
}

#[test]
fn entries_in_subfolders_take_the_folder_into_their_id() {
    let tree = Tree::new();
    tree.write("usr/applications/kde/dolphin.desktop", entry("Dolphin", "inode/directory;"));
    tree.write("usr/applications/kde/deep/x.desktop", entry("X", ""));
    tree.write("usr/applications/readme.txt", entry("NotAnEntry", ""));
    let apps = load(&tree);
    assert!(apps.get("kde-dolphin.desktop").is_some());
    assert!(apps.get("kde-deep-x.desktop").is_some());
    assert_eq!(apps.apps.len(), 2);
}

#[test]
fn a_program_whose_try_exec_is_missing_is_dropped() {
    let tree = Tree::new();
    let bin = tree.root().join("bin");
    tree.program("bin/present");
    tree.write("bin/not-executable", "data");
    let present_abs = bin.join("present");
    let with = |name: &str, try_exec: &str| {
        tree.write(
            &format!("usr/applications/{name}.desktop"),
            format!("[Desktop Entry]\nType=Application\nName={name}\nExec=x\nTryExec={try_exec}\n"),
        );
    };
    with("bare", "present");
    with("absolute", &present_abs.to_string_lossy());
    with("missing", "absent");
    with("missing-abs", &bin.join("absent").to_string_lossy());
    with("plain-file", "not-executable");
    with("folder", &bin.to_string_lossy());
    let path_var = OsString::from(format!("relative:{}:/nonexistent", bin.display()));
    let apps = Apps::load(&tree.dirs(), "C", Some(&path_var));
    let mut found: Vec<&str> = apps.apps.iter().map(|a| a.id.as_str()).collect();
    found.sort_unstable();
    assert_eq!(found, ["absolute.desktop", "bare.desktop"]);

    let apps = Apps::load(&tree.dirs(), "C", None);
    let found: Vec<&str> = apps.apps.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(found, ["absolute.desktop"], "with no search path only an absolute path is found");
}

#[test]
fn only_applications_count_and_hidden_from_menus_still_opens_files() {
    let tree = Tree::new();
    tree.write("usr/applications/link.desktop", "[Desktop Entry]\nType=Link\nName=Link\nURL=https://example.com\n");
    tree.write(
        "usr/applications/quiet.desktop",
        "[Desktop Entry]\nType=Application\nName=Quiet\nExec=quiet %u\nNoDisplay=true\nTerminal=true\n",
    );
    tree.write("usr/applications/noexec.desktop", "[Desktop Entry]\nType=Application\nName=NoExec\n");
    tree.write("usr/applications/noname.desktop", "[Desktop Entry]\nType=Application\nExec=x\n");
    tree.write("usr/applications/othergroup.desktop", "[Other]\nType=Application\nName=X\nExec=x\n");
    let apps = load(&tree);
    let quiet = apps.get("quiet.desktop").expect("kept");
    assert!(quiet.terminal);
    assert_eq!(apps.apps.len(), 1);
}

#[test]
fn the_name_is_in_the_persons_language() {
    let tree = Tree::new();
    tree.write(
        "usr/applications/files.desktop",
        "[Desktop Entry]\nType=Application\nExec=files\n\
         Name=Files\nName[tr]=Dosyalar\nName[tr_TR]=Dosyalar (TR)\nName[sr@latin]=Datoteke\n",
    );
    tree.write(
        "usr/applications/notes.desktop",
        "[Desktop Entry]\nType=Application\nExec=notes\nName=Notes\nName[tr]=Notlar\n",
    );
    let name = |lang: &str, id: &str| {
        Apps::load(&tree.dirs(), lang, None).get(id).map(|app| app.name.clone()).expect("installed")
    };
    assert_eq!(name("tr_TR.UTF-8", "files.desktop"), "Dosyalar (TR)");
    assert_eq!(name("tr_TR.UTF-8", "notes.desktop"), "Notlar");
    assert_eq!(name("tr", "files.desktop"), "Dosyalar");
    assert_eq!(name("sr_RS.UTF-8@latin", "files.desktop"), "Datoteke");
    assert_eq!(name("de_DE.UTF-8", "files.desktop"), "Files");
    assert_eq!(name("C", "files.desktop"), "Files");
    assert_eq!(name("", "files.desktop"), "Files");
}

#[test]
fn values_are_read_with_their_escapes() {
    let tree = Tree::new();
    let path = tree.write(
        "usr/applications/odd.desktop",
        "[Desktop Entry]\n\
         # a comment\n\
         Type = Application\n\
         Name=Odd\\sOne\n\
         Icon=odd-icon\n\
         Exec=\"/opt/Odd App/odd\" --title \"say \\\\\"hi\\\\\"\" %f\n\
         MimeType=text/plain; image/png;text/x-semi\\;colon;\n\
         [Desktop Action New]\nName=New\nExec=other\n",
    );
    let apps = load(&tree);
    let odd = apps.get("odd.desktop").expect("read");
    assert_eq!(odd.name, "Odd One");
    assert_eq!(odd.icon.as_deref(), Some("odd-icon"));
    assert_eq!(odd.path, path);
    assert_eq!(odd.mime_types, ["text/plain", "image/png", "text/x-semi;colon"]);
    assert_eq!(
        odd.command(Path::new("/tmp/a b.txt")),
        Some(["/opt/Odd App/odd", "--title", "say \"hi\"", "/tmp/a b.txt"].map(OsString::from).to_vec())
    );
}

#[test]
fn broken_files_never_panic() {
    let tree = Tree::new();
    let mut garbage: Vec<u8> = (0..=255u8).cycle().take(4000).collect();
    garbage.extend(b"\n[Desktop Entry\n=\n[]\n==\n[Default Applications]\n=x\ntext/plain=\n");
    tree.write("usr/applications/garbage.desktop", &garbage);
    tree.write("usr/applications/mimeapps.list", &garbage);
    tree.write("home/config/mimeapps.list", &garbage);
    tree.write("usr/applications/empty.desktop", "");
    let mut broken = entry("Broken", "text/plain;").into_bytes();
    broken.extend([b'X', b'=', 0xff, 0xfe, b'\n']);
    tree.write("usr/applications/broken.desktop", broken);
    std::fs::create_dir_all(tree.path("etc/mimeapps.list")).expect("a folder where a file belongs");
    let apps = load(&tree);
    let db = MimeDb::default();
    assert_eq!(
        ids(&apps.for_mime(&db, "text/plain")),
        ["broken.desktop"],
        "a line that is not UTF-8 costs only itself"
    );
}
