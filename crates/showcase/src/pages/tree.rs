//! Tree: a project that opens and closes, folders read from disk when they open, and a folder
//! with fifty thousand files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qframe::prelude::*;
use qframe::widgets::{Select, Tree, TreeNode};

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
    "crates/showcase/Cargo.toml",
    "docs/guide/getting-started.md",
    "docs/design/framework.md",
    "CATALOG.toml",
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
    }
    Command::none()
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

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        let roots = match state.contents {
            0 => {
                let mut roots = project_nodes(state, "");
                let assets = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
                roots.insert(0, disk_node(state, &format!("{DISK}{assets}"), t!("tree.disk")));
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
        h.click_text("Showcase assets");
        assert!(h.screen().contains("locales"), "{}", h.screen());
        h.send(send(Msg::Contents(1)));
        h.click_text("logs").press("end");
        let screen = h.screen();
        assert!(screen.contains("request-50000.log"), "{screen}");
        h.send(send(Msg::Contents(2)));
        assert!(h.screen().contains("No files"));
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
