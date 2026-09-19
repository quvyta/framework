//! Terminal: the user's shell running inside the showcase, in the theme's colours.

use qframe::prelude::*;
use qframe::widgets::{Terminal, TerminalEvent, TerminalSession};

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "terminal";

/// The running shell, if any, and the last start error.
#[derive(Debug, Default)]
pub struct State {
    session: Option<TerminalSession>,
    /// Counts started shells, so a watch of an earlier shell is recognised and ignored.
    run: u64,
    exit: Option<Option<u32>>,
    error: Option<String>,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Start,
    Stop,
    Changed(u64, TerminalEvent),
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
fn toggle(message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (target, entry) = if matches!(message, Msg::Leave) { ("start", "left") } else { ("shell", "entered") };
    log.push(PAGE, "Terminal#focus", entry);
    Command::focus(target)
}
// endregion

// region: terminal-watch
/// Waits on a background thread for the shell's next output or its exit.
fn watch(session: &TerminalSession, run: u64) -> Command<AppMsg> {
    let watch = session.watch();
    Command::perform(move || send(Msg::Changed(run, watch.next())))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Start => match TerminalSession::shell(&super::home_folder()) {
            Ok(session) => {
                state.run += 1;
                state.exit = None;
                state.error = None;
                let command = watch(&session, state.run);
                state.session = Some(session);
                log.push(PAGE, "Terminal#shell", "started");
                return command;
            }
            Err(error) => state.error = Some(error.to_string()),
        },
        Msg::Changed(run, TerminalEvent::Output) if run == state.run => {
            if let Some(session) = &state.session {
                return watch(session, run);
            }
        }
        Msg::Changed(run, TerminalEvent::Exited(code)) if run == state.run => {
            state.exit = Some(code);
            log.push(PAGE, "Terminal#shell", format!("exited {code:?}"));
        }
        // endregion
        Msg::Changed(..) => {}
        Msg::Leave | Msg::Enter => return toggle(message, log),
        Msg::Stop => {
            if let Some(session) = state.session.take() {
                session.kill();
                log.push(PAGE, "Terminal#shell", "stopped");
            }
        }
    }
    Command::none()
}

/// The live demo.
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
        h.send(send(Msg::Changed(7, TerminalEvent::Exited(Some(0)))));
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
}
