//! Selecting several nodes and dropping them into a folder.

use std::collections::BTreeSet;
use std::time::Duration;

use super::*;
use crate::event::{Event, MouseEvent, MouseKind};
use crate::icons::GlyphMode;
use crate::keymap::Modifiers;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;

/// A file or folder, as a file manager keeps it.
#[derive(Clone, Debug)]
struct Item {
    key: String,
    /// `Some` for a folder.
    children: Option<Vec<Item>>,
}

fn file(key: &str) -> Item {
    Item { key: key.to_owned(), children: None }
}

fn folder(key: &str, children: Vec<Item>) -> Item {
    Item { key: key.to_owned(), children: Some(children) }
}

struct Files {
    roots: Vec<Item>,
    open: BTreeSet<String>,
    cursor: Option<String>,
    chosen: Vec<String>,
    activated: Vec<String>,
    menu: Vec<(&'static str, String)>,
    drops: Vec<TreeDrop>,
    moves: Vec<TreeMove>,
    droppable: bool,
    reorderable: bool,
}

#[derive(Clone)]
enum FileMsg {
    Cursor(String),
    Choose(Vec<String>),
    Activate(String),
    Expand(String, bool),
    Menu(&'static str, String),
    Drop(TreeDrop),
    Move(TreeMove),
}

impl Files {
    fn new() -> Self {
        Self {
            roots: vec![
                folder("docs", vec![file("a.md"), file("b.md"), file("c.md"), folder("old", vec![file("x.md")])]),
                folder("src", vec![file("main.rs")]),
                file("notes.txt"),
            ],
            open: ["docs".to_owned()].into(),
            cursor: None,
            chosen: Vec::new(),
            activated: Vec::new(),
            menu: Vec::new(),
            drops: Vec::new(),
            moves: Vec::new(),
            droppable: true,
            reorderable: false,
        }
    }

    /// The keys of every folder.
    fn folders(&self) -> Vec<String> {
        fn walk(items: &[Item], out: &mut Vec<String>) {
            for item in items {
                if let Some(children) = &item.children {
                    out.push(item.key.clone());
                    walk(children, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.roots, &mut out);
        out
    }

    /// The keys of the children of `folder`, or of the top level.
    fn listing(&self, folder: Option<&str>) -> Vec<String> {
        fn find<'a>(items: &'a [Item], key: &str) -> Option<&'a Item> {
            items.iter().find_map(|item| {
                if item.key == key { Some(item) } else { item.children.as_deref().and_then(|c| find(c, key)) }
            })
        }
        let items = match folder {
            None => &self.roots,
            Some(key) => find(&self.roots, key).and_then(|item| item.children.as_ref()).expect("a folder"),
        };
        items.iter().map(|item| item.key.clone()).collect()
    }

    /// Takes the items with `keys` out of `items`, wherever they are.
    fn take(items: &mut Vec<Item>, keys: &[String], out: &mut Vec<Item>) {
        let mut at = 0;
        while at < items.len() {
            if keys.contains(&items[at].key) {
                out.push(items.remove(at));
            } else {
                if let Some(children) = &mut items[at].children {
                    Self::take(children, keys, out);
                }
                at += 1;
            }
        }
    }

    fn apply(&mut self, drop: &TreeDrop) {
        fn find<'a>(items: &'a mut [Item], key: &str) -> Option<&'a mut Vec<Item>> {
            for item in items {
                let found = item.key == key;
                if let Some(children) = &mut item.children {
                    if found {
                        return Some(children);
                    }
                    if let Some(inner) = find(children, key) {
                        return Some(inner);
                    }
                }
            }
            None
        }
        let mut moved = Vec::new();
        Self::take(&mut self.roots, &drop.keys, &mut moved);
        let target = match drop.into.as_deref() {
            None => &mut self.roots,
            Some(key) => find(&mut self.roots, key).expect("a folder"),
        };
        target.extend(moved);
    }

    fn node(&self, item: &Item) -> TreeNode {
        let node = TreeNode::new(item.key.clone(), item.key.clone());
        match &item.children {
            Some(children) => node
                .expandable(true)
                .expanded(self.open.contains(&item.key))
                .children(children.iter().map(|child| self.node(child))),
            None => node,
        }
    }
}

impl App for Files {
    type Msg = FileMsg;
    fn update(&mut self, msg: FileMsg) -> Command<FileMsg> {
        match msg {
            FileMsg::Cursor(key) => self.cursor = Some(key),
            FileMsg::Choose(keys) => self.chosen = keys,
            FileMsg::Activate(key) => self.activated.push(key),
            FileMsg::Expand(key, true) => {
                self.open.insert(key);
            }
            FileMsg::Expand(key, false) => {
                self.open.remove(&key);
            }
            FileMsg::Menu(action, key) => self.menu.push((action, key)),
            FileMsg::Drop(drop) => {
                self.apply(&drop);
                self.drops.push(drop);
            }
            FileMsg::Move(step) => self.moves.push(step),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, FileMsg>) {
        let mut tree = Tree::new(self.roots.iter().map(|item| self.node(item)))
            .selected(self.cursor.as_deref())
            .on_select(|key| FileMsg::Cursor(key.to_owned()))
            .on_activate(|key| FileMsg::Activate(key.to_owned()))
            .on_expand(|key, open| FileMsg::Expand(key.to_owned(), open))
            .multi_select(&self.chosen, FileMsg::Choose)
            .context_menu(|key| vec![ContextItem::new("Delete", FileMsg::Menu("delete", key.to_owned()))]);
        if self.droppable {
            let folders = self.folders();
            tree = tree.droppable(FileMsg::Drop, move |key| folders.iter().any(|folder| folder == key));
        }
        if self.reorderable {
            tree = tree.reorderable(FileMsg::Move);
        }
        ui.add(tree).fill().id("files");
    }
}

fn files() -> Harness<Files> {
    let mut h = Harness::new(Files::new(), 26, 8);
    h.set_glyph_mode(GlyphMode::Unicode);
    h.set_reduced_motion(true);
    h
}

fn keys(list: &[&str]) -> Vec<String> {
    list.iter().map(|key| (*key).to_owned()).collect()
}

/// A left press and release at `(x, y)` with `mods` held.
fn click_with(h: &mut Harness<Files>, x: i32, y: i32, mods: Modifiers) {
    let event = |kind| Event::Mouse(MouseEvent { kind, x, y, mods });
    h.events(&[event(MouseKind::Down(MouseButton::Left)), event(MouseKind::Up(MouseButton::Left))]);
}

const CTRL: Modifiers = Modifiers { ctrl: true, alt: false, shift: false };
const SHIFT: Modifiers = Modifiers { ctrl: false, alt: false, shift: true };

/// The rows of the screen that carry the pillar.
fn pillars(h: &Harness<Files>) -> Vec<usize> {
    h.screen().lines().enumerate().filter(|(_, line)| line.starts_with('▌')).map(|(row, _)| row).collect()
}

#[test]
fn ctrl_click_adds_and_removes_and_shift_click_selects_a_range() {
    let mut h = files();
    assert_eq!(h.screen(), "  ▾ docs\n      a.md\n      b.md\n      c.md\n    ▸ old\n  ▸ src\n    notes.txt\n\n");
    h.click_text("a.md");
    assert_eq!((h.app().chosen.clone(), h.app().activated.clone()), (keys(&["a.md"]), keys(&["a.md"])));
    click_with(&mut h, 8, 3, CTRL);
    assert_eq!(h.app().chosen, keys(&["a.md", "c.md"]));
    assert_eq!(h.app().cursor.as_deref(), Some("c.md"), "the cursor follows the click");
    assert_eq!(h.app().activated, keys(&["a.md"]), "a Ctrl+click only selects");
    click_with(&mut h, 8, 3, CTRL);
    assert_eq!(h.app().chosen, keys(&["a.md"]), "a second Ctrl+click takes the row out");
    click_with(&mut h, 8, 6, SHIFT);
    assert_eq!(h.app().chosen, keys(&["c.md", "old", "src", "notes.txt"]), "from the last Ctrl+click on");
    click_with(&mut h, 8, 1, SHIFT);
    assert_eq!(h.app().chosen, keys(&["a.md", "b.md", "c.md"]), "the anchor stays, the range reshapes");
    h.click_text("notes.txt");
    assert_eq!(h.app().chosen, keys(&["notes.txt"]), "a plain click selects that row alone");
}

#[test]
fn shift_arrows_extend_space_toggles_and_esc_reduces() {
    let mut h = files();
    h.press("tab").press("down").press("down");
    assert_eq!(h.app().chosen, keys(&["a.md"]), "a plain arrow selects the row it reaches");
    h.press("shift+down").press("shift+down");
    assert_eq!(h.app().chosen, keys(&["a.md", "b.md", "c.md"]));
    h.press("shift+up");
    assert_eq!(h.app().chosen, keys(&["a.md", "b.md"]));
    assert_eq!(h.app().cursor.as_deref(), Some("b.md"));
    h.press("down").press("space");
    assert_eq!(h.app().chosen, Vec::<String>::new(), "Space takes the cursor's row out");
    h.press("space");
    assert_eq!(h.app().chosen, keys(&["c.md"]));
    assert!(h.app().activated.is_empty(), "Space selects instead of activating");
    h.press("shift+end").press("esc");
    assert_eq!(h.app().chosen, keys(&["notes.txt"]), "Esc keeps the cursor's row");
    h.press("enter");
    assert_eq!(h.app().activated, keys(&["notes.txt"]), "Enter still activates");
}

#[test]
fn every_selected_row_takes_the_tone_and_only_the_cursor_the_pillar() {
    let mut h = files();
    h.press("tab").press("down").press("down").press("shift+down").press("shift+down");
    assert_eq!(pillars(&h), [3], "{}", h.screen());
    let selected = h.env().theme().style("list-item", None, &[State::Selected]).paint("bg").map(|paint| paint.at(0.0));
    for row in 1..=3 {
        assert_eq!(h.bg(20, row), selected, "row {row}:\n{}", h.screen());
    }
    assert_ne!(h.bg(20, 4), selected);
    // Only the cursor's label slides; the others keep their column.
    assert_eq!(h.screen().lines().nth(1), Some("      a.md"), "{}", h.screen());
    assert_eq!(h.screen().lines().nth(3), Some("▌      c.md"), "{}", h.screen());
}

#[test]
fn a_right_click_keeps_the_selection_it_is_on_and_reduces_one_it_is_not_on() {
    let mut h = files();
    h.press("tab").press("down").press("down").press("shift+down");
    h.mouse(MouseKind::Down(MouseButton::Right), 8, 1).mouse(MouseKind::Up(MouseButton::Right), 8, 1);
    assert!(h.screen().contains("Delete"), "{}", h.screen());
    assert_eq!(h.app().chosen, keys(&["a.md", "b.md"]), "the menu acts on the whole selection");
    h.press("esc");
    h.mouse(MouseKind::Down(MouseButton::Right), 8, 6).mouse(MouseKind::Up(MouseButton::Right), 8, 6);
    assert_eq!(h.app().chosen, keys(&["notes.txt"]), "outside the selection the row becomes it");
    assert_eq!(h.app().cursor.as_deref(), Some("notes.txt"));
    h.click_text("Delete");
    assert_eq!(h.app().menu, [("delete", "notes.txt".to_owned())]);
}

#[test]
fn selection_reads_the_same_in_ascii() {
    let mut h = files();
    h.set_glyph_mode(GlyphMode::Ascii);
    h.press("tab").press("down").press("down").press("shift+down");
    let screen = h.screen();
    assert!(screen.is_ascii() && !screen.contains(['[', ']', '(', ')', '|']), "{screen}");
}

/// Selects a.md, b.md and c.md with a click and a Shift+click.
fn three_selected() -> Harness<Files> {
    let mut h = files();
    h.click_text("a.md");
    click_with(&mut h, 8, 3, SHIFT);
    assert_eq!(h.app().chosen, keys(&["a.md", "b.md", "c.md"]));
    h
}

fn drop_bg(h: &Harness<Files>) -> Option<crate::color::Rgb> {
    h.env().theme().style("tree-drop", None, &[]).paint("bg").map(|paint| paint.at(0.0))
}

#[test]
fn dragging_three_nodes_into_a_folder_sends_one_drop_with_the_three_keys() {
    let mut h = three_selected();
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 2).mouse(MouseKind::Drag(MouseButton::Left), 8, 5);
    assert_eq!(h.app().chosen, keys(&["a.md", "b.md", "c.md"]), "a press on the selection keeps it");
    assert_eq!(h.bg(20, 5), drop_bg(&h), "the target takes the accent tone:\n{}", h.screen());
    let screen = h.screen();
    assert_eq!(screen.lines().nth(5), Some("  ▸ src"), "the rows stay still:\n{screen}");
    h.mouse(MouseKind::Up(MouseButton::Left), 8, 5);
    assert_eq!(h.app().drops, [TreeDrop { keys: keys(&["a.md", "b.md", "c.md"]), into: Some("src".to_owned()) }]);
    assert_eq!(h.app().listing(Some("src")), keys(&["main.rs", "a.md", "b.md", "c.md"]));
    assert!(h.app().activated.iter().all(|key| key == "a.md"), "a drop activates nothing");
}

#[test]
fn a_node_never_drops_into_itself_its_descendants_or_where_it_already_is() {
    let mut h = files();
    h.press("tab").press("down");
    assert_eq!(h.app().chosen, keys(&["docs"]));
    let resting = h.bg(20, 4);
    // docs onto its own folder "old": refused, the target stays faint and flat.
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 0).mouse(MouseKind::Drag(MouseButton::Left), 8, 4);
    assert_eq!(h.bg(20, 4), resting, "{}", h.screen());
    assert_eq!(h.fg(6, 4), h.env().theme().color("muted"), "{}", h.screen());
    h.mouse(MouseKind::Drag(MouseButton::Left), 8, 0).mouse(MouseKind::Up(MouseButton::Left), 8, 0);
    assert!(h.app().drops.is_empty(), "onto itself: nothing");
    // a.md is already in docs.
    h.drag((8, 1), (8, 0));
    assert!(h.app().drops.is_empty());
    assert_eq!(h.app().listing(Some("docs")), keys(&["a.md", "b.md", "c.md", "old"]));
    // The same node goes into another folder.
    h.drag((8, 1), (8, 5));
    assert_eq!(h.app().drops.last().map(|drop| drop.into.clone()), Some(Some("src".to_owned())));
}

#[test]
fn a_closed_folder_opens_when_a_drag_rests_on_it() {
    let mut h = three_selected();
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 2).mouse(MouseKind::Drag(MouseButton::Left), 8, 5);
    h.advance(Duration::from_millis(399));
    assert!(!h.app().open.contains("src"), "passing over a folder does not open it");
    h.advance(Duration::from_millis(1));
    assert!(h.app().open.contains("src"), "resting on it does, reduced motion or not:\n{}", h.screen());
    assert_eq!(h.screen().lines().nth(6), Some("      main.rs"), "{}", h.screen());
    // Onto the folder inside docs, which is closed too, then away before it opens.
    h.mouse(MouseKind::Drag(MouseButton::Left), 8, 4);
    h.advance(Duration::from_millis(200));
    h.mouse(MouseKind::Drag(MouseButton::Left), 8, 6).advance(Duration::from_millis(400));
    assert!(!h.app().open.contains("old"), "{}", h.screen());
    h.mouse(MouseKind::Up(MouseButton::Left), 8, 6);
    assert!(h.app().drops.is_empty(), "main.rs is a file: no drop");
}

#[test]
fn the_free_space_below_the_rows_is_the_top_level() {
    let mut h = three_selected();
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 1).mouse(MouseKind::Drag(MouseButton::Left), 8, 7);
    assert_eq!(h.bg(20, 7), drop_bg(&h), "{}", h.screen());
    h.mouse(MouseKind::Up(MouseButton::Left), 8, 7);
    assert_eq!(h.app().drops.last().map(|drop| drop.into.clone()), Some(None));
    assert_eq!(h.app().listing(None), keys(&["docs", "src", "notes.txt", "a.md", "b.md", "c.md"]));
}

#[test]
fn with_reordering_folders_take_nodes_in_and_other_rows_are_places_among_siblings() {
    let mut app = Files::new();
    app.reorderable = true;
    let mut h = Harness::new(app, 26, 8);
    h.set_glyph_mode(GlyphMode::Unicode);
    h.drag((8, 1), (8, 2));
    assert_eq!(h.app().moves, [TreeMove { key: "a.md".to_owned(), parent: Some("docs".to_owned()), from: 0, to: 1 }]);
    assert!(h.app().drops.is_empty());
    h.drag((8, 6), (8, 5));
    assert_eq!(h.app().drops, [TreeDrop { keys: keys(&["notes.txt"]), into: Some("src".to_owned()) }]);
    assert_eq!(h.app().moves.len(), 1);
}

#[test]
fn without_dropping_a_selection_drags_as_before() {
    let mut app = Files::new();
    app.droppable = false;
    app.reorderable = true;
    let mut h = Harness::new(app, 26, 8);
    h.set_glyph_mode(GlyphMode::Unicode);
    h.click_text("a.md");
    click_with(&mut h, 8, 2, SHIFT);
    h.drag((8, 2), (8, 1));
    assert_eq!(h.app().chosen, keys(&["b.md"]), "a reorder moves the pressed node alone");
    assert_eq!(h.app().moves, [TreeMove { key: "b.md".to_owned(), parent: Some("docs".to_owned()), from: 1, to: 0 }]);
}

#[test]
fn a_drop_reads_the_same_in_ascii() {
    let mut h = three_selected();
    h.set_glyph_mode(GlyphMode::Ascii);
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 2).mouse(MouseKind::Drag(MouseButton::Left), 8, 5);
    let screen = h.screen();
    assert!(screen.is_ascii() && !screen.contains(['[', ']', '(', ')', '|']), "{screen}");
    assert_eq!(h.bg(20, 5), drop_bg(&h), "{screen}");
}
