//! Terminal: the user's shell running inside the showcase, in the theme's colours, with the
//! title, folder, bell and notifications it reports.

use std::path::PathBuf;
use std::time::Duration;

use qframe::prelude::*;
use qframe::widgets::{Badge, Segmented, Terminal, TerminalChange, TerminalSession};

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "terminal";

/// Scrollback lengths the playground offers; the next shell keeps that many lines.
const SCROLLBACKS: [usize; 4] = [0, 1000, 5000, 10_000];

/// The framework's default scrollback, chosen until the playground says otherwise.
const DEFAULT_SCROLLBACK: usize = 2;

/// Seconds the shell has to end after a hangup before it is killed.
const GRACE: Duration = Duration::from_secs(2);

/// The running shell, if any, what it last reported and the last start error.
#[derive(Debug)]
pub struct State {
    session: Option<TerminalSession>,
    /// Counts started shells, so a watch of an earlier shell is recognised and ignored.
    run: u64,
    exit: Option<Option<u32>>,
    error: Option<String>,
    title: Option<String>,
    folder: Option<PathBuf>,
    bells: u32,
    notification: Option<String>,
    /// Index into [`SCROLLBACKS`].
    scrollback: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            session: None,
            run: 0,
            exit: None,
            error: None,
            title: None,
            folder: None,
            bells: 0,
            notification: None,
            scrollback: DEFAULT_SCROLLBACK,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Start,
    Stop,
    Changed(u64, TerminalChange),
    Scrollback(usize),
    /// `terminal-focus` pressed inside the shell.
    Leave,
    /// `terminal-focus` pressed anywhere else on the page.
    Enter,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Terminal(message))
}

// region: terminal-toggle
/// The page's answer to `terminal-focus` pressed outside the shell. Inside it the shell's node
/// answers first, see [`view`], so this is only reached from the rest of the page.
pub fn action(state: &State, name: &str) -> Option<AppMsg> {
    (name == "terminal-focus" && state.session.is_some() && state.exit.is_none()).then(|| send(Msg::Enter))
}

/// Moves focus for the toggle: from the shell to the start button, or back into the shell.
fn toggle(leave: bool, log: &mut EventLog) -> Command<AppMsg> {
    let (target, entry) = if leave { ("start", "left") } else { ("shell", "entered") };
    log.push(PAGE, "Terminal#focus", entry);
    Command::focus(target)
}
// endregion

// region: terminal-watch
/// Starts the user's shell with the playground's scrollback. Output is reported at most once per
/// frame, however fast the shell writes.
fn start(state: &State) -> std::io::Result<TerminalSession> {
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    TerminalSession::builder(shell)
        .folder(super::home_folder())
        .env("QUVYTA_SHOWCASE", "1")
        .size(100, 18)
        .scrollback(SCROLLBACKS[state.scrollback])
        .coalesce(Duration::from_millis(16))
        .spawn()
}

/// Waits on a background thread for the shell's next change: output, a title, a folder, the
/// bell, a notification or its exit.
fn watch(session: &TerminalSession, run: u64) -> Command<AppMsg> {
    let watch = session.watch();
    Command::perform(move || send(Msg::Changed(run, watch.next_change())))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Start => match start(state) {
            Ok(session) => {
                state.run += 1;
                state.exit = None;
                state.error = None;
                state.title = None;
                state.folder = None;
                state.bells = 0;
                state.notification = None;
                let command = watch(&session, state.run);
                state.session = Some(session);
                log.push(PAGE, "Terminal#shell", "started");
                return command;
            }
            Err(error) => state.error = Some(error.to_string()),
        },
        Msg::Changed(run, TerminalChange::Exited(code)) if run == state.run => {
            state.exit = Some(code);
            log.push(PAGE, "Terminal#shell", format!("exited {code:?}"));
        }
        Msg::Changed(run, change) if run == state.run => {
            match change {
                TerminalChange::Title(title) => {
                    log.push(PAGE, "Terminal#shell", format!("title {title:?}"));
                    state.title = Some(title);
                }
                TerminalChange::WorkingFolder(folder) => state.folder = Some(folder),
                TerminalChange::Bell => {
                    state.bells += 1;
                    log.push(PAGE, "Terminal#shell", "bell");
                }
                TerminalChange::Notify { title, body } => {
                    log.push(PAGE, "Terminal#shell", format!("notification {body:?}"));
                    state.notification = Some(match title {
                        Some(title) => format!("{title}: {body}"),
                        None => body,
                    });
                }
                _ => {}
            }
            if let Some(session) = &state.session {
                return watch(session, run);
            }
        }
        // endregion
        Msg::Changed(..) => {}
        Msg::Leave => return toggle(true, log),
        Msg::Enter => return toggle(false, log),
        Msg::Stop => {
            // A hangup first, as when a terminal window closes, so the shell can save its
            // history; killed if it has not ended after the grace. The session stays here while
            // that happens: dropping it would end the shell at once, and the watch still has the
            // exit to report.
            if let Some(session) = &state.session {
                session.terminate(GRACE);
                log.push(PAGE, "Terminal#shell", "stopped");
            }
        }
        Msg::Scrollback(index) => {
            state.scrollback = index;
            log.push(PAGE, "Playground", format!("scrollback = {}", SCROLLBACKS[index]));
        }
    }
    Command::none()
}

// region: terminal-strip
/// What the shell reported last: its title and folder, how often it rang and its latest
/// notification.
fn strip(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.row(|ui| {
        match state.title.clone().filter(|title| !title.is_empty()) {
            Some(title) => ui.add(Text::new(title).no_wrap()),
            None => ui.add(Text::new(t!("terminal.no-title")).role("faint").no_wrap()),
        };
        if let Some(folder) = &state.folder {
            ui.add(Text::new(folder.display().to_string()).role("secondary").no_wrap()).width(Length::Fill(1));
        } else {
            ui.spacer().width(Length::Fill(1));
        }
        if let Some(notification) = &state.notification {
            ui.add(Badge::new(notification.clone()).variant("info"));
        }
        if state.bells > 0 {
            ui.add(Badge::new(t!("terminal.bell")).variant("warning").count(state.bells));
        }
    })
    .gap(2)
    .fill_width();
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(1), |ui| {
        ui.row(|ui| {
            let running = state.session.is_some() && state.exit.is_none();
            let label = if state.session.is_some() { t!("terminal.restart") } else { t!("terminal.start") };
            ui.add(Button::new(label).variant("primary").on_press(send(Msg::Start))).id("start");
            ui.add(Button::new(t!("terminal.stop")).disabled(!running).on_press(send(Msg::Stop))).id("stop");
            ui.add(Text::new(t!("terminal.focus-hint")).role("faint").no_wrap());
        })
        .gap(2)
        .fill_width();
        if state.session.is_some() {
            strip(state, ui);
        }
        // region: terminal-view
        match &state.session {
            Some(session) => {
                // `f1` still opens the help while the shell has focus; `?` stays a character for
                // the shell. The palette's `ctrl p` and the menu's `ctrl b` are left to the shell,
                // where readline and tmux use them. `terminal-focus` leaves the shell: its node
                // answers the key while focus is inside, and `action` above answers it elsewhere.
                let terminal = Terminal::new(session)
                    .pass_through(Scope::Global, "help")
                    .pass_through(Scope::App, "terminal-focus");
                ui.add(terminal).width(Length::Fill(1)).height(Length::Cells(18)).id("shell").on_action(
                    Scope::App,
                    "terminal-focus",
                    send(Msg::Leave),
                );
            }
            None => {
                ui.add(Text::new(state.error.clone().unwrap_or_else(|| t!("terminal.idle"))).role("faint"))
                    .height(Length::Cells(18));
            }
        }
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("terminal.scrollback"), |ui| {
            let labels = SCROLLBACKS.map(|lines| lines.to_string());
            ui.add(Segmented::new(labels).selected(state.scrollback).on_select(|i| send(Msg::Scrollback(i))))
                .id("scrollback");
        });
        ui.add(Text::new(t!("terminal.scrollback-hint")).role("faint"));
        ui.add(Text::new(t!("terminal.notices-hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::{showcase_on, showcase_tall};

    #[test]
    fn idle_until_started_and_ignores_old_watches() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("Start a shell"), "{}", h.screen());
        // Starting a real shell here would block the inline test runner on its watch; the
        // framework's terminal tests run programs in a pseudo-terminal instead.
        h.send(send(Msg::Changed(7, TerminalChange::Exited(Some(0)))));
        assert_eq!(h.app().pages.terminal.exit, None);
    }

    #[test]
    fn one_key_leaves_the_shell_and_the_same_key_goes_back() {
        // A shell set in place without its watch: the watch would block the inline test runner.
        let session =
            TerminalSession::spawn("/bin/cat".as_ref(), &[] as &[&str], &super::super::home_folder()).expect("pty");
        let mut showcase = Showcase::new();
        showcase.pages.terminal.session = Some(session.clone());
        let mut h = showcase_tall(showcase, PAGE, 60);
        h.send(send(Msg::Enter));
        assert!(h.is_focused("shell"));
        h.press("ctrl+alt+space");
        assert!(h.is_focused("start"), "the key left the shell");
        h.press("ctrl+alt+space");
        assert!(h.is_focused("shell"), "the same key went back in");
        let heard: Vec<&str> = h.app().log.recent(PAGE, 3).iter().map(|entry| entry.message.as_str()).collect();
        assert_eq!(heard, ["entered", "left", "entered"]);
        session.kill();
    }

    #[test]
    fn the_strip_shows_what_the_shell_reported() {
        // A program that reports and ends, so the watch chain the harness runs inline finishes.
        let script = "printf '\\033]0;from the shell\\007\\033]7;file://h/tmp/a%%20b\\033\\\\\\a\\033]777;notify;Build;done\\007'";
        let session =
            TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], &super::super::home_folder()).expect("pty");
        let mut showcase = Showcase::new();
        showcase.pages.terminal.session = Some(session);
        let mut h = showcase_tall(showcase, PAGE, 60);
        assert!(h.screen().contains("The shell has set no title"), "{}", h.screen());
        h.send(send(Msg::Changed(0, TerminalChange::Output)));
        // Each step runs one round of the watch chain; the program ends after a few.
        for _ in 0..20 {
            if h.app().pages.terminal.exit.is_some() {
                break;
            }
            h.render();
        }
        let state = &h.app().pages.terminal;
        assert_eq!(state.title.as_deref(), Some("from the shell"));
        assert_eq!(state.folder, Some(PathBuf::from("/tmp/a b")));
        assert_eq!(state.bells, 1);
        assert_eq!(state.notification.as_deref(), Some("Build: done"));
        assert_eq!(state.exit, Some(Some(0)));
        let screen = h.screen();
        for shown in ["from the shell", "/tmp/a b", "Build: done", "bell"] {
            assert!(screen.contains(shown), "{shown} missing: {screen}");
        }
    }

    #[test]
    fn the_playground_sets_the_next_shells_scrollback() {
        let mut h = showcase_on(PAGE);
        assert_eq!(SCROLLBACKS[h.app().pages.terminal.scrollback], 5000, "the framework's default");
        h.send(send(Msg::Scrollback(1)));
        assert_eq!(SCROLLBACKS[h.app().pages.terminal.scrollback], 1000);
        let heard: Vec<&str> = h.app().log.recent(PAGE, 1).iter().map(|entry| entry.message.as_str()).collect();
        assert_eq!(heard, ["scrollback = 1000"]);
    }
}
