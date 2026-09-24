use std::fs;

use super::*;
use crate::desktop::fixture::Tree;

/// A database read from a tree whose person's data folder holds `globs2`.
fn with_globs(tree: &Tree, globs2: &str) -> MimeDb {
    tree.write("home/data/mime/globs2", globs2);
    MimeDb::load(&tree.dirs())
}

#[test]
fn the_heaviest_pattern_wins_then_the_longest() {
    let tree = Tree::new();
    let db = with_globs(
        &tree,
        "# a comment\n\
         50:application/gzip:*.gz\n\
         50:application/x-compressed-tar:*.tar.gz\n\
         80:text/x-special:special.*\n\
         50:text/x-other:*.other\n",
    );
    assert_eq!(
        db.guess("backup.tar.gz").as_deref(),
        Some("application/x-compressed-tar"),
        "of two equal weights the longer pattern says more"
    );
    assert_eq!(db.guess("notes.gz").as_deref(), Some("application/gzip"));
    assert_eq!(
        db.guess("special.other").as_deref(),
        Some("text/x-special"),
        "a heavier pattern wins even when it is not the longer one"
    );
    assert_eq!(db.guess("nothing.known"), None);
}

#[test]
fn letter_case_matters_only_when_the_pattern_says_so() {
    let tree = Tree::new();
    let db = with_globs(
        &tree,
        "50:image/jpeg:*.jpg\n\
         50:text/x-c++src:*.C:cs\n\
         50:text/x-csrc:*.c\n\
         50:text/x-makefile:Makefile:cs\n",
    );
    assert_eq!(db.guess("PHOTO.JPG").as_deref(), Some("image/jpeg"));
    assert_eq!(db.guess("main.C").as_deref(), Some("text/x-c++src"));
    assert_eq!(db.guess("main.c").as_deref(), Some("text/x-csrc"));
    assert_eq!(db.guess("Makefile").as_deref(), Some("text/x-makefile"));
    assert_eq!(db.guess("makefile"), None, "the literal name is case-sensitive");
}

#[test]
fn patterns_know_single_characters_and_sets() {
    let tree = Tree::new();
    let db = with_globs(
        &tree,
        "50:text/x-troff-man:*.[1-9]\n\
         50:text/x-csrc:*.[ch]\n\
         50:text/x-odd:?odd\n\
         50:text/x-bracket:*.[x\n",
    );
    assert_eq!(db.guess("ls.1").as_deref(), Some("text/x-troff-man"));
    assert_eq!(db.guess("ls.0"), None);
    assert_eq!(db.guess("x.h").as_deref(), Some("text/x-csrc"));
    assert_eq!(db.guess("aodd").as_deref(), Some("text/x-odd"));
    assert_eq!(db.guess("odd"), None);
    assert_eq!(db.guess("a.[x").as_deref(), Some("text/x-bracket"), "a set never closed is an ordinary bracket");
}

#[test]
fn a_more_important_folder_can_take_a_kinds_patterns_away() {
    let tree = Tree::new();
    tree.write("home/data/mime/globs2", "50:text/x-mine:*.mine\n50:text/x-old:__NOGLOBS__\n");
    tree.write("usr/mime/globs2", "50:text/x-old:*.old\n50:text/x-kept:*.kept\n");
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(db.guess("a.mine").as_deref(), Some("text/x-mine"));
    assert_eq!(db.guess("a.old"), None);
    assert_eq!(db.guess("a.kept").as_deref(), Some("text/x-kept"));
}

#[test]
fn an_alias_resolves_to_the_kinds_own_name_and_the_person_wins() {
    let tree = Tree::new();
    tree.write("home/data/mime/aliases", "application/x-pdf application/x-mine\n");
    tree.write("usr/mime/aliases", "# comment\napplication/x-pdf application/pdf\ntext/x-rust text/rust\n");
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(db.canonical("text/x-rust"), "text/rust");
    assert_eq!(db.canonical("application/x-pdf"), "application/x-mine");
    assert_eq!(db.canonical("image/png"), "image/png");
}

#[test]
fn every_text_is_plain_text_and_everything_is_a_stream_of_bytes() {
    let tree = Tree::new();
    tree.write("usr/mime/subclasses", "text/x-rust text/plain\n");
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(db.ancestors("text/x-rust"), ["text/x-rust", "text/plain", "application/octet-stream"]);
    assert_eq!(
        db.ancestors("text/x-unknown"),
        ["text/x-unknown", "text/plain", "application/octet-stream"],
        "a text shared-mime-info does not know is still text"
    );
    assert_eq!(db.ancestors("image/png"), ["image/png", "application/octet-stream"]);
    assert_eq!(db.ancestors("inode/directory"), ["inode/directory"]);
    assert_eq!(db.ancestors("application/octet-stream"), ["application/octet-stream"]);
}

#[test]
fn ancestors_are_walked_breadth_first_through_aliases() {
    let tree = Tree::new();
    tree.write(
        "usr/mime/subclasses",
        "application/x-child application/x-parent\n\
         application/x-child application/x-alias\n\
         application/x-parent application/x-grandparent\n\
         application/x-other application/octet-stream\n\
         application/x-other application/x-last\n",
    );
    tree.write("usr/mime/aliases", "application/x-alias application/x-other\n");
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(
        db.ancestors("application/x-child"),
        [
            "application/x-child",
            "application/x-parent",
            "application/x-other",
            "application/x-grandparent",
            "application/x-last",
            "application/octet-stream",
        ],
        "an alias parent is its own kind, and the stream of bytes always comes last"
    );
}

#[test]
fn an_alias_is_asked_for_under_both_names() {
    let tree = Tree::new();
    tree.write("usr/mime/aliases", "text/x-rust text/rust\n");
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(db.ancestors("text/x-rust"), ["text/x-rust", "text/rust", "text/plain", "application/octet-stream"]);
}

#[test]
fn contents_decide_when_the_name_says_nothing() {
    let tree = Tree::new();
    let db = with_globs(&tree, "50:image/png:*.png\n");
    let text = tree.write("files/notes", "merhaba dünya\n");
    let binary = tree.write("files/blob", [0x7f, b'E', b'L', b'F', 0, 1, 2]);
    let invalid = tree.write("files/latin1", [b'a', 0xe7, b'b']);
    let empty = tree.write("files/empty", "");
    let picture = tree.write("files/photo.png", "not really a picture");
    fs::create_dir_all(tree.path("files/folder.png")).expect("folder");
    assert_eq!(db.sniff(&text), "text/plain");
    assert_eq!(db.sniff(&binary), "application/octet-stream");
    assert_eq!(db.sniff(&invalid), "application/octet-stream");
    assert_eq!(db.sniff(&empty), "text/plain");
    assert_eq!(db.sniff(&picture), "image/png", "the name wins over the contents");
    assert_eq!(db.sniff(&tree.path("files/folder.png")), "inode/directory", "a folder is a folder whatever its name");
    assert_eq!(db.sniff(&tree.path("files/missing")), "application/octet-stream");
}

#[test]
fn a_character_cut_at_the_end_of_what_is_read_is_still_text() {
    let tree = Tree::new();
    let db = MimeDb::default();
    let mut long = "a".repeat(SNIFF - 1).into_bytes();
    long.extend("ç and more".as_bytes());
    let path = tree.write("files/long", long);
    assert_eq!(db.sniff(&path), "text/plain");
    let mut early = b"a".to_vec();
    early.push(0xc3);
    early.push(b'b');
    let path = tree.write("files/broken", early);
    assert_eq!(db.sniff(&path), "application/octet-stream", "only a character cut by the 4 KiB limit is forgiven");
}

#[test]
fn only_the_first_four_kib_are_read() {
    let tree = Tree::new();
    let mut late_nul = "a".repeat(SNIFF).into_bytes();
    late_nul.push(0);
    let path = tree.write("files/late", late_nul);
    assert_eq!(MimeDb::default().sniff(&path), "text/plain");
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_utf8_is_still_matched_by_its_ending() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let tree = Tree::new();
    let db = with_globs(&tree, "50:image/png:*.png\n");
    let path = tree.path("files").join(OsStr::from_bytes(b"f\xffoto.png"));
    fs::create_dir_all(tree.path("files")).expect("folder");
    fs::write(&path, [0u8, 1, 2]).expect("file");
    assert_eq!(db.sniff(&path), "image/png");
}

#[test]
fn broken_databases_never_panic_and_good_lines_still_count() {
    let tree = Tree::new();
    let mut garbage: Vec<u8> = (0..=255u8).cycle().take(3000).collect();
    garbage.extend(b"\n50:text/x-good:*.good\n");
    garbage.extend(b"not a number:text/x-bad:*.bad\n:::\n50::*.empty\n50:text/x-none:\n");
    garbage.extend([0xff, 0xfe, b'\n']);
    garbage.extend(b"50:text/x-late:*.late\n");
    tree.write("home/data/mime/globs2", &garbage);
    tree.write("home/data/mime/aliases", &garbage);
    tree.write("home/data/mime/subclasses", &garbage);
    tree.write("usr/mime/aliases", "one two three\nlonely\n");
    fs::create_dir_all(tree.path("usr/local/mime/globs2")).expect("a folder where a file belongs");
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(db.guess("a.good").as_deref(), Some("text/x-good"));
    assert_eq!(db.guess("a.late").as_deref(), Some("text/x-late"));
    assert_eq!(db.guess("a.bad"), None);
    assert_eq!(db.guess("a.empty"), None);
    assert_eq!(db.canonical("one"), "one", "a line of three words is no alias");
    let _ = db.ancestors("text/x-good");
}

#[test]
fn nothing_is_read_when_no_folder_exists() {
    let tree = Tree::new();
    let db = MimeDb::load(&tree.dirs());
    assert_eq!(db.guess("a.txt"), None);
    let db = MimeDb::load(&XdgDirs::default());
    assert_eq!(db.canonical("text/plain"), "text/plain");
}
