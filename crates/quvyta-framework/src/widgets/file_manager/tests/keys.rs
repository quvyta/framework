//! Ctrl+X, Ctrl+C and Ctrl+V on a file manager's rows, the way a desktop file explorer takes
//! them, driven with real keys in all three views; and the same keys still copying and pasting
//! text in a field beside the rows and in text selected with the mouse.

use super::mouse::{click, click_with, double_click, project, shown};
use super::*;
use crate::widgets::TextInput;

/// Makes `key` the folder the next paste goes into: the tree pastes into the folder the cursor
/// is on, so a click is enough; a flat view pastes into the folder it shows, so it is stepped
/// into.
fn go_to(h: &mut Harness<Demo>, view: FileView, key: &str) {
    match view {
        FileView::Tree => click(h, key),
        _ => {
            double_click(h, key);
            h.advance(MOMENT);
            assert_eq!(state(h).folder(), key, "{view:?}: stepped into `{key}`\n{}", h.screen());
        }
    }
}

#[test]
fn ctrl_c_on_the_rows_and_ctrl_v_in_another_folder_copies_the_file_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("keys-copy-{view:?}"));
        let root = scratch.root();
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        h.press("ctrl+c").advance(MOMENT);
        assert_eq!(state(&h).copied(), ["README.md"], "{view:?}: the selection waits to be copied");

        go_to(&mut h, view, "docs");
        h.press("ctrl+v").advance(MOMENT);
        assert_eq!(fs::read_to_string(root.join("docs/README.md")).ok().as_deref(), Some("hello\n"), "{view:?}");
        assert!(root.join("README.md").is_file(), "{view:?}: what was copied stays where it was");
        assert!(h.copied().is_empty(), "{view:?}: no text was copied on the way");
    }
}

#[test]
fn ctrl_x_and_ctrl_v_move_the_whole_selection_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("keys-move-{view:?}"));
        let root = scratch.root();
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        click_with(&mut h, "plan.txt", CTRL);
        h.press("ctrl+x").advance(MOMENT);
        assert_eq!(state(&h).cut(), ["README.md", "plan.txt"], "{view:?}: both wait to be moved");

        go_to(&mut h, view, "docs");
        h.press("ctrl+v").advance(MOMENT);
        assert!(root.join("docs/README.md").is_file() && root.join("docs/plan.txt").is_file(), "{view:?}");
        assert!(!root.join("README.md").exists() && !root.join("plan.txt").exists(), "{view:?}: moved, not copied");
        assert!(state(&h).pending().is_empty(), "{view:?}: the cut is used up");
    }
}

#[test]
fn a_taken_name_is_refused_on_a_keyboard_paste_and_nothing_is_overwritten() {
    for view in VIEWS {
        let scratch = project(&format!("keys-taken-{view:?}"));
        let root = scratch.root();
        fs::write(root.join("docs/README.md"), "already here\n").expect("a file of the same name");
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        h.press("ctrl+c").advance(MOMENT);
        go_to(&mut h, view, "docs");
        h.press("ctrl+v").advance(MOMENT);
        assert_eq!(fs::read_to_string(root.join("docs/README.md")).ok().as_deref(), Some("already here\n"), "{view:?}");
        assert!(h.screen().contains("already has something called"), "{view:?}: said why\n{}", h.screen());
    }
}

/// A screen with a text field and a line of selectable text above a file manager, as a Files
/// window with a path bar has.
struct Beside {
    manager: FileManagerState,
    note: String,
    view: FileView,
}

#[derive(Clone)]
enum Said {
    Files(FileManagerMsg),
    Note(String),
}

impl App for Beside {
    type Msg = Said;

    fn init(&mut self) -> Command<Said> {
        self.manager.load(Said::Files)
    }

    fn update(&mut self, msg: Said) -> Command<Said> {
        match msg {
            Said::Files(message) => return self.manager.update(message, Said::Files),
            Said::Note(note) => self.note = note,
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Said>) {
        ui.add(TextInput::new(self.note.clone()).on_change(Said::Note)).id("note").fill_width();
        ui.add(Text::new("words to take along")).selectable(true);
        FileManager::new(&self.manager, Said::Files).view(self.view).id("files").show(ui).fill();
    }
}

fn beside(scratch: &Scratch, view: FileView) -> Harness<Beside> {
    let app = Beside { manager: FileManagerState::new(scratch.root()).confined(), note: String::new(), view };
    let mut h = Harness::new(app, SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h.advance(MOMENT);
    h
}

/// A click on `text` in the screen of [`Beside`], then a pause.
fn click_on(h: &mut Harness<Beside>, text: &str) {
    let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` is on screen:\n{}", h.screen()));
    h.click(x, y).advance(MOMENT);
}

#[test]
fn a_focused_text_field_beside_the_rows_keeps_copying_and_pasting_text_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("keys-field-{view:?}"));
        let root = scratch.root();
        let mut h = beside(&scratch, view);
        click_on(&mut h, "README.md");
        assert!(!h.is_focused("note"), "{view:?}: a click takes the focus to the rows");

        h.press("shift+tab").advance(MOMENT);
        assert!(h.is_focused("note"), "{view:?}");
        h.type_text("hello").press("ctrl+a").press("ctrl+c").advance(MOMENT);
        assert_eq!(h.copied().last().map(String::as_str), Some("hello"), "{view:?}: the field copied its text");
        assert!(h.app().manager.pending().is_empty(), "{view:?}: and no file waits to be pasted");

        h.press("ctrl+x").advance(MOMENT);
        assert_eq!(h.app().note, "", "{view:?}: Ctrl+X cut the text");
        assert!(h.app().manager.pending().is_empty(), "{view:?}");

        h.press("ctrl+v").advance(MOMENT);
        assert_eq!(h.app().note, "hello", "{view:?}: Ctrl+V pasted the text into the field");
        assert!(!root.join("README.md").is_dir() && root.join("README.md").is_file(), "{view:?}");
        let entries = fs::read_dir(&root).expect("the root").count();
        assert_eq!(entries, 4, "{view:?}: no file was made anywhere");
    }
}

#[test]
fn text_selected_with_the_mouse_is_what_ctrl_c_copies_while_the_rows_have_focus_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("keys-selection-{view:?}"));
        let mut h = beside(&scratch, view);
        click_on(&mut h, "README.md");
        assert!(!h.is_focused("note"), "{view:?}: a click takes the focus to the rows");
        let (x, y) = h.find("words").expect("the text");
        h.drag((x, y), (x + 4, y));
        assert!(!h.is_focused("note"), "{view:?}: selecting text leaves the focus on the rows");
        h.press("ctrl+c").advance(MOMENT);
        assert_eq!(h.copied().last().map(String::as_str), Some("words"), "{view:?}: the selection was copied");
        assert!(h.app().manager.pending().is_empty(), "{view:?}: the file was not");
    }
}

#[test]
fn ctrl_c_and_ctrl_v_with_no_row_focused_do_not_touch_files_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("keys-unfocused-{view:?}"));
        let root = scratch.root();
        let mut h = beside(&scratch, view);
        click_on(&mut h, "README.md");
        h.press("ctrl+c").advance(MOMENT);
        assert_eq!(h.app().manager.copied(), ["README.md"], "{view:?}: copied from the rows");
        click_on(&mut h, "plan.txt");

        // The rows keep their selection while the field has the keys.
        h.press("shift+tab").advance(MOMENT);
        assert!(h.is_focused("note"), "{view:?}");
        h.press("ctrl+c").press("ctrl+x").advance(MOMENT);
        assert_eq!(h.app().manager.copied(), ["README.md"], "{view:?}: the field's keys took nothing from the rows");
        h.press("ctrl+v").advance(MOMENT);
        let entries = fs::read_dir(&root).expect("the root").count();
        assert_eq!(entries, 4, "{view:?}: nothing was pasted into a folder\n{}", h.screen());
        assert!(!root.join("docs/README.md").exists(), "{view:?}");
    }
}

#[test]
fn ctrl_v_while_text_is_selected_with_the_mouse_leaves_the_files_alone_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("keys-selection-paste-{view:?}"));
        let root = scratch.root();
        let mut h = beside(&scratch, view);
        click_on(&mut h, "README.md");
        h.press("ctrl+c").advance(MOMENT);
        let (x, y) = h.find("docs").expect("the folder");
        h.click(x, y);
        if view != FileView::Tree {
            h.click(x, y);
        }
        h.advance(MOMENT);

        let (x, y) = h.find("words").expect("the text");
        h.drag((x, y), (x + 4, y));
        h.press("ctrl+v").advance(MOMENT);
        assert!(!root.join("docs/README.md").exists(), "{view:?}: the selected text had the key");

        h.press("ctrl+v").advance(MOMENT);
        assert!(
            root.join("docs/README.md").is_file(),
            "{view:?}: with the text let go the rows have it\n{}",
            h.screen()
        );
    }
}
