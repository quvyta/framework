//! Nerd Font: whether the symbols are on this machine, where they would go, sample glyphs to
//! judge by eye, and an install that really runs.

use std::path::PathBuf;

use qframe::icons::nerd_font::{self, Install, Progress};
use qframe::icons::{GlyphMode, GlyphSample};
use qframe::prelude::*;
use qframe::runtime::{TaskEvent, TaskId, TaskOutcome};
use qframe::widgets::{Badge, ProgressBar};

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "nerd-font";

/// What the page installs, where it looks for fonts, and how the last install went.
#[derive(Debug)]
pub struct State {
    /// The install the button runs.
    install: Install,
    /// The folders searched for a Nerd Font.
    font_dirs: Vec<PathBuf>,
    /// Whether a Nerd Font was found there, looked up at the start and after each install.
    installed: bool,
    /// The last step of the running or finished install.
    progress: Option<Progress>,
    /// The install while it runs, to cancel it.
    task: Option<TaskId>,
}

impl Default for State {
    fn default() -> Self {
        let (install, font_dirs) = setup();
        let installed = nerd_font::installed_in(&font_dirs);
        Self { install, font_dirs, installed, progress: None, task: None }
    }
}

/// The real install into the user's font folder, and the folders glyph detection searches.
#[cfg(not(test))]
fn setup() -> (Install, Vec<PathBuf>) {
    (Install::new(), qframe::icons::default_font_dirs(|name| std::env::var(name).ok()))
}

/// Under test nothing reaches the user's fonts or the network: the folder is a temporary one of
/// this process, the system is not told, and the archive is a file that does not exist, so a test
/// that forgets to give its own archive fails at once instead of downloading.
#[cfg(test)]
fn setup() -> (Install, Vec<PathBuf>) {
    let root = std::env::temp_dir().join(format!("quvyta-showcase-fonts-{}", std::process::id()));
    let install = Install::new()
        .archive(nerd_font::Archive::new(format!("file://{}", root.join("missing.tar").display()), &"0".repeat(64)))
        .target(root.join("QuvytaNerdFont"))
        .register(false);
    (install, vec![root])
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Install,
    Cancel,
    Progress(Progress),
    Event(TaskEvent),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::NerdFont(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: install
        // The task is built here, in update, where the language is known: its label and notes
        // are translated once, and every step comes back as a message.
        Msg::Install => {
            let task = state
                .install
                .clone()
                .task(|progress| send(Msg::Progress(progress)))
                .on_event(|event| send(Msg::Event(event)));
            state.task = Some(task.id());
            state.progress = Some(Progress::Downloading { fraction: None });
            log.push(PAGE, "Install", format!("into {}", target_text(&state.install)));
            return Command::task(task);
        }
        Msg::Progress(progress) => {
            if matches!(progress, Progress::Done { .. }) {
                // Look again, so the status reads what is on disk now.
                state.installed = nerd_font::installed_in(&state.font_dirs);
            }
            state.progress = Some(progress);
        }
        // endregion
        Msg::Cancel => {
            if let Some(id) = state.task {
                log.push(PAGE, "Button#cancel", "cancel the install");
                return Command::cancel_task(id);
            }
        }
        Msg::Event(TaskEvent::Finished { outcome, .. }) => {
            state.task = None;
            let text = match outcome {
                TaskOutcome::Done => "done".to_owned(),
                TaskOutcome::Failed(reason) => format!("failed: {reason}"),
                TaskOutcome::Cancelled => {
                    state.progress = None;
                    "cancelled".to_owned()
                }
            };
            log.push(PAGE, "Task", text);
        }
        Msg::Event(_) => {}
    }
    Command::none()
}

fn target_text(install: &Install) -> String {
    install.target_dir().map_or_else(|| t!("nerd-font.no-folder"), |dir| dir.display().to_string())
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("nerd-font.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: status
        let (variant, label) =
            if state.installed { ("success", t!("nerd-font.found")) } else { ("warning", t!("nerd-font.missing")) };
        ui.row(|ui| {
            ui.add(Badge::new(label).variant(variant));
            ui.add(Text::new(nerd_font::status_text(state.installed)).role("secondary")).fill_width().id("status");
        })
        .gap(2)
        .fill_width();
        row(ui, t!("nerd-font.folder"), |ui| {
            ui.add(Text::new(target_text(&state.install)).no_wrap()).fill_width().id("folder");
        });
        // endregion
        ui.spacer().height(Length::Cells(1));
        // region: samples
        // The same icons twice: the Nerd glyphs, which need the font, beside the Unicode ones,
        // which every terminal draws. The user's eye says whether the first row reads as shapes.
        row(ui, t!("nerd-font.sample-nerd"), |ui| {
            ui.add(GlyphSample::new(GlyphMode::Nerd)).id("nerd");
        });
        row(ui, t!("nerd-font.sample-unicode"), |ui| {
            ui.add(GlyphSample::new(GlyphMode::Unicode)).id("unicode");
        });
        // endregion
        ui.spacer().height(Length::Cells(1));
        let running = state.task.is_some();
        ui.row(|ui| {
            let install = Button::new(t!("nerd-font.install")).disabled(running).on_press(send(Msg::Install));
            let install = if state.installed { install } else { install.variant("primary") };
            ui.add(install).id("install");
            if running {
                ui.add(Button::new(t!("nerd-font.cancel")).on_press(send(Msg::Cancel))).id("cancel");
            }
        })
        .gap(2)
        .fill_width();
        progress(state, ui);
    })
    .fill_width();
}

// region: progress
/// The step of the install while it runs, and what it came to: the folder to delete to undo it
/// and the honest word about terminals, or the reason it failed.
fn progress(state: &State, ui: &mut View<'_, AppMsg>) {
    let Some(progress) = &state.progress else { return };
    ui.spacer().height(Length::Cells(1));
    match progress {
        Progress::Downloading { fraction: Some(fraction) } => {
            ui.add(ProgressBar::new(*fraction).percent(true)).fill_width().id("bar");
        }
        Progress::Downloading { fraction: None } | Progress::Verifying | Progress::Installing => {
            ui.add(ProgressBar::indeterminate()).fill_width().id("bar");
        }
        Progress::Done { .. } | Progress::Failed(_) => {}
    }
    let (variant, label) = match progress {
        Progress::Done { .. } => ("success", t!("nerd-font.badge-done")),
        Progress::Failed(_) => ("danger", t!("nerd-font.badge-failed")),
        _ => ("info", t!("nerd-font.badge-running")),
    };
    ui.row(|ui| {
        ui.add(Badge::new(label).variant(variant));
        ui.add(Text::new(progress.text()).role("secondary")).fill_width().id("step");
    })
    .gap(2)
    .fill_width();
    if matches!(progress, Progress::Done { .. }) {
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(nerd_font::after_install_text())).fill_width().id("after");
    }
}
// endregion

/// A label column and its content, as the rows of a settings screen.
fn row(ui: &mut View<'_, AppMsg>, label: String, content: impl FnOnce(&mut View<'_, AppMsg>)) {
    ui.row(|ui| {
        ui.add(Text::new(label).role("secondary").no_wrap()).width(Length::Cells(14));
        content(ui);
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command as Shell;
    use std::time::Duration;

    use qframe::icons::nerd_font::{Archive, FONT_FILE};
    use qframe::runtime::Harness;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_tall;

    /// A folder of the test's own with a fake release archive, and the page set to install from
    /// it into a folder beside it.
    fn page_with_archive(name: &str, good: bool) -> (Harness<Showcase>, PathBuf) {
        let root = std::env::temp_dir().join(format!("quvyta-showcase-nerd-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("content")).expect("content folder");
        fs::write(root.join("content").join(FONT_FILE), b"glyphs").expect("font");
        let archive = root.join("symbols.tar");
        let status = Shell::new("tar")
            .arg("-cf")
            .arg(&archive)
            .arg("-C")
            .arg(root.join("content"))
            .arg(FONT_FILE)
            .status()
            .expect("tar");
        assert!(status.success());
        let digest = if good { sha256(&archive) } else { "0".repeat(64) };
        let fonts = root.join("fonts");
        let mut showcase = Showcase::new();
        showcase.pages.nerd_font.install = Install::new()
            .archive(Archive::new(format!("file://{}", archive.display()), &digest))
            .target(fonts.join("QuvytaNerdFont"))
            .register(false);
        showcase.pages.nerd_font.font_dirs = vec![fonts.clone()];
        showcase.pages.nerd_font.installed = false;
        (showcase_tall(showcase, PAGE, 60), root)
    }

    fn sha256(file: &Path) -> String {
        let output = Shell::new("sha256sum").arg(file).output().expect("sha256sum");
        String::from_utf8_lossy(&output.stdout).split_whitespace().next().expect("digest").to_owned()
    }

    #[test]
    fn shows_the_status_the_folder_and_both_samples() {
        let (h, root) = page_with_archive("status", true);
        let screen = h.screen();
        assert!(screen.contains("No Nerd Font was found on this machine"), "{screen}");
        assert!(screen.contains("QuvytaNerdFont"), "the folder: {screen}");
        assert!(screen.contains("\u{f07b}  \u{f00c}"), "nerd sample: {screen}");
        assert!(screen.contains("■  ✓  ⌕  ▤"), "unicode sample: {screen}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn install_really_installs_into_the_given_folder_and_says_what_next() {
        let (mut h, root) = page_with_archive("install", true);
        h.click_text("Install Symbols Nerd Font Mono");
        // The install runs real programs, so the harness waits for them rather than a clock.
        h.advance(Duration::from_millis(0));
        let screen = h.screen();
        assert!(root.join("fonts/QuvytaNerdFont").join(FONT_FILE).is_file(), "{screen}");
        assert!(screen.contains("A Nerd Font is installed on this machine"), "{screen}");
        assert!(screen.contains("JetBrainsMono Nerd Font"), "the honest next step: {screen}");
        assert!(h.app().pages.nerd_font.task.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_wrong_checksum_says_so_and_installs_nothing() {
        let (mut h, root) = page_with_archive("checksum", false);
        h.click_text("Install Symbols Nerd Font Mono");
        h.advance(Duration::from_millis(0));
        let screen = h.screen();
        assert!(screen.contains("checksum"), "{screen}");
        assert!(!root.join("fonts").exists(), "{screen}");
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message.starts_with("failed:")), "{log:?}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn the_default_under_test_never_touches_the_real_font_folder() {
        let state = State::default();
        let target = state.install.target_dir().expect("a folder");
        assert!(target.starts_with(std::env::temp_dir()), "{}", target.display());
        assert!(state.install.clone().archive(Archive::release()) != state.install, "not the release");
    }
}
