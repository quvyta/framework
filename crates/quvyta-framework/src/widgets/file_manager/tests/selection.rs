//! The rest of a desktop file explorer's selection and dragging, driven with real keys and real
//! pointer events in all three views: Shift with the arrows, Ctrl+A and Esc, the drop target lit
//! while a drag is over it, a folder never taking itself or a folder of its own, and a drop on the
//! row of the shown folder moving the entries up into the folder above it.

use super::mouse::{click, double_click, drag_with, pointer, project, shown, spot};
use super::*;

/// Every entry of [`project`]'s root, in the order every view shows them.
const ALL: [&str; 4] = ["docs", "src", "README.md", "plan.txt"];

#[test]
fn shift_with_the_arrows_extends_the_selection_from_where_it_started_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("select-shift-{view:?}"));
        let mut h = shown(&scratch, view, None);
        // The icons stand side by side, so the next one is to the right; the rows are below.
        let (next, back) =
            if view == FileView::Icons { ("shift+right", "shift+left") } else { ("shift+down", "shift+up") };
        click(&mut h, "docs");
        h.press(next).advance(MOMENT);
        assert_eq!(state(&h).chosen(), ["docs", "src"], "{view:?}:\n{}", h.screen());
        assert_eq!(state(&h).selected(), Some("src"), "{view:?}: the cursor goes along");
        h.press(back).advance(MOMENT);
        assert_eq!(state(&h).chosen(), ["docs"], "{view:?}: back to where it started");
        h.press("shift+end").advance(MOMENT);
        assert_eq!(state(&h).chosen(), ALL, "{view:?}:\n{}", h.screen());
        assert!(h.app().opened.is_empty(), "{view:?}: selecting opens nothing");
    }
}

#[test]
fn ctrl_a_selects_every_entry_and_esc_leaves_the_one_under_the_cursor_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("select-all-{view:?}"));
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        h.press("ctrl+a").advance(MOMENT);
        // The folder's own row is where the person is, not an entry to carry away.
        assert_eq!(state(&h).chosen(), ALL, "{view:?}:\n{}", h.screen());
        assert_eq!(state(&h).selected(), Some("README.md"), "{view:?}: the cursor stays");
        h.press("esc").advance(MOMENT);
        assert_eq!(state(&h).chosen(), ["README.md"], "{view:?}:\n{}", h.screen());
        assert!(h.app().opened.is_empty(), "{view:?}");
    }
}

#[test]
fn a_folder_a_drag_is_over_is_lit_and_a_file_is_not_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("select-lit-{view:?}"));
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        let (from, docs, plan) = (spot(&h, "README.md"), spot(&h, "docs"), spot(&h, "plan.txt"));
        let cell = |(x, y): (i32, i32)| (u16::try_from(x).expect("a column"), u16::try_from(y).expect("a row"));
        let left = MouseButton::Left;
        h.events(&[
            pointer(MouseKind::Down(left), from, Modifiers::default()),
            pointer(MouseKind::Drag(left), plan, Modifiers::default()),
        ]);
        let (x, y) = cell(plan);
        let over_file = h.bg(x, y);
        h.events(&[pointer(MouseKind::Drag(left), docs, Modifiers::default())]);
        let (x, y) = cell(docs);
        assert_ne!(h.bg(x, y), over_file, "{view:?}: the folder takes the target's tone\n{}", h.screen());
        h.events(&[pointer(MouseKind::Up(left), from, Modifiers::default())]);
    }
}

#[test]
fn a_folder_dragged_onto_a_folder_inside_it_stays_where_it_is() {
    let scratch = project("select-inside");
    fs::create_dir_all(scratch.root().join("src/inner")).expect("a folder inside a folder");
    let mut h = shown(&scratch, FileView::Tree, None);
    double_click(&mut h, "src");
    h.advance(MOMENT);
    click(&mut h, "src");
    let (from, onto) = (spot(&h, "src"), spot(&h, "inner"));
    drag_with(&mut h, from, onto, Modifiers::default());
    drag_with(&mut h, from, onto, CTRL);
    let root = scratch.root();
    assert!(root.join("src/inner").is_dir() && root.join("src/main.rs").is_file(), "{}", h.screen());
    assert!(!root.join("src/inner/src").exists(), "a folder never goes into itself");
}

#[test]
fn a_drop_on_the_row_of_the_shown_folder_moves_the_entries_into_the_folder_above_in_the_flat_views() {
    for view in [FileView::List, FileView::Icons] {
        let scratch = project(&format!("select-up-{view:?}"));
        fs::write(scratch.root().join("src/lib.rs"), "\n").expect("a second file");
        let root = scratch.root();
        let mut h = shown(&scratch, view, None);
        double_click(&mut h, "src");
        h.advance(MOMENT);
        assert_eq!(state(&h).folder(), "src", "{view:?}:\n{}", h.screen());

        // The row of the shown folder is the way up, so it takes a drop the way the folder above
        // it would, and it is lit while the drag is over it.
        click(&mut h, "main.rs");
        let (from, up, other) = (spot(&h, "main.rs"), spot(&h, "src"), spot(&h, "lib.rs"));
        let cell = |(x, y): (i32, i32)| (u16::try_from(x).expect("a column"), u16::try_from(y).expect("a row"));
        let left = MouseButton::Left;
        h.events(&[
            pointer(MouseKind::Down(left), from, Modifiers::default()),
            pointer(MouseKind::Drag(left), other, Modifiers::default()),
        ]);
        let (x, y) = cell(other);
        let over_file = h.bg(x, y);
        h.events(&[pointer(MouseKind::Drag(left), up, Modifiers::default())]);
        let (x, y) = cell(up);
        assert_ne!(h.bg(x, y), over_file, "{view:?}: the way up is lit\n{}", h.screen());
        h.events(&[pointer(MouseKind::Up(left), up, Modifiers::default())]);
        h.advance(MOMENT);
        assert!(root.join("main.rs").is_file() && !root.join("src/main.rs").exists(), "{view:?}: moved up");

        click(&mut h, "lib.rs");
        let (from, up) = (spot(&h, "lib.rs"), spot(&h, "src"));
        drag_with(&mut h, from, up, CTRL);
        assert!(root.join("lib.rs").is_file() && root.join("src/lib.rs").is_file(), "{view:?}: copied up");

        // The root has nothing above it, so its row takes nothing.
        double_click(&mut h, "src");
        h.advance(MOMENT);
        assert_eq!(state(&h).folder(), FileManagerState::ROOT, "{view:?}");
        click(&mut h, "plan.txt");
        let (from, top) = (spot(&h, "plan.txt"), spot(&h, "Project"));
        drag_with(&mut h, from, top, Modifiers::default());
        assert!(root.join("plan.txt").is_file(), "{view:?}: nothing moved");
        assert!(h.app().opened.is_empty(), "{view:?}");
    }
}
