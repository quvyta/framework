//! Terminal: the user's shell running inside the showcase, in the theme's colours, with the
//! title, folder, bell and notifications it reports.

use std::path::PathBuf;
use std::time::{Duration, Instant};

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

/// How long both the shell and the person have to have been quiet before a line is delivered.
/// Short enough to try out by hand; a real application waits a second or two.
const QUIET: Duration = Duration::from_millis(600);

/// The running shell, if any, what it last reported and the last start error.
#[derive(Debug)]
pub struct State {
    session: Option<TerminalSession>,
    /// A program that has ended, kept only so its last screen can be read.
    ended: Option<TerminalSession>,
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
    /// What the last attempt to deliver a line found.
    delivery: Option<Delivery>,
}

/// What the page found when it last tried to hand the shell a line: how long each side had been
/// quiet, and whether the line went in.
#[derive(Debug, Clone, Copy)]
pub struct Delivery {
    /// How long the shell had written nothing.
    program: Duration,
    /// How long no key had been typed into it.
    person: Duration,
    /// Whether the line was pasted, or held back because one side had just spoken.
    pasted: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            session: None,
            ended: None,
            run: 0,
            exit: None,
            error: None,
            title: None,
            folder: None,
            bells: 0,
            notification: None,
            scrollback: DEFAULT_SCROLLBACK,
            delivery: None,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Start,
    Stop,
    /// Run the small program whose last screen is shown beside the shell.
    LastScreen,
    /// That program said something or ended.
    LastChanged(TerminalChange),
    Changed(u64, TerminalChange),
    Scrollback(usize),
    /// Hand the shell a line, if both it and the person have been quiet.
    Deliver,
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
                state.delivery = None;
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
        // region: terminal-view-only
        Msg::LastScreen => {
            // A program that writes two lines and stops, so there is a real last screen to read.
            // Its text is given as arguments, never built into the script.
            let script = "printf '\\033[1;31m%s\\033[0m\\n\\033[32m%s\\033[0m\\n' \"$1\" \"$2\"; exit 2";
            let args = ["-c", script, "sh", &t!("terminal.ended-error"), &t!("terminal.ended-note")];
            match TerminalSession::spawn("/bin/sh".as_ref(), &args, &super::home_folder()) {
                Ok(session) => {
                    let watch = session.watch();
                    state.ended = Some(session);
                    log.push(PAGE, "Terminal#last", "started");
                    return Command::perform(move || send(Msg::LastChanged(watch.next_change())));
                }
                Err(error) => state.error = Some(error.to_string()),
            }
        }
        // The session is kept after the program ends: it holds the screen and the scrollback the
        // view-only terminal draws, and dropping it would take both away.
        Msg::LastChanged(TerminalChange::Exited(code)) => {
            log.push(PAGE, "Terminal#last", format!("exited {code:?}"));
        }
        Msg::LastChanged(_) => {
            if let Some(session) = &state.ended {
                let watch = session.watch();
                return Command::perform(move || send(Msg::LastChanged(watch.next_change())));
            }
        }
        // endregion
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
        // region: terminal-deliver
        Msg::Deliver => {
            if let Some(session) = &state.session {
                // The two quiet times, read at the moment of the decision: a program in the
                // middle of answering is not reading its input, and a person in the middle of a
                // sentence would have it cut in half.
                let now = Instant::now();
                let program = now.saturating_duration_since(session.last_output());
                let person = now.saturating_duration_since(session.last_input());
                let pasted = program >= QUIET && person >= QUIET;
                if pasted {
                    // As if the person had pasted it: one piece, and the line breaks in it are
                    // not read as Enter.
                    if let Err(error) = session.paste(&t!("terminal.line")) {
                        state.error = Some(error.to_string());
                    }
                }
                state.delivery = Some(Delivery { program, person, pasted });
                log.push(PAGE, "TerminalSession#paste", if pasted { "delivered" } else { "held back" });
            }
        }
        // endregion
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

// region: terminal-quiet
/// Seconds with one decimal, the pace a quiet time is read at.
fn seconds(span: Duration) -> String {
    format!("{:.1}", span.as_secs_f64())
}

/// What the last delivery found: how long each side had been quiet and what was decided. The two
/// times come from `last_output` and `last_input`, and they are read when the decision is made,
/// not while drawing.
fn quiet(state: &State, ui: &mut View<'_, AppMsg>) {
    let Some(delivery) = state.delivery else {
        ui.add(Text::new(t!("terminal.deliver-idle")).role("faint").no_wrap());
        return;
    };
    let times = t!("terminal.deliver-times", program = seconds(delivery.program), person = seconds(delivery.person));
    ui.row(|ui| {
        ui.add(Text::new(times).role("secondary").no_wrap());
        let (label, variant) =
            if delivery.pasted { (t!("terminal.delivered"), "success") } else { (t!("terminal.held-back"), "warning") };
        ui.add(Badge::new(label).variant(variant));
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
            ui.add(Button::new(t!("terminal.deliver")).disabled(!running).on_press(send(Msg::Deliver))).id("deliver");
            ui.add(Button::new(t!("terminal.last-screen")).on_press(send(Msg::LastScreen))).id("last-screen");
            ui.add(Text::new(t!("terminal.focus-hint")).role("faint").no_wrap());
        })
        .gap(2)
        .fill_width();
        if state.session.is_some() {
            strip(state, ui);
            quiet(state, ui);
        }
        ui.row(|ui| {
            // region: terminal-view
            match &state.session {
                Some(session) => {
                    // `f1` still opens the help while the shell has focus; `?` stays a character
                    // for the shell. The palette's `ctrl p` and the menu's `ctrl b` are left to
                    // the shell, where readline and tmux use them. `terminal-focus` leaves the
                    // shell: its node answers the key while focus is inside, and `action` above
                    // answers it elsewhere.
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
                        .width(Length::Fill(1))
                        .height(Length::Cells(18));
                }
            }
            // endregion
            // region: terminal-view-only
            if let Some(ended) = &state.ended {
                // The last screen of a program that has ended. `read_only` draws it faint, keeps
                // it out of the focus order and never writes to it, while its colours, the cursor
                // it left and scrolling back stay exactly as they are: Tab goes past it, and what
                // is typed or pasted reaches the page instead of disappearing into a dead program.
                ui.add(Terminal::new(ended).read_only()).width(Length::Fill(1)).height(Length::Cells(18)).id("last");
            }
            // endregion
        })
        .gap(2)
        .fill_width();
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
        ui.add(Text::new(t!("terminal.deliver-hint")).role("faint"));
        ui.add(Text::new(t!("terminal.view-only-hint")).role("faint"));
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
        // A live but silent program in the widget: the page's own watch would block the inline
        // test runner, so the test asks for a change within a bound instead.
        let session =
            TerminalSession::spawn("/bin/cat".as_ref(), &[] as &[&str], &super::super::home_folder()).expect("pty");
        assert_eq!(
            session.watch().next_change_within(Duration::from_millis(200)),
            None,
            "the program says nothing until it is written to"
        );
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
    fn a_line_is_delivered_only_once_the_shell_and_the_person_are_both_quiet() {
        let session =
            TerminalSession::spawn("/bin/cat".as_ref(), &[] as &[&str], &super::super::home_folder()).expect("pty");
        let mut showcase = Showcase::new();
        showcase.pages.terminal.session = Some(session.clone());
        let mut h = showcase_tall(showcase, PAGE, 60);

        // A key just written: the person is in the middle of something, so nothing is handed over.
        session.write(b"x").expect("write");
        h.send(send(Msg::Deliver));
        let held = h.app().pages.terminal.delivery.expect("a decision was made");
        assert!(!held.pasted, "{held:?}");
        assert!(held.person < QUIET, "{held:?}");
        assert!(h.screen().contains("held back"), "{}", h.screen());

        // Wait for the quiet instead of sleeping: a bounded wait answering None says the program
        // has written nothing for that long, and nothing has been typed since the key above.
        let watch = session.watch();
        let started = std::time::Instant::now();
        while watch.next_change_within(QUIET).is_some() {
            assert!(started.elapsed() < Duration::from_secs(20), "the program never fell silent");
        }
        h.send(send(Msg::Deliver));
        let sent = h.app().pages.terminal.delivery.expect("a decision was made");
        assert!(sent.pasted, "{sent:?}");
        assert!(sent.program >= QUIET && sent.person >= QUIET, "{sent:?}");
        assert!(h.screen().contains("delivered"), "{}", h.screen());
        let heard: Vec<&str> = h.app().log.recent(PAGE, 2).iter().map(|entry| entry.message.as_str()).collect();
        assert_eq!(heard, ["held back", "delivered"]);
        session.kill();
    }

    #[test]
    fn the_last_screen_of_a_program_that_ended_is_shown_and_never_entered() {
        let mut h = showcase_tall(Showcase::new(), PAGE, 60);
        h.send(send(Msg::LastScreen));
        // Each step runs one round of the watch chain; the small program ends after a few.
        for _ in 0..20 {
            if h.app().log.recent(PAGE, 1).iter().any(|entry| entry.message.starts_with("exited")) {
                break;
            }
            h.render();
        }
        let shown = "error: two files could not be read";
        let screen = h.screen();
        assert!(screen.contains(shown), "the last screen is drawn: {screen}");
        let at = h.find(shown).expect("the line is on the screen");
        for _ in 0..20 {
            assert!(!h.is_focused("last"), "the last screen is no Tab stop");
            if h.is_focused("start") {
                break;
            }
            h.press("tab");
        }
        assert!(h.is_focused("start"), "Tab went around the page and reached the start button");
        // A click never enters it; it lands on the nearest focusable thing around it, as a click
        // on any part of the page that takes no focus does.
        h.click(at.0, at.1);
        assert!(!h.is_focused("last"), "a click did not enter the last screen");
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
