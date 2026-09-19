//! Tree: a project that opens and closes, folders read from disk when they open, a folder with
//! fifty thousand files, focus categories that are reordered and managed from a menu, and files
//! selected several at a time and dragged into folders.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qframe::prelude::*;
use qframe::widgets::{ContextItem, Select, Tree, TreeDrop, TreeMove, TreeNode};

use super::{PageMsg, setting, slide_setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "tree";

/// Files of the project shown in the demo; folders come from the paths.
const PROJECT: [&str; 14] = [
    "crates/quvyta-framework/src/widgets/table.rs",
    "crates/quvyta-framework/src/widgets/tree.rs",
    "crates/quvyta-framework/src/widgets/list.rs",
    "crates/quvyta-framework/src/runtime/engine.rs",
    "crates/quvyta-framework/src/lib.rs",
    "crates/quvyta-framework/Cargo.toml",
    "crates/showcase/src/main.rs",
    "crates/showcase/CATALOG.toml",
    "crates/showcase/Cargo.toml",
    "docs/guide/getting-started.md",
    "docs/design/framework.md",
    "Cargo.toml",
    "LICENSE",
    "rustfmt.toml",
];

/// Files in the very large folder.
const MANY: usize = 50_000;

/// Key prefix of nodes read from disk.
const DISK: &str = "disk:";

/// What reading a folder gave: entries (name, is a folder) or an error text.
type Listing = Result<Vec<(String, bool)>, String>;

/// A focus category of the reorder demo: its key names its label in the language files.
#[derive(Debug, Clone)]
pub struct Category {
    key: &'static str,
    archived: bool,
    children: Vec<Category>,
}

fn category(key: &'static str, children: &[&'static str]) -> Category {
    let children = children.iter().map(|key| Category { key, archived: false, children: Vec::new() }).collect();
    Category { key, archived: false, children }
}

fn categories() -> Vec<Category> {
    vec![
        category("work", &["work/rust", "work/writing", "work/review"]),
        category("health", &["health/running", "health/sleep"]),
        category("learning", &["learning/japanese"]),
    ]
}

/// A file or folder of the moving demo; its name is its key, since names are unique there.
#[derive(Debug, Clone)]
pub struct FileItem {
    name: &'static str,
    /// `Some` for a folder.
    children: Option<Vec<FileItem>>,
}

fn leaf(name: &'static str) -> FileItem {
    FileItem { name, children: None }
}

fn folder(name: &'static str, children: Vec<FileItem>) -> FileItem {
    FileItem { name, children: Some(children) }
}

fn sample_files() -> Vec<FileItem> {
    vec![
        folder("notes", vec![leaf("ideas.md"), leaf("todo.md")]),
        folder("photos", vec![leaf("beach.png"), leaf("city.png")]),
        folder("archive", Vec::new()),
        leaf("budget.csv"),
        leaf("draft.md"),
        leaf("report.pdf"),
    ]
}

/// A menu action on a category.
#[derive(Debug, Clone, Copy)]
pub enum CategoryAction {
    Up,
    Down,
    Archive,
}

/// Open nodes, selection, folders read from disk and playground settings.
#[derive(Debug)]
pub struct State {
    open: BTreeSet<String>,
    selected: Option<String>,
    loading: BTreeSet<String>,
    listings: BTreeMap<String, Listing>,
    contents: usize,
    icons: bool,
    details: bool,
    categories: Vec<Category>,
    category_open: BTreeSet<String>,
    category_selected: Option<String>,
    files: Vec<FileItem>,
    files_open: BTreeSet<String>,
    files_cursor: Option<String>,
    files_chosen: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        let open = ["crates", "crates/quvyta-framework", "crates/quvyta-framework/src"].map(str::to_owned).into();
        Self {
            open,
            selected: Some("crates/quvyta-framework".to_owned()),
            loading: BTreeSet::new(),
            listings: BTreeMap::new(),
            contents: 0,
            icons: false,
            details: false,
            categories: categories(),
            category_open: ["work".to_owned()].into(),
            category_selected: None,
            files: sample_files(),
            files_open: ["notes".to_owned()].into(),
            files_cursor: None,
            files_chosen: Vec::new(),
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Select(String),
    Activate(String),
    Expand(String, bool),
    Loaded(String, Listing),
    Contents(usize),
    Icons(bool),
    Details(bool),
    CategorySelect(String),
    CategoryExpand(String, bool),
    CategoryMove(TreeMove),
    CategoryMenu(CategoryAction, String),
    FilesCursor(String),
    FilesChoose(Vec<String>),
    FilesExpand(String, bool),
    FilesDrop(TreeDrop),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Tree(message))
}

/// The folder a disk node stands for.
fn disk_path(key: &str) -> Option<PathBuf> {
    key.strip_prefix(DISK).map(PathBuf::from)
}

// region: tree-lazy
fn read_folder(path: &Path) -> Listing {
    let entries = std::fs::read_dir(path).map_err(|error| error.to_string())?;
    let mut out: Vec<(String, bool)> = entries
        .filter_map(Result::ok)
        .map(|entry| (entry.file_name().to_string_lossy().into_owned(), entry.path().is_dir()))
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    Ok(out)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Expand(key, true) => {
            log.push(PAGE, "Tree#files", format!("open {key}"));
            state.open.insert(key.clone());
            if let Some(path) = disk_path(&key)
                && !state.listings.contains_key(&key)
            {
                state.loading.insert(key.clone());
                return Command::perform(move || send(Msg::Loaded(key, read_folder(&path))));
            }
        }
        Msg::Loaded(key, listing) => {
            state.loading.remove(&key);
            state.listings.insert(key, listing);
        }
        // endregion
        Msg::Expand(key, false) => {
            log.push(PAGE, "Tree#files", format!("close {key}"));
            state.open.remove(&key);
        }
        Msg::Select(key) => {
            log.push(PAGE, "Tree#files", format!("selected {key}"));
            state.selected = Some(key);
        }
        Msg::Activate(key) => log.push(PAGE, "Tree#files", format!("activated {key}")),
        Msg::Contents(index) => {
            state.contents = index;
            log.push(PAGE, "Playground", format!("contents = {index}"));
        }
        Msg::Icons(on) => {
            state.icons = on;
            log.push(PAGE, "Playground", format!("icons = {on}"));
        }
        Msg::Details(on) => {
            state.details = on;
            log.push(PAGE, "Playground", format!("details = {on}"));
        }
        Msg::CategorySelect(key) => state.category_selected = Some(key),
        Msg::CategoryExpand(key, open) => {
            if open {
                state.category_open.insert(key);
            } else {
                state.category_open.remove(&key);
            }
        }
        Msg::CategoryMove(step) => move_category(state, &step, log),
        Msg::FilesCursor(key) => state.files_cursor = Some(key),
        Msg::FilesChoose(keys) => {
            log.push(PAGE, "Tree#moving", format!("selected {}", keys.join(", ")));
            state.files_chosen = keys;
        }
        Msg::FilesExpand(key, open) => {
            if open {
                state.files_open.insert(key);
            } else {
                state.files_open.remove(&key);
            }
        }
        Msg::FilesDrop(drop) => {
            let into = drop.into.as_deref().unwrap_or("/");
            log.push(PAGE, "Tree#moving", format!("moved {} into {into}", drop.keys.join(", ")));
            move_files(&mut state.files, &drop);
        }
        Msg::CategoryMenu(action, key) => {
            log.push(PAGE, "Tree#categories", format!("menu {action:?} {key}"));
            if let Some(step) = category_action(state, action, &key) {
                move_category(state, &step, log);
            }
        }
    }
    Command::none()
}

// region: tree-move
/// Applies a move a drag, ctrl+shift+↑/↓ or the menu asked for to the categories.
fn move_category(state: &mut State, step: &TreeMove, log: &mut EventLog) {
    log.push(PAGE, "Tree#categories", format!("moved {} from {} to {}", step.key, step.from, step.to));
    if let Some(siblings) = siblings_of(&mut state.categories, step.parent.as_deref()) {
        step.apply(siblings);
    }
}
// endregion

// region: tree-drop
/// Moves the dropped files into their folder, or to the top level, keeping folders first and
/// names in order the way a file manager lists them.
fn move_files(files: &mut Vec<FileItem>, drop: &TreeDrop) {
    fn take(items: &mut Vec<FileItem>, keys: &[String], out: &mut Vec<FileItem>) {
        let mut at = 0;
        while at < items.len() {
            if keys.iter().any(|key| key == items[at].name) {
                out.push(items.remove(at));
                continue;
            }
            if let Some(children) = &mut items[at].children {
                take(children, keys, out);
            }
            at += 1;
        }
    }
    fn folder_mut<'a>(items: &'a mut [FileItem], key: &str) -> Option<&'a mut Vec<FileItem>> {
        items.iter_mut().find_map(|item| {
            let found = item.name == key;
            let children = item.children.as_mut()?;
            if found { Some(children) } else { folder_mut(children, key) }
        })
    }
    let mut moved = Vec::new();
    take(files, &drop.keys, &mut moved);
    let target = match drop.into.as_deref() {
        Some(key) => match folder_mut(files, key) {
            Some(children) => children,
            None => files,
        },
        None => files,
    };
    target.extend(moved);
    target.sort_by(|a, b| b.children.is_some().cmp(&a.children.is_some()).then_with(|| a.name.cmp(b.name)));
}
// endregion

/// The node of a file or folder of the moving demo.
fn file_node(state: &State, item: &FileItem) -> TreeNode {
    let node = TreeNode::new(item.name, item.name);
    match &item.children {
        Some(children) => node
            .icon("folder", Some("accent"))
            .expandable(true)
            .expanded(state.files_open.contains(item.name))
            .children(children.iter().map(|child| file_node(state, child))),
        None => node.icon("file", None),
    }
}

/// The names of every folder, the nodes that take drops.
fn folder_names(items: &[FileItem], out: &mut Vec<String>) {
    for item in items {
        if let Some(children) = &item.children {
            out.push(item.name.to_owned());
            folder_names(children, out);
        }
    }
}

/// The children of `parent`, or the top-level categories.
fn siblings_of<'a>(categories: &'a mut Vec<Category>, parent: Option<&str>) -> Option<&'a mut Vec<Category>> {
    match parent {
        None => Some(categories),
        Some(parent) => categories.iter_mut().find(|category| category.key == parent).map(|found| &mut found.children),
    }
}

/// The parent key and position of `key` among its siblings.
fn place_of(categories: &[Category], key: &str) -> Option<(Option<&'static str>, usize, usize)> {
    if let Some(at) = categories.iter().position(|category| category.key == key) {
        return Some((None, at, categories.len()));
    }
    categories.iter().find_map(|parent| {
        let at = parent.children.iter().position(|child| child.key == key)?;
        Some((Some(parent.key), at, parent.children.len()))
    })
}

/// A menu action on `key`: the move one place up or down, the same a drag or ctrl+shift+↑/↓
/// asks for, or archiving, done here.
fn category_action(state: &mut State, action: CategoryAction, key: &str) -> Option<TreeMove> {
    let (parent, from, count) = place_of(&state.categories, key)?;
    let to = match action {
        CategoryAction::Up if from > 0 => from - 1,
        CategoryAction::Down if from + 1 < count => from + 1,
        CategoryAction::Up | CategoryAction::Down => return None,
        CategoryAction::Archive => {
            let category = siblings_of(&mut state.categories, parent)?.get_mut(from)?;
            category.archived = !category.archived;
            return None;
        }
    };
    Some(TreeMove { key: key.to_owned(), parent: parent.map(str::to_owned), from, to })
}

// region: tree-nodes
/// Nodes for the paths under `prefix`: folders first, each folder with its own children.
fn project_nodes(state: &State, prefix: &str) -> Vec<TreeNode> {
    let mut folders = BTreeSet::new();
    let mut files = Vec::new();
    for path in PROJECT.iter().filter_map(|path| path.strip_prefix(prefix)) {
        match path.split_once('/') {
            Some((folder, _)) => {
                folders.insert(folder);
            }
            None => files.push(path),
        }
    }
    let mut nodes: Vec<TreeNode> = folders
        .into_iter()
        .map(|folder| {
            let key = format!("{prefix}{folder}");
            let children = project_nodes(state, &format!("{key}/"));
            let count = children.len();
            let mut node = TreeNode::new(key.as_str(), folder).expanded(state.open.contains(&key)).children(children);
            if state.icons {
                node = node.icon("folder", Some("accent"));
            }
            if state.details { node.detail(count.to_string()) } else { node }
        })
        .collect();
    nodes.extend(files.into_iter().map(|file| {
        let node = TreeNode::new(format!("{prefix}{file}"), file);
        if state.icons { node.icon("file", None) } else { node }
    }));
    nodes
}
// endregion

/// The showcase's own asset folder, read from disk when opened.
fn disk_node(state: &State, key: &str, name: String) -> TreeNode {
    let open = state.open.contains(key);
    let mut node = TreeNode::new(key, name).expandable(true).expanded(open);
    if state.icons {
        node = node.icon("folder", Some("accent"));
    }
    node = node.loading(state.loading.contains(key));
    match state.listings.get(key) {
        Some(Ok(entries)) => node.children(entries.iter().map(|(name, folder)| {
            let child = format!("{key}/{name}");
            if *folder {
                disk_node(state, &child, name.clone())
            } else {
                let leaf = TreeNode::new(child, name.as_str());
                if state.icons { leaf.icon("file", None) } else { leaf }
            }
        })),
        Some(Err(error)) => node.children([TreeNode::new(format!("{key}#error"), error.as_str())
            .icon("error", Some("danger"))
            .faint(true)]),
        None => node,
    }
}

fn category_node(state: &State, category: &Category) -> TreeNode {
    let label = t!(&format!("tree.category.{}", category.key.replace('/', "-")));
    TreeNode::new(category.key, label)
        .faint(category.archived)
        .expanded(state.category_open.contains(category.key))
        .children(category.children.iter().map(|child| category_node(state, child)))
}

// region: tree-menu
/// The actions of a category: moving it among its siblings from the keyboard's menu too, and
/// archiving it.
fn category_menu(categories: &[Category], key: &str) -> Vec<ContextItem<AppMsg>> {
    let (_, at, count) = place_of(categories, key).unwrap_or((None, 0, 1));
    let archived = categories
        .iter()
        .flat_map(|category| std::iter::once(category).chain(&category.children))
        .any(|category| category.key == key && category.archived);
    let item = |label: String, action: CategoryAction| {
        ContextItem::new(label, send(Msg::CategoryMenu(action, key.to_owned())))
    };
    vec![
        item(t!("tree.move-up"), CategoryAction::Up).shortcut("ctrl shift ↑").disabled(at == 0),
        item(t!("tree.move-down"), CategoryAction::Down).shortcut("ctrl shift ↓").disabled(at + 1 >= count),
        ContextItem::gap(),
        item(t!(if archived { "tree.restore" } else { "tree.archive" }), CategoryAction::Archive),
    ]
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        let roots = match state.contents {
            0 => {
                let mut roots = project_nodes(state, "");
                let home = super::home_folder();
                roots.insert(0, disk_node(state, &format!("{DISK}{}", home.display()), t!("tree.disk")));
                roots
            }
            1 => {
                let key = "logs".to_owned();
                let files = (1..=MANY).map(|n| TreeNode::new(format!("logs/{n}"), format!("request-{n:05}.log")));
                vec![TreeNode::new(key.as_str(), "logs").expanded(state.open.contains(&key)).children(files)]
            }
            _ => Vec::new(),
        };
        // region: tree-widget
        let tree = Tree::new(roots)
            .selected(state.selected.as_deref())
            .empty_text(t!("tree.empty"))
            .on_select(|key| send(Msg::Select(key.to_owned())))
            .on_activate(|key| send(Msg::Activate(key.to_owned())))
            .on_expand(|key, open| send(Msg::Expand(key.to_owned(), open)));
        ui.add(tree).width(Length::Fill(1)).height(Length::Cells(12)).id("files");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("tree.reorder")).gap(0), |ui| {
        ui.add(Text::new(t!("tree.reorder-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: tree-reorder
        let nodes = state.categories.iter().map(|category| category_node(state, category));
        let tree = Tree::new(nodes)
            .selected(state.category_selected.as_deref())
            .on_select(|key| send(Msg::CategorySelect(key.to_owned())))
            .on_expand(|key, open| send(Msg::CategoryExpand(key.to_owned(), open)))
            .reorderable(|step| send(Msg::CategoryMove(step)))
            .context_menu({
                let categories = state.categories.clone();
                move |key| category_menu(&categories, key)
            });
        ui.add(tree).width(Length::Fill(1)).height(Length::Cells(9)).id("categories");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("tree.moving")).gap(0), |ui| {
        ui.add(Text::new(t!("tree.moving-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: tree-multi
        let mut folders = Vec::new();
        folder_names(&state.files, &mut folders);
        let nodes = state.files.iter().map(|item| file_node(state, item));
        let to_top = TreeDrop { keys: state.files_chosen.clone(), into: None };
        let tree = Tree::new(nodes)
            .selected(state.files_cursor.as_deref())
            .on_select(|key| send(Msg::FilesCursor(key.to_owned())))
            .on_expand(|key, open| send(Msg::FilesExpand(key.to_owned(), open)))
            .multi_select(&state.files_chosen, |keys| send(Msg::FilesChoose(keys)))
            .droppable(|drop| send(Msg::FilesDrop(drop)), move |key| folders.iter().any(|folder| folder == key))
            // The menu of a selected row acts on the whole selection, which the state holds.
            .context_menu(move |_| vec![ContextItem::new(t!("tree.to-top"), send(Msg::FilesDrop(to_top.clone())))]);
        ui.add(tree).width(Length::Fill(1)).height(Length::Cells(11)).id("moving");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("tree.contents"), |ui| {
            let names = [t!("tree.project"), t!("tree.many"), t!("tree.nothing")];
            ui.add(Select::new(names).selected(Some(state.contents)).on_select(|i| send(Msg::Contents(i))))
                .width(Length::Cells(22))
                .id("contents");
        });
        setting(ui, t!("tree.icons"), |ui| {
            ui.add(toggle(state.icons, |on| send(Msg::Icons(on)))).id("icons");
        });
        setting(ui, t!("tree.details"), |ui| {
            ui.add(toggle(state.details, |on| send(Msg::Details(on)))).id("details");
        });
        slide_setting(ui, PAGE);
        ui.add(Text::new(t!("tree.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn opens_folders_reads_disk_and_handles_huge_folders() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("widgets"), "{}", h.screen());
        h.click_text("docs");
        assert!(h.app().pages.tree.open.contains("docs"));
        assert!(h.screen().contains("guide"));
        // Tests read this crate's folder where the installed program reads the home folder.
        h.click_text("Home folder");
        assert!(h.screen().contains("assets"), "{}", h.screen());
        h.send(send(Msg::Contents(1)));
        h.click_text("logs").press("end");
        let screen = h.screen();
        assert!(screen.contains("request-50000.log"), "{screen}");
        h.send(send(Msg::Contents(2)));
        assert!(h.screen().contains("No files"));
    }

    fn order(h: &qframe::runtime::Harness<crate::app::Showcase>) -> Vec<&'static str> {
        h.app().pages.tree.categories[0].children.iter().map(|category| category.key).collect()
    }

    #[test]
    fn categories_move_by_drag_by_keys_and_from_the_menu() {
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::default(), PAGE, 60);
        h.set_reduced_motion(true);
        let (x, y) = h.find("Code review").expect("the categories demo");
        let (_, rust) = h.find("Rust").expect("the first work category");
        h.drag((x, y), (x, rust));
        assert_eq!(order(&h), ["work/review", "work/rust", "work/writing"], "{}", h.screen());
        h.press("ctrl+shift+down");
        assert_eq!(order(&h), ["work/rust", "work/review", "work/writing"], "the keys move the selected category");
        let (x, y) = h.find("Writing").expect("writing");
        crate::tests::right_click(&mut h, x, y);
        h.click_text("Move up");
        assert_eq!(order(&h), ["work/rust", "work/writing", "work/review"]);
        let (x, y) = h.find("Writing").expect("writing");
        crate::tests::right_click(&mut h, x, y);
        h.click_text("Archive");
        assert!(h.app().pages.tree.categories[0].children[1].archived);
        let log: Vec<String> = h.app().log.recent(PAGE, 3).iter().map(|entry| entry.message.clone()).collect();
        assert!(log.iter().any(|line| line == "moved work/writing from 2 to 1"), "{log:?}");
    }

    #[test]
    fn files_are_selected_together_and_dragged_into_a_folder() {
        use qframe::event::{Event, MouseButton, MouseEvent, MouseKind};
        use qframe::keymap::Modifiers;
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::default(), PAGE, 80);
        h.set_reduced_motion(true);
        let (x, y) = h.find("budget.csv").expect("the moving demo");
        h.click(x, y);
        let (_, report) = h.find("report.pdf").expect("report");
        let shift = Modifiers { shift: true, ..Modifiers::default() };
        let event = |kind| Event::Mouse(MouseEvent { kind, x, y: report, mods: shift });
        h.events(&[event(MouseKind::Down(MouseButton::Left)), event(MouseKind::Up(MouseButton::Left))]);
        assert_eq!(h.app().pages.tree.files_chosen, ["budget.csv", "draft.md", "report.pdf"]);
        let (_, archive) = h.find("archive").expect("archive");
        h.drag((x, report), (x, archive));
        let names = |items: &[FileItem]| items.iter().map(|item| item.name).collect::<Vec<_>>();
        let files = &h.app().pages.tree.files;
        assert_eq!(names(files), ["notes", "photos", "archive"], "{}", h.screen());
        assert_eq!(names(files[2].children.as_deref().unwrap_or(&[])), ["budget.csv", "draft.md", "report.pdf"]);
        // The drop was quick, so archive stayed closed; opening it shows the files, still selected.
        h.send(send(Msg::FilesExpand("archive".to_owned(), true)));
        let (_, draft) = h.find("draft.md").expect("the moved files show");
        crate::tests::right_click(&mut h, x, draft);
        h.click_text("Move to the top");
        assert_eq!(
            names(&h.app().pages.tree.files),
            ["archive", "notes", "photos", "budget.csv", "draft.md", "report.pdf"]
        );
    }

    #[test]
    fn chevrons_stay_put_while_icon_and_name_slide() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Icons(true)));
        // The folder icon draws `■` in Unicode mode, so the chevron is the `▸` before it.
        let (chevron, label) = super::super::mark_and_label(&h, '▸', "■ docs");
        assert_eq!(label, chevron + 2);
        let (x, y) = h.find("docs").expect("docs row");
        h.hover(x + 1, y);
        assert_eq!(super::super::mark_and_label(&h, '▸', "■ docs"), (chevron, chevron + 3), "only icon and name slide");
        h.click(i32::try_from(chevron).unwrap_or(0), y);
        assert!(h.app().pages.tree.open.contains("docs"), "the chevron is where it was drawn");
        assert_ne!(h.app().pages.tree.selected.as_deref(), Some("docs"), "and it only opens");
    }
}
