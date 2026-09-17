//! Example application: a file explorer over this repository built from Tree, Table, a path,
//! a code or Markdown preview and a status line.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use qframe::prelude::*;
use qframe::widgets::{
    CodeView, Column, ColumnWidth, FileEntry, Language, Listing, Markdown, ScrollView, SortDirection, Table, TableCell,
    TableRow, Tree, TreeNode, read_folder,
};

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "example-file-explorer";

/// The most of a file the preview reads.
const PREVIEW_BYTES: u64 = 64 * 1024;

/// The most lines the preview shows.
const PREVIEW_LINES: usize = 400;

/// A folder as the explorer knows it.
#[derive(Debug, Clone)]
enum Folder {
    Loading,
    Ready(Arc<[FileEntry]>),
    Failed,
}

/// What the preview pane shows.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Preview {
    Nothing,
    Text(PathBuf, String),
    Binary(PathBuf),
    Failed(PathBuf, String),
}

/// Folders read so far, the open and current folder, the selected row, the sort and the preview.
///
/// A folder or file on its way (`going`, `previewing`) does not replace what is shown: the table
/// and the preview keep the last answer until the new one arrives, so quick reads never flash.
#[derive(Debug)]
pub struct State {
    root: PathBuf,
    folders: BTreeMap<PathBuf, Folder>,
    open: BTreeSet<PathBuf>,
    current: PathBuf,
    going: Option<PathBuf>,
    previewing: Option<PathBuf>,
    entries: Vec<FileEntry>,
    selected: Option<usize>,
    sort: (usize, SortDirection),
    preview: Preview,
}

impl Default for State {
    fn default() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = root.canonicalize().unwrap_or(root);
        // Page states are built once at start-up, before any view; the first folder is read here so
        // the example opens ready. Every later read runs in a background command.
        let listing = read_folder(&root);
        let mut state = Self {
            folders: BTreeMap::new(),
            open: BTreeSet::from([root.clone()]),
            current: root.clone(),
            going: None,
            previewing: None,
            root,
            entries: Vec::new(),
            selected: None,
            sort: (0, SortDirection::Ascending),
            preview: Preview::Nothing,
        };
        let root = state.root.clone();
        state.store(&root, listing);
        state
    }
}

/// Explorer messages.
#[derive(Debug, Clone)]
pub enum Msg {
    FolderSelected(String),
    FolderExpanded(String, bool),
    Loaded(PathBuf, Listing),
    RowSelected(usize),
    RowOpened(usize),
    Sort(usize, SortDirection),
    PreviewLoaded(PathBuf, Result<Option<String>, String>),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ExampleFileExplorer(message))
}

// region: explorer-io
/// Reads a folder on a background thread.
fn load_folder(path: PathBuf) -> Command<AppMsg> {
    Command::perform(move || {
        let listing = read_folder(&path);
        send(Msg::Loaded(path, listing))
    })
}

/// Reads the start of a file on a background thread: text, `None` for binary data, or an error.
fn load_preview(path: PathBuf) -> Command<AppMsg> {
    Command::perform(move || {
        let read = std::fs::File::open(&path).and_then(|file| {
            let mut bytes = Vec::new();
            file.take(PREVIEW_BYTES).read_to_end(&mut bytes).map(|_| bytes)
        });
        let result = match read {
            Ok(bytes) if bytes.contains(&0) => Ok(None),
            Ok(bytes) => {
                Ok(Some(String::from_utf8_lossy(&bytes).lines().take(PREVIEW_LINES).collect::<Vec<_>>().join("\n")))
            }
            Err(error) => Err(error.to_string()),
        };
        send(Msg::PreviewLoaded(path, result))
    })
}
// endregion

impl State {
    fn store(&mut self, path: &Path, listing: Listing) {
        let folder = match listing {
            Ok(entries) => Folder::Ready(entries),
            Err(_) => Folder::Failed,
        };
        self.folders.insert(path.to_path_buf(), folder);
        if self.going.as_deref() == Some(path) {
            self.arrive(path.to_path_buf());
        } else if path == self.current {
            self.refresh_entries();
        }
    }

    /// Shows `path`, whose listing is known, as the current folder.
    fn arrive(&mut self, path: PathBuf) {
        self.going = None;
        self.previewing = None;
        self.current = path;
        self.preview = Preview::Nothing;
        self.refresh_entries();
    }

    fn refresh_entries(&mut self) {
        self.entries = match self.folders.get(&self.current) {
            Some(Folder::Ready(entries)) => entries.iter().filter(|e| !e.is_hidden()).cloned().collect(),
            _ => Vec::new(),
        };
        let (column, direction) = self.sort;
        self.entries.sort_by(|a, b| {
            let order = match column {
                1 => a.size().cmp(&b.size()),
                _ => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
            };
            let order = if direction == SortDirection::Descending { order.reverse() } else { order };
            // Folders stay on top whichever way the rest is sorted.
            b.is_folder().cmp(&a.is_folder()).then(order)
        });
        self.selected = (!self.entries.is_empty()).then_some(0);
    }

    /// Makes `path` the current folder: at once when it was read, otherwise when its listing
    /// arrives, with the folder shown until then left as it is.
    fn go_to(&mut self, path: PathBuf) -> Command<AppMsg> {
        // Opening a folder from the table opens its ancestors in the tree too.
        let mut ancestor = path.parent();
        while let Some(folder) = ancestor.filter(|folder| folder.starts_with(&self.root)) {
            self.open.insert(folder.to_path_buf());
            ancestor = folder.parent();
        }
        match self.folders.get(&path) {
            Some(Folder::Ready(_) | Folder::Failed) => {
                self.arrive(path);
                Command::none()
            }
            // Already being read for the tree: its answer will bring us there.
            Some(Folder::Loading) => {
                self.going = Some(path);
                Command::none()
            }
            None => {
                self.folders.insert(path.clone(), Folder::Loading);
                self.going = Some(path.clone());
                load_folder(path)
            }
        }
    }

    /// Reads the start of the file at `path`; the preview shown stays until it arrives.
    fn preview_file(&mut self, path: PathBuf) -> Command<AppMsg> {
        self.previewing = Some(path.clone());
        load_preview(path)
    }
}

/// Applies an explorer message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: explorer-update
        Msg::FolderSelected(key) => {
            log.push(PAGE, "Tree#folders", format!("selected {key}"));
            state.go_to(PathBuf::from(key))
        }
        Msg::FolderExpanded(key, open) => {
            let path = PathBuf::from(key);
            if !open {
                state.open.remove(&path);
                return Command::none();
            }
            state.open.insert(path.clone());
            if state.folders.contains_key(&path) {
                return Command::none();
            }
            state.folders.insert(path.clone(), Folder::Loading);
            load_folder(path)
        }
        Msg::Loaded(path, listing) => {
            state.store(&path, listing);
            Command::none()
        }
        Msg::RowSelected(index) => {
            state.selected = Some(index);
            let Some(entry) = state.entries.get(index) else { return Command::none() };
            if entry.is_folder() {
                state.previewing = None;
                state.preview = Preview::Nothing;
                return Command::none();
            }
            let path = state.current.join(entry.name());
            state.preview_file(path)
        }
        Msg::RowOpened(index) => {
            let Some(entry) = state.entries.get(index) else { return Command::none() };
            let path = state.current.join(entry.name());
            log.push(PAGE, "Table#entries", format!("opened {}", entry.name()));
            if entry.is_folder() {
                return state.go_to(path);
            }
            state.preview_file(path)
        }
        // endregion
        Msg::Sort(column, direction) => {
            state.sort = (column, direction);
            state.refresh_entries();
            log.push(PAGE, "Table#entries", format!("sort {column} {direction:?}"));
            Command::none()
        }
        Msg::PreviewLoaded(path, result) => {
            if state.previewing.as_ref() == Some(&path) {
                state.previewing = None;
                state.preview = match result {
                    Ok(Some(text)) => Preview::Text(path, text),
                    Ok(None) => Preview::Binary(path),
                    Err(error) => Preview::Failed(path, error),
                };
            }
            Command::none()
        }
    }
}

/// Human file sizes.
fn size_text(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KiB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MiB", bytes as f64 / 1_048_576.0),
    }
}

// region: explorer-tree
/// Folder nodes under `path`, opened as the user opens them.
fn folder_node(state: &State, path: &Path, name: String) -> TreeNode {
    let key = path.to_string_lossy().into_owned();
    let open = state.open.contains(path);
    let node = TreeNode::new(key.as_str(), name).expandable(true).expanded(open);
    match state.folders.get(path) {
        Some(Folder::Ready(entries)) => node.children(
            entries
                .iter()
                .filter(|entry| entry.is_folder() && !entry.is_hidden())
                .map(|entry| folder_node(state, &path.join(entry.name()), entry.name().to_owned())),
        ),
        // The folder the explorer is going to spins too, on its own row; the tree shows the
        // spinner only when the read is slow.
        Some(Folder::Loading) => node.loading(open || state.going.as_deref() == Some(path)),
        Some(Folder::Failed) => {
            node.children([TreeNode::new(format!("{key}#error"), t!("example-file-explorer.unreadable"))
                .icon("error", Some("danger"))
                .faint(true)])
        }
        None => node,
    }
}
// endregion

/// The explorer.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("example-file-explorer.title")).gap(1), |ui| {
        path_row(state, ui);
        // region: explorer-layout
        ui.row(|ui| {
            let root_name = state.root.file_name().map_or_else(|| "/".to_owned(), |n| n.to_string_lossy().into_owned());
            let target = state.going.as_ref().unwrap_or(&state.current);
            let tree = Tree::new([folder_node(state, &state.root, root_name)])
                .selected(Some(target.to_string_lossy().as_ref()))
                .on_select(|key| send(Msg::FolderSelected(key.to_owned())))
                .on_expand(|key, open| send(Msg::FolderExpanded(key.to_owned(), open)));
            ui.add(tree).width(Length::Cells(24)).height(Length::Fill(1)).id("folders");
            entries_table(state, ui);
            preview(state, ui);
        })
        .gap(2)
        .height(Length::Cells(22))
        .fill_width();
        // endregion
        status_row(state, ui);
        ui.add(Text::new(t!("example-file-explorer.keys")).role("faint"));
    })
    .fill_width();
}

fn path_row(state: &State, ui: &mut View<'_, AppMsg>) {
    let separator = ui.env().icons().glyph("path-separator").into_owned();
    let relative = state.current.strip_prefix(&state.root).unwrap_or(&state.current);
    let root = state.root.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut spans = vec![Span::new(root).role(if relative.as_os_str().is_empty() { "body" } else { "secondary" })];
    let parts: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    for (index, part) in parts.iter().enumerate() {
        spans.push(Span::new(format!("  {separator}  ")).role("faint"));
        let span = Span::new(part.clone());
        spans.push(if index + 1 == parts.len() { span.role("body").bold() } else { span.role("secondary") });
    }
    ui.add(Text::rich(spans).no_wrap()).fill_width();
}

fn entries_table(state: &State, ui: &mut View<'_, AppMsg>) {
    // The current folder is always one whose answer arrived; a folder on its way is not shown yet.
    if let Some(Folder::Failed) = state.folders.get(&state.current) {
        ui.add(Text::new(t!("example-file-explorer.unreadable")).color("danger")).width(Length::Fill(3));
        return;
    }
    let rows: Vec<TableRow> = state
        .entries
        .iter()
        .map(|entry| {
            let (icon, tone, kind) = if entry.is_folder() {
                ("folder", "accent", t!("example-file-explorer.folder"))
            } else {
                ("file", "muted", extension_kind(entry.name()))
            };
            TableRow::new([
                TableCell::new(entry.name()).icon(icon, Some(tone)),
                TableCell::new(entry.size().map(size_text).unwrap_or_default()),
                TableCell::new(kind),
            ])
        })
        .collect();
    let columns = [
        Column::new(t!("example-file-explorer.name")).min(12).sortable(true),
        Column::new(t!("example-file-explorer.size")).width(ColumnWidth::Fixed(9)).align(Align::End).sortable(true),
        Column::new(t!("example-file-explorer.kind")).width(ColumnWidth::Fixed(7)),
    ];
    let (column, direction) = state.sort;
    let table = Table::new(columns, rows)
        .selected(state.selected)
        .sort(column, direction)
        .empty_text(t!("example-file-explorer.empty"))
        .on_select(|index| send(Msg::RowSelected(index)))
        .on_activate(|index| send(Msg::RowOpened(index)))
        .on_sort(|column, direction| send(Msg::Sort(column, direction)));
    ui.add(table).width(Length::Fill(3)).height(Length::Fill(1)).id("entries");
}

fn extension_kind(name: &str) -> String {
    Path::new(name)
        .extension()
        .map_or_else(|| t!("example-file-explorer.file"), |ext| ext.to_string_lossy().to_lowercase())
}

fn preview(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.column(|ui| match &state.preview {
        Preview::Nothing => {
            ui.add(Text::new(t!("example-file-explorer.no-preview")).role("faint"));
        }
        Preview::Binary(_) => {
            ui.add(Text::new(t!("example-file-explorer.binary")).role("faint"));
        }
        Preview::Failed(_, error) => {
            ui.add(Text::new(error.clone()).color("danger"));
        }
        // region: explorer-preview
        Preview::Text(path, text) => {
            let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            ui.add_with(ScrollView::new(), |ui| match extension.as_str() {
                "md" => {
                    ui.add(Markdown::new(text)).fill_width();
                }
                "rs" => {
                    ui.add(CodeView::new(text.clone(), Language::Rust)).fill_width();
                }
                "toml" => {
                    ui.add(CodeView::new(text.clone(), Language::Toml)).fill_width();
                }
                _ => {
                    ui.add(CodeView::new(text.clone(), Language::Plain)).fill_width();
                }
            })
            .fill()
            .id("preview");
        } // endregion
    })
    .width(Length::Fill(2))
    .height(Length::Fill(1));
}

fn status_row(state: &State, ui: &mut View<'_, AppMsg>) {
    let folders = state.entries.iter().filter(|e| e.is_folder()).count();
    let files = state.entries.len() - folders;
    let bytes: u64 = state.entries.iter().filter_map(FileEntry::size).sum();
    let mut spans = vec![
        Span::new(t!("example-file-explorer.counts", folders = folders, files = files)).role("secondary"),
        Span::new(format!("   {}", size_text(bytes))).role("faint"),
    ];
    if let Some(entry) = state.selected.and_then(|i| state.entries.get(i)) {
        spans.push(Span::new(format!("   {}", entry.name())).role("body"));
    }
    ui.add(Text::rich(spans).no_wrap()).fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn browses_the_repository_and_previews_files() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("Cargo.lock") && screen.contains("crates"), "{screen}");
        let catalog = h.app().pages.example_file_explorer.entries.iter().position(|e| e.name() == "CATALOG.toml");
        h.send(send(Msg::RowSelected(catalog.expect("the catalog is listed"))));
        assert!(h.screen().contains("Every component"), "{}", h.screen());
        let crates = h.app().pages.example_file_explorer.entries.iter().position(|e| e.name() == "crates");
        h.send(send(Msg::RowOpened(crates.expect("crates is listed"))));
        assert!(h.screen().contains("quvyta-framework"), "{}", h.screen());
        assert!(h.app().pages.example_file_explorer.current.ends_with("crates"));
        h.send(send(Msg::Sort(1, SortDirection::Descending)));
        assert_eq!(h.app().pages.example_file_explorer.sort, (1, SortDirection::Descending));
    }

    #[test]
    fn a_folder_on_its_way_leaves_the_table_and_preview_as_they_are() {
        // The read commands are dropped: the test delivers each answer itself, as a slow disk would.
        let mut state = State::default();
        let mut log = EventLog::new();
        let position = |state: &State, name: &str| state.entries.iter().position(|e| e.name() == name);
        let catalog = position(&state, "CATALOG.toml").expect("the catalog is listed");
        let path = state.root.join("CATALOG.toml");
        let _read = update(&mut state, Msg::RowSelected(catalog), &mut log);
        let _answer = update(&mut state, Msg::PreviewLoaded(path, Ok(Some("catalog".into()))), &mut log);
        let readme = position(&state, "README.md").expect("the readme is listed");
        let _read = update(&mut state, Msg::RowSelected(readme), &mut log);
        // The next file is on its way: the catalog's preview stays.
        assert!(matches!(&state.preview, Preview::Text(_, text) if text == "catalog"));
        let docs = state.root.join("docs");
        let entries = state.entries.clone();
        let _read = update(&mut state, Msg::FolderSelected(docs.to_string_lossy().into_owned()), &mut log);
        // So is a folder: the table and the preview stay as they were.
        assert_eq!((&state.current, &state.entries), (&state.root, &entries));
        assert!(matches!(&state.preview, Preview::Text(_, text) if text == "catalog"));
        let _answer = update(&mut state, Msg::Loaded(docs.clone(), read_folder(&docs)), &mut log);
        assert_eq!(state.current, docs);
        assert_eq!(state.preview, Preview::Nothing);
        assert!(position(&state, "design.md").is_some());
    }
}
