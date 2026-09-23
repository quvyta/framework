//! Terminal handoff: giving the terminal to another program for a while and taking the screen
//! back afterwards, and the opening that gives nothing away at all.

use qframe::prelude::*;
use qframe::runtime::{
    ChildLine, DetachedHandoff, DetachedOutcome, Handoff, HandoffOutcome, LiveChild, Open, OpenOutcome,
};
use qframe::widgets::Badge;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "handoff";

/// A shell that prints one line and waits for Enter: the shortest program that really takes the
/// keyboard. The line comes from the environment, so no text is built into the script.
const WAIT_SCRIPT: &str = r#"printf '%s\n' "$QUVYTA_HANDOFF_LINE"; read -r _"#;

/// A small helper: it asks for a line on the terminal, as `pkexec` asks for a password, says
/// `ready` on its output and then echoes every line it is sent until its input ends. Its standard
/// input and output are the application's pipes, so the question goes to standard error and the
/// answer is read from the terminal itself.
const HELPER_SCRIPT: &str = r#"printf '%s ' "$QUVYTA_HELPER_ASK" >&2; read -r _ < /dev/tty; echo ready; exec cat"#;

/// A program nobody has installed, for the outcome of a handoff that cannot start.
const MISSING: &str = "quvyta-not-installed";

/// What the silent opening hands the desktop: the ecosystem's own page, so trying it out on a real
/// machine opens something harmless in whatever browser this person uses.
const ADDRESS: &str = "https://quvyta.com";

/// The outcome of the last handoff and the playground.
#[derive(Debug)]
pub struct State {
    outcome: Option<HandoffOutcome>,
    pause: bool,
    notice: bool,
    /// The helper of the detached handoff while it runs.
    helper: Option<LiveChild>,
    /// How many lines were sent to the helper, to number the next.
    sent: usize,
    /// What came of the last silent opening.
    opened: Option<OpenOutcome>,
}

impl Default for State {
    fn default() -> Self {
        Self { outcome: None, pause: false, notice: true, helper: None, sent: 0, opened: None }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Shell,
    Editor,
    Missing,
    Ended(HandoffOutcome),
    Pause(bool),
    Notice(bool),
    Helper,
    HelperStarted(DetachedOutcome),
    HelperSaid(ChildLine),
    HelperSend,
    HelperStop,
    Open,
    OpenMissing,
    Opened(OpenOutcome),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Handoff(message))
}

/// The editor the user chose, or `vi`, which every Unix has.
fn editor() -> String {
    std::env::var("EDITOR").ok().filter(|name| !name.is_empty()).unwrap_or_else(|| "vi".to_owned())
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: hand-over
        Msg::Shell => {
            // The shell owns the terminal while it runs: it prints, it reads the Enter key
            // itself, and the showcase draws nothing until it ends.
            let handoff = Handoff::new("sh", |outcome| send(Msg::Ended(outcome)))
                .args(["-c", WAIT_SCRIPT])
                .env("QUVYTA_HANDOFF_LINE", t!("handoff.shell-line"))
                .pause(state.pause);
            let handoff = if state.notice { handoff.notice(t!("handoff.shell-notice")) } else { handoff };
            log.push(PAGE, "Command::handoff", "sh -c");
            return Command::handoff(handoff);
        }
        Msg::Ended(outcome) => {
            log.push(PAGE, "Handoff", outcome_line(&outcome));
            state.outcome = Some(outcome);
        }
        // endregion
        Msg::Editor => {
            let program = editor();
            log.push(PAGE, "Command::handoff", program.clone());
            let handoff = Handoff::new(program, |outcome| send(Msg::Ended(outcome)))
                // The editor writes over the screen and its last frame is worth reading, so a
                // handoff that opens one is the place for `pause`.
                .pause(state.pause);
            let handoff = if state.notice { handoff.notice(t!("handoff.editor-notice")) } else { handoff };
            return Command::handoff(handoff);
        }
        Msg::Missing => {
            log.push(PAGE, "Command::handoff", MISSING);
            return Command::handoff(Handoff::new(MISSING, |outcome| send(Msg::Ended(outcome))).pause(state.pause));
        }
        // region: detach
        Msg::Helper => {
            // The helper owns the terminal only until it says `ready`; then the screen comes back
            // and it keeps running, its lines arriving as `HelperSaid`.
            let handoff = DetachedHandoff::new("sh", |outcome| send(Msg::HelperStarted(outcome)))
                .args(["-c", HELPER_SCRIPT])
                .env("QUVYTA_HELPER_ASK", t!("handoff.helper-ask"))
                .on_line(|line| send(Msg::HelperSaid(line)))
                .pause(state.pause);
            let handoff = if state.notice { handoff.notice(t!("handoff.helper-notice")) } else { handoff };
            log.push(PAGE, "Command::handoff_detached", "sh -c");
            return Command::handoff_detached(handoff);
        }
        Msg::HelperStarted(DetachedOutcome::Detached { child, first_line }) => {
            log.push(PAGE, "DetachedOutcome", format!("detached after {first_line:?}"));
            state.helper = Some(child);
        }
        Msg::HelperStarted(DetachedOutcome::Finished { code }) => {
            // Ended before it was ready, such as a refused password: a handoff that finished.
            log.push(PAGE, "DetachedOutcome", outcome_line(&HandoffOutcome::Finished { code }));
            state.outcome = Some(HandoffOutcome::Finished { code });
        }
        Msg::HelperStarted(DetachedOutcome::Failed(reason)) => {
            log.push(PAGE, "DetachedOutcome", outcome_line(&HandoffOutcome::Failed(reason.clone())));
            state.outcome = Some(HandoffOutcome::Failed(reason));
        }
        Msg::HelperSaid(ChildLine::Line(line)) => log.push(PAGE, "LiveChild", line),
        Msg::HelperSaid(ChildLine::Ended { code }) => {
            log.push(PAGE, "LiveChild", outcome_line(&HandoffOutcome::Finished { code }));
            state.helper = None;
        }
        Msg::HelperSend => {
            if let Some(helper) = &state.helper {
                state.sent += 1;
                let line = t!("handoff.helper-line", n = state.sent);
                if let Err(error) = helper.write_line(&line) {
                    log.push(PAGE, "LiveChild", format!("write failed: {error}"));
                }
            }
        }
        Msg::HelperStop => {
            // Closing its input is how the helper is asked to finish; its end arrives as a line.
            if let Some(helper) = &state.helper {
                helper.close_stdin();
                log.push(PAGE, "LiveChild", "close_stdin");
            }
        }
        // endregion
        // region: open
        Msg::Open => {
            // Nothing of this happens in the terminal, so the screen is never given away: no
            // step aside, no blink, nothing drawn again.
            log.push(PAGE, "Command::open_with", ADDRESS);
            return Command::open_with(Open::new(ADDRESS).answer(|outcome| send(Msg::Opened(outcome))));
        }
        Msg::OpenMissing => {
            log.push(PAGE, "Command::open_with", MISSING);
            return Command::open_with(Open::program(MISSING).answer(|outcome| send(Msg::Opened(outcome))));
        }
        Msg::Opened(outcome) => {
            log.push(
                PAGE,
                "OpenOutcome",
                match &outcome {
                    OpenOutcome::Opened => "opened".to_owned(),
                    OpenOutcome::Failed(reason) => format!("failed: {reason}"),
                },
            );
            state.opened = Some(outcome);
        }
        // endregion
        Msg::Pause(on) => {
            log.push(PAGE, "Playground", format!("pause = {on}"));
            state.pause = on;
        }
        Msg::Notice(on) => {
            log.push(PAGE, "Playground", format!("notice = {on}"));
            state.notice = on;
        }
    }
    Command::none()
}

/// How the handoff ended, for the event log: plain words, no punctuation of its own.
fn outcome_line(outcome: &HandoffOutcome) -> String {
    match outcome {
        HandoffOutcome::Finished { code: Some(code) } => format!("finished with code {code}"),
        HandoffOutcome::Finished { code: None } => "ended by a signal".to_owned(),
        HandoffOutcome::Failed(reason) => format!("failed: {reason}"),
    }
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("handoff.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("handoff.shell")).variant("primary").on_press(send(Msg::Shell))).id("shell");
            ui.add(Button::new(t!("handoff.editor")).on_press(send(Msg::Editor))).id("editor");
            ui.add(Button::new(t!("handoff.missing")).on_press(send(Msg::Missing))).id("missing");
        })
        .gap(2)
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("handoff.helper-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let running = state.helper.is_some();
        ui.row(|ui| {
            ui.add(Button::new(t!("handoff.helper")).disabled(running).on_press(send(Msg::Helper))).id("helper");
            ui.add(Button::new(t!("handoff.helper-send")).disabled(!running).on_press(send(Msg::HelperSend)))
                .id("helper-send");
            ui.add(Button::new(t!("handoff.helper-stop")).disabled(!running).on_press(send(Msg::HelperStop)))
                .id("helper-stop");
        })
        .gap(2)
        .fill_width();
        if running {
            ui.spacer().height(Length::Cells(1));
            ui.row(|ui| {
                ui.add(Badge::new(t!("handoff.badge-running")).variant("success"));
                ui.add(Text::new(t!("handoff.running")).role("secondary")).fill_width().id("helper-state");
            })
            .gap(2)
            .fill_width();
        }
        ui.spacer().height(Length::Cells(1));
        // region: open
        ui.add(Text::new(t!("handoff.open-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("handoff.open")).on_press(send(Msg::Open))).id("open");
            ui.add(Button::new(t!("handoff.open-missing")).on_press(send(Msg::OpenMissing))).id("open-missing");
        })
        .gap(2)
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        match &state.opened {
            None => {
                ui.add(Text::new(t!("handoff.open-idle")).role("faint")).id("opened");
            }
            Some(outcome) => {
                let (variant, label, detail) = match outcome {
                    OpenOutcome::Opened => ("success", t!("handoff.badge-opened"), t!("handoff.opened")),
                    OpenOutcome::Failed(reason) => {
                        ("danger", t!("handoff.badge-not-opened"), t!("handoff.not-opened", reason = reason.clone()))
                    }
                };
                ui.row(|ui| {
                    ui.add(Badge::new(label).variant(variant));
                    ui.add(Text::new(detail).role("secondary")).fill_width().id("opened");
                })
                .gap(2)
                .fill_width();
            }
        }
        // endregion
        ui.spacer().height(Length::Cells(1));
        // region: outcome
        // Every outcome is read the same way: the badge carries the tone and its marker, the
        // line beside it says what happened.
        match &state.outcome {
            None => {
                ui.add(Text::new(t!("handoff.idle")).role("faint")).id("outcome");
            }
            Some(outcome) => {
                let (variant, label, detail) = match outcome {
                    HandoffOutcome::Finished { code: Some(0) } => {
                        ("success", t!("handoff.badge-done"), t!("handoff.done"))
                    }
                    HandoffOutcome::Finished { code: Some(code) } => {
                        ("warning", t!("handoff.badge-code"), t!("handoff.code", code = *code))
                    }
                    HandoffOutcome::Finished { code: None } => {
                        ("warning", t!("handoff.badge-signal"), t!("handoff.signal"))
                    }
                    HandoffOutcome::Failed(reason) => {
                        ("danger", t!("handoff.badge-failed"), t!("handoff.failed", reason = reason.clone()))
                    }
                };
                ui.row(|ui| {
                    ui.add(Badge::new(label).variant(variant));
                    ui.add(Text::new(detail).role("secondary")).fill_width().id("outcome");
                })
                .gap(2)
                .fill_width();
            }
        }
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("handoff.pause"), |ui| {
            ui.add(toggle(state.pause, |on| send(Msg::Pause(on)))).id("pause");
        });
        setting(ui, t!("handoff.notice"), |ui| {
            ui.add(toggle(state.notice, |on| send(Msg::Notice(on)))).id("notice");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn the_shell_handoff_carries_its_script_notice_and_pause() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("Nothing has been handed the terminal yet"), "{}", h.screen());
        h.click_text("Hand over a shell");
        let asked = h.handoffs();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].program, OsString::from("sh"));
        assert_eq!(asked[0].args, ["-c", WAIT_SCRIPT].map(OsString::from));
        assert_eq!(asked[0].notice.as_deref(), Some("Handing the terminal to a shell"));
        assert!(!asked[0].pause, "the playground starts without the pause");
        // The harness answers with a plain success, and the demo says so.
        assert!(h.screen().contains("came back"), "{}", h.screen());
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "finished with code 0"), "{log:?}");
    }

    #[test]
    fn the_playground_turns_the_pause_on_and_the_notice_off() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Pause(true)));
        h.send(send(Msg::Notice(false)));
        h.click_text("Hand over a shell");
        let asked = h.handoffs();
        assert!(asked[0].pause, "the pause switch of the playground reaches the handoff");
        assert_eq!(asked[0].notice, None, "without the notice nothing is printed before the program");
    }

    #[test]
    fn every_outcome_says_what_happened() {
        let mut h = showcase_on(PAGE);
        h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(3) });
        h.click_text("Hand over a shell");
        assert!(h.screen().contains("exit code 3"), "{}", h.screen());
        h.set_handoff_outcome(HandoffOutcome::Finished { code: None });
        h.click_text("Open the editor");
        assert!(h.screen().contains("signal"), "{}", h.screen());
        h.set_handoff_outcome(HandoffOutcome::Failed("no such file or directory".to_owned()));
        h.click_text("A program that is missing");
        assert!(h.screen().contains("no such file or directory"), "{}", h.screen());
        let programs: Vec<&OsString> = h.handoffs().iter().map(|request| &request.program).collect();
        assert_eq!(programs, [&OsString::from("sh"), &OsString::from(editor()), &OsString::from(MISSING)]);
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "ended by a signal"), "{log:?}");
    }

    #[test]
    fn the_helper_detaches_echoes_what_the_page_sends_and_stops() {
        let mut h = showcase_on(PAGE);
        let (child, program) = LiveChild::for_tests();
        h.set_detached_outcome(DetachedOutcome::Detached { child, first_line: "ready".to_owned() });
        h.click_text("Start a helper");
        let asked = h.detached_handoffs();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].args, ["-c", HELPER_SCRIPT].map(OsString::from));
        assert_eq!(asked[0].notice.as_deref(), Some("Handing the terminal to a helper until it is ready"));
        assert!(h.screen().contains("runs in the background"), "{}", h.screen());
        h.click_text("Send a line");
        assert_eq!(program.written(), ["hello 1"]);
        program.say("hello 1");
        h.render();
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.source == "LiveChild" && entry.message == "hello 1"), "{log:?}");
        h.click_text("Stop the helper");
        assert!(!program.stdin_open(), "stopping closes the helper's input");
        program.exit(Some(0));
        h.render();
        assert!(!h.screen().contains("runs in the background"), "{}", h.screen());
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "finished with code 0"), "{log:?}");
    }

    #[test]
    fn opening_an_address_hands_it_over_without_giving_the_screen_away() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("Nothing has been opened yet"), "{}", h.screen());
        h.click_text("Open the Quvyta page");
        let asked = h.opens();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].target.as_deref(), Some(std::ffi::OsStr::new(ADDRESS)));
        // This is the whole point of an opening: the terminal is never handed over.
        assert!(h.handoffs().is_empty(), "nothing waited for it");
        assert!(h.detached_handoffs().is_empty(), "and nothing detached");
        assert!(h.screen().contains("was handed to this desktop"), "{}", h.screen());
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.source == "OpenOutcome" && entry.message == "opened"), "{log:?}");
    }

    #[test]
    fn a_desktop_with_no_opener_says_the_address_was_not_opened() {
        let mut h = showcase_on(PAGE);
        h.set_open_outcome(OpenOutcome::Failed("no such file or directory".to_owned()));
        h.click_text("An opener that is missing");
        assert_eq!(h.opens()[0].program, OsString::from(MISSING));
        assert_eq!(h.opens()[0].target, None, "a program of its own is handed to no opener");
        assert!(h.screen().contains("no such file or directory"), "{}", h.screen());
    }

    #[test]
    fn a_helper_that_ends_before_it_is_ready_shows_its_code() {
        let mut h = showcase_on(PAGE);
        h.set_detached_outcome(DetachedOutcome::Finished { code: Some(126) });
        h.click_text("Start a helper");
        assert!(h.screen().contains("exit code 126"), "{}", h.screen());
        assert!(!h.screen().contains("runs in the background"), "{}", h.screen());
    }
}
