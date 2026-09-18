use std::collections::BTreeSet;
use std::time::Duration;

use super::*;
use crate::event::MouseKind;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;

#[derive(Default)]
struct Demo {
    open: BTreeSet<String>,
    selected: Option<String>,
    activated: Vec<String>,
    loaded: bool,
    many: usize,
    icons: bool,
}

/// Gives `node` and everything below it the file icon.
fn with_icons(mut node: TreeNode) -> TreeNode {
    node.icon = Some("file".to_owned());
    node.children = node.children.into_iter().map(with_icons).collect();
    node
}

#[derive(Clone)]
enum Msg {
    Select(String),
    Activate(String),
    Expand(String, bool),
    /// The lazy node's children arrived.
    Loaded,
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(key) => self.selected = Some(key),
            Msg::Loaded => self.loaded = true,
            Msg::Activate(key) => self.activated.push(key),
            Msg::Expand(key, true) => {
                self.open.insert(key);
            }
            Msg::Expand(key, false) => {
                self.open.remove(&key);
            }
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let open = |key: &str| self.open.contains(key);
        let nodes = if self.many > 0 {
            vec![
                TreeNode::new("logs", "logs")
                    .children((0..self.many).map(|i| TreeNode::new(format!("l{i}"), format!("day-{i}.log"))))
                    .expanded(open("logs")),
            ]
        } else {
            let lazy = TreeNode::new("remote", "remote").expandable(true).expanded(open("remote"));
            let lazy = if self.loaded {
                lazy.children([TreeNode::new("remote/a", "a.txt")])
            } else {
                lazy.loading(open("remote"))
            };
            vec![
                TreeNode::new("src", "src").expanded(open("src")).detail("2").children([
                    TreeNode::new("src/widgets", "widgets")
                        .expanded(open("src/widgets"))
                        .children([TreeNode::new("src/widgets/tree.rs", "tree.rs")]),
                    TreeNode::new("src/lib.rs", "lib.rs"),
                ]),
                lazy,
                TreeNode::new("Cargo.toml", "Cargo.toml"),
            ]
        };
        let nodes: Vec<TreeNode> = if self.icons { nodes.into_iter().map(with_icons).collect() } else { nodes };
        let tree = Tree::new(nodes)
            .selected(self.selected.as_deref())
            .on_select(|key| Msg::Select(key.to_owned()))
            .on_activate(|key| Msg::Activate(key.to_owned()))
            .on_expand(|key, open| Msg::Expand(key.to_owned(), open));
        ui.add(tree).fill().id("tree");
    }
}

fn harness(app: Demo, height: u16) -> Harness<Demo> {
    let mut h = Harness::new(app, 26, height);
    h.set_glyph_mode(GlyphMode::Unicode);
    h
}

#[test]
fn opens_with_keys_and_indents_children() {
    let mut h = harness(Demo::default(), 6);
    assert_eq!(h.screen(), "  ▸ src                 2\n  ▸ remote\n    Cargo.toml\n\n\n\n");
    h.press("tab").press("down").press("right");
    assert!(h.app().open.contains("src"));
    h.press("right");
    assert_eq!(h.app().selected.as_deref(), Some("src/widgets"));
    h.press("enter").press("down");
    assert_eq!(
        h.screen(),
        "  ▾ src                 2\n    ▾ widgets\n▌        tree.rs\n      lib.rs\n  ▸ remote\n    Cargo.toml\n"
    );
    h.press("enter");
    assert_eq!(h.app().activated, vec!["src/widgets/tree.rs".to_owned()]);
    h.press("left");
    assert_eq!(h.app().selected.as_deref(), Some("src/widgets"));
    h.press("left");
    assert!(!h.app().open.contains("src/widgets"));
}

#[test]
fn chevron_click_only_toggles_and_row_click_selects() {
    let mut h = harness(Demo::default(), 6);
    h.click(2, 0);
    assert!(h.app().open.contains("src"));
    assert_eq!(h.app().selected, None);
    h.click_text("lib.rs");
    assert_eq!(h.app().selected.as_deref(), Some("src/lib.rs"));
    assert_eq!(h.app().activated, vec!["src/lib.rs".to_owned()]);
}

#[test]
fn lazy_nodes_spin_until_children_arrive() {
    let mut h = harness(Demo::default(), 6);
    h.click_text("remote");
    assert!(h.app().open.contains("remote"));
    // A quick load keeps the chevron: no spinner for the first 300 ms.
    assert_eq!(h.screen().lines().nth(1), Some("▌ ▾  remote"), "{}", h.screen());
    h.advance(Duration::from_millis(299));
    assert_eq!(h.screen().lines().nth(1), Some("▌ ▾  remote"), "{}", h.screen());
    // The spinner takes the chevron's place and, like it, stays put while the label slides.
    h.advance(Duration::from_millis(1));
    assert_eq!(h.screen().lines().nth(1), Some("▌ ⠸  remote"), "{}", h.screen());
    h.advance(Duration::from_millis(90));
    assert_eq!(h.screen().lines().nth(1), Some("▌ ⠼  remote"), "{}", h.screen());
    let mut app = Demo { loaded: true, ..Demo::default() };
    app.open.insert("remote".to_owned());
    let h = harness(app, 6);
    assert_eq!(h.screen(), "  ▸ src                 2\n  ▾ remote\n      a.txt\n    Cargo.toml\n\n\n");
}

/// Whether the lazy node's row shows a spinner frame.
fn remote_spins(h: &Harness<Demo>) -> bool {
    h.screen().lines().nth(1).is_some_and(|line| line.contains(|c| "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".contains(c)))
}

#[test]
fn quick_loads_never_spin_and_slow_ones_spin_at_least_the_minimum() {
    let mut h = harness(Demo::default(), 6);
    h.click_text("remote").advance(Duration::from_millis(120)).send(Msg::Loaded);
    assert_eq!(h.screen(), "  ▸ src                 2\n▌ ▾  remote\n      a.txt\n    Cargo.toml\n\n\n");
    h.advance(Duration::from_millis(400));
    assert!(!remote_spins(&h), "{}", h.screen());

    let mut h = harness(Demo::default(), 6);
    h.click_text("remote").advance(Duration::from_millis(300)).advance(Duration::from_millis(50)).send(Msg::Loaded);
    // The children are in; the spinner shown at 300 ms stays until 800 ms.
    assert!(remote_spins(&h) && h.screen().contains("a.txt"), "{}", h.screen());
    h.advance(Duration::from_millis(449));
    assert!(remote_spins(&h), "{}", h.screen());
    h.advance(Duration::from_millis(1));
    assert_eq!(h.screen().lines().nth(1), Some("▌ ▾  remote"), "{}", h.screen());
}

#[test]
fn chevrons_and_indentation_stay_put_while_icon_and_label_slide() {
    let mut open = BTreeSet::new();
    open.insert("src".to_owned());
    let mut h = harness(Demo { icons: true, open: open.clone(), ..Demo::default() }, 5);
    assert_eq!(
        h.screen(),
        "  ▾ ▪ src               2\n    ▸ ▪ widgets\n      ▪ lib.rs\n  ▸ ▪ remote\n    ▪ Cargo.toml\n"
    );
    h.hover(12, 1);
    assert_eq!(h.screen().lines().nth(1), Some("▌   ▸  ▪ widgets"), "{}", h.screen());
    h.hover(12, 0);
    assert_eq!(h.screen().lines().next(), Some("▌ ▾  ▪ src              2"), "{}", h.screen());
    let theme = h.env().theme();
    assert_eq!(h.fg(2, 0), theme.color("dim"), "a hovered row's chevron brightens in place");
    assert_eq!(h.fg(2, 3), theme.color("muted"));
    let chevron = |line: &str, glyph: char| line.chars().position(|c| c == glyph);
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert_eq!(chevron(lines[0], '▾'), Some(2), "hovered or not, a chevron keeps its column");
    assert_eq!(chevron(lines[1], '▸'), Some(4));
    assert_eq!(chevron(lines[3], '▸'), Some(2));

    let mut env = crate::env::Env::builtin();
    env.set_slide(false);
    let mut h = Harness::with_env(Demo { icons: true, open, ..Demo::default() }, env, 26, 5);
    h.set_glyph_mode(GlyphMode::Unicode);
    h.hover(12, 1);
    assert_eq!(h.screen().lines().nth(1), Some("▌   ▸ ▪ widgets"), "{}", h.screen());
}

#[test]
fn a_click_on_a_sliding_row_still_finds_its_chevron() {
    let mut h = harness(Demo { icons: true, ..Demo::default() }, 6);
    h.hover(10, 0);
    h.click(2, 0);
    assert!(h.app().open.contains("src"), "the chevron is where it was drawn");
    assert_eq!(h.app().selected, None, "the chevron only opens");
    h.click(3, 0);
    assert!(!h.app().open.contains("src"), "the cell after the chevron closes it again");
    h.click(5, 0);
    assert_eq!(h.app().selected.as_deref(), Some("src"), "the icon and label select");
}

#[test]
fn labels_are_cut_at_the_same_place_resting_and_sliding() {
    let mut h = Harness::new(Demo { icons: true, ..Demo::default() }, 14, 3);
    h.set_glyph_mode(GlyphMode::Unicode);
    assert_eq!(h.screen().lines().nth(2), Some("    ▪ Cargo…"), "{}", h.screen());
    h.hover(8, 2);
    assert_eq!(h.screen().lines().nth(2), Some("▌    ▪ Cargo…"), "{}", h.screen());
}

#[test]
fn large_open_trees_are_virtualised() {
    let mut app = Demo { many: 50_000, ..Demo::default() };
    app.open.insert("logs".to_owned());
    let mut h = harness(app, 5);
    h.press("tab").press("end");
    assert_eq!(h.app().selected.as_deref(), Some("l49999"));
    let screen = h.screen();
    assert!(screen.contains("day-49999.log"), "{screen}");
    assert!(!super::super::scrollbar::column(&h, 25).contains(' '), "{screen}");
}

#[test]
fn ascii_chevrons_and_empty_text() {
    let mut h = harness(Demo::default(), 3);
    h.set_glyph_mode(GlyphMode::Ascii);
    assert!(h.screen().starts_with("  + src"), "{}", h.screen());
    struct Empty;
    impl App for Empty {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Tree::new(Vec::new()).empty_text("No folders")).fill();
        }
    }
    assert_eq!(Harness::new(Empty, 20, 1).screen(), "  No folders\n");
}

#[test]
fn labels_wider_than_any_screen_do_not_overflow_the_measure() {
    struct Wide;

    impl App for Wide {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let long = "n".repeat(70_000);
            ui.add(Tree::new([TreeNode::new("wide", long.clone()).detail(long)]));
        }
    }

    let h = Harness::new(Wide, 12, 1);
    // The detail keeps its right anchor and covers the row; the label has no room left.
    assert_eq!(h.screen(), "nnnnnnnnnnn\n");
}

// Reordering and the context menu.

/// A category of the plan, as an application keeps it.
#[derive(Clone)]
struct Category {
    key: &'static str,
    children: Vec<Category>,
}

fn category(key: &'static str, children: &[&'static str]) -> Category {
    Category { key, children: children.iter().map(|key| Category { key, children: Vec::new() }).collect() }
}

struct Plan {
    roots: Vec<Category>,
    open: BTreeSet<String>,
    selected: Option<String>,
    moves: Vec<TreeMove>,
    chosen: Vec<(&'static str, String)>,
    activated: Vec<String>,
    reorderable: bool,
    menu: bool,
}

#[derive(Clone)]
enum PlanMsg {
    Select(String),
    Activate(String),
    Expand(String, bool),
    Move(TreeMove),
    Chosen(&'static str, String),
}

impl Plan {
    fn new() -> Self {
        Self {
            roots: vec![
                category("work", &["rust", "writing", "review"]),
                category("health", &["running", "sleep"]),
                category("study", &[]),
            ],
            open: ["work".to_owned()].into(),
            selected: None,
            moves: Vec::new(),
            chosen: Vec::new(),
            activated: Vec::new(),
            reorderable: true,
            menu: true,
        }
    }

    fn order(&self, parent: Option<&str>) -> Vec<&'static str> {
        let list = match parent {
            None => &self.roots,
            Some(parent) => &self.roots.iter().find(|root| root.key == parent).expect("a root").children,
        };
        list.iter().map(|category| category.key).collect()
    }

    fn node(&self, category: &Category) -> TreeNode {
        let node = TreeNode::new(category.key, category.key).expanded(self.open.contains(category.key));
        node.children(category.children.iter().map(|child| self.node(child)))
    }
}

impl App for Plan {
    type Msg = PlanMsg;
    fn update(&mut self, msg: PlanMsg) -> Command<PlanMsg> {
        match msg {
            PlanMsg::Select(key) => self.selected = Some(key),
            PlanMsg::Activate(key) => self.activated.push(key),
            PlanMsg::Expand(key, true) => {
                self.open.insert(key);
            }
            PlanMsg::Expand(key, false) => {
                self.open.remove(&key);
            }
            PlanMsg::Move(step) => {
                let siblings = match step.parent.as_deref() {
                    None => &mut self.roots,
                    Some(parent) => {
                        &mut self.roots.iter_mut().find(|root| root.key == parent).expect("a root").children
                    }
                };
                step.apply(siblings);
                self.moves.push(step);
            }
            PlanMsg::Chosen(action, key) => self.chosen.push((action, key)),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, PlanMsg>) {
        let mut tree = Tree::new(self.roots.iter().map(|root| self.node(root)))
            .selected(self.selected.as_deref())
            .on_select(|key| PlanMsg::Select(key.to_owned()))
            .on_activate(|key| PlanMsg::Activate(key.to_owned()))
            .on_expand(|key, open| PlanMsg::Expand(key.to_owned(), open));
        if self.reorderable {
            tree = tree.reorderable(PlanMsg::Move);
        }
        if self.menu {
            tree = tree.context_menu(|key| {
                vec![
                    ContextItem::new("Rename", PlanMsg::Chosen("rename", key.to_owned())),
                    ContextItem::new("Archive", PlanMsg::Chosen("archive", key.to_owned())),
                ]
            });
        }
        ui.add(tree).fill().id("plan");
    }
}

fn plan(height: u16) -> Harness<Plan> {
    let mut h = Harness::new(Plan::new(), 26, height);
    h.set_glyph_mode(GlyphMode::Unicode);
    h.set_reduced_motion(true);
    h
}

fn moved(key: &str, parent: Option<&str>, from: usize, to: usize) -> TreeMove {
    TreeMove { key: key.to_owned(), parent: parent.map(str::to_owned), from, to }
}

#[test]
fn ctrl_shift_arrows_move_the_selected_node_among_its_siblings() {
    let mut h = plan(8);
    assert_eq!(h.screen(), "  ▾ work\n      rust\n      writing\n      review\n  ▸ health\n    study\n\n\n");
    h.press("tab").press("down").press("down").press("ctrl+shift+down");
    assert_eq!(h.app().moves, [moved("rust", Some("work"), 0, 1)]);
    assert_eq!(h.app().order(Some("work")), ["writing", "rust", "review"]);
    // The selection follows its key: the moved row still slides its label alone, one cell.
    assert_eq!(h.screen().lines().nth(2), Some("▌      rust"), "{}", h.screen());
    h.press("ctrl+shift+up").press("ctrl+shift+up");
    assert_eq!(h.app().order(Some("work")), ["rust", "writing", "review"]);
    assert_eq!(h.app().moves.len(), 2, "the first place is the end: nothing to send");
    h.press("up").press("ctrl+shift+down").press("ctrl+shift+down");
    assert_eq!(h.app().order(None), ["health", "study", "work"], "top-level nodes move among themselves");
    assert_eq!(h.app().moves.last(), Some(&moved("work", None, 1, 2)));
    assert!(h.app().activated.is_empty() && h.app().open.contains("work"));
}

#[test]
fn a_drag_moves_a_node_with_a_ghost_and_its_siblings_making_room() {
    let mut h = plan(8);
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 3).mouse(MouseKind::Drag(MouseButton::Left), 8, 1);
    assert_eq!(h.app().selected.as_deref(), Some("review"), "the press selects");
    assert!(h.app().activated.is_empty(), "a press to drag does not activate");
    let screen = h.screen();
    assert_eq!(screen.lines().nth(1), Some("      review"), "the ghost under the pointer:\n{screen}");
    assert_eq!(screen.lines().nth(2), Some("      rust"), "the siblings make room:\n{screen}");
    assert_eq!(screen.lines().nth(3), Some("      writing"), "{screen}");
    let ghost = h.env().theme().style("tab-ghost", None, &[]).paint("bg").map(|paint| paint.at(0.0));
    assert_eq!(h.bg(20, 1), ghost);
    h.mouse(MouseKind::Up(MouseButton::Left), 8, 1);
    assert_eq!(h.app().moves, [moved("review", Some("work"), 2, 0)]);
    assert_eq!(h.app().order(Some("work")), ["review", "rust", "writing"]);
    assert!(h.app().activated.is_empty(), "a drop does not activate");
}

#[test]
fn a_dragged_node_keeps_its_parent_and_folds_its_children() {
    let mut h = plan(8);
    // Past the last sibling, onto another parent's rows: the node lands last among its siblings.
    h.drag((8, 1), (8, 5));
    assert_eq!(h.app().moves, [moved("rust", Some("work"), 0, 2)]);
    // Dragging an open node folds it while the drag lasts; its siblings take the rows. Over a
    // child of a sibling the node lands after that sibling, in the tinted slot below its children.
    h.click(2, 4);
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 0).mouse(MouseKind::Drag(MouseButton::Left), 8, 2);
    let screen = h.screen();
    assert!(!screen.contains("writing"), "the children travel folded:\n{screen}");
    assert_eq!(screen.lines().nth(2), Some("  ▾ work"), "the ghost follows the pointer:\n{screen}");
    let slot = h.env().theme().style("tab-drop", None, &[]).paint("bg").map(|paint| paint.at(0.0));
    assert_eq!(h.bg(20, 3), slot, "the landing slot:\n{screen}");
    assert_eq!(screen.lines().nth(4), Some("    study"), "{screen}");
    h.mouse(MouseKind::Up(MouseButton::Left), 8, 2);
    assert_eq!(h.app().order(None), ["health", "work", "study"]);
    assert!(h.app().open.contains("work"), "and it opens again where it lands");
}

#[test]
fn with_reordering_a_click_acts_on_release_and_the_chevron_still_only_toggles() {
    let mut h = plan(8);
    h.click(8, 4);
    assert!(h.app().open.contains("health"), "a click on a parent opens it");
    assert_eq!(h.app().selected.as_deref(), Some("health"));
    h.click_text("sleep");
    assert_eq!(h.app().activated, ["sleep"]);
    h.click(2, 0);
    assert!(!h.app().open.contains("work"), "the chevron closes");
    assert_eq!(h.app().selected.as_deref(), Some("sleep"), "and selects nothing");
    assert!(h.app().moves.is_empty());
}

#[test]
fn a_drag_held_on_the_bottom_row_scrolls_the_tree() {
    let mut h = plan(3);
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 1).mouse(MouseKind::Drag(MouseButton::Left), 8, 2);
    assert!(!h.screen().contains("review"), "{}", h.screen());
    h.advance(Duration::from_millis(399));
    assert!(!h.screen().contains("review"), "passing over the edge does not scroll:\n{}", h.screen());
    h.advance(Duration::from_millis(1));
    assert!(h.screen().contains("review"), "the rows below come into view:\n{}", h.screen());
    h.mouse(MouseKind::Up(MouseButton::Left), 8, 2);
    assert_eq!(h.app().moves, [moved("rust", Some("work"), 0, 2)]);
}

#[test]
fn right_click_opens_the_menu_of_that_node_and_keeps_its_row_raised() {
    let mut h = plan(8);
    let resting = h.bg(20, 2);
    h.mouse(MouseKind::Down(MouseButton::Right), 8, 2).mouse(MouseKind::Up(MouseButton::Right), 8, 2);
    assert!(h.screen().contains("Archive"), "{}", h.screen());
    assert_ne!(h.bg(20, 2), resting, "the row the menu acts on stays raised");
    assert_eq!(h.app().selected, None, "a right click selects nothing");
    h.click_text("Archive");
    assert_eq!(h.app().chosen, [("archive", "writing".to_owned())]);
    assert!(!h.screen().contains("Archive"));
    h.mouse(MouseKind::Down(MouseButton::Right), 8, 7);
    assert!(!h.screen().contains("Rename"), "below the rows there is no node to ask about");
}

#[test]
fn the_menu_key_opens_the_menu_of_the_selected_node() {
    let mut h = plan(8);
    h.press("tab").press("down").press("down").press("down").press("shift+f10");
    assert!(h.screen().contains("Rename"), "{}", h.screen());
    h.press("enter");
    assert_eq!(h.app().chosen, [("rename", "writing".to_owned())]);
    h.press("menu").press("down").press("enter");
    assert_eq!(h.app().chosen.last(), Some(&("archive", "writing".to_owned())));
}

#[test]
fn without_the_options_the_tree_behaves_as_before() {
    let mut app = Plan::new();
    app.reorderable = false;
    app.menu = false;
    let mut h = Harness::new(app, 26, 8);
    h.set_glyph_mode(GlyphMode::Unicode);
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 3);
    assert_eq!(h.app().activated, ["review"], "a press activates at once, as before");
    h.mouse(MouseKind::Drag(MouseButton::Left), 8, 1).mouse(MouseKind::Up(MouseButton::Left), 8, 1);
    h.press("ctrl+shift+up");
    h.mouse(MouseKind::Down(MouseButton::Right), 8, 2);
    assert!(h.app().moves.is_empty() && !h.screen().contains("Rename"), "{}", h.screen());
}

#[test]
fn a_drag_reads_the_same_in_ascii() {
    let mut h = plan(8);
    h.set_glyph_mode(GlyphMode::Ascii);
    h.mouse(MouseKind::Down(MouseButton::Left), 8, 0).mouse(MouseKind::Drag(MouseButton::Left), 8, 2);
    let screen = h.screen();
    assert!(screen.is_ascii() && !screen.contains(['[', ']', '|']), "{screen}");
}

#[test]
fn tiny_trees_survive_drags_and_menus() {
    for (width, height) in [(0, 0), (1, 1), (4, 1), (26, 1)] {
        let mut h = Harness::new(Plan::new(), width, height);
        h.drag((1, 0), (1, 3)).mouse(MouseKind::Down(MouseButton::Right), 1, 0).press("esc");
        h.press("tab").press("ctrl+shift+down").press("shift+f10").press("esc");
    }
}
