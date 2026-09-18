//! Getting started: a tiny application with state, messages and a view, a screen with
//! messages of its own that does background work, and the application's lifecycle: the first
//! focus, the size in `update`, a quit that asks first and an end the system asks for.

use qframe::prelude::*;
use qframe::runtime::Termination;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "getting-started";

// region: state
/// Everything the demo remembers. The runtime draws from it and changes it only through messages.
#[derive(Debug, Default)]
pub struct State {
    count: i64,
    folder: folder::Screen,
    /// The terminal size, as `App::resized` reported it.
    size: Size,
    /// Whether quitting asks first.
    ask_before_quit: bool,
}

/// Everything that can happen in the demo. The folder screen's messages ride inside `Folder`.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Increment,
    Decrement,
    Reset,
    Folder(folder::Msg),
    Resized(Size),
    AskBeforeQuit(bool),
    QuitAsked,
    Terminating(Termination),
    Quit,
}
// endregion

// region: screen
/// A screen with messages of its own. It knows nothing of the application around it: its
/// update returns commands of its own messages, and its view sends them.
pub mod folder {
    use qframe::prelude::*;

    #[derive(Debug, Default)]
    pub struct Screen {
        pub entries: Option<usize>,
        pub counting: bool,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Msg {
        Count,
        Counted(usize),
    }

    impl Screen {
        pub fn update(&mut self, message: Msg) -> Command<Msg> {
            match message {
                Msg::Count => {
                    self.counting = true;
                    Command::perform(|| Msg::Counted(std::fs::read_dir(".").map_or(0, |dir| dir.flatten().count())))
                }
                Msg::Counted(entries) => {
                    self.counting = false;
                    self.entries = Some(entries);
                    Command::none()
                }
            }
        }

        pub fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add_with(Panel::new().title(t!("getting-started.background")), |ui| {
                ui.add(Text::new(t!("getting-started.background-text")).role("secondary"));
                ui.row(|ui| {
                    ui.add(
                        Button::new(t!("getting-started.count-files"))
                            .icon("folder")
                            .loading(self.counting)
                            .on_press(Msg::Count),
                    )
                    .id("count-files");
                    if let Some(entries) = self.entries {
                        ui.add(Text::new(t!("getting-started.files", n = entries)).color("success").no_wrap());
                    }
                })
                .gap(3);
            })
            .fill_width();
        }
    }
}
// endregion

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::GettingStarted(message))
}

// region: lifecycle
/// Runs once, before the first frame is built: the menu has the keyboard from the start, so the
/// very first ↓ moves in it.
pub fn init() -> Command<AppMsg> {
    Command::focus("menu")
}

/// Hears the terminal size when the showcase starts and after every resize. `update` keeps it,
/// where work that needs the size (a pty of the right width) would start.
pub fn resized(size: Size) -> Option<AppMsg> {
    Some(send(Msg::Resized(size)))
}

/// Asked before the quit key or the command palette quits. With the switch on, the showcase
/// answers with a message and stays; the question then decides.
pub fn before_quit(state: &State) -> Option<AppMsg> {
    state.ask_before_quit.then(|| send(Msg::QuitAsked))
}

/// Hears that the system is ending the showcase: a `SIGTERM` (`kill` from another terminal) or a
/// `SIGHUP` (the terminal went away). `update` decides, so the event log shows it too.
pub fn terminating(cause: Termination) -> Option<AppMsg> {
    Some(send(Msg::Terminating(cause)))
}
// endregion

// region: update
/// Applies a message. Slow work never runs here: it is handed to the runtime as a command.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Increment => state.count += 1,
        Msg::Decrement => state.count -= 1,
        Msg::Reset => state.count = 0,
        Msg::Resized(size) => {
            state.size = size;
            log.push(PAGE, "App::resized", format!("{} × {}", size.width, size.height));
            return Command::none();
        }
        Msg::AskBeforeQuit(on) => {
            state.ask_before_quit = on;
            log.push(PAGE, "Playground", format!("ask before quitting = {on}"));
            return Command::none();
        }
        Msg::QuitAsked => {
            log.push(PAGE, "App::before_quit", "asked");
            return Command::confirm(quit_question(t!("getting-started.quit-message")));
        }
        Msg::Terminating(cause) => {
            log.push(PAGE, "App::terminating", format!("{cause:?}"));
            // The showcase has nothing to save; an application with unsaved work writes it here,
            // before its quit. Nobody can answer a question after a hangup, so only a terminate
            // asks, and only with the switch on; a second signal or the grace ends it anyway.
            return match cause {
                Termination::Terminate if state.ask_before_quit => {
                    Command::confirm(quit_question(t!("getting-started.terminate-message")))
                }
                _ => Command::quit(),
            };
        }
        // Decided: the application's own quit is not asked about again.
        Msg::Quit => return Command::quit(),
        // The screen's commands become the application's, the `Counted` its work sends later too.
        Msg::Folder(message) => {
            log.push(PAGE, "folder::Screen", format!("{message:?}"));
            return state.folder.update(message).map(|message| send(Msg::Folder(message)));
        }
    }
    log.push(PAGE, "Button#counter", format!("count = {}", state.count));
    Command::none()
}

/// The question before quitting; its answer quits with `Command::quit`, which is not asked about.
fn quit_question(message: String) -> Confirm<AppMsg> {
    Confirm::new(t!("getting-started.quit-title"), send(Msg::Quit))
        .message(message)
        .confirm_label(t!("getting-started.quit-confirm"))
        .cancel_label(t!("getting-started.quit-cancel"))
}
// endregion

// region: view
/// Describes the screen. It runs after every change and only reads the state.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::rich([
            Span::new(t!("getting-started.count")).role("secondary"),
            Span::new(format!("  {}", state.count)).role("title"),
        ]));
        ui.row(|ui| {
            ui.add(Button::new("−").on_press(send(Msg::Decrement))).id("decrement");
            ui.add(Button::new("+").variant("primary").on_press(send(Msg::Increment))).id("increment");
            ui.add(Button::new(t!("getting-started.reset")).on_press(send(Msg::Reset))).id("reset");
        })
        .gap(2);
    })
    .fill_width();

    // The screen draws with its own messages; one line places it and converts what it sends.
    ui.map(|message| send(Msg::Folder(message)), |ui| state.folder.view(ui)).fill_width();

    ui.add_with(Panel::new().title(t!("getting-started.lifecycle")).gap(0), |ui| {
        let size = t!("getting-started.size-value", width = state.size.width, height = state.size.height);
        setting(ui, t!("getting-started.size"), |ui| {
            ui.add(Text::new(size).role("title").no_wrap()).id("known-size");
        });
        ui.add(Text::new(t!("getting-started.size-hint")).role("faint"));
        setting(ui, t!("getting-started.ask"), |ui| {
            ui.add(toggle(state.ask_before_quit, |on| send(Msg::AskBeforeQuit(on)))).id("ask-before-quit");
        });
        ui.add(Text::new(t!("getting-started.lifecycle-hint")).role("faint"));
        ui.add(Text::new(t!("getting-started.terminating-hint")).role("faint"));
    })
    .fill_width();
}
// endregion

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn counts_and_runs_background_work() {
        let mut h = showcase_on(PAGE);
        h.click_text("+").click_text("+").click_text("−");
        assert_eq!(h.app().pages.getting_started.count, 1);
        h.click_text("Count entries");
        let folder = &h.app().pages.getting_started.folder;
        assert!(folder.entries.is_some() && !folder.counting, "the mapped perform came back");
        assert!(h.screen().contains("Counted("), "{}", h.screen());
    }

    #[test]
    fn the_menu_has_the_keyboard_from_the_first_frame() {
        let h = crate::tests::fresh();
        assert!(h.is_focused("menu"), "focused before any key");
    }

    #[test]
    fn update_knows_the_size_and_follows_a_resize() {
        let mut h = showcase_on(PAGE);
        let (width, height) = crate::tests::SIZE;
        assert_eq!(h.app().pages.getting_started.size, Size::new(width, height));
        assert!(h.screen().contains(&format!("{width} × {height} cells")), "{}", h.screen());
        h.resize(120, 40);
        assert_eq!(h.app().pages.getting_started.size, Size::new(120, 40));
        assert!(h.screen().contains("120 × 40 cells"), "{}", h.screen());
    }

    #[test]
    fn with_the_switch_on_the_quit_key_asks_first() {
        let mut h = showcase_on(PAGE);
        h.press("ctrl+q");
        assert!(h.quit_requested(), "the switch is off: quitting does not ask");
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::AskBeforeQuit(true))).press("ctrl+q").advance(std::time::Duration::from_millis(300));
        assert!(!h.quit_requested(), "the showcase stays");
        assert!(h.screen().contains("Quit the showcase?"), "{}", h.screen());
        h.press("esc");
        assert!(!h.quit_requested(), "keeping it running is an answer too");
        h.press("ctrl+q").advance(std::time::Duration::from_millis(300));
        h.click_text("Quit now");
        assert!(h.quit_requested(), "the answer quits without asking again");
    }

    #[test]
    fn a_terminate_asks_with_the_switch_on_and_a_second_one_quits() {
        let mut h = showcase_on(PAGE);
        h.terminate(Termination::Terminate);
        assert!(h.quit_requested(), "the switch is off: the showcase quits");

        let mut h = showcase_on(PAGE);
        h.send(send(Msg::AskBeforeQuit(true))).terminate(Termination::Terminate);
        h.advance(std::time::Duration::from_millis(300));
        assert!(!h.quit_requested(), "the showcase stays while the question is open");
        assert!(h.screen().contains("Quit the showcase?"), "{}", h.screen());
        assert!(h.screen().contains("App::terminating"), "the event log shows it: {}", h.screen());
        h.terminate(Termination::Terminate);
        assert!(h.quit_requested(), "a second signal is not swallowed");
    }

    #[test]
    fn a_hangup_quits_without_asking() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::AskBeforeQuit(true))).terminate(Termination::Hangup);
        assert!(h.quit_requested(), "nobody could answer after the terminal went away");
    }

    #[test]
    fn a_question_nobody_answers_ends_with_the_grace() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::AskBeforeQuit(true))).terminate(Termination::Terminate);
        h.advance(Termination::Terminate.grace());
        assert!(h.quit_requested());
    }
}
