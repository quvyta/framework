//! The mouse of a file manager, the way a desktop file explorer's works, driven with real pointer
//! events in all three views: a click selects, a double click opens, Ctrl and Shift select
//! several, a box drawn on the free space selects what it covers, and dragging the selection onto
//! a folder moves it there or, with Ctrl, copies it.

use super::*;

/// Shift held, as a Shift+click holds it.
const SHIFT: Modifiers = Modifiers { ctrl: false, alt: false, shift: true };

/// A row of the screen below every entry of these small folders, inside the manager in every
/// view: the free space a box starts from.
const FREE_Y: i32 = 15;

/// A pointer event at `at` with `mods` held.
fn pointer(kind: MouseKind, (x, y): (i32, i32), mods: Modifiers) -> Event {
    Event::Mouse(MouseEvent { kind, x, y, mods })
}

/// Where `text` is on screen.
pub(super) fn spot(h: &Harness<Demo>, text: &str) -> (i32, i32) {
    h.find(text).unwrap_or_else(|| panic!("`{text}` is on screen:\n{}", h.screen()))
}

/// A left press and release on `text` with `mods` held, then a pause long enough that the next
/// click is a click of its own.
pub(super) fn click_with(h: &mut Harness<Demo>, text: &str, mods: Modifiers) {
    let at = spot(h, text);
    let left = MouseButton::Left;
    h.events(&[pointer(MouseKind::Down(left), at, mods), pointer(MouseKind::Up(left), at, mods)]);
    h.advance(MOMENT);
}

/// A plain click on `text`, then a pause.
pub(super) fn click(h: &mut Harness<Demo>, text: &str) {
    click_with(h, text, Modifiers::default());
}

/// Two clicks on `text` with no time between them, as a person's double click arrives.
pub(super) fn double_click(h: &mut Harness<Demo>, text: &str) {
    let (x, y) = spot(h, text);
    h.click(x, y).click(x, y);
}

/// A press at `from`, a drag to `to` and a release there with `mods` held at the release.
fn drag_with(h: &mut Harness<Demo>, from: (i32, i32), to: (i32, i32), mods: Modifiers) {
    let left = MouseButton::Left;
    h.events(&[
        pointer(MouseKind::Down(left), from, Modifiers::default()),
        pointer(MouseKind::Drag(left), to, Modifiers::default()),
        pointer(MouseKind::Up(left), to, mods),
    ]);
    h.advance(MOMENT);
}

/// The demo of `scratch` drawn in `view`, opening entries with `open_on` when it is given.
pub(super) fn shown(scratch: &Scratch, view: FileView, open_on: Option<Click>) -> Harness<Demo> {
    let mut demo = Demo::new(scratch.root());
    demo.view = view;
    demo.open_on = open_on;
    let mut h = Harness::new(demo, SIZE.0, SIZE.1);
    h.set_glyph_mode(GlyphMode::Unicode).set_reduced_motion(true).render();
    h.advance(MOMENT);
    h
}

/// A scratch folder whose project also holds a `docs` folder and a `plan.txt`, so every view
/// shows `docs`, `src`, `README.md` and `plan.txt` in that order.
pub(super) fn project(name: &str) -> Scratch {
    let scratch = Scratch::new(name);
    fs::create_dir_all(scratch.root().join("docs")).expect("a folder to drop into");
    fs::write(scratch.root().join("plan.txt"), "plan\n").expect("a second file");
    scratch
}

/// Whether the folder `key` is what the view shows open: stepped into in the flat views, opened
/// in place in the tree.
fn is_opened(h: &Harness<Demo>, view: FileView, key: &str) -> bool {
    match view {
        FileView::Tree => state(h).is_open(key),
        _ => state(h).folder() == key,
    }
}

#[test]
fn a_click_selects_an_entry_and_opens_nothing_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-click-{view:?}"));
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        assert_eq!(state(&h).selected(), Some("README.md"), "{view:?}:\n{}", h.screen());
        assert_eq!(state(&h).chosen(), ["README.md"], "{view:?}");
        assert!(h.app().opened.is_empty(), "{view:?}: a click opens nothing");

        click(&mut h, "src");
        assert_eq!(state(&h).chosen(), ["src"], "{view:?}");
        assert!(!is_opened(&h, view, "src"), "{view:?}: a click does not open a folder either");
        // Clicks far enough apart are two clicks, not a double click.
        click(&mut h, "src");
        assert!(!is_opened(&h, view, "src"), "{view:?}: two slow clicks are two clicks");
    }
}

#[test]
fn a_double_click_opens_a_file_once_and_opens_a_folder_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-double-{view:?}"));
        let mut h = shown(&scratch, view, None);
        double_click(&mut h, "README.md");
        h.advance(MOMENT);
        assert_eq!(h.app().opened, [scratch.root().join("README.md")], "{view:?}: once, and only once");
        assert_eq!(state(&h).selected(), Some("README.md"), "{view:?}");

        double_click(&mut h, "src");
        h.advance(MOMENT);
        assert!(is_opened(&h, view, "src"), "{view:?}: a folder opens\n{}", h.screen());
        assert!(h.screen().contains("main.rs"), "{view:?}:\n{}", h.screen());
        assert_eq!(h.app().opened.len(), 1, "{view:?}: a folder is not handed to the application");
    }
}

#[test]
fn enter_opens_the_selected_entry_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-enter-{view:?}"));
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        assert!(h.app().opened.is_empty(), "{view:?}");
        h.press("enter").advance(MOMENT);
        assert_eq!(h.app().opened, [scratch.root().join("README.md")], "{view:?}");

        click(&mut h, "src");
        h.press("enter").advance(MOMENT);
        assert!(is_opened(&h, view, "src"), "{view:?}:\n{}", h.screen());
    }
}

#[test]
fn ctrl_click_adds_and_removes_and_shift_click_selects_a_range_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-several-{view:?}"));
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        click_with(&mut h, "docs", CTRL);
        assert_eq!(state(&h).chosen(), ["README.md", "docs"], "{view:?}:\n{}", h.screen());
        click_with(&mut h, "README.md", CTRL);
        assert_eq!(state(&h).chosen(), ["docs"], "{view:?}: a second Ctrl+click takes it out");
        assert!(h.app().opened.is_empty(), "{view:?}: choosing opens nothing");

        click(&mut h, "docs");
        click_with(&mut h, "README.md", SHIFT);
        assert_eq!(state(&h).chosen(), ["docs", "src", "README.md"], "{view:?}:\n{}", h.screen());
        click_with(&mut h, "src", SHIFT);
        assert_eq!(state(&h).chosen(), ["docs", "src"], "{view:?}: the range starts where it started");
        assert!(!is_opened(&h, view, "src") && !is_opened(&h, view, "docs"), "{view:?}");
    }
}

#[test]
fn a_box_drawn_on_the_free_space_selects_what_it_covers_and_ctrl_adds_to_the_selection_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-box-{view:?}"));
        let mut h = shown(&scratch, view, None);
        let (readme, plan) = (spot(&h, "README.md"), spot(&h, "plan.txt"));
        let free = (readme.0, FREE_Y);
        // From the free space below the entries up to README.md: in the icons the box also
        // reaches across to plan.txt, beside README.md.
        let corner = (plan.0, readme.1);
        let (x, y) = (u16::try_from(free.0).expect("a column"), u16::try_from(free.1).expect("a row"));
        let ground = h.bg(x, y);
        let left = MouseButton::Left;
        h.events(&[pointer(MouseKind::Down(left), free, Modifiers::default())]);
        h.events(&[pointer(MouseKind::Drag(left), corner, Modifiers::default())]);
        assert_eq!(state(&h).chosen(), ["README.md", "plan.txt"], "{view:?}: selected while it is drawn");
        let screen = h.screen();
        assert!(
            !screen.contains('┌') && !screen.contains('─') && !screen.contains('│'),
            "{view:?}: no frame\n{screen}"
        );
        assert_ne!(h.bg(x, y), ground, "{view:?}: the box is a tone on the cells it covers");
        h.events(&[pointer(MouseKind::Up(left), corner, Modifiers::default())]);
        h.advance(MOMENT);
        assert_eq!(state(&h).chosen(), ["README.md", "plan.txt"], "{view:?}:\n{}", h.screen());
        assert_eq!(h.bg(x, y), ground, "{view:?}: the box goes when the button is let go");
        assert!(h.app().opened.is_empty() && !is_opened(&h, view, "src"), "{view:?}");

        click(&mut h, "docs");
        drag_with(&mut h, free, corner, Modifiers::default());
        assert_eq!(state(&h).chosen(), ["README.md", "plan.txt"], "{view:?}: a plain box replaces the selection");
        click(&mut h, "docs");
        let left = MouseButton::Left;
        h.events(&[
            pointer(MouseKind::Down(left), free, CTRL),
            pointer(MouseKind::Drag(left), corner, CTRL),
            pointer(MouseKind::Up(left), corner, CTRL),
        ]);
        assert_eq!(state(&h).chosen(), ["docs", "README.md", "plan.txt"], "{view:?}: with Ctrl it adds");

        // A click on the free space is a box that covers nothing: the selection is let go.
        h.advance(MOMENT);
        h.click(free.0, free.1);
        assert!(state(&h).chosen().is_empty(), "{view:?}:\n{}", h.screen());
    }
}

#[test]
fn dragging_the_selection_onto_a_folder_moves_it_there_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-move-{view:?}"));
        let root = scratch.root();
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        click_with(&mut h, "plan.txt", CTRL);
        let (from, to) = (spot(&h, "plan.txt"), spot(&h, "docs"));
        drag_with(&mut h, from, to, Modifiers::default());
        assert!(root.join("docs/README.md").is_file() && root.join("docs/plan.txt").is_file(), "{view:?}");
        assert!(!root.join("README.md").exists() && !root.join("plan.txt").exists(), "{view:?}: moved");
        assert!(h.app().opened.is_empty(), "{view:?}: pressing to drag opens nothing");
    }
}

#[test]
fn a_drag_released_with_ctrl_copies_the_selection_into_the_folder_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-copy-{view:?}"));
        let root = scratch.root();
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        click_with(&mut h, "plan.txt", CTRL);
        let (from, to) = (spot(&h, "plan.txt"), spot(&h, "docs"));
        drag_with(&mut h, from, to, CTRL);
        assert!(root.join("docs/README.md").is_file() && root.join("docs/plan.txt").is_file(), "{view:?}");
        assert!(root.join("README.md").is_file() && root.join("plan.txt").is_file(), "{view:?}: copied, not moved");
    }
}

#[test]
fn a_drag_released_on_a_file_or_on_itself_does_nothing_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-nowhere-{view:?}"));
        let root = scratch.root();
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        let (from, onto) = (spot(&h, "README.md"), spot(&h, "plan.txt"));
        drag_with(&mut h, from, onto, Modifiers::default());
        drag_with(&mut h, from, onto, CTRL);
        assert!(root.join("README.md").is_file() && root.join("plan.txt").is_file(), "{view:?}");
        assert!(!root.join("plan.txt").is_dir(), "{view:?}");
        let docs = fs::read_dir(root.join("docs")).expect("the folder").count();
        assert_eq!(docs, 0, "{view:?}: nothing went anywhere");
        assert!(h.app().opened.is_empty(), "{view:?}");
    }
}

#[test]
fn a_dragged_row_is_not_the_first_half_of_a_double_click() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-drag-click-{view:?}"));
        let mut h = shown(&scratch, view, None);
        let (from, onto) = (spot(&h, "README.md"), spot(&h, "plan.txt"));
        let left = MouseButton::Left;
        h.events(&[
            pointer(MouseKind::Down(left), from, Modifiers::default()),
            pointer(MouseKind::Drag(left), onto, Modifiers::default()),
            pointer(MouseKind::Drag(left), from, Modifiers::default()),
            pointer(MouseKind::Up(left), from, Modifiers::default()),
        ]);
        h.click(from.0, from.1);
        assert!(h.app().opened.is_empty(), "{view:?}: a drag and a click are not a double click");
    }
}

#[test]
fn the_tree_opens_and_closes_a_folder_with_one_click_on_its_chevron_and_with_the_arrows() {
    let scratch = project("mouse-chevron");
    let mut h = shown(&scratch, FileView::Tree, None);
    let (x, y) = spot(&h, "src");
    // The chevron stands two cells before the icon, which stands two before the name.
    h.click(x - 4, y).advance(MOMENT);
    assert!(state(&h).is_open("src"), "one click on the chevron opens:\n{}", h.screen());
    h.click(x - 4, y).advance(MOMENT);
    assert!(!state(&h).is_open("src"), "and one more closes:\n{}", h.screen());

    click(&mut h, "src");
    h.press("right").advance(MOMENT);
    assert!(state(&h).is_open("src"), "→ opens");
    h.press("left").advance(MOMENT);
    assert!(!state(&h).is_open("src"), "← closes");
}

#[test]
fn a_manager_asked_to_open_on_one_click_opens_on_one_click_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-single-{view:?}"));
        let mut h = shown(&scratch, view, Some(Click::Single));
        click(&mut h, "README.md");
        assert_eq!(h.app().opened, [scratch.root().join("README.md")], "{view:?}");
        click(&mut h, "src");
        assert!(is_opened(&h, view, "src"), "{view:?}:\n{}", h.screen());
        assert_eq!(h.app().opened.len(), 1, "{view:?}");
    }
}

#[test]
fn a_right_click_opens_the_menu_of_the_selection_or_of_the_row_it_selects_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-menu-{view:?}"));
        let mut h = shown(&scratch, view, None);
        click(&mut h, "README.md");
        click_with(&mut h, "plan.txt", CTRL);
        right_click(&mut h, "plan.txt");
        assert!(h.screen().contains("Cut 2 entries"), "{view:?}: the selection's menu\n{}", h.screen());
        h.press("esc").advance(MOMENT);

        right_click(&mut h, "docs");
        assert_eq!(state(&h).chosen(), ["docs"], "{view:?}: the row outside it becomes the selection");
        assert!(h.screen().contains("Rename"), "{view:?}:\n{}", h.screen());
    }
}

#[test]
fn the_keys_move_the_selection_with_the_cursor_in_every_view() {
    for view in VIEWS {
        let scratch = project(&format!("mouse-keys-{view:?}"));
        let mut h = shown(&scratch, view, None);
        click(&mut h, "docs");
        click_with(&mut h, "src", CTRL);
        h.press("end").advance(MOMENT);
        assert_eq!(state(&h).selected(), Some("plan.txt"), "{view:?}:\n{}", h.screen());
        assert_eq!(state(&h).chosen(), ["plan.txt"], "{view:?}: a key without Shift selects the one entry");
        h.press("space").advance(MOMENT);
        assert!(state(&h).chosen().is_empty(), "{view:?}: Space takes the cursor's entry out");
        h.press("space").advance(MOMENT);
        assert_eq!(state(&h).chosen(), ["plan.txt"], "{view:?}: and puts it back");
        assert!(h.app().opened.is_empty(), "{view:?}");
    }
}
