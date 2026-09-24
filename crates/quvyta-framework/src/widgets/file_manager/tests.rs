//! The file manager, run end to end on real temporary folders through the messages a click or a
//! key sends, and its operations checked on their own.
//!
//! Nothing here touches a folder of the user's: every test works in a folder of its own under the
//! system's temporary folder and takes it away again when it ends.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::event::{Event, MouseButton, MouseEvent, MouseKind};
use crate::icons::GlyphMode;
use crate::keymap::Modifiers;
use crate::runtime::{App, Command, Harness, TaskEvent, TaskOutcome};
use crate::widget::View;
use crate::widgets::TreeDrop;

mod focus;
mod keys;
mod mouse;

use super::ops::{
    FileChange, FileError, NameProblem, check_name, copy_into, create_file, create_folder, delete, move_into, rename,
    to_trash,
};

/// Ctrl held, as a Ctrl+click holds it.
const CTRL: Modifiers = Modifiers { ctrl: true, alt: false, shift: false };

/// A moment for a menu or a dialog to settle.
const MOMENT: Duration = Duration::from_millis(400);

/// The size the screens of these tests are drawn at.
const SIZE: (u16, u16) = (72, 24);

/// A folder of this test's own, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    /// A folder holding `Project/src/main.rs`, `Project/README.md` and, beside the project,
    /// `outside.txt`, so an escape can be seen not to reach it.
    fn new(name: &str) -> Self {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = std::env::temp_dir().join(format!("qframe-file-manager-{name}-{stamp}"));
        fs::create_dir_all(path.join("Project").join("src")).expect("a project folder");
        fs::write(path.join("Project").join("src").join("main.rs"), "fn main() {}\n").expect("a file");
        fs::write(path.join("Project").join("README.md"), "hello\n").expect("a file");
        fs::write(path.join("outside.txt"), "keep\n").expect("a file outside the project");
        Self(path)
    }

    fn root(&self) -> PathBuf {
        self.0.join("Project")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A folder a test made unreadable is readable again first, or nothing under it can go.
        let _ = fs::set_permissions(self.root().join("locked"), fs::Permissions::from_mode(0o755));
        let _ = fs::remove_dir_all(&self.0);
    }
}

use std::os::unix::fs::PermissionsExt;

/// An application showing one file manager, as qcode's panel and qdesk's Files both do.
struct Demo {
    manager: FileManagerState,
    opened: Vec<PathBuf>,
    terminals: Vec<PathBuf>,
    noted: Vec<String>,
    /// Whether the application offers a terminal and an item of its own.
    extras: bool,
    disabled: bool,
    /// The entries the application marks, and how.
    marked: Vec<(String, RowMark)>,
    /// Whether the long operation's task is driven by hand rather than started, so a test can hold
    /// it in the middle and look at the screen.
    driven: bool,
    /// The shape the manager is drawn in.
    view: FileView,
    /// Whether the work the manager asks for is dropped, so a read never finishes and the reading
    /// state can be looked at.
    slow: bool,
    /// How many clicks open an entry, when the demo asks for it rather than taking the default.
    open_on: Option<Click>,
}

impl Demo {
    /// A demo whose deleting puts entries in the trash folder `trash`, never the person's own.
    fn trashing(root: PathBuf, trash: PathBuf) -> Self {
        let mut demo = Self::new(root);
        demo.manager = FileManagerState::new(demo.manager.root().to_path_buf()).confined().trashing_in(trash);
        demo
    }

    fn new(root: PathBuf) -> Self {
        Self {
            manager: FileManagerState::new(root).confined(),
            opened: Vec::new(),
            terminals: Vec::new(),
            noted: Vec::new(),
            extras: false,
            disabled: false,
            marked: Vec::new(),
            driven: false,
            view: FileView::Tree,
            slow: false,
            open_on: None,
        }
    }
}

#[derive(Clone)]
enum Msg {
    Files(FileManagerMsg),
    Open(PathBuf),
    Terminal(PathBuf),
    Note(String),
    /// The demo offers a terminal and an item of its own from now on.
    Extras,
    /// The manager is taken away from the person.
    Disable,
    /// The application marks this entry this way.
    Mark(String, RowMark),
    /// The manager is drawn in this shape from now on.
    View(FileView),
    /// The work the manager asks for is dropped from now on, so a read never ends.
    Slow,
}

impl App for Demo {
    type Msg = Msg;

    fn init(&mut self) -> Command<Msg> {
        self.manager.load(Msg::Files)
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Files(message) => {
                let command = self.manager.update(message, Msg::Files);
                if self.slow {
                    return Command::none();
                }
                if self.driven && self.manager.work().is_some() {
                    // The task is not started: the test sends its events itself.
                    return Command::none();
                }
                return command;
            }
            Msg::Open(path) => self.opened.push(path),
            Msg::Terminal(path) => self.terminals.push(path),
            Msg::Note(key) => self.noted.push(key),
            Msg::Extras => self.extras = true,
            Msg::Disable => self.disabled = true,
            Msg::Mark(key, mark) => self.marked.push((key, mark)),
            Msg::View(view) => self.view = view,
            Msg::Slow => self.slow = true,
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut manager = FileManager::new(&self.manager, Msg::Files)
            .view(self.view)
            .on_open(|path| Msg::Open(path.to_path_buf()))
            .disabled(self.disabled);
        if let Some(click) = self.open_on {
            manager = manager.open_on(click);
        }
        if self.extras {
            manager = manager.on_open_terminal(|path| Msg::Terminal(path.to_path_buf())).menu_items(|key, targets| {
                vec![ContextItem::new(format!("Note {} of {}", key, targets.len()), Msg::Note(key.to_owned()))]
            });
        }
        if !self.marked.is_empty() {
            let marked = self.marked.clone();
            manager = manager.row_mark(move |key| {
                marked.iter().find(|(marked, _)| marked == key).map_or_else(RowMark::new, |(_, mark)| mark.clone())
            });
        }
        manager.id("files").show(ui).fill();
    }
}

/// The demo with its root read, motion off so menus and dialogs are there at once.
fn harness(scratch: &Scratch) -> Harness<Demo> {
    let mut h = Harness::new(Demo::new(scratch.root()), SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h
}

fn state(h: &Harness<Demo>) -> &FileManagerState {
    &h.app().manager
}

/// Right-clicks the row showing `text`.
fn right_click(h: &mut Harness<Demo>, text: &str) {
    let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` is on screen:\n{}", h.screen()));
    h.mouse(MouseKind::Down(MouseButton::Right), x, y);
    h.mouse(MouseKind::Up(MouseButton::Right), x, y);
    h.advance(MOMENT);
}

/// Selects the entries `keys` together, the cursor on the first, the way Ctrl+clicks leave them.
fn choose(h: &mut Harness<Demo>, keys: &[&str]) {
    h.send(Msg::Files(FileManagerMsg::Select(keys[0].to_owned())));
    h.send(Msg::Files(FileManagerMsg::Choose(keys.iter().map(|key| (*key).to_owned()).collect())));
}

#[test]
fn names_are_checked_the_way_the_dialog_shows_them() {
    let siblings = ["src", "README.md"];
    let check = |name: &str, current| check_name(name, siblings.iter().copied(), current);
    assert_eq!(check("", None), Err(NameProblem::Empty));
    assert_eq!(check("   ", None), Err(NameProblem::Empty));
    assert_eq!(check("a/b", None), Err(NameProblem::Slash));
    assert_eq!(check("a\0b", None), Err(NameProblem::Nul));
    assert_eq!(check(".", None), Err(NameProblem::Dots));
    assert_eq!(check("..", None), Err(NameProblem::Dots));
    assert_eq!(check("src", None), Err(NameProblem::Taken));
    assert_eq!(check("src", Some("src")), Ok(()), "an entry's own name is not taken from it");
    assert_eq!(check("lib.rs", None), Ok(()));
    assert_eq!(check(".hidden", None), Ok(()), "a name may start with a dot");
}

#[test]
fn the_name_before_the_extension_is_what_a_rename_selects() {
    assert_eq!(stem("report.final.md", false), 0..12);
    assert_eq!(stem("main.rs", false), 0..4);
    assert_eq!(stem("şğü.txt", false), 0..3, "characters, not bytes");
    assert_eq!(stem("Makefile", false), 0..8);
    assert_eq!(stem(".gitignore", false), 0..10, "a dotfile is all name");
    assert_eq!(stem(".env.local", false), 0..4);
    assert_eq!(stem("v1.2", true), 0..4, "a folder has no extension");
}

#[test]
fn a_key_that_climbs_out_of_the_root_is_no_key_at_all() {
    for key in ["", "../secret.txt", "guide/../../secret.md", "/etc/passwd", "guide//x.md", "./x.md", "a/."] {
        assert!(!is_inside(key), "{key:?}");
    }
    for key in ["x.md", "guide/x.md", "..hidden.txt", "a/b/c.rs", "it's $HOME.txt"] {
        assert!(is_inside(key), "{key:?}");
    }
    assert!(!is_within("srcs", "src"), "a folder whose name only starts the same is another folder");
    assert_eq!(parent_key("a/b/c"), "a/b");
    assert_eq!(parent_key("a"), FileManagerState::ROOT);
    assert_eq!(name_of("a/b/c"), "c");
    assert_eq!(child_key(FileManagerState::ROOT, "a"), "a");
    assert_eq!(child_key("a", "b"), "a/b");
}

#[test]
fn keys_that_reach_outside_the_root_are_refused() {
    let scratch = Scratch::new("outside");
    let root = scratch.root();
    for key in ["..", "../outside.txt", "src/../../outside.txt", "./README.md", "src//x", "/etc"] {
        assert_eq!(delete(&root, key, true), Err(FileError::Outside), "{key}");
        assert_eq!(rename(&root, key, "x", true), Err(FileError::Outside), "{key}");
        assert_eq!(move_into(&root, key, "src", true), Err(FileError::Outside), "{key}");
        // A manager that shows the whole file system still refuses a key that is not a key.
        assert_eq!(delete(&root, key, false), Err(FileError::Outside), "{key}, unconfined");
    }
    assert_eq!(create_file(&root, "..", "x", true), Err(FileError::Outside));
    assert_eq!(move_into(&root, "README.md", "..", true), Err(FileError::Outside));
    assert_eq!(rename(&root, "README.md", "../x", true), Err(FileError::Name(NameProblem::Slash)));
    assert!(scratch.0.join("outside.txt").exists(), "nothing outside was touched");
    assert!(!scratch.0.join("x").exists());
}

#[test]
fn a_link_out_of_the_root_is_only_followed_when_the_manager_is_not_confined() {
    let scratch = Scratch::new("link");
    let root = scratch.root();
    let away = scratch.0.join("away");
    fs::create_dir_all(&away).expect("a folder outside");
    fs::write(away.join("secret.txt"), "keep\n").expect("a file outside");
    std::os::unix::fs::symlink(&away, root.join("door")).expect("a link out");

    assert_eq!(create_file(&root, "door", "x", true), Err(FileError::Outside), "nothing is made through it");
    assert_eq!(delete(&root, "door/secret.txt", true), Err(FileError::Outside), "nothing is deleted through it");
    assert_eq!(move_into(&root, "README.md", "door", true), Err(FileError::Outside), "nothing moves into it");
    assert!(!away.join("x").exists());

    // Without confinement the link is a folder like any other: this is the one rule qdesk drops.
    assert_eq!(create_file(&root, "door", "x", false), Ok(FileChange::Created("door/x".to_owned())));
    assert!(away.join("x").is_file());

    assert_eq!(delete(&root, "door", true), Ok(FileChange::Deleted("door".to_owned())), "the link itself goes");
    assert!(away.join("secret.txt").exists(), "and what it pointed at stays");
}

#[test]
fn entries_are_made_renamed_moved_and_deleted() {
    let scratch = Scratch::new("ops");
    let root = scratch.root();
    assert_eq!(create_file(&root, "src", "lib.rs", true), Ok(FileChange::Created("src/lib.rs".to_owned())));
    assert!(root.join("src/lib.rs").is_file());
    assert_eq!(create_folder(&root, FileManagerState::ROOT, "docs", true), Ok(FileChange::Created("docs".to_owned())));
    assert!(root.join("docs").is_dir());
    assert_eq!(
        rename(&root, "README.md", "GUIDE.md", true),
        Ok(FileChange::Moved("README.md".into(), "GUIDE.md".into()))
    );
    assert!(root.join("GUIDE.md").is_file() && !root.join("README.md").exists());
    assert_eq!(
        move_into(&root, "GUIDE.md", "docs", true),
        Ok(FileChange::Moved("GUIDE.md".into(), "docs/GUIDE.md".into()))
    );
    assert_eq!(fs::read_to_string(root.join("docs/GUIDE.md")).ok().as_deref(), Some("hello\n"));
    assert_eq!(move_into(&root, "docs", "src", true), Ok(FileChange::Moved("docs".into(), "src/docs".into())));
    assert_eq!(delete(&root, "src", true), Ok(FileChange::Deleted("src".to_owned())));
    assert!(!root.join("src").exists(), "a folder goes with everything in it");
}

#[test]
fn a_name_already_there_is_refused_and_nothing_is_overwritten() {
    let scratch = Scratch::new("clash");
    let root = scratch.root();
    fs::write(root.join("src/README.md"), "other\n").expect("a clashing file");
    assert_eq!(
        create_file(&root, FileManagerState::ROOT, "README.md", true),
        Err(FileError::Taken("README.md".to_owned()))
    );
    assert_eq!(create_folder(&root, FileManagerState::ROOT, "src", true), Err(FileError::Taken("src".to_owned())));
    assert_eq!(rename(&root, "README.md", "src", true), Err(FileError::Taken("src".to_owned())));
    assert_eq!(move_into(&root, "README.md", "src", true), Err(FileError::Taken("README.md".to_owned())));
    assert_eq!(fs::read_to_string(root.join("README.md")).ok().as_deref(), Some("hello\n"));
    assert_eq!(fs::read_to_string(root.join("src/README.md")).ok().as_deref(), Some("other\n"));
}

#[test]
fn a_folder_cannot_go_into_itself_or_below_itself() {
    let scratch = Scratch::new("itself");
    let root = scratch.root();
    fs::create_dir_all(root.join("src/deep")).expect("a folder below");
    assert_eq!(move_into(&root, "src", "src", true), Err(FileError::IntoItself));
    assert_eq!(move_into(&root, "src", "src/deep", true), Err(FileError::IntoItself));
    assert!(root.join("src/deep").is_dir());
}

#[test]
fn a_new_file_is_made_in_a_folder_from_its_menu() {
    let scratch = Scratch::new("new-file");
    let mut h = harness(&scratch);
    right_click(&mut h, "src");
    h.click_text("New file").advance(MOMENT);
    let text = h.screen();
    assert!(text.contains("Name"), "a dialog asks for the name:\n{text}");
    assert!(h.is_focused("file-manager-name"), "and its field has the keyboard");

    h.type_text("lib.rs").press("enter").advance(MOMENT);
    assert!(scratch.root().join("src/lib.rs").is_file(), "the file is on disk:\n{}", h.screen());
    let text = h.screen();
    assert!(text.contains("lib.rs") && text.contains("main.rs"), "the folder opened and shows it:\n{text}");
    assert_eq!(state(&h).selected(), Some("src/lib.rs"), "the selection follows the new file");
    assert!(state(&h).naming().is_none(), "the dialog closed");
}

#[test]
fn a_new_folder_is_made_at_the_root_from_its_own_row() {
    let scratch = Scratch::new("new-folder");
    let mut h = harness(&scratch);
    right_click(&mut h, "Project");
    let text = h.screen();
    assert!(text.contains("New folder") && text.contains("Refresh"), "the root's menu:\n{text}");
    assert!(!text.contains("Rename"), "nothing to rename at the root:\n{text}");
    h.click_text("New folder").advance(MOMENT);
    h.type_text("docs").press("enter").advance(MOMENT);
    assert!(scratch.root().join("docs").is_dir(), "{}", h.screen());
    assert!(h.screen().contains("docs"), "{}", h.screen());
    assert_eq!(state(&h).selected(), Some("docs"));
}

#[test]
fn names_are_checked_as_they_are_typed() {
    let scratch = Scratch::new("names");
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::NewFile(String::new()))).advance(MOMENT);
    assert!(!h.screen().contains("cannot be empty"), "an empty field is not scolded at once");
    h.press("enter").advance(MOMENT);
    assert!(h.screen().contains("A name cannot be empty."), "{}", h.screen());
    assert!(state(&h).naming().is_some(), "and the dialog stays");

    for (typed, said) in [
        ("README.md", "This folder already has an entry with"),
        ("a/b", "A name cannot contain “/”."),
        ("..", "“.” and “..” already mean this folder"),
    ] {
        h.send(Msg::Files(FileManagerMsg::Name(typed.to_owned()))).advance(MOMENT);
        assert!(h.screen().contains(said), "`{typed}`:\n{}", h.screen());
    }
    h.press("enter").advance(MOMENT);
    assert!(!scratch.0.join("..").join("x").exists());
    h.press("esc").advance(MOMENT);
    assert!(state(&h).naming().is_none(), "Esc closes the dialog:\n{}", h.screen());
    assert_eq!(fs::read_dir(scratch.root()).map(Iterator::count).unwrap_or_default(), 2, "nothing was made");
}

#[test]
fn a_rename_starts_from_the_old_name_and_follows_the_entry() {
    let scratch = Scratch::new("rename");
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::Expand("src".to_owned(), true)));
    right_click(&mut h, "src");
    h.click_text("Rename").advance(MOMENT);
    let naming = state(&h).naming().expect("the dialog is open").clone();
    assert_eq!(naming.purpose, NameFor::Rename("src".to_owned()));
    assert_eq!(naming.value, "src", "the field starts with the old name");
    assert!(h.screen().contains("New name for src"), "{}", h.screen());
    // An unchanged name is not a clash with itself.
    assert!(!h.screen().contains("already has"), "{}", h.screen());

    h.send(Msg::Files(FileManagerMsg::Name("code".to_owned())));
    h.press("enter").advance(MOMENT);
    let root = scratch.root();
    assert!(root.join("code/main.rs").is_file() && !root.join("src").exists(), "{}", h.screen());
    assert_eq!(state(&h).selected(), Some("code"));
    assert!(state(&h).is_open("code"), "an open folder stays open under its new name");
    assert!(h.screen().contains("main.rs"), "{}", h.screen());
}

#[test]
fn a_rename_starts_with_the_name_before_its_extension_selected() {
    let scratch = Scratch::new("rename-stem");
    let root = scratch.root();
    fs::write(root.join("report.final.md"), "").expect("a file with two dots");
    fs::write(root.join(".gitignore"), "").expect("a dotfile");
    let mut h = harness(&scratch);

    // Typing replaces what is selected, so what survives shows what was.
    h.send(Msg::Files(FileManagerMsg::Rename("report.final.md".to_owned()))).advance(MOMENT);
    h.type_text("summary").press("enter").advance(MOMENT);
    assert!(root.join("summary.md").is_file(), "only the part before the last dot:\n{}", h.screen());

    h.send(Msg::Files(FileManagerMsg::Rename(".gitignore".to_owned()))).advance(MOMENT);
    h.type_text("ignored").press("enter").advance(MOMENT);
    assert!(root.join("ignored").is_file(), "a dotfile is all name:\n{}", h.screen());

    fs::create_dir_all(root.join("v1.2")).expect("a folder with a dot");
    h.send(Msg::Files(FileManagerMsg::Refresh));
    h.send(Msg::Files(FileManagerMsg::Rename("v1.2".to_owned()))).advance(MOMENT);
    h.type_text("old").press("enter").advance(MOMENT);
    assert!(root.join("old").is_dir(), "a folder has no extension:\n{}", h.screen());
}

#[test]
fn cut_and_paste_moves_an_entry_into_a_folder() {
    let scratch = Scratch::new("move");
    let mut h = harness(&scratch);
    right_click(&mut h, "README.md");
    h.click_text("Cut").advance(MOMENT);
    assert_eq!(state(&h).cut(), ["README.md"]);

    right_click(&mut h, "src");
    h.click_text("Paste here").advance(MOMENT);
    let root = scratch.root();
    assert!(root.join("src/README.md").is_file() && !root.join("README.md").exists(), "{}", h.screen());
    assert_eq!(fs::read_to_string(root.join("src/README.md")).ok().as_deref(), Some("hello\n"), "moved, not made anew");
    assert!(state(&h).cut().is_empty(), "the cut is used up");
    assert_eq!(state(&h).selected(), Some("src/README.md"), "the selection follows it");
    assert!(state(&h).is_open("src"), "into a folder that opens to show it");
}

#[test]
fn ctrl_click_selects_several_entries() {
    let scratch = Scratch::new("choose");
    let mut h = harness(&scratch);
    h.click_text("README.md");
    let (x, y) = h.find("src").expect("the folder's row");
    h.events(&[
        Event::Mouse(MouseEvent { kind: MouseKind::Down(MouseButton::Left), x, y, mods: CTRL }),
        Event::Mouse(MouseEvent { kind: MouseKind::Up(MouseButton::Left), x, y, mods: CTRL }),
    ]);
    assert_eq!(state(&h).chosen(), ["README.md", "src"], "{}", h.screen());
}

#[test]
fn the_menu_of_a_selected_row_cuts_the_whole_selection_and_paste_moves_it_all() {
    let scratch = Scratch::new("cut-many");
    let root = scratch.root();
    fs::create_dir_all(root.join("docs")).expect("a folder to move into");
    fs::write(root.join("plan.txt"), "plan\n").expect("a second file");
    let mut h = harness(&scratch);
    choose(&mut h, &["README.md", "plan.txt", "src"]);
    right_click(&mut h, "plan.txt");
    let text = h.screen();
    assert!(text.contains("Cut 3 entries") && text.contains("Delete 3 entries"), "{text}");
    assert!(!text.contains("Rename"), "a name is for one entry:\n{text}");
    h.click_text("Cut 3 entries").advance(MOMENT);
    assert_eq!(state(&h).cut(), ["README.md", "plan.txt", "src"]);

    right_click(&mut h, "docs");
    h.click_text("Paste here").advance(MOMENT);
    for name in ["README.md", "plan.txt", "src/main.rs"] {
        assert!(root.join("docs").join(name).exists(), "{name} moved:\n{}", h.screen());
        assert!(!root.join(name).exists(), "{name} left its place");
    }
    assert!(state(&h).cut().is_empty(), "the cut is used up");
    assert_eq!(state(&h).chosen(), ["docs/README.md", "docs/plan.txt", "docs/src"], "the selection follows them");
}

#[test]
fn a_file_inside_a_selected_folder_travels_with_it() {
    let scratch = Scratch::new("nested");
    let root = scratch.root();
    fs::create_dir_all(root.join("docs")).expect("a folder to move into");
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::Expand("src".to_owned(), true)));
    choose(&mut h, &["src", "src/main.rs"]);
    h.send(Msg::Files(FileManagerMsg::Cut("src".to_owned())));
    assert_eq!(state(&h).cut(), ["src"], "only the folder is cut; its file goes inside it");
    h.send(Msg::Files(FileManagerMsg::Paste("docs".to_owned()))).advance(MOMENT);
    assert!(root.join("docs/src/main.rs").is_file(), "{}", h.screen());
    assert!(!h.screen().contains("could not be handled"), "nothing failed:\n{}", h.screen());
}

#[test]
fn deleting_the_selection_asks_once_for_all_of_it() {
    let scratch = Scratch::new("delete-many");
    let root = scratch.root();
    let mut h = harness(&scratch);
    choose(&mut h, &["README.md", "src"]);
    h.send(Msg::Files(FileManagerMsg::Delete("src".to_owned()))).advance(MOMENT);
    let text = h.screen();
    assert!(text.contains("Delete 2 entries?"), "{text}");
    assert!(text.contains("README.md, src") && text.contains("everything in them"), "{text}");
    h.press("tab").press("enter").advance(MOMENT);
    assert!(!root.join("README.md").exists() && !root.join("src").exists(), "{}", h.screen());
    assert!(state(&h).chosen().is_empty(), "nothing gone stays selected");
}

#[test]
fn a_row_outside_the_selection_acts_on_itself_alone() {
    let scratch = Scratch::new("outside-selection");
    let mut h = harness(&scratch);
    fs::write(scratch.root().join("plan.txt"), "").expect("another file");
    h.send(Msg::Files(FileManagerMsg::Refresh));
    choose(&mut h, &["README.md", "plan.txt"]);
    right_click(&mut h, "src");
    let text = h.screen();
    assert!(text.contains("Rename") && !text.contains("entries"), "{text}");
    h.click_text("Cut").advance(MOMENT);
    assert_eq!(state(&h).cut(), ["src"]);
}

#[test]
fn dragging_the_selection_onto_a_folder_moves_it_there() {
    let scratch = Scratch::new("drag");
    let root = scratch.root();
    fs::create_dir_all(root.join("docs")).expect("a folder to drop into");
    fs::write(root.join("plan.txt"), "plan\n").expect("a second file");
    let mut h = harness(&scratch);
    choose(&mut h, &["README.md", "plan.txt"]);
    let from = h.find("plan.txt").expect("a selected row");
    let to = h.find("docs").expect("the folder");
    h.drag(from, to).advance(MOMENT);
    assert!(root.join("docs/README.md").is_file() && root.join("docs/plan.txt").is_file(), "{}", h.screen());
    assert!(!root.join("README.md").exists() && !root.join("plan.txt").exists());
    assert!(state(&h).is_open("docs"), "the folder opens to show what came in");
}

#[test]
fn a_drop_follows_the_same_rules_as_a_paste() {
    let scratch = Scratch::new("drop-rules");
    let root = scratch.root();
    fs::create_dir_all(root.join("src/deep")).expect("a folder below");
    let mut h = harness(&scratch);
    let drop = |keys: &[&str], into: Option<&str>| {
        Msg::Files(FileManagerMsg::Drop(TreeDrop {
            keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            into: into.map(str::to_owned),
        }))
    };
    h.send(drop(&["src"], Some("src/deep"))).advance(MOMENT);
    assert!(h.screen().contains("A folder cannot go into itself"), "{}", h.screen());
    assert!(root.join("src/deep").is_dir());

    h.send(drop(&["src/main.rs"], None)).advance(MOMENT);
    assert!(root.join("main.rs").is_file(), "the free space below the rows is the root folder");
}

#[test]
fn a_move_that_partly_fails_says_which_entries_stayed_and_why() {
    let scratch = Scratch::new("partial");
    let root = scratch.root();
    fs::write(root.join("src/README.md"), "other\n").expect("a clashing file");
    fs::write(root.join("plan.txt"), "plan\n").expect("a file that can move");
    let mut h = harness(&scratch);
    choose(&mut h, &["README.md", "plan.txt"]);
    h.send(Msg::Files(FileManagerMsg::Cut("README.md".to_owned())));
    h.send(Msg::Files(FileManagerMsg::Paste("src".to_owned()))).advance(MOMENT);
    assert!(root.join("src/plan.txt").is_file(), "what could move moved:\n{}", h.screen());
    assert_eq!(fs::read_to_string(root.join("src/README.md")).ok().as_deref(), Some("other\n"), "nothing overwritten");
    assert_eq!(fs::read_to_string(root.join("README.md")).ok().as_deref(), Some("hello\n"));
    let text = h.screen();
    assert!(text.contains("1 of 2 entries could not be handled"), "{text}");
    // The toast wraps its body, so the reason is looked for in parts.
    assert!(text.contains("README.md: The target folder") && text.contains("already has"), "{text}");
    assert_eq!(state(&h).cut(), ["README.md"], "what stayed stays cut, to try elsewhere");
}

#[test]
fn the_cut_row_is_faint_and_letting_it_go_makes_it_itself_again() {
    let scratch = Scratch::new("faint");
    let mut h = harness(&scratch);
    let (x, y) = h.find("README.md").expect("the row");
    let (x, y) = (u16::try_from(x).expect("a column"), u16::try_from(y).expect("a row"));
    let before = h.fg(x, y);
    h.send(Msg::Files(FileManagerMsg::Cut("README.md".to_owned())));
    h.hover(0, 0).render();
    assert_ne!(h.fg(x, y), before, "the cut row is drawn in another tone:\n{}", h.screen());

    h.send(Msg::Files(FileManagerMsg::DropCut));
    assert!(state(&h).cut().is_empty());
    assert_eq!(h.fg(x, y), before, "and the row is itself again");
}

#[test]
fn a_folder_cannot_go_into_itself_and_a_clash_is_refused_with_a_reason() {
    let scratch = Scratch::new("refuse");
    let root = scratch.root();
    fs::create_dir_all(root.join("src/deep")).expect("a folder below");
    fs::write(root.join("src/README.md"), "other\n").expect("a clashing file");
    let mut h = harness(&scratch);

    h.send(Msg::Files(FileManagerMsg::Cut("src".to_owned())));
    h.send(Msg::Files(FileManagerMsg::Paste("src/deep".to_owned()))).advance(MOMENT);
    assert!(h.screen().contains("A folder cannot go into itself"), "{}", h.screen());
    assert!(root.join("src/deep").is_dir());

    h.send(Msg::Files(FileManagerMsg::Cut("README.md".to_owned())));
    h.send(Msg::Files(FileManagerMsg::Paste("src".to_owned()))).advance(MOMENT);
    assert!(h.screen().contains("already has something called"), "{}", h.screen());
    assert_eq!(fs::read_to_string(root.join("README.md")).ok().as_deref(), Some("hello\n"));
    assert_eq!(state(&h).cut(), ["README.md"], "a refused paste keeps the cut to try elsewhere");
}

#[test]
fn a_folder_inside_the_cut_one_cannot_be_chosen_to_paste_into() {
    let scratch = Scratch::new("paste-disabled");
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::Cut("src".to_owned())));
    right_click(&mut h, "src");
    h.click_text("Paste here").advance(MOMENT);
    assert!(scratch.root().join("src/main.rs").is_file(), "nothing moved:\n{}", h.screen());
    assert_eq!(state(&h).cut(), ["src"]);
}

#[test]
fn deleting_asks_first_and_a_folder_says_everything_in_it_goes() {
    let scratch = Scratch::new("delete");
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::Delete("src".to_owned()))).advance(MOMENT);
    let text = h.screen();
    assert!(text.contains("Delete src?"), "{text}");
    // The dialog wraps its message, so the words are looked for on their own.
    assert!(text.contains("together") && text.contains("everything in it"), "{text}");
    let root = scratch.root();
    assert!(root.join("src").is_dir(), "nothing goes before the answer");

    h.press("esc").advance(MOMENT);
    assert!(root.join("src").is_dir(), "Esc keeps it:\n{}", h.screen());

    h.send(Msg::Files(FileManagerMsg::Delete("README.md".to_owned()))).advance(MOMENT);
    assert!(!h.screen().contains("everything in it"), "a file is only itself");
    h.press("tab").press("enter").advance(MOMENT);
    assert!(!root.join("README.md").exists(), "confirming deletes it:\n{}", h.screen());
    assert!(!h.screen().contains("README.md"), "and the tree follows:\n{}", h.screen());
}

#[test]
fn paths_that_escape_the_root_are_refused_by_the_running_manager_too() {
    let scratch = Scratch::new("escape");
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::DeleteConfirmed(vec!["../outside.txt".to_owned()]))).advance(MOMENT);
    assert!(scratch.0.join("outside.txt").exists());
    assert!(h.screen().contains("outside the folder shown"), "{}", h.screen());
    h.send(Msg::Files(FileManagerMsg::Cut("README.md".to_owned())));
    h.send(Msg::Files(FileManagerMsg::Paste("..".to_owned()))).advance(MOMENT);
    assert!(scratch.root().join("README.md").exists() && !scratch.0.join("README.md").exists());
}

#[test]
fn each_row_offers_what_can_be_done_to_it() {
    let scratch = Scratch::new("menus");
    let mut h = harness(&scratch);
    right_click(&mut h, "README.md");
    let text = h.screen();
    for item in ["Rename", "Cut", "Delete"] {
        assert!(text.contains(item), "`{item}` on a file:\n{text}");
    }
    for item in ["New file", "New folder", "Paste here"] {
        assert!(!text.contains(item), "no `{item}` on a file:\n{text}");
    }
    h.press("esc").advance(MOMENT);

    right_click(&mut h, "src");
    let text = h.screen();
    for item in ["New file", "New folder", "Rename", "Cut", "Delete"] {
        assert!(text.contains(item), "`{item}` on a folder:\n{text}");
    }
    assert!(!text.contains("Paste here"), "nothing to paste yet:\n{text}");
    h.press("esc").advance(MOMENT);

    h.send(Msg::Files(FileManagerMsg::Cut("README.md".to_owned())));
    right_click(&mut h, "src");
    let text = h.screen();
    assert!(text.contains("Paste here") && text.contains("Cancel the move"), "{text}");
    h.click_text("Cancel the move").advance(MOMENT);
    assert!(state(&h).cut().is_empty(), "the mouse lets a cut go too");
}

#[test]
fn the_application_adds_its_own_items_and_a_terminal_to_a_folders_menu() {
    let scratch = Scratch::new("extras");
    let mut h = harness(&scratch);
    h.send(Msg::Extras).render();
    right_click(&mut h, "src");
    let text = h.screen();
    assert!(text.contains("Note src of 1"), "the application's own item, with what it acts on:\n{text}");
    assert!(text.contains("Open a terminal here"), "{text}");
    h.click_text("Open a terminal here").advance(MOMENT);
    assert_eq!(h.app().terminals, [scratch.root().join("src")], "with the folder's own path");

    right_click(&mut h, "README.md");
    let text = h.screen();
    assert!(text.contains("Note README.md of 1"), "{text}");
    assert!(!text.contains("Open a terminal here"), "a file is no place for a terminal:\n{text}");
    h.click_text("Note README.md of 1").advance(MOMENT);
    assert_eq!(h.app().noted, ["README.md"]);
}

#[test]
fn a_double_click_or_enter_on_a_file_asks_the_application_to_open_it_while_ctrl_click_only_chooses() {
    let scratch = Scratch::new("open-or-choose");
    let mut h = harness(&scratch);
    h.click_text("src").click_text("src").advance(MOMENT);
    let (x, y) = h.find("README.md").expect("the file's row");
    h.events(&[
        Event::Mouse(MouseEvent { kind: MouseKind::Down(MouseButton::Left), x, y, mods: CTRL }),
        Event::Mouse(MouseEvent { kind: MouseKind::Up(MouseButton::Left), x, y, mods: CTRL }),
    ]);
    assert_eq!(state(&h).chosen(), ["src", "README.md"], "{}", h.screen());
    assert!(h.app().opened.is_empty(), "a Ctrl+click chooses without opening");
    h.press("space").advance(MOMENT);
    assert!(h.app().opened.is_empty(), "Space chooses too, it does not open");

    h.click_text("main.rs").advance(MOMENT);
    assert!(h.app().opened.is_empty(), "a plain click only selects");
    assert_eq!(state(&h).chosen(), ["src/main.rs"], "and makes it the one selected entry");
    h.click_text("main.rs").click_text("main.rs").advance(MOMENT);
    assert_eq!(h.app().opened, [scratch.root().join("src/main.rs")], "a double click asks for the path");

    h.click_text("README.md").press("enter").advance(MOMENT);
    assert_eq!(h.app().opened.len(), 2, "{:?}", h.app().opened);
    assert_eq!(h.app().opened[1], scratch.root().join("README.md"));
}

#[test]
fn the_keyboard_reaches_the_same_menus() {
    let scratch = Scratch::new("keys");
    let mut h = harness(&scratch);
    h.click_text("src").advance(MOMENT);
    h.press("end").advance(MOMENT);
    assert_eq!(state(&h).selected(), Some("README.md"), "{}", h.screen());
    assert!(h.is_focused("files"), "{}", h.screen());
    h.press("shift+f10").advance(MOMENT);
    let text = h.screen();
    assert!(text.contains("Rename") && text.contains("Delete"), "the selected row's menu:\n{text}");
    h.press("enter").advance(MOMENT);
    assert!(
        matches!(state(&h).naming().map(|naming| &naming.purpose), Some(NameFor::Rename(key)) if key == "README.md"),
        "{}",
        h.screen()
    );
}

#[test]
fn the_keyboard_reaches_the_roots_menu_from_its_row() {
    let scratch = Scratch::new("root-keys");
    let mut h = harness(&scratch);
    h.click_text("src").advance(MOMENT);
    h.press("home").advance(MOMENT);
    assert_eq!(state(&h).selected(), Some(FileManagerState::ROOT), "the top row is the root folder");
    h.press("shift+f10").advance(MOMENT);
    let text = h.screen();
    assert!(text.contains("New folder") && text.contains("Refresh"), "{text}");
    assert!(!text.contains("Rename") && !text.contains("Delete"), "the folder itself stays:\n{text}");
}

#[test]
fn an_empty_root_still_has_its_row_to_make_the_first_entry() {
    let scratch = Scratch::new("empty");
    let root = scratch.root();
    fs::remove_dir_all(&root).expect("clear the root");
    fs::create_dir_all(&root).expect("an empty root folder");
    let mut h = harness(&scratch);
    let text = h.screen();
    assert!(text.contains("empty"), "the row says the folder is empty:\n{text}");
    right_click(&mut h, "empty");
    h.click_text("New file").advance(MOMENT);
    h.type_text("first.txt").press("enter").advance(MOMENT);
    assert!(root.join("first.txt").is_file(), "{}", h.screen());
    assert!(h.screen().contains("first.txt"), "{}", h.screen());
}

#[test]
fn a_folder_that_cannot_be_read_says_why_and_nothing_below_it_is_lost() {
    let scratch = Scratch::new("locked");
    let root = scratch.root();
    let locked = root.join("locked");
    fs::create_dir_all(locked.join("inside")).expect("a folder to lock");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("take away every right");
    if fs::read_dir(&locked).is_ok() {
        // Running as a user the permissions do not hold back, root among them.
        return;
    }
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::Expand("locked".to_owned(), true))).advance(MOMENT);
    assert_eq!(state(&h).children("locked"), Some(&[][..]), "a folder that could not be read holds nothing");
    assert!(!state(&h).is_loading("locked"), "and it is not left reading for ever");
    assert!(h.screen().contains("locked"), "the folder keeps its row:\n{}", h.screen());
    assert!(state(&h).error().is_none(), "one folder below the root is no trouble with the root");
    // Keeping the row is not enough: without the reason, a folder that holds nothing and one
    // that may not be looked into draw exactly the same.
    assert!(state(&h).folder_error("locked").is_some(), "the reason the folder could not be read is kept");
    assert!(
        h.screen().contains("cannot be read"),
        "and the row says so, instead of looking like an empty folder:\n{}",
        h.screen()
    );
    assert!(state(&h).folder_error("inside").is_none(), "a folder that was never read has no reason of its own");

    // The root itself, unreadable, says what the system said instead of showing a gap.
    let mut own = Harness::new(Demo::new(locked.clone()), SIZE.0, SIZE.1);
    own.set_reduced_motion(true).render();
    let text = own.screen();
    assert!(text.contains("This folder could not be read."), "{text}");
    assert!(own.app().manager.error().is_some(), "{text}");
}

#[test]
fn refresh_reads_the_open_folders_again() {
    let scratch = Scratch::new("refresh");
    let root = scratch.root();
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::Expand("src".to_owned(), true)));
    fs::write(root.join("src/late.rs"), "").expect("a file made by someone else");
    fs::write(root.join("GUIDE.md"), "").expect("another");
    assert!(!h.screen().contains("late.rs"), "nothing is watched or polled here");
    h.send(Msg::Files(FileManagerMsg::Refresh));
    let text = h.screen();
    assert!(text.contains("late.rs") && text.contains("GUIDE.md"), "{text}");
}

#[test]
fn a_folder_removed_by_another_program_leaves_nothing_behind() {
    let scratch = Scratch::new("removed");
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::Expand("src".to_owned(), true)));
    h.send(Msg::Files(FileManagerMsg::Cut("src/main.rs".to_owned())));
    fs::remove_dir_all(scratch.root().join("src")).expect("removed by someone else");
    h.send(Msg::Files(FileManagerMsg::Refresh));
    assert!(!state(&h).is_open("src"), "a folder that is gone is not remembered open");
    assert!(state(&h).cut().is_empty(), "and nothing in it waits to be pasted");
    fs::create_dir_all(scratch.root().join("src")).expect("a new folder of the same name");
    h.send(Msg::Files(FileManagerMsg::Refresh));
    assert!(!state(&h).is_open("src"), "a new folder of the old name starts closed");
}

#[test]
fn a_disabled_manager_is_faint_and_answers_nothing() {
    let scratch = Scratch::new("disabled");
    let mut h = harness(&scratch);
    let (x, y) = h.find("README.md").expect("the row");
    let (x, y) = (u16::try_from(x).expect("a column"), u16::try_from(y).expect("a row"));
    let before = h.fg(x, y);
    h.send(Msg::Disable);
    h.hover(0, 0).render();
    assert_ne!(h.fg(x, y), before, "every row is faint:\n{}", h.screen());
    h.click_text("README.md").advance(MOMENT);
    assert!(h.app().opened.is_empty(), "a click opens nothing");
    right_click(&mut h, "README.md");
    assert!(!h.screen().contains("Rename"), "and there is no menu:\n{}", h.screen());
    assert!(h.screen().contains("README.md"), "the rows are still read:\n{}", h.screen());
}

#[test]
fn the_menu_and_the_dialog_draw_no_brackets_or_frames() {
    let scratch = Scratch::new("look");
    let mut h = harness(&scratch);
    for mode in [GlyphMode::Unicode, GlyphMode::Ascii] {
        h.set_glyph_mode(mode).render();
        right_click(&mut h, "src");
        let menu = h.screen();
        h.press("esc").advance(MOMENT);
        h.send(Msg::Files(FileManagerMsg::Rename("src".to_owned()))).advance(MOMENT);
        let dialog = h.screen();
        h.press("esc").advance(MOMENT);
        for text in [menu, dialog] {
            for mark in ['[', ']', '|', '│', '┌', '┐', '└', '┘', '─'] {
                assert!(!text.contains(mark), "{mode:?} draws `{mark}`:\n{text}");
            }
        }
    }
}

#[test]
fn the_file_manager_speaks_turkish() {
    let scratch = Scratch::new("turkish");
    let mut h = harness(&scratch);
    h.set_locale("tr").render();
    right_click(&mut h, "src");
    let text = h.screen();
    for item in ["Yeni dosya", "Yeni klasör", "Yeniden adlandır", "Kes", "Sil"] {
        assert!(text.contains(item), "`{item}`:\n{text}");
    }
    h.press("esc").advance(MOMENT);
    h.send(Msg::Files(FileManagerMsg::Delete("src".to_owned()))).advance(MOMENT);
    let text = h.screen();
    assert!(text.contains("src silinsin mi?") && text.contains("içindeki her şeyle"), "{text}");
}

#[test]
fn every_text_is_there_in_all_nine_languages() {
    let keys = [
        "empty",
        "unreadable",
        "new-file",
        "new-folder",
        "rename",
        "cut",
        "cut-many",
        "paste",
        "drop-cut",
        "delete",
        "delete-many",
        "refresh",
        "open-terminal",
        "new-file-title",
        "new-folder-title",
        "rename-title",
        "name-label",
        "cancel",
        "create",
        "rename-do",
        "name-empty",
        "name-slash",
        "name-nul",
        "name-dots",
        "name-taken",
        "delete-title",
        "delete-file-text",
        "delete-folder-text",
        "delete-many-title",
        "delete-many-text",
        "delete-many-folders-text",
        "and-more",
        "failed",
        "failed-some",
        "outside",
        "into-itself",
        "taken",
        "cross-device",
        "denied",
        "copy",
        "copy-many",
        "drop-copy",
        "trash",
        "trash-many",
        "delete-forever",
        "no-trash-title",
        "no-trash-text",
        "no-trash-many-title",
        "no-trash-many-text",
        "not-readable",
        "missing",
        "no-trash",
        "no-room",
        "stopped",
        "copying",
        "stop",
        "size",
        "read-only",
        "read-and-write",
        "column-name",
        "column-size",
        "column-modified",
        "column-permissions",
        "reading",
        "entries",
        "up",
    ];
    let i18n = crate::i18n::I18n::builtin();
    for (code, _) in crate::assets::LOCALES {
        for key in keys {
            let key = format!("quvyta.file-manager.{key}");
            assert!(i18n.has(code, &key), "{code} has no {key} of its own");
        }
    }
}

/// How many entries the big folder holds.
const MANY: usize = 10_000;

/// Fills `folder` with `count` entries of one width, so two folders of different sizes paint the
/// same cells when only the window of them is drawn.
fn fill(folder: &std::path::Path, count: usize) {
    fs::create_dir_all(folder).expect("the folder");
    for index in 0..count {
        fs::write(folder.join(format!("entry-{index:05}.txt")), "").expect("an entry");
    }
}

#[test]
fn a_folder_of_ten_thousand_entries_paints_no_more_cells_than_a_small_one() {
    let scratch = Scratch::new("many");
    // Two roots of names the same width, each holding nothing but its entries, so the two screens
    // can only differ by the work the entries cost.
    fill(&scratch.0.join("small"), 200);
    fill(&scratch.0.join("large"), MANY);

    /// The cells with something in them, and how many rows the screen has.
    fn painted(h: &Harness<Demo>) -> (usize, usize) {
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        // The scrollbar is left out: its thumb is as long as the window is of the whole.
        let cells = lines
            .iter()
            .map(|line| line.chars().take(usize::from(SIZE.0) - 2).filter(|c| !c.is_whitespace()).count())
            .sum();
        (cells, lines.len())
    }

    let open = |folder: &str| {
        let mut h = Harness::new(Demo::new(scratch.0.join(folder)), SIZE.0, SIZE.1);
        h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
        h
    };
    let (mut small, mut large) = (open("small"), open("large"));
    assert!(large.screen().contains("entry-00000.txt"), "{}", large.screen());
    assert_eq!(
        state(&large).children(FileManagerState::ROOT).map(<[FolderEntry]>::len),
        Some(MANY),
        "all of it was read, in one answer"
    );
    assert_eq!(painted(&small), painted(&large), "a folder of {MANY} paints what a folder of 200 paints");

    // And it stays that way at the far end of the folder, which only the window is drawn of.
    small.press("tab").press("end").render();
    large.press("tab").press("end").render();
    assert!(large.screen().contains(&format!("entry-{:05}.txt", MANY - 1)), "{}", large.screen());
    assert!(!large.screen().contains("entry-05000.txt"), "the middle is not drawn:\n{}", large.screen());
    assert_eq!(painted(&small), painted(&large), "at the end too");
}

/// Applies `message` to `state` without running the work it asks for.
///
/// A manager that follows the disk waits for the next batch on a background thread. A harness runs
/// that work in line, where the wait would never end, so these tests drive the state themselves and
/// do each read and take each batch by hand, the way the waiting thread would hand it over.
fn apply(state: &mut FileManagerState, message: FileManagerMsg) {
    drop(state.update(message, Msg::Files));
}

/// Answers the reading of the folder `key` the way its background work would.
fn read(state: &mut FileManagerState, key: &str) {
    let entries = FolderEntry::read_folder(&state.path(key));
    apply(state, FileManagerMsg::Read(key.to_owned(), entries));
}

/// The keys of the folders the manager watches.
fn watched(state: &FileManagerState) -> Vec<String> {
    match &state.live {
        super::watch::Live::On(watching) => watching.watched().into_iter().map(str::to_owned).collect(),
        other => panic!("the manager is watched, not {other:?}"),
    }
}

/// Waits for changes until a batch has one that `wanted` picks, handing every batch to the
/// manager; the system may split one burst over two batches.
fn changes_until(state: &mut FileManagerState, wanted: impl Fn(&crate::storage::FolderChange) -> bool) {
    for _ in 0..5 {
        let (run, changes) = match &state.live {
            super::watch::Live::On(watching) => (watching.run(), watching.changes()),
            other => panic!("the manager is watched, not {other:?}"),
        };
        let batch = changes.next();
        let done = batch.iter().any(&wanted);
        apply(state, FileManagerMsg::Changed(run, batch));
        if done {
            return;
        }
    }
    panic!("the change never came");
}

/// A manager of `root` that follows the disk, with its root read.
fn live(root: PathBuf) -> FileManagerState {
    let mut state = FileManagerState::new(root).confined().following(true);
    drop(state.load(Msg::Files));
    read(&mut state, FileManagerState::ROOT);
    state
}

#[test]
fn a_change_from_outside_arrives_while_the_person_works() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let scratch = Scratch::new("watched");
    let root = scratch.root();
    let mut state = live(root.clone());
    apply(&mut state, FileManagerMsg::Expand("src".to_owned(), true));
    read(&mut state, "src");
    assert_eq!(watched(&state), [FileManagerState::ROOT, "src"], "the folders on screen are the watched ones");

    fs::write(root.join("src/late.rs"), "").expect("a file made by another program");
    changes_until(&mut state, |change| change.kind == crate::storage::FolderChangeKind::Created);
    assert!(state.is_loading("src"), "the folder it appeared in is read again, and nothing else");
    assert!(!state.is_loading(FileManagerState::ROOT));
    read(&mut state, "src");
    let names: Vec<&str> = state.children("src").expect("read").iter().map(|entry| entry.name.as_str()).collect();
    assert!(names.contains(&"late.rs"), "{names:?}");

    // While a name is being typed the change still arrives; the dialog keeps what was typed.
    apply(&mut state, FileManagerMsg::NewFile("src".to_owned()));
    apply(&mut state, FileManagerMsg::Name("mine.rs".to_owned()));
    fs::write(root.join("src/later.rs"), "").expect("and another");
    changes_until(&mut state, |change| change.name.as_deref() == Some(std::ffi::OsStr::new("later.rs")));
    assert_eq!(state.naming().map(|naming| naming.value.clone()), Some("mine.rs".to_owned()));

    // A change of content alone reads nothing: the rows are names, not what is in the files.
    read(&mut state, "src");
    fs::write(root.join("README.md"), "changed\n").expect("a file written again");
    changes_until(&mut state, |change| change.kind == crate::storage::FolderChangeKind::Modified);
    assert!(!state.is_loading(FileManagerState::ROOT), "nothing is read for a change of content");

    // A batch of a watch that was let go is ignored.
    let run = match &state.live {
        super::watch::Live::On(watching) => watching.run(),
        other => panic!("watched, not {other:?}"),
    };
    let late = vec![crate::storage::FolderChange {
        folder: root.clone(),
        name: Some("ghost.rs".into()),
        kind: crate::storage::FolderChangeKind::Created,
    }];
    apply(&mut state, FileManagerMsg::Changed(run + 1, late));
    assert!(!state.is_loading(FileManagerState::ROOT), "an earlier watch's batch reads nothing");

    apply(&mut state, FileManagerMsg::Expand("src".to_owned(), false));
    assert_eq!(watched(&state), [FileManagerState::ROOT], "a folder that left the screen is let go");
}

#[test]
fn a_manager_that_does_not_follow_the_disk_watches_nothing() {
    let scratch = Scratch::new("unwatched");
    let mut state = FileManagerState::new(scratch.root());
    assert!(!state.follows_changes());
    drop(state.load(Msg::Files));
    assert!(matches!(state.live, super::watch::Live::Off), "{:?}", state.live);
}

/// The line of the screen the text `text` is on, and the column it starts at.
fn row_of(h: &Harness<Demo>, text: &str) -> (String, u16) {
    let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` is on screen:\n{}", h.screen()));
    let line = h.screen().lines().nth(usize::try_from(y).expect("a row")).expect("the line").to_owned();
    (line, u16::try_from(x).expect("a column"))
}

/// The colour the text `text` is drawn in.
fn colour_of(h: &Harness<Demo>, text: &str) -> Option<crate::color::Rgb> {
    let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` is on screen:\n{}", h.screen()));
    h.fg(u16::try_from(x).expect("a column"), u16::try_from(y).expect("a row"))
}

#[test]
fn an_entry_the_application_marks_takes_its_sign_and_its_tone() {
    let scratch = Scratch::new("marked");
    let mut h = harness(&scratch);
    let (plain_line, _) = row_of(&h, "README.md");
    assert!(plain_line.contains('\u{25aa}'), "an unmarked file has the file icon:\n{plain_line}");
    let plain_name = colour_of(&h, "README.md");

    h.send(Msg::Mark("README.md".to_owned(), RowMark::new().sign("warning", "warning").faint(true)));
    h.render();
    let (marked_line, _) = row_of(&h, "README.md");
    assert!(marked_line.contains('\u{25b2}'), "the warning sign of the icon set:\n{marked_line}");
    assert!(!marked_line.contains('\u{25aa}'), "the sign takes the file icon's place:\n{marked_line}");
    assert_ne!(colour_of(&h, "README.md"), plain_name, "the name itself is faint:\n{}", h.screen());

    // The sign is in the warning tone, which is not the tone of the name beside it.
    let (_, y) = h.find("README.md").expect("the row");
    let y = u16::try_from(y).expect("a row");
    let sign_x = u16::try_from(marked_line.chars().position(|c| c == '\u{25b2}').expect("the sign")).expect("a column");
    assert_ne!(h.fg(sign_x, y), colour_of(&h, "README.md"), "the sign carries the tone, not the name");

    // A mark that says nothing leaves the row exactly as it was: no mark is the default.
    h.send(Msg::Mark("src".to_owned(), RowMark::new())).render();
    let (folder_line, _) = row_of(&h, "src");
    assert!(folder_line.contains('\u{25a0}'), "a folder with an empty mark keeps its folder icon:\n{folder_line}");
}

#[test]
fn a_mark_cannot_make_a_cut_or_disabled_row_louder() {
    let scratch = Scratch::new("mark-faint");
    let mut h = harness(&scratch);
    h.send(Msg::Mark("README.md".to_owned(), RowMark::new().sign("warning", "warning").faint(false)));
    h.render();
    let loud = colour_of(&h, "README.md");
    h.send(Msg::Files(FileManagerMsg::Cut("README.md".to_owned()))).render();
    assert_ne!(colour_of(&h, "README.md"), loud, "a cut entry stays faint whatever the mark says:\n{}", h.screen());
    h.send(Msg::Files(FileManagerMsg::DropCut)).send(Msg::Disable).render();
    assert_ne!(colour_of(&h, "README.md"), loud, "and so does a disabled manager's rows:\n{}", h.screen());
}

#[test]
fn a_marks_tone_never_comes_without_a_sign() {
    // The mark is built so the two cannot be parted: `sign` takes both, and nothing else sets a
    // tone. The constitution's rule is a rule about the type, not about the caller's care.
    let mark = RowMark::new().sign("warning", "warning");
    assert_eq!((mark.icon(), mark.tone()), (Some("warning"), Some("warning")));
    let quiet = RowMark::new().faint(true);
    assert_eq!((quiet.icon(), quiet.tone()), (None, None), "faintness alone says nothing in colour");
    assert!(RowMark::new().is_empty() && !quiet.is_empty());
}

#[test]
fn an_entry_is_copied_beside_being_moved_and_the_first_one_stays() {
    let scratch = Scratch::new("copy");
    let root = scratch.root();
    assert_eq!(copy_into(&root, "README.md", "src", true), Ok(FileChange::Copied("src/README.md".to_owned())));
    assert!(root.join("README.md").is_file(), "what was copied stays where it was");
    assert_eq!(fs::read_to_string(root.join("src/README.md")).expect("the copy"), "hello\n");

    // A folder is copied with everything in it, and what is copied keeps its mode.
    fs::create_dir_all(root.join("src/deep")).expect("a folder");
    fs::write(root.join("src/deep/run.sh"), "#!/bin/sh\n").expect("a file");
    fs::set_permissions(root.join("src/deep/run.sh"), fs::Permissions::from_mode(0o755)).expect("a mode");
    assert_eq!(copy_into(&root, "src", "", true), Err(FileError::Taken("src".to_owned())), "the root has it already");
    fs::create_dir(root.join("copies")).expect("somewhere to put it");
    assert_eq!(copy_into(&root, "src", "copies", true), Ok(FileChange::Copied("copies/src".to_owned())));
    let copied = root.join("copies/src/deep/run.sh");
    assert!(copied.is_file(), "the whole tree came along");
    assert_eq!(fs::metadata(&copied).expect("the copy").permissions().mode() & 0o777, 0o755, "and kept its mode");

    // A folder cannot be copied into itself; nothing already there is ever overwritten.
    assert_eq!(copy_into(&root, "src", "src/deep", true), Err(FileError::IntoItself));
    assert_eq!(copy_into(&root, "README.md", "src", true), Err(FileError::Taken("README.md".to_owned())));
}

#[test]
fn copying_and_moving_follow_a_link_only_when_the_manager_is_not_confined() {
    let scratch = Scratch::new("copy-link");
    let root = scratch.root();
    let away = scratch.0.join("away");
    fs::create_dir_all(&away).expect("a folder outside");
    std::os::unix::fs::symlink(&away, root.join("door")).expect("a link out of the root");

    // Confined: the link is a wall for a copy exactly as it is for a move.
    assert_eq!(copy_into(&root, "README.md", "door", true), Err(FileError::Outside));
    assert_eq!(move_into(&root, "README.md", "door", true), Err(FileError::Outside));
    assert!(!away.join("README.md").exists(), "nothing reached past the link");

    // Unconfined: the link is a folder like any other, so both go through it.
    assert_eq!(copy_into(&root, "README.md", "door", false), Ok(FileChange::Copied("door/README.md".to_owned())));
    assert!(
        away.join("README.md").is_file() && root.join("README.md").is_file(),
        "the copy is there and so is the first"
    );
    fs::remove_file(away.join("README.md")).expect("clear it again");
    assert_eq!(
        move_into(&root, "README.md", "door", false),
        Ok(FileChange::Moved("README.md".to_owned(), "door/README.md".to_owned()))
    );
    assert!(away.join("README.md").is_file() && !root.join("README.md").exists(), "a move leaves nothing behind");

    // The link itself is copied as a link, so what it points at is not duplicated.
    fs::create_dir(root.join("copies")).expect("somewhere to put it");
    assert_eq!(copy_into(&root, "door", "copies", false), Ok(FileChange::Copied("copies/door".to_owned())));
    let meta = fs::symlink_metadata(root.join("copies/door")).expect("the copy");
    assert!(meta.file_type().is_symlink(), "a link is copied as a link");
}

#[test]
fn an_entry_goes_to_a_trash_on_the_same_file_system_and_can_be_found_there_again() {
    let scratch = Scratch::new("trash");
    let root = scratch.root();
    // A trash of this test's own, beside the root and so on the same file system as it. The
    // person's own trash is never touched.
    let trash = scratch.0.join("Trash");

    assert_eq!(to_trash(&trash, &root, "README.md", true), Ok(FileChange::Trashed("README.md".to_owned())));
    assert!(!root.join("README.md").exists(), "it left the folder");
    assert_eq!(fs::read_to_string(trash.join("files/README.md")).expect("in the trash"), "hello\n");
    let note = fs::read_to_string(trash.join("info/README.md.trashinfo")).expect("its note");
    assert!(note.starts_with("[Trash Info]\n"), "{note}");
    assert!(note.contains(&format!("Path={}", root.join("README.md").display())), "{note}");
    assert!(note.contains("\nDeletionDate=") && note.contains('T'), "{note}");

    // A second entry of the same name takes the next name the specification allows.
    fs::write(root.join("README.md"), "again\n").expect("another of the same name");
    assert_eq!(to_trash(&trash, &root, "README.md", true), Ok(FileChange::Trashed("README.md".to_owned())));
    assert_eq!(fs::read_to_string(trash.join("files/README.md.1")).expect("the second"), "again\n");
    assert!(trash.join("info/README.md.1.trashinfo").is_file(), "with a note of its own");

    // A name with characters a URL cannot hold is escaped in the note, not left as it is.
    assert_eq!(to_trash(&trash, &root, "it's here.txt", true), Err(FileError::Missing), "it is not there at all");
    fs::write(root.join("it's here.txt"), "").expect("a name with room for trouble");
    assert!(to_trash(&trash, &root, "it's here.txt", true).is_ok());
    let note = fs::read_to_string(trash.join("info/it's here.txt.trashinfo")).expect("its note");
    assert!(note.contains("it%27s%20here.txt"), "{note}");
}

#[test]
fn an_entry_the_trash_cannot_take_is_said_so_rather_than_deleted_quietly() {
    let scratch = Scratch::new("no-trash");
    let root = scratch.root();
    // A trash the entry cannot be renamed into: the path is a file, so `files` cannot be made.
    let blocked = scratch.0.join("blocked-trash");
    fs::write(&blocked, "not a folder\n").expect("a file where a trash folder would be");
    assert_eq!(to_trash(&blocked, &root, "README.md", true), Err(FileError::NoTrash));
    assert!(root.join("README.md").is_file(), "nothing was deleted behind the person's back");
}

#[test]
fn a_manager_with_a_trash_offers_it_instead_of_deleting_and_asks_nothing() {
    let scratch = Scratch::new("trash-menu");
    let trash = scratch.0.join("Trash");
    let mut h = Harness::new(Demo::trashing(scratch.root(), trash.clone()), SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    assert!(state(&h).is_trashing());

    right_click(&mut h, "README.md");
    let menu = h.screen();
    assert!(menu.contains("Move to the trash"), "{menu}");
    assert!(!menu.contains("Delete"), "the trash takes the place of deleting for good:\n{menu}");
    h.click_text("Move to the trash").advance(MOMENT);
    // Nothing is asked: the trash can be looked in again.
    assert!(!h.screen().contains("cannot be undone"), "{}", h.screen());
    assert!(trash.join("files/README.md").is_file(), "{}", h.screen());
    assert!(!h.screen().contains("README.md"), "the row is gone:\n{}", h.screen());
}

#[test]
fn a_manager_whose_trash_cannot_take_an_entry_asks_about_deleting_it_for_good() {
    let scratch = Scratch::new("trash-fallback");
    let blocked = scratch.0.join("blocked-trash");
    fs::write(&blocked, "not a folder\n").expect("a file where a trash folder would be");
    let mut h = Harness::new(Demo::trashing(scratch.root(), blocked), SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h.send(Msg::Files(FileManagerMsg::Trash("README.md".to_owned()))).advance(MOMENT);
    let asked = h.screen();
    assert!(asked.contains("Delete README.md for good?"), "{asked}");
    assert!(asked.contains("no trash") && asked.contains("cannot be undone"), "{asked}");
    assert!(asked.contains("Delete for good"), "the button says what it does:\n{asked}");
    assert!(scratch.root().join("README.md").is_file(), "nothing happened before the answer");
    h.click_text("Delete for good").advance(MOMENT);
    assert!(!scratch.root().join("README.md").exists(), "{}", h.screen());
}

#[test]
fn hidden_entries_are_shown_only_when_the_application_asks_and_still_count_as_names() {
    let scratch = Scratch::new("hidden");
    let root = scratch.root();
    fs::write(root.join(".env"), "SECRET=1\n").expect("a hidden file");
    fs::create_dir(root.join(".git")).expect("a hidden folder");
    let mut h = harness(&scratch);
    let text = h.screen();
    assert!(!text.contains(".env") && !text.contains(".git"), "hidden entries are hidden by default:\n{text}");
    assert!(!state(&h).shows_hidden());
    // They were read all the same, so showing them goes nowhere near the disk.
    assert!(state(&h).children("").expect("read").iter().any(|entry| entry.name == ".env"));
    assert!(state(&h).shown_children("").expect("read").iter().all(|entry| !entry.is_hidden()));

    h.send(Msg::Files(FileManagerMsg::ShowHidden(true))).render();
    let text = h.screen();
    assert!(text.contains(".env") && text.contains(".git"), "{text}");
    h.send(Msg::Files(FileManagerMsg::ShowHidden(false))).render();
    assert!(!h.screen().contains(".env"), "{}", h.screen());

    // A new entry cannot take a hidden entry's name, shown or not.
    h.send(Msg::Files(FileManagerMsg::NewFile(String::new())));
    h.send(Msg::Files(FileManagerMsg::Name(".env".to_owned())));
    assert_eq!(state(&h).naming_problem(), Some(NameProblem::Taken), "a hidden name is a name that is taken");

    // A folder that holds nothing but hidden entries opens and shows no rows, rather than showing
    // them because there is nothing else to show.
    let only_hidden = root.join("quiet");
    fs::create_dir(&only_hidden).expect("a folder");
    fs::write(only_hidden.join(".keep"), "").expect("one hidden entry");
    h.send(Msg::Files(FileManagerMsg::CloseNaming));
    h.send(Msg::Files(FileManagerMsg::Refresh)).advance(MOMENT);
    h.send(Msg::Files(FileManagerMsg::Expand("quiet".to_owned(), true))).advance(MOMENT);
    assert!(h.screen().contains("quiet") && !h.screen().contains(".keep"), "{}", h.screen());
}

#[test]
fn a_refusal_the_system_gave_is_said_in_plain_words() {
    let scratch = Scratch::new("plain");
    let root = scratch.root();
    let locked = root.join("locked");
    fs::create_dir_all(locked.join("inside")).expect("a folder to lock");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("take away every right");
    if fs::read_dir(&locked).is_ok() {
        // Running as a user the permissions do not hold back, root among them.
        return;
    }
    // Reading is about seeing, changing is about changing, and neither hands the system's own words
    // on: no "os error 13" ever reaches the person.
    assert_eq!(FolderEntry::list(&locked), Err(FileError::NotReadable), "reading is about seeing");
    assert_eq!(create_file(&root, "locked", "x", true), Err(FileError::Denied), "changing is about changing");
    assert_eq!(delete(&root, "locked/inside", true), Err(FileError::Denied));
    assert_eq!(copy_into(&root, "locked/inside", "src", true), Err(FileError::Denied));
    // An entry another program took away while the rows still showed it says so plainly.
    assert_eq!(delete(&root, "gone.txt", true), Err(FileError::Missing));

    // The manager puts those words on the screen; no "os error 13" ever reaches the person.
    let mut h = harness(&scratch);
    h.send(Msg::Files(FileManagerMsg::DeleteConfirmed(vec!["gone.txt".to_owned()]))).advance(MOMENT);
    let screen = h.screen();
    assert!(screen.contains("This is not there any more."), "{screen}");
    assert!(!screen.contains("os error"), "{screen}");
    h.send(Msg::Files(FileManagerMsg::Expand("locked".to_owned(), true))).advance(MOMENT);

    // A root nobody may look into says so in the same words.
    let mut own = Harness::new(Demo::new(locked.clone()), SIZE.0, SIZE.1);
    own.set_reduced_motion(true).render();
    let screen = own.screen();
    assert!(screen.contains("You may not see what is in this folder."), "{screen}");
    assert!(!screen.contains("os error"), "{screen}");
}

/// A file of `size` bytes at `path`, big enough that a copy of it can be caught in the middle.
fn big_file(path: &std::path::Path, size: usize) {
    fs::write(path, vec![b'x'; size]).expect("a big file");
}

#[test]
fn a_big_copy_runs_in_the_background_with_its_progress_and_can_be_stopped() {
    let scratch = Scratch::new("copy-progress");
    let root = scratch.root();
    fs::create_dir(root.join("to")).expect("somewhere to copy to");
    // Big enough to be copied in many blocks, so progress is reported more than once.
    big_file(&root.join("big.bin"), 2 * 1024 * 1024);
    let mut h = harness(&scratch);

    h.send(Msg::Files(FileManagerMsg::Copy("big.bin".to_owned())));
    assert_eq!(state(&h).copied(), ["big.bin"], "a copy waits, and nothing about it is faint");
    assert!(!state(&h).is_cut("big.bin"));
    h.send(Msg::Files(FileManagerMsg::Paste("to".to_owned()))).advance(MOMENT);
    let text = h.screen();
    assert!(root.join("to/big.bin").is_file(), "{text}");
    assert!(state(&h).work().is_none(), "the row goes away when the work is over:\n{text}");
    assert!(!text.contains("Stop"), "{text}");
}

#[test]
fn a_running_copy_shows_how_far_it_has_come_and_offers_to_stop() {
    let scratch = Scratch::new("copy-row");
    let root = scratch.root();
    fs::create_dir(root.join("to")).expect("somewhere to copy to");
    let mut demo = Demo::new(root.clone());
    demo.driven = true;
    let mut h = Harness::new(demo, SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    assert!(!h.screen().contains("Stop"), "nothing is there while nothing runs:\n{}", h.screen());

    h.send(Msg::Files(FileManagerMsg::Copy("README.md".to_owned())));
    h.send(Msg::Files(FileManagerMsg::Paste("to".to_owned())));
    let work = state(&h).work().expect("a long operation started");
    let (id, entries) = (work.id(), work.entries());
    assert_eq!(entries, 1);
    assert_eq!(work.done(), 0.0, "nothing is claimed before the first block");

    let progress = TaskEvent::Progress { id, fraction: Some(0.4), note: Some("README.md".to_owned()) };
    h.send(Msg::Files(FileManagerMsg::Work(progress)));
    let text = h.screen();
    assert!(text.contains("Copying 1 entry"), "{text}");
    assert!(text.contains("40%"), "the share done is said in numbers as well as in colour:\n{text}");
    assert!(text.contains("Stop"), "{text}");
    assert!(text.contains("src"), "the rows stay readable while it runs:\n{text}");
    for mark in ['[', ']', '|', '\u{2502}', '\u{250c}', '\u{2500}'] {
        assert!(!text.contains(mark), "the row draws `{mark}`:\n{text}");
    }

    // A batch of an earlier operation cannot move this one's bar.
    let other = TaskEvent::Progress {
        id: crate::runtime::Task::new("other", |_| Ok(Msg::Note(String::new()))).id(),
        fraction: Some(0.9),
        note: None,
    };
    h.send(Msg::Files(FileManagerMsg::Work(other)));
    assert_eq!(state(&h).work().map(FileWork::done), Some(0.4), "{}", h.screen());

    // Stopping ends the operation, and the row goes away with it.
    h.click_text("Stop").advance(MOMENT);
    h.send(Msg::Files(FileManagerMsg::Work(TaskEvent::Finished { id, outcome: TaskOutcome::Cancelled })));
    assert!(state(&h).work().is_none(), "{}", h.screen());
    assert!(!h.screen().contains("Stop"), "{}", h.screen());
}

#[test]
fn a_copy_stopped_in_the_middle_takes_the_half_file_away_again() {
    let scratch = Scratch::new("copy-half");
    let root = scratch.root();
    fs::create_dir(root.join("to")).expect("somewhere to copy to");
    big_file(&root.join("big.bin"), 4 * 1024 * 1024);
    // The copy itself is asked to stop after the first block, the way a cancelled task asks it.
    let stopped = std::sync::atomic::AtomicUsize::new(0);
    let looks = || {
        // The first look says no, so one block is written; every look after it says stop.
        stopped.fetch_add(1, std::sync::atomic::Ordering::Relaxed) > 0
    };
    let watch = super::ops::Watch { progress: &|_, _| {}, stopped: &looks };
    let outcome = super::ops::copy_watched(&root, "big.bin", "to", true, &watch);
    assert_eq!(outcome, Err(FileError::Stopped));
    assert!(!root.join("to/big.bin").exists(), "half a file is worse than no file");
    assert!(root.join("big.bin").is_file(), "and what was being copied is untouched");
    assert_eq!(FileError::Stopped, FileError::Stopped);
}

/// Counts how many entries the system was asked about: every `Detail` that reaches the state is
/// one call per key, so the test can hold the manager to the page rule.
fn detailed(state: &mut FileManagerState, keys: Vec<String>) -> usize {
    let before = keys.iter().filter(|key| !state.has_details(key)).count();
    drop(state.detail(keys, Msg::Files));
    before
}

/// Answers a detail request the way its background work would.
fn read_details(state: &mut FileManagerState, keys: &[String]) {
    let read = keys.iter().map(|key| (key.clone(), FileDetails::read(&state.path(key)))).collect();
    apply(state, FileManagerMsg::Detailed(read));
}

#[test]
fn details_are_read_for_the_entries_asked_for_and_kept() {
    let scratch = Scratch::new("details");
    let mut state = FileManagerState::new(scratch.root()).confined();
    read(&mut state, FileManagerState::ROOT);

    assert_eq!(state.details("README.md"), None, "nothing is known before it is asked for");
    assert_eq!(detailed(&mut state, vec!["README.md".to_owned()]), 1);
    assert!(state.has_details("README.md"), "the ask is remembered while it is on its way");
    read_details(&mut state, &["README.md".to_owned()]);

    let details = state.details("README.md").expect("asked for").expect("the file is there");
    assert_eq!(details.size, fs::metadata(scratch.root().join("README.md")).expect("the file").len());
    assert!(details.modified.is_some(), "the system says when it changed");
    assert!(!details.modified_text().is_empty());
    #[cfg(unix)]
    assert!(details.permissions_text(false).starts_with('-'), "{}", details.permissions_text(false));

    // The same ask again reads nothing: what is known is not read twice.
    assert_eq!(detailed(&mut state, vec!["README.md".to_owned()]), 0);
}

#[test]
fn a_tree_row_reads_no_details_at_all() {
    let scratch = Scratch::new("details-tree");
    let mut h = harness(&scratch);
    h.press("tab").press("down").press("down").render();
    assert!(state(&h).selected().is_some(), "the cursor moved through the rows");
    for key in ["README.md", "src", "src/main.rs"] {
        assert!(!state(&h).has_details(key), "the tree asked about {key}");
    }
}

#[test]
fn a_page_of_details_is_read_around_the_cursor_and_never_the_whole_folder() {
    let scratch = Scratch::new("details-page");
    fill(&scratch.0.join("many"), MANY);
    let mut state = FileManagerState::new(scratch.0.join("many"));
    read(&mut state, FileManagerState::ROOT);
    assert_eq!(state.children(FileManagerState::ROOT).map(<[FolderEntry]>::len), Some(MANY));

    let page = |state: &FileManagerState| {
        (0..MANY).filter(|index| state.has_details(&format!("entry-{index:05}.txt"))).collect::<Vec<_>>()
    };
    drop(state.detail_page(FileManagerState::ROOT, Msg::Files));
    let asked = page(&state);
    assert_eq!(asked.len(), 200, "one page, never the {MANY} entries of the folder");
    assert_eq!(asked.first().copied(), Some(0), "with no cursor the page starts at the top");

    // The cursor decides where the page sits, and the page around it reaches both ways.
    state.select("entry-05000.txt");
    drop(state.detail_page(FileManagerState::ROOT, Msg::Files));
    let asked = page(&state);
    assert_eq!(asked.len(), 400, "a second page and no more");
    assert!(asked.contains(&4900) && asked.contains(&5099), "the page sits around the cursor: {asked:?}");
    assert!(!asked.contains(&4899) && !asked.contains(&5100), "and stops there");
}

#[test]
fn what_is_known_about_an_entry_is_let_go_when_it_moves_or_goes_away() {
    let scratch = Scratch::new("details-forget");
    let mut state = FileManagerState::new(scratch.root()).confined();
    read(&mut state, FileManagerState::ROOT);
    let keys = vec!["README.md".to_owned(), "src".to_owned()];
    drop(state.detail(keys.clone(), Msg::Files));
    read_details(&mut state, &keys);
    assert!(state.details("README.md").is_some_and(|details| details.is_some()));

    apply(
        &mut state,
        FileManagerMsg::Done(vec![(
            "README.md".to_owned(),
            Ok(FileChange::Moved("README.md".to_owned(), "src/README.md".to_owned())),
        )]),
    );
    assert!(!state.has_details("README.md"), "the old key is gone");
    assert!(state.details("src/README.md").is_some(), "and what was known travelled with it");

    apply(&mut state, FileManagerMsg::Done(vec![("src".to_owned(), Ok(FileChange::Deleted("src".to_owned())))]));
    assert!(!state.has_details("src") && !state.has_details("src/README.md"), "a deleted folder takes them along");
}

#[test]
fn a_folder_read_again_asks_about_its_entries_afresh() {
    let scratch = Scratch::new("details-reread");
    let mut state = FileManagerState::new(scratch.root()).confined();
    read(&mut state, FileManagerState::ROOT);
    let keys = vec!["README.md".to_owned()];
    drop(state.detail(keys.clone(), Msg::Files));
    read_details(&mut state, &keys);
    let first = state.details("README.md").expect("asked for").expect("there").size;

    fs::write(scratch.root().join("README.md"), "a longer README than before\n").expect("the file");
    read(&mut state, FileManagerState::ROOT);
    assert!(!state.has_details("README.md"), "what was known of a folder read again is let go");
    drop(state.detail(keys.clone(), Msg::Files));
    read_details(&mut state, &keys);
    let second = state.details("README.md").expect("asked for").expect("there").size;
    assert_ne!(first, second, "and the new size is read");
}

#[test]
fn a_size_is_said_in_the_largest_unit_that_still_means_something() {
    let mut english = crate::i18n::I18n::builtin();
    english.set_active("en");
    crate::i18n::scope(std::sync::Arc::new(english), || {
        let details = |size| FileDetails { size, modified: None, mode: None, readonly: false };
        assert_eq!(details(0).size_text(false), "0 B");
        assert_eq!(details(840).size_text(false), "840 B");
        assert_eq!(details(9_400).size_text(false), "9.4 kB");
        assert_eq!(details(12_000_000).size_text(false), "12 MB");
        assert_eq!(details(4_000_000_000).size_text(false), "4.0 GB");
        assert_eq!(details(9_000).size_text(true), "", "a folder's own size says nothing about what is in it");
    });
}

/// A manager of `scratch` drawn in `view`, its root read.
fn viewing(scratch: &Scratch, view: FileView) -> Harness<Demo> {
    let mut h = harness(scratch);
    h.send(Msg::View(view));
    h.advance(MOMENT);
    h
}

/// Every shape the manager is drawn in.
const VIEWS: [FileView; 3] = [FileView::Tree, FileView::List, FileView::Icons];

#[test]
fn a_flat_view_shows_one_folder_and_steps_into_it_and_out_of_it() {
    let scratch = Scratch::new("flat-walk");
    for view in [FileView::List, FileView::Icons] {
        let mut h = viewing(&scratch, view);
        let screen = h.screen();
        assert!(screen.contains("README.md") && screen.contains("src"), "{view:?}:\n{screen}");
        assert!(!screen.contains("main.rs"), "a flat view shows one folder, not the tree:\n{screen}");

        h.click_text("src").click_text("src").advance(MOMENT);
        assert_eq!(state(&h).folder(), "src", "{view:?}");
        let screen = h.screen();
        assert!(screen.contains("main.rs"), "{view:?}:\n{screen}");
        assert!(!screen.contains("README.md"), "the folder that was left is gone:\n{screen}");

        // The folder's own row is the way back out.
        h.click_text("src").click_text("src").advance(MOMENT);
        assert_eq!(state(&h).folder(), FileManagerState::ROOT, "{view:?}");
        assert!(h.screen().contains("README.md"), "{view:?}:\n{}", h.screen());
        assert_eq!(state(&h).selected(), Some("src"), "the cursor is on the folder that was left");
    }
}

#[test]
fn the_list_shows_the_size_the_date_and_the_permissions_of_its_rows() {
    let scratch = Scratch::new("list-details");
    let h = viewing(&scratch, FileView::List);
    let screen = h.screen();
    assert!(screen.contains("Size") && screen.contains("Changed") && screen.contains("Permissions"), "{screen}");
    assert!(screen.contains("README.md"), "{screen}");
    // The page of details was asked for and answered by the harness's own run of the work.
    let details = state(&h).details("README.md").expect("the page was asked for").expect("the file is there");
    assert!(details.size > 0);
    assert!(h.screen().contains(&details.modified_text()), "the date is on screen:\n{}", h.screen());
    #[cfg(unix)]
    assert!(h.screen().contains(&details.permissions_text(false)), "{}", h.screen());
    assert!(!state(&h).has_details(FileManagerState::ROOT), "the folder's own row says nothing about itself");
}

#[test]
fn a_reread_the_application_starts_brings_the_details_back_without_a_key() {
    let scratch = Scratch::new("reread-details");
    let mut h = viewing(&scratch, FileView::List);
    let size_of =
        |h: &Harness<Demo>| state(h).details("README.md").flatten().map(|details| format!("{} B", details.size));
    let before = size_of(&h).expect("the list read the details once");
    assert!(h.screen().contains(&before), "the size is drawn:\n{}", h.screen());
    // Another program writes the file; the application reads the folder again on its own, as it
    // does after an archive is unpacked or a program it handed the screen to comes back.
    fs::write(scratch.root().join("README.md"), "a longer text than before, so the size changes\n").expect("a write");
    h.send(Msg::Files(FileManagerMsg::Refresh));
    h.advance(MOMENT).advance(MOMENT);
    let after = size_of(&h).expect("the details came back without a key being pressed");
    assert_ne!(after, before, "they are the file's new ones");
    assert!(h.screen().contains(&after), "and the new size is drawn:\n{}", h.screen());
}

#[test]
fn the_tree_asks_for_no_details_whatever_the_list_asked_for() {
    let scratch = Scratch::new("views-details");
    let mut h = viewing(&scratch, FileView::Icons);
    h.press("tab").press("right").render();
    assert!(!state(&h).has_details("README.md"), "the icons show names, so they read nothing");
    let mut h = viewing(&scratch, FileView::Tree);
    h.press("tab").press("down").render();
    assert!(!state(&h).has_details("README.md"), "and the tree reads nothing either");
}

#[test]
fn a_page_is_the_most_the_list_ever_asks_about() {
    let scratch = Scratch::new("list-page");
    fill(&scratch.0.join("many"), MANY);
    let mut h = Harness::new(Demo::new(scratch.0.join("many")), SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h.send(Msg::View(FileView::List));
    h.advance(MOMENT);
    let asked = (0..MANY).filter(|index| state(&h).has_details(&format!("entry-{index:05}.txt"))).count();
    assert!(asked > 0, "the list asked about the rows it shows");
    assert!(asked <= 200, "a page and no more, never the {MANY} of the folder: {asked}");

    // Moving through the folder keeps a page around the cursor, and still never the whole folder.
    h.press("tab").press("end").advance(MOMENT);
    let asked = (0..MANY).filter(|index| state(&h).has_details(&format!("entry-{index:05}.txt"))).count();
    assert!(asked <= 400, "two pages at the most after moving to the far end: {asked}");
    assert!(state(&h).has_details(&format!("entry-{:05}.txt", MANY - 1)), "including the row the cursor is on");
}

#[test]
fn ten_thousand_entries_paint_the_same_cells_in_every_view() {
    let scratch = Scratch::new("views-many");
    fill(&scratch.0.join("small"), 200);
    fill(&scratch.0.join("large"), MANY);

    /// The cells with something in them, the scrollbar column left out, and the foot with it: the
    /// foot says how many entries the folder holds, which is the one thing that may differ.
    fn painted(h: &Harness<Demo>) -> usize {
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        lines[..lines.len().saturating_sub(1)]
            .iter()
            .map(|line| line.chars().take(usize::from(SIZE.0) - 2).filter(|c| !c.is_whitespace()).count())
            .sum()
    }

    for view in VIEWS {
        let open = |folder: &str| {
            let mut h = Harness::new(Demo::new(scratch.0.join(folder)), SIZE.0, SIZE.1);
            h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
            h.send(Msg::View(view));
            h.advance(MOMENT);
            h
        };
        let (mut small, mut large) = (open("small"), open("large"));
        assert!(large.screen().contains("entry-00000.txt"), "{view:?}:\n{}", large.screen());
        assert_eq!(painted(&small), painted(&large), "{view:?}: {MANY} entries paint what 200 paint");

        small.press("tab").press("end").advance(MOMENT);
        large.press("tab").press("end").advance(MOMENT);
        assert!(large.screen().contains(&format!("entry-{:05}.txt", MANY - 1)), "{view:?}:\n{}", large.screen());
        assert!(!large.screen().contains("entry-05000.txt"), "{view:?}: the middle is not drawn");
        if view == FileView::Icons {
            // The last row of a grid holds whatever is left over, so two folders of different
            // sizes may end on rows of different lengths; the work still does not grow with them.
            assert!(painted(&large) <= painted(&small), "{view:?}: at the far end too");
        } else {
            assert_eq!(painted(&small), painted(&large), "{view:?}: at the far end too");
        }
    }
}

#[test]
fn every_operation_is_reached_from_a_rows_menu_in_every_view() {
    for view in VIEWS {
        let scratch = Scratch::new(&format!("views-ops-{view:?}"));
        let mut h = viewing(&scratch, view);
        right_click(&mut h, "README.md");
        let screen = h.screen();
        for item in ["Rename", "Cut", "Copy", "Delete"] {
            assert!(screen.contains(item), "{view:?} has no {item}:\n{screen}");
        }
        h.click_text("Cut").advance(MOMENT);
        assert_eq!(state(&h).pending(), ["README.md"], "{view:?}");

        right_click(&mut h, "src");
        assert!(h.screen().contains("Paste here"), "{view:?}:\n{}", h.screen());
        h.click_text("Paste here").advance(MOMENT);
        assert!(scratch.root().join("src/README.md").is_file(), "{view:?}: {}", h.screen());
        assert!(!scratch.root().join("README.md").exists(), "{view:?}");

        // And a new entry is made from the folder's own row, which every view has.
        right_click(&mut h, "Project");
        assert!(h.screen().contains("New file"), "{view:?}:\n{}", h.screen());
        h.click_text("New file").advance(MOMENT);
        h.type_text("notes.md").press("enter").advance(MOMENT);
        assert!(scratch.root().join("notes.md").is_file(), "{view:?}: {}", h.screen());
    }
}

#[test]
fn a_menu_acts_on_the_row_it_was_opened_on_in_every_view() {
    for view in VIEWS {
        let scratch = Scratch::new(&format!("views-menu-{view:?}"));
        let mut h = viewing(&scratch, view);
        h.send(Msg::Files(FileManagerMsg::Select("src".to_owned())));
        h.advance(MOMENT);
        right_click(&mut h, "README.md");
        h.click_text("Cut").advance(MOMENT);
        assert_eq!(state(&h).pending(), ["README.md"], "{view:?}: the row that was clicked, not the cursor's");

        // The keyboard reaches the menu of the row the cursor is on, and no other.
        h.send(Msg::Files(FileManagerMsg::DropCut));
        h.send(Msg::Files(FileManagerMsg::Select("src".to_owned())));
        h.press("menu").advance(MOMENT);
        assert!(h.screen().contains("Cut"), "{view:?}:\n{}", h.screen());
        h.click_text("Cut").advance(MOMENT);
        assert_eq!(state(&h).pending(), ["src"], "{view:?}");
    }
}

#[test]
fn the_empty_loading_narrow_ascii_and_disabled_states_are_drawn_in_every_view() {
    for view in VIEWS {
        let scratch = Scratch::new(&format!("views-states-{view:?}"));
        fs::create_dir_all(scratch.root().join("hollow")).expect("an empty folder");
        let mut h = viewing(&scratch, view);

        // Empty: a manager rooted at a folder with nothing in it says so, in every view.
        let mut hollow = Harness::new(Demo::new(scratch.root().join("hollow")), SIZE.0, SIZE.1);
        hollow.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
        hollow.send(Msg::View(view));
        hollow.advance(MOMENT);
        assert!(hollow.screen().contains("empty"), "{view:?} says nothing about an empty folder:\n{}", hollow.screen());

        // ASCII: no glyph of another mode is left on screen.
        h.set_glyph_mode(GlyphMode::Ascii).render();
        assert!(h.screen().contains("README.md"), "{view:?} in ASCII:\n{}", h.screen());
        assert!(h.screen().is_ascii(), "{view:?} draws something that is not ASCII:\n{}", h.screen());
        h.set_glyph_mode(GlyphMode::Unicode).render();

        // Narrow: the rows are still readable in a fraction of the width.
        h.resize(28, SIZE.1).render();
        assert!(h.screen().contains("src"), "{view:?} narrow:\n{}", h.screen());
        h.resize(SIZE.0, SIZE.1).render();

        // Disabled: nothing answers.
        h.send(Msg::Disable).render();
        h.click_text("README.md").advance(MOMENT);
        assert!(h.app().opened.is_empty(), "{view:?} answered while disabled");
    }
}

#[test]
fn a_folder_that_cannot_be_read_says_so_in_every_view() {
    let scratch = Scratch::new("views-denied");
    let denied = scratch.root().join("denied");
    fs::create_dir_all(&denied).expect("the folder");
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o000)).expect("no permissions");
    if fs::read_dir(&denied).is_ok() {
        // Running as a user who may read anything, which a test cannot argue with.
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o755)).expect("back again");
        return;
    }
    for view in [FileView::List, FileView::Icons] {
        let mut h = Harness::new(Demo::new(denied.clone()), SIZE.0, SIZE.1);
        h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
        h.send(Msg::View(view));
        h.advance(MOMENT);
        assert!(h.screen().contains("This folder could not be read."), "{view:?}:\n{}", h.screen());
    }
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o755)).expect("back again");
}

#[test]
fn a_slow_read_spins_in_the_foot_of_a_flat_view_and_a_quick_one_never_does() {
    /// The frames of the spinner the framework draws.
    const SPINNER: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

    let scratch = Scratch::new("flat-reading");
    for view in [FileView::List, FileView::Icons] {
        let mut h = viewing(&scratch, view);
        // A read that already happened shows nothing: a quick read never flashes an indicator.
        assert!(!h.screen().contains(|c| SPINNER.contains(c)), "{view:?}:\n{}", h.screen());

        h.send(Msg::Slow);
        h.send(Msg::Files(FileManagerMsg::Enter("src".to_owned())));
        h.advance(Duration::from_millis(299));
        assert!(!h.screen().contains(|c| SPINNER.contains(c)), "{view:?} spun too soon:\n{}", h.screen());
        h.advance(Duration::from_millis(2));
        assert!(h.screen().contains(|c| SPINNER.contains(c)), "{view:?} never spun:\n{}", h.screen());
        assert!(h.screen().contains("src"), "{view:?} still shows the folder it is reading:\n{}", h.screen());
    }
}

#[test]
fn with_a_bounded_wait_a_screen_test_sees_what_another_program_made() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let scratch = Scratch::new("bounded-follow");
    let mut demo = Demo::new(scratch.root());
    demo.manager = FileManagerState::new(scratch.root()).confined().following_within(Duration::from_millis(50));
    let mut h = Harness::new(demo, SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    assert!(h.screen().contains("README.md"), "the root is read:\n{}", h.screen());
    assert!(state(&h).follows_changes());
    fs::write(scratch.root().join("NOTES.md"), "").expect("a file another program made");
    // Each step waits for the watch at most the bound, so the steps always end.
    for _ in 0..100 {
        if h.screen().contains("NOTES.md") {
            break;
        }
        h.advance(MOMENT);
    }
    assert!(h.screen().contains("NOTES.md"), "the new file came on screen by itself:\n{}", h.screen());
}
