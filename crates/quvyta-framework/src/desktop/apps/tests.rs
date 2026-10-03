use std::ffi::OsString;

use super::*;
use crate::desktop::fixture::Tree;

/// A desktop entry for a program that opens `mime_types`.
fn entry(name: &str, mime_types: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName={name}\nExec={} %f\nMimeType={mime_types}\n", name.to_lowercase())
}

/// A desktop entry with every key a launcher shows a program by, in English and in Turkish.
const FULL: &str = "[Desktop Entry]
Type=Application
Name=Notes
Name[tr]=Notlar
Comment=Take notes in plain text
Comment[tr]=Düz metinde not tut
GenericName=Text Editor
GenericName[tr]=Metin Düzenleyici
Keywords=notes;text;editor;
Keywords[tr]=notlar;metin;
Categories=Utility;TextEditor;
Path=/home/ada/notes
NoDisplay=true
TryExec=present
Exec=notes %f
";

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
fn an_entry_says_what_a_launcher_needs_about_a_program() {
    let tree = Tree::new();
    tree.program("bin/present");
    tree.write("usr/applications/notes.desktop", FULL);
    tree.write("usr/applications/other.desktop", entry("Other", "text/plain;"));
    let path_var = OsString::from(format!("relative:{}", tree.root().join("bin").display()));
    let read = |lang: &str| Apps::load(&tree.dirs(), lang, Some(&path_var));

    let apps = read("C");
    let english = apps.details("notes.desktop").expect("the program is read");
    assert_eq!(english.comment.as_deref(), Some("Take notes in plain text"));
    assert_eq!(english.generic_name.as_deref(), Some("Text Editor"));
    assert_eq!(english.categories, ["Utility", "TextEditor"]);
    assert_eq!(english.keywords, ["notes", "text", "editor"]);
    assert_eq!(english.folder.as_deref(), Some(Path::new("/home/ada/notes")));
    assert!(english.no_display);
    assert!(english.installed);
    assert_eq!(english.try_exec.as_deref(), Some("present"));
    assert!(!english.hidden, "a plain read never keeps an entry the person deleted");

    let apps = read("tr_TR.UTF-8");
    assert_eq!(apps.get("notes.desktop").map(|app| app.name.as_str()), Some("Notlar"), "the name too");
    let turkish = apps.details("notes.desktop").expect("the program is read");
    assert_eq!(turkish.comment.as_deref(), Some("Düz metinde not tut"));
    assert_eq!(turkish.generic_name.as_deref(), Some("Metin Düzenleyici"));
    assert_eq!(
        turkish.keywords,
        ["notlar", "metin", "notes", "text", "editor"],
        "the words in the person's language come first and the plain ones follow, each written once"
    );

    let other = apps.details("other.desktop").expect("every program has an entry of its own");
    assert_eq!(other.comment, None, "a key the entry does not write says nothing");
    assert!(other.keywords.is_empty() && other.categories.is_empty() && other.folder.is_none());
    assert!(other.installed, "an entry with no TryExec is installed");
    assert_eq!(other.try_exec, None);
}

#[test]
fn a_launcher_starts_a_program_with_no_file_to_open() {
    let tree = Tree::new();
    tree.write("usr/applications/foo.desktop", "[Desktop Entry]\nType=Application\nName=Foo\nExec=foo %U --x %%\n");
    tree.write("usr/applications/bar.desktop", "[Desktop Entry]\nType=Application\nName=Bar\nExec=bar %f\n");
    let apps = load(&tree);
    let foo = apps.get("foo.desktop").expect("read");
    assert_eq!(
        foo.launch_command(),
        Some(["foo", "--x", "%"].map(OsString::from).to_vec()),
        "the codes that take a file or a URL are gone and a doubled percent is a percent"
    );
    assert_eq!(
        foo.command(Path::new("/tmp/a.txt")),
        Some(["foo", "/tmp/a.txt", "--x", "%"].map(OsString::from).to_vec()),
        "the same line opens a file"
    );
    assert_eq!(
        apps.get("bar.desktop").and_then(DesktopApp::launch_command),
        Some(["bar"].map(OsString::from).to_vec()),
        "nothing is added where the file would have been"
    );
}

#[test]
fn the_launch_command_keeps_the_codes_that_are_not_about_a_file() {
    let tree = Tree::new();
    let path = tree.write(
        "usr/applications/odd.desktop",
        "[Desktop Entry]\nType=Application\nName=Odd One\nIcon=odd-icon\nExec=odd %i --class %c --desktop %k %f\n",
    );
    let apps = load(&tree);
    let odd = apps.get("odd.desktop").expect("read");
    let expected: Vec<OsString> = ["odd", "--icon", "odd-icon", "--class", "Odd One", "--desktop"]
        .map(OsString::from)
        .into_iter()
        .chain([path.into_os_string()])
        .collect();
    assert_eq!(odd.launch_command(), Some(expected), "the name, the icon and the entry stay");
}

#[test]
fn a_program_that_is_not_installed_is_dropped_unless_it_is_kept() {
    let tree = Tree::new();
    tree.write(
        "usr/applications/gone.desktop",
        "[Desktop Entry]\nType=Application\nName=Gone\nExec=gone %f\nTryExec=absent\n",
    );
    let apps = load(&tree);
    assert!(apps.get("gone.desktop").is_none(), "a program that is not installed opens nothing");
    assert!(apps.details("gone.desktop").is_none());

    let apps = Apps::load_including(&tree.dirs(), "C", None, Include::MISSING);
    assert_eq!(apps.get("gone.desktop").map(|app| app.name.as_str()), Some("Gone"), "a launcher lists it");
    let kept = apps.details("gone.desktop").expect("with what its entry says");
    assert_eq!(kept.try_exec.as_deref(), Some("absent"));
    assert!(!kept.installed, "the program it names is nowhere on this machine");
    let db = MimeDb::default();
    assert!(apps.for_mime(&db, "text/plain").is_empty(), "and it is not a choice for a file either");
}

#[test]
fn an_entry_the_person_deleted_is_kept_only_when_asked() {
    let tree = Tree::new();
    tree.write(
        "home/data/applications/gone.desktop",
        "[Desktop Entry]\nType=Application\nName=Gone\nExec=gone %f\nHidden=true\nMimeType=text/plain;\n",
    );
    tree.write("usr/applications/other.desktop", entry("Other", "text/plain;"));
    let apps = load(&tree);
    assert_eq!(
        ids(&apps.all().iter().collect::<Vec<_>>()),
        ["other.desktop"],
        "a deleted entry takes its id and leaves nothing behind"
    );
    assert!(apps.details("gone.desktop").is_none());

    let apps = Apps::load_including(&tree.dirs(), "C", None, Include::HIDDEN);
    let deleted = apps.details("gone.desktop").expect("what the person removed is still there to be seen");
    assert!(deleted.hidden);
    assert!(deleted.installed, "its program is there; it is the entry that is hidden");
    let db = MimeDb::default();
    assert_eq!(
        ids(&apps.for_mime(&db, "text/plain")),
        ["other.desktop"],
        "a deleted program opens nothing, whatever kinds it declared"
    );
    assert_eq!(apps.default_for(&db, "text/plain").map(|app| app.id.as_str()), Some("other.desktop"));
}

#[test]
fn a_launcher_may_ask_for_both_kinds_of_entry_at_once() {
    let tree = Tree::new();
    tree.write(
        "usr/applications/gone.desktop",
        "[Desktop Entry]\nType=Application\nName=Gone\nExec=gone %f\nTryExec=absent\n",
    );
    tree.write(
        "usr/applications/old.desktop",
        "[Desktop Entry]\nType=Application\nName=Old\nExec=old %f\nHidden=true\n",
    );
    let kept = |keep| {
        Apps::load_including(&tree.dirs(), "C", None, keep)
            .all()
            .iter()
            .map(|app| app.id.clone())
            .collect::<Vec<String>>()
    };
    assert_eq!(kept(Include::ALL), ["gone.desktop", "old.desktop"]);
    assert_eq!(kept(Include::NONE), Vec::<String>::new(), "the same as a plain read");
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
fn a_line_that_makes_no_sense_is_reported_where_it_stands() {
    let tree = Tree::new();
    tree.write(
        "usr/applications/odd.desktop",
        "Name=Odd\n  Exec=odd %f\n[Desktop Entry]\n  Type=Application\nName=Odd\n  Exec=odd \"unclosed %f\n",
    );
    let apps = load(&tree);
    let at: Vec<(usize, usize)> =
        apps.diagnostics().iter().filter_map(|one| one.location.as_ref().map(|at| (at.line, at.column))).collect();
    assert_eq!(at, [(1, 1), (2, 3), (6, 3)], "a problem names its line, and a key the column it is written at");
    assert!(apps.all().is_empty(), "an entry whose Exec line gives no command is no program");
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
