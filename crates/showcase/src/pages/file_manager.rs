//! File manager: a folder of this run shown as a tree, with every file operation on it, the
//! application's own menu items and the states an empty, unreadable, narrow or disabled manager
//! shows.

use std::path::{Path, PathBuf};
use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::Task;
use qframe::widgets::{
    ContextItem, FileManager, FileManagerMsg, FileManagerState, FileView, FolderEntry, RowMark, Select,
};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "file-manager";

/// How long a read takes on the playground's slow disk: well past the reading indicator's 300 ms
/// delay, so the slow path can be compared with the instant one.
const SLOW_DISK: Duration = Duration::from_secs(1);

/// The folders the playground can root the manager at, all inside the folder of this run.
const FOLDERS: [&str; 3] = ["files", "empty", "denied"];

/// The shapes the playground can draw the folder in, in the order the switcher offers them.
const VIEWS: [FileView; 3] = [FileView::Tree, FileView::List, FileView::Icons];

/// What each of [`VIEWS`] is called, as locale keys.
const VIEW_NAMES: [&str; 3] = ["tree", "list", "icons"];

/// How wide the manager is drawn while the playground asks for a narrow one.
const NARROW: u16 = 22;

/// How many rows the manager is given.
const ROWS: u16 = 14;

/// How many of the latest things the application was asked for are kept on screen.
const KEPT: usize = 4;

/// Tells the demo folders of two pages in one process apart, which the tests need.
static DEMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A folder of this run for the demo's files, never one of the user's own.
fn demo_dir() -> PathBuf {
    let ticket = DEMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("quvyta-showcase-file-manager-{}-{ticket}", std::process::id()))
}

/// How big the page's own file is, so a copy of it is worth a progress bar rather than over before
/// it is drawn.
const BIG: usize = 6 * 1024 * 1024;

/// Makes the three folders the playground offers, with a small tree in the first.
fn make_demo(demo: &Path) {
    let files = demo.join("files");
    let _ = std::fs::create_dir_all(files.join("notes"));
    let _ = std::fs::create_dir_all(files.join("src"));
    let _ = std::fs::write(files.join("README.md"), "# The harbour\n");
    let _ = std::fs::write(files.join("notes").join("tide.md"), "The tide turns at six.\n");
    let _ = std::fs::write(files.join("src").join("main.rs"), "fn main() {}\n");
    let _ = std::fs::write(files.join("it's $HOME.txt"), "a name with room for trouble\n");
    let _ = std::fs::write(files.join(".hidden.txt"), "the platform hides this one\n");
    // A file big enough that copying it takes a moment, so the progress of a long operation can be
    // seen. It is made of one repeated byte, so it costs nothing to write.
    let _ = std::fs::write(files.join("big.bin"), vec![b'q'; BIG]);
    let _ = std::fs::create_dir_all(demo.join("empty"));
    let denied = demo.join("denied");
    let _ = std::fs::create_dir_all(&denied);
    // A folder the user may not read, made here rather than borrowed from the system, so the
    // unreadable state is shown without going anywhere near the user's own files.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000));
    }
}

/// The manager, the folder of this run it shows, and the playground.
pub struct State {
    /// The folder of this run; taken away with the page.
    demo: PathBuf,
    manager: FileManagerState,
    folder: usize,
    /// The shape the folder is drawn in.
    view: usize,
    confined: bool,
    narrow: bool,
    disabled: bool,
    extras: bool,
    slow: bool,
    /// Whether the application marks the entries its backup would leave out.
    marked: bool,
    /// Whether deleting puts entries in a trash of this page's own.
    trashing: bool,
    /// Whether the entries the platform hides are shown.
    hidden: bool,
    /// The latest things the application was asked for, newest last.
    asked: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        let demo = demo_dir();
        make_demo(&demo);
        let mut state = Self {
            manager: manager(&demo, 0, true, false, false),
            demo,
            folder: 0,
            view: 0,
            confined: true,
            narrow: false,
            disabled: false,
            extras: true,
            slow: false,
            marked: false,
            trashing: false,
            hidden: false,
            asked: Vec::new(),
        };
        // The showcase builds page states once at start-up, before any view; the root is read here
        // so the page opens ready. Every later read runs in a background command.
        read_now(&mut state.manager, FileManagerState::ROOT);
        state
    }
}

impl Drop for State {
    /// Takes the demo folder away with the page, so a run leaves nothing in the temporary folder.
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(self.demo.join("denied"), std::fs::Permissions::from_mode(0o755));
        }
        let _ = std::fs::remove_dir_all(&self.demo);
    }
}

// region: file-manager-state
/// The state the application holds: the folder to show, whether operations may leave it, where a
/// deleted entry goes and whether the hidden entries are shown.
///
/// The trash is a folder of this run, never the person's own: a page nobody asked anything of must
/// not put things in the trash they empty themselves.
fn manager(demo: &Path, folder: usize, confined: bool, trashing: bool, hidden: bool) -> FileManagerState {
    let mut state = FileManagerState::new(demo.join(FOLDERS[folder])).showing_hidden(hidden);
    if confined {
        state = state.confined();
    }
    if trashing {
        state = state.trashing_in(demo.join("Trash"));
    }
    state
}
// endregion

/// Reads the folder `key` here and now, which only start-up and the playground's own reads do.
fn read_now(state: &mut FileManagerState, key: &str) {
    let entries = FolderEntry::read_folder(&state.path(key));
    drop(state.update(FileManagerMsg::Read(key.to_owned(), entries), send));
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Manager(FileManagerMsg),
    /// A file was asked to be opened.
    Open(PathBuf),
    /// A terminal was asked for in this folder.
    Terminal(PathBuf),
    /// The application's own menu item was chosen on this row.
    Mine(String),
    Folder(usize),
    View(usize),
    Confine(bool),
    Narrow(bool),
    Disable(bool),
    Extras(bool),
    Slow(bool),
    Mark(bool),
    Trash(bool),
    Hidden(bool),
}

fn send(message: FileManagerMsg) -> AppMsg {
    AppMsg::Page(PageMsg::FileManager(Msg::Manager(message)))
}

fn send_page(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::FileManager(message))
}

// region: file-manager-slow
/// Reads `key` the way a slow disk would, answering after [`SLOW_DISK`]: the manager's own read is
/// dropped for it, which is all an application has to do to read a folder its own way.
fn read_slowly(state: &FileManagerState, key: String) -> Command<AppMsg> {
    let path = state.path(&key);
    Command::task(Task::new("slow disk", move |cx| {
        if !cx.sleep(SLOW_DISK) {
            return Err("stopped".into());
        }
        Ok(send(FileManagerMsg::Read(key, FolderEntry::read_folder(&path))))
    }))
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: file-manager-update
        Msg::Manager(message) => {
            let slow = state.slow.then(|| slow_key(&message)).flatten();
            let command = state.manager.update(message, send);
            match slow {
                Some(key) => read_slowly(&state.manager, key),
                None => command,
            }
        }
        Msg::Open(path) => {
            // What opening means is the application's own: a tab, a window, or an answer.
            asked(state, log, t!("file-manager.opened", name = shown(&path)))
        }
        // endregion
        Msg::Terminal(path) => asked(state, log, t!("file-manager.in-terminal", name = shown(&path))),
        Msg::Mine(key) => asked(state, log, t!("file-manager.mine", name = key.as_str())),
        Msg::Folder(index) => {
            state.folder = index;
            log.push(PAGE, "Playground", format!("folder = {}", FOLDERS[index]));
            restart(state)
        }
        // region: file-manager-shape
        Msg::View(index) => {
            // The shape is the view's own; what is read and what is shown follow from it, so
            // nothing else has to change when it does.
            state.view = index;
            log.push(PAGE, "Playground", format!("view = {}", VIEW_NAMES[index]));
            Command::none()
        }
        // endregion
        Msg::Confine(on) => {
            state.confined = on;
            log.push(PAGE, "Playground", format!("confined = {on}"));
            restart(state)
        }
        Msg::Narrow(on) => {
            state.narrow = on;
            log.push(PAGE, "Playground", format!("narrow = {on}"));
            Command::none()
        }
        Msg::Disable(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
            Command::none()
        }
        Msg::Extras(on) => {
            state.extras = on;
            log.push(PAGE, "Playground", format!("extras = {on}"));
            Command::none()
        }
        Msg::Slow(on) => {
            state.slow = on;
            log.push(PAGE, "Playground", format!("slow disk = {on}"));
            Command::none()
        }
        Msg::Mark(on) => {
            state.marked = on;
            log.push(PAGE, "Playground", format!("marked = {on}"));
            Command::none()
        }
        Msg::Trash(on) => {
            state.trashing = on;
            log.push(PAGE, "Playground", format!("trashing = {on}"));
            restart(state)
        }
        Msg::Hidden(on) => {
            state.hidden = on;
            log.push(PAGE, "Playground", format!("hidden = {on}"));
            // Hidden entries were read with the rest, so showing them goes nowhere near the disk.
            state.manager.update(FileManagerMsg::ShowHidden(on), send)
        }
    }
}

/// The folder a message is about to have read, while the playground's disk is slow.
fn slow_key(message: &FileManagerMsg) -> Option<String> {
    match message {
        FileManagerMsg::Expand(key, true) => Some(key.clone()),
        _ => None,
    }
}

/// The name of `path`, which is what the page has room to say about it.
fn shown(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}

/// Keeps what the application was asked for, the latest [`KEPT`] of them.
fn asked(state: &mut State, log: &mut EventLog, text: String) -> Command<AppMsg> {
    log.push(PAGE, "FileManager#files", text.clone());
    state.asked.push(text);
    if state.asked.len() > KEPT {
        state.asked.remove(0);
    }
    Command::none()
}

/// Builds the manager again for the playground's settings and reads its root.
fn restart(state: &mut State) -> Command<AppMsg> {
    state.manager = manager(&state.demo, state.folder, state.confined, state.trashing, state.hidden);
    state.asked.clear();
    state.manager.load(send)
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("file-manager.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: file-manager-view
        let mut manager = FileManager::new(&state.manager, send)
            .view(VIEWS[state.view])
            .root_label(t!("file-manager.root"))
            .on_open(|path| send_page(Msg::Open(path.to_path_buf())))
            .disabled(state.disabled);
        if state.extras {
            manager = manager.on_open_terminal(|path| send_page(Msg::Terminal(path.to_path_buf()))).menu_items(
                |key, targets| {
                    let label = t!("file-manager.add-to-project", n = targets.len());
                    vec![ContextItem::new(label, send_page(Msg::Mine(key.to_owned())))]
                },
            );
        }
        if state.marked {
            // What an entry means to the application the manager cannot know: here a backup leaves
            // every text file out, which the row says with a warning sign and a faint name.
            manager = manager.row_mark(|key| {
                if key.ends_with(".txt") {
                    RowMark::new().sign("warning", "warning").faint(true)
                } else {
                    RowMark::new()
                }
            });
        }
        let width = if state.narrow { Length::Cells(NARROW) } else { Length::Fill(1) };
        manager.show(ui).width(width).height(Length::Cells(ROWS)).id("files");
        // endregion
        ui.spacer().height(Length::Cells(1));
        if state.asked.is_empty() {
            ui.add(Text::new(t!("file-manager.nothing-asked")).role("faint"));
        }
        for text in state.asked.iter().rev() {
            ui.add(Text::new(text.clone()).role("body").no_wrap());
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("file-manager.folder"), |ui| {
            let names = FOLDERS.map(|name| t!(&format!("file-manager.folder-{name}")));
            ui.add(Select::new(names).selected(Some(state.folder)).on_select(|i| send_page(Msg::Folder(i))))
                .width(Length::Cells(28))
                .id("folder");
        });
        setting(ui, t!("file-manager.view"), |ui| {
            let names = VIEW_NAMES.map(|name| t!(&format!("file-manager.view-{name}")));
            ui.add(Select::new(names).selected(Some(state.view)).on_select(|i| send_page(Msg::View(i))))
                .width(Length::Cells(28))
                .id("view");
        });
        setting(ui, t!("file-manager.confine"), |ui| {
            ui.add(toggle(state.confined, |on| send_page(Msg::Confine(on)))).id("confine");
        });
        setting(ui, t!("file-manager.extras"), |ui| {
            ui.add(toggle(state.extras, |on| send_page(Msg::Extras(on)))).id("extras");
        });
        setting(ui, t!("file-manager.trashing"), |ui| {
            ui.add(toggle(state.trashing, |on| send_page(Msg::Trash(on)))).id("trashing");
        });
        setting(ui, t!("file-manager.hidden"), |ui| {
            ui.add(toggle(state.hidden, |on| send_page(Msg::Hidden(on)))).id("hidden");
        });
        setting(ui, t!("file-manager.marks"), |ui| {
            ui.add(toggle(state.marked, |on| send_page(Msg::Mark(on)))).id("marks");
        });
        setting(ui, t!("file-manager.slow"), |ui| {
            ui.add(toggle(state.slow, |on| send_page(Msg::Slow(on)))).id("slow");
        });
        setting(ui, t!("file-manager.narrow"), |ui| {
            ui.add(toggle(state.narrow, |on| send_page(Msg::Narrow(on)))).id("narrow");
        });
        setting(ui, t!("file-manager.disabled"), |ui| {
            ui.add(toggle(state.disabled, |on| send_page(Msg::Disable(on)))).id("disabled");
        });
        ui.add(Text::new(t!("file-manager.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    /// A moment for a menu or a dialog to settle.
    const MOMENT: Duration = Duration::from_millis(400);

    #[test]
    fn the_page_shows_the_folder_and_makes_a_file_in_it() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("README.md") && screen.contains("notes"), "{screen}");
        assert!(screen.contains("Nothing has been asked for yet"), "{screen}");

        h.click_text("notes").advance(MOMENT);
        assert!(h.screen().contains("tide.md"), "a folder opens:\n{}", h.screen());
        h.click_text("tide.md").advance(MOMENT);
        let screen = h.screen();
        assert!(screen.contains("Open tide.md"), "opening is the application's own:\n{screen}");

        let (x, y) = h.find("src").expect("the folder's row");
        h.mouse(qframe::event::MouseKind::Down(qframe::event::MouseButton::Right), x, y);
        h.mouse(qframe::event::MouseKind::Up(qframe::event::MouseButton::Right), x, y);
        h.advance(MOMENT);
        let screen = h.screen();
        assert!(screen.contains("New file") && screen.contains("Open a terminal here"), "{screen}");
        assert!(screen.contains("Add to the project"), "the application's own item:\n{screen}");
        h.click_text("New file").advance(MOMENT);
        h.type_text("lib.rs").press("enter").advance(MOMENT);
        let demo = &h.app().pages.file_manager;
        assert!(demo.demo.join("files/src/lib.rs").is_file(), "{}", h.screen());
        assert!(h.screen().contains("lib.rs"), "{}", h.screen());
    }

    #[test]
    fn the_playground_shows_an_empty_folder_one_that_cannot_be_read_and_a_slow_disk() {
        let mut h = showcase_on(PAGE);
        h.send(send_page(Msg::Folder(1))).advance(MOMENT);
        assert!(h.screen().contains("empty"), "{}", h.screen());

        #[cfg(unix)]
        if std::fs::read_dir(h.app().pages.file_manager.demo.join("denied")).is_err() {
            h.send(send_page(Msg::Folder(2))).advance(MOMENT);
            assert!(h.screen().contains("This folder could not be read."), "{}", h.screen());
        }

        h.send(send_page(Msg::Folder(0))).advance(MOMENT);
        h.send(send_page(Msg::Slow(true)));
        h.send(send(FileManagerMsg::Expand("notes".to_owned(), true)));
        // The read is on its way: nothing is claimed about the folder and no spinner before 300 ms.
        h.advance(Duration::from_millis(299));
        assert!(!h.screen().contains("tide.md"), "{}", h.screen());
        h.advance(Duration::from_millis(1));
        assert!(h.screen().contains(|c| "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".contains(c)), "the row spins:\n{}", h.screen());
        h.advance(Duration::from_millis(800));
        assert!(h.screen().contains("tide.md"), "{}", h.screen());
    }

    #[test]
    fn an_entry_is_copied_put_in_the_trash_and_the_hidden_ones_shown() {
        let mut h = showcase_on(PAGE);
        assert!(!h.screen().contains(".hidden.txt"), "hidden to begin with:\n{}", h.screen());
        h.send(send_page(Msg::Hidden(true))).advance(MOMENT);
        assert!(h.screen().contains(".hidden.txt"), "{}", h.screen());
        h.send(send_page(Msg::Hidden(false))).advance(MOMENT);

        // A copy leaves what it was made from where it is.
        h.send(send(FileManagerMsg::Copy("README.md".to_owned())));
        h.send(send(FileManagerMsg::Paste("notes".to_owned()))).advance(MOMENT);
        let demo = h.app().pages.file_manager.demo.clone();
        assert!(demo.join("files/README.md").is_file() && demo.join("files/notes/README.md").is_file());

        // With a trash, deleting puts the entry aside and asks nothing.
        h.send(send_page(Msg::Trash(true))).advance(MOMENT);
        h.send(send(FileManagerMsg::Trash("README.md".to_owned()))).advance(MOMENT);
        assert!(demo.join("Trash/files/README.md").is_file(), "{}", h.screen());
        assert!(!demo.join("files/README.md").exists(), "{}", h.screen());
    }

    #[test]
    fn a_long_copy_is_a_background_task_with_a_way_to_stop_it() {
        let mut h = showcase_on(PAGE);
        h.send(send(FileManagerMsg::Copy("big.bin".to_owned())));
        h.send(send(FileManagerMsg::Paste("notes".to_owned())));
        // The copy runs as a task, so the page answers at once rather than holding the frame.
        h.advance(MOMENT);
        let demo = h.app().pages.file_manager.demo.clone();
        assert!(demo.join("files/notes/big.bin").is_file(), "{}", h.screen());
        assert!(demo.join("files/big.bin").is_file(), "what was copied stays where it is");
        assert!(h.app().pages.file_manager.manager.work().is_none(), "the row goes when the work is over");
    }

    #[test]
    fn the_application_marks_what_its_backup_leaves_out() {
        let mut h = showcase_on(PAGE);
        assert!(!h.screen().contains('\u{25b2}'), "nothing is marked to begin with:\n{}", h.screen());
        h.send(send_page(Msg::Mark(true))).advance(MOMENT);
        let screen = h.screen();
        assert!(screen.contains('\u{25b2}'), "the text file carries the warning sign:\n{screen}");
        assert!(screen.contains("README.md"), "and the other rows are as they were:\n{screen}");
    }

    #[test]
    fn the_shape_switches_between_the_tree_the_list_and_the_icons() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("README.md"), "{}", h.screen());

        h.send(send_page(Msg::View(1))).advance(MOMENT);
        let screen = h.screen();
        assert!(screen.contains("Size") && screen.contains("Changed"), "the list shows its columns:\n{screen}");
        assert!(screen.contains("README.md"), "{screen}");
        assert!(!screen.contains("tide.md"), "a flat view shows one folder:\n{screen}");
        h.advance(MOMENT);
        assert!(h.screen().contains(" B "), "a size was read for the rows on screen:\n{}", h.screen());

        // A folder is stepped into and the row of the folder itself is the way back out.
        h.click_text("notes").advance(MOMENT);
        assert!(h.screen().contains("tide.md"), "{}", h.screen());
        h.click_text("notes").advance(MOMENT);
        assert!(h.screen().contains("README.md"), "{}", h.screen());

        h.send(send_page(Msg::View(2))).advance(MOMENT);
        assert!(h.screen().contains("README.md"), "the icons show the same folder:\n{}", h.screen());
        h.send(send_page(Msg::View(0))).advance(MOMENT);
        assert!(h.screen().contains("README.md"), "{}", h.screen());
    }

    #[test]
    fn narrow_and_disabled_are_states_of_their_own() {
        let mut h = showcase_on(PAGE);
        h.send(send_page(Msg::Narrow(true))).advance(MOMENT);
        assert!(h.screen().contains("README.md"), "a narrow manager still shows its rows:\n{}", h.screen());
        h.send(send_page(Msg::Disable(true))).advance(MOMENT);
        h.click_text("README.md").advance(MOMENT);
        assert!(h.app().pages.file_manager.asked.is_empty(), "a disabled manager answers nothing");
    }
}
