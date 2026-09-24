//! File and folder picker: browsing the showcase's own folders, extension filters, folder mode,
//! one click or two to open, the states of folders that cannot be read, and a slow disk that
//! shows the delayed reading indicator.

use std::path::PathBuf;
use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::Task;
use qframe::widgets::{Click, FileBrowser, FilePicker, FilePickerMsg, PickMode, Select, read_folder};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "file-picker";

/// How long a read takes on the playground's slow disk: well past the indicator's 300 ms delay,
/// so the slow path can be compared with the instant one.
const SLOW_DISK: Duration = Duration::from_secs(1);

/// Start folders the playground offers.
const STARTS: [&str; 3] = ["home", "denied", "missing"];

/// The picker, what was chosen and the playground.
#[derive(Debug)]
pub struct State {
    browser: FileBrowser,
    chosen: Option<PathBuf>,
    folders: bool,
    extensions: bool,
    slow: bool,
    /// Whether one click opens a folder or chooses a file, instead of two.
    single: bool,
    start: usize,
}

fn start_folder(index: usize) -> PathBuf {
    match STARTS[index] {
        "home" => super::home_folder(),
        // A folder only its owner may read on Unix systems.
        "denied" => PathBuf::from("/root"),
        _ => super::home_folder().join("quvyta-no-such-folder"),
    }
}

fn browser(start: usize, folders: bool, extensions: bool) -> FileBrowser {
    let mode = if folders { PickMode::Folders } else { PickMode::Files };
    let browser = FileBrowser::new(start_folder(start), mode);
    if extensions { browser.extensions(["rs", "toml", "md"]) } else { browser }
}

impl Default for State {
    fn default() -> Self {
        let mut browser = browser(0, false, false);
        // The showcase builds page states once at start-up, before any view; the first folder is
        // read here so the page opens ready. Every later read runs in a background command.
        let folder = browser.folder().to_path_buf();
        let listing = read_folder(&folder);
        let _ = browser.update(FilePickerMsg::Loaded(folder, listing), send);
        Self { browser, chosen: None, folders: false, extensions: false, slow: false, single: false, start: 0 }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Picker(FilePickerMsg),
    Folders(bool),
    Extensions(bool),
    Slow(bool),
    SingleClick(bool),
    Start(usize),
}

fn send(message: FilePickerMsg) -> AppMsg {
    AppMsg::Page(PageMsg::FilePicker(Msg::Picker(message)))
}

fn send_page(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::FilePicker(message))
}

/// Builds a fresh picker for the playground settings and reads its folder.
fn restart(state: &mut State) -> Command<AppMsg> {
    state.browser = browser(state.start, state.folders, state.extensions);
    state.chosen = None;
    let folder = state.browser.folder().to_path_buf();
    let read = state.browser.open(folder, send);
    on_disk(state, read)
}

// region: slow-disk
/// Reads `folder` the way a slow disk would, answering after [`SLOW_DISK`].
fn read_slowly(folder: PathBuf) -> Command<AppMsg> {
    Command::task(Task::new("slow disk", move |cx| {
        if !cx.sleep(SLOW_DISK) {
            return Err("stopped".into());
        }
        let listing = read_folder(&folder);
        Ok(send(FilePickerMsg::Loaded(folder, listing)))
    }))
}

/// The command that reads the folder the browser waits for: its own quick `read`, or the slow
/// disk's when the playground asks for it. A command only runs when returned, so the quick read
/// is simply not started.
fn on_disk(state: &State, read: Command<AppMsg>) -> Command<AppMsg> {
    if let Some(folder) = state.browser.loading().filter(|_| state.slow) {
        read_slowly(folder.to_path_buf())
    } else {
        read
    }
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let reads = matches!(message, Msg::Picker(FilePickerMsg::Open(_) | FilePickerMsg::Refresh));
    let command = apply(state, message, log);
    if reads { on_disk(state, command) } else { command }
}

fn apply(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: picker-update
        Msg::Picker(FilePickerMsg::Chosen(path)) => {
            log.push(PAGE, "FilePicker#files", format!("chose {}", path.display()));
            state.chosen = Some(path);
            Command::none()
        }
        Msg::Picker(message) => {
            if let FilePickerMsg::Open(folder) = &message {
                log.push(PAGE, "FilePicker#files", format!("open {}", folder.display()));
            }
            state.browser.update(message, send)
        }
        // endregion
        Msg::Folders(on) => {
            state.folders = on;
            log.push(PAGE, "Playground", format!("folders = {on}"));
            restart(state)
        }
        Msg::Extensions(on) => {
            state.extensions = on;
            log.push(PAGE, "Playground", format!("extensions = {on}"));
            restart(state)
        }
        Msg::Slow(on) => {
            state.slow = on;
            log.push(PAGE, "Playground", format!("slow disk = {on}"));
            Command::none()
        }
        Msg::SingleClick(on) => {
            state.single = on;
            log.push(PAGE, "Playground", format!("open_on = {}", if on { "Single" } else { "Double" }));
            Command::none()
        }
        Msg::Start(index) => {
            state.start = index;
            log.push(PAGE, "Playground", format!("start = {}", STARTS[index]));
            restart(state)
        }
    }
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        // region: picker-view
        let open_on = if state.single { Click::Single } else { Click::Double };
        FilePicker::new(&state.browser, send)
            .open_on(open_on)
            .show(ui)
            .width(Length::Fill(1))
            .height(Length::Cells(18));
        // endregion
        let chosen = state.chosen.as_ref().map_or_else(|| t!("file-picker.none"), |path| path.display().to_string());
        ui.add(
            Text::rich([Span::new(format!("{}  ", t!("file-picker.chosen"))).role("faint"), Span::new(chosen)])
                .no_wrap(),
        );
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("file-picker.start"), |ui| {
            let names = STARTS.map(|name| t!(&format!("file-picker.start-{name}")));
            ui.add(Select::new(names).selected(Some(state.start)).on_select(|i| send_page(Msg::Start(i))))
                .width(Length::Cells(26))
                .id("start");
        });
        setting(ui, t!("file-picker.folders"), |ui| {
            ui.add(toggle(state.folders, |on| send_page(Msg::Folders(on)))).id("folders");
        });
        setting(ui, t!("file-picker.extensions"), |ui| {
            ui.add(toggle(state.extensions, |on| send_page(Msg::Extensions(on)))).id("extensions");
        });
        setting(ui, t!("file-picker.single"), |ui| {
            ui.add(toggle(state.single, |on| send_page(Msg::SingleClick(on)))).id("single");
        });
        setting(ui, t!("file-picker.slow"), |ui| {
            ui.add(toggle(state.slow, |on| send_page(Msg::Slow(on)))).id("slow");
        });
        ui.add(Text::new(t!("file-picker.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::runtime::Harness;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_tall;

    /// Clicks the switch of the playground row labelled `label`; the switch stands after the
    /// label's column of 24 cells.
    fn click_setting(h: &mut Harness<Showcase>, label: &str) {
        let (x, y) = h.find(label).unwrap_or_else(|| panic!("the {label} row:\n{}", h.screen()));
        h.click(x + 25, y);
    }

    #[test]
    fn browses_chooses_and_shows_errors() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 60);
        let screen = h.screen();
        assert!(screen.contains("assets") && screen.contains("Cargo.toml"), "{screen}");
        h.click_text("Cargo.toml");
        assert_eq!(h.app().pages.file_picker.chosen, None, "a click only selects");
        h.click_text("Cargo.toml");
        assert_eq!(h.app().pages.file_picker.chosen, Some(start_folder(0).join("Cargo.toml")), "two choose");
        h.click_text("src").click_text("src");
        assert!(h.screen().contains("main.rs"), "{}", h.screen());
        click_setting(&mut h, "Choose folders");
        assert!(h.app().pages.file_picker.folders, "the switch turned folder mode on");
        // Nothing inside was pointed at, so the folder shown is the one chosen.
        h.click_text("Choose folder");
        assert_eq!(h.app().pages.file_picker.chosen, Some(start_folder(0)));
        h.send(send_page(Msg::Start(2)));
        assert!(h.screen().contains("This folder does not exist"), "{}", h.screen());
    }

    /// Whether the picker's path line shows the reading spinner.
    fn spinning(screen: &str) -> bool {
        let path = start_folder(0);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        screen.lines().filter(|line| line.contains(&name)).any(|line| line.contains(|c| "◜◠◝◞◡◟".contains(c)))
    }

    #[test]
    fn one_click_opens_when_the_playground_asks_for_it() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 60);
        click_setting(&mut h, "One click opens");
        assert!(h.app().pages.file_picker.single, "the switch turned it on");
        h.click_text("Cargo.toml");
        assert_eq!(h.app().pages.file_picker.chosen, Some(start_folder(0).join("Cargo.toml")), "one click chose");
        h.click_text("src");
        assert!(h.screen().contains("main.rs"), "one click opened:\n{}", h.screen());
    }

    #[test]
    fn a_quick_disk_never_shows_reading_and_a_slow_one_does_without_clearing() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 60);
        h.click_text("src").click_text("src");
        assert!(h.screen().contains("main.rs") && !spinning(&h.screen()), "{}", h.screen());
        click_setting(&mut h, "Slow disk");
        assert!(h.app().pages.file_picker.slow, "the switch turned the slow disk on");
        h.click_text("Parent folder").click_text("Parent folder");
        // The slow read is on its way: the src listing stays, and no spinner before 300 ms.
        h.advance(Duration::from_millis(299));
        assert!(h.screen().contains("main.rs") && !spinning(&h.screen()), "{}", h.screen());
        h.advance(Duration::from_millis(1));
        assert!(h.screen().contains("main.rs") && spinning(&h.screen()), "{}", h.screen());
        h.advance(Duration::from_millis(700));
        let screen = h.screen();
        assert!(screen.contains("Cargo.toml") && !screen.contains("main.rs"), "{screen}");
        // The read took a second, longer than the minimum: the spinner goes with it.
        assert!(!spinning(&screen), "{screen}");
    }
}
