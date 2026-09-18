//! Ending because the system asked: each signal simulated through the harness reaches the
//! application's answer, a second one ends the run, the grace bounds it, and an application that
//! implements nothing quits.

use std::time::Duration;

use crate::runtime::{App, Command, Harness, Termination};
use crate::widget::View;
use crate::widgets::Text;

/// Records what it hears and answers every termination with the message `answer` makes.
struct Timer {
    /// What happened, oldest first: `asked`, `saving after Hangup`, `saved`.
    steps: Vec<String>,
    answer: fn(Termination) -> Msg,
}

#[derive(Clone, Copy)]
enum Msg {
    /// A question the user would see.
    Ask,
    /// Saves and quits.
    SaveAndQuit(Termination),
    /// Starts saving and stays until the save is done.
    StartSaving(Termination),
    Saved,
}

impl App for Timer {
    type Msg = Msg;

    fn terminating(&self, cause: Termination) -> Option<Msg> {
        Some((self.answer)(cause))
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        record(&mut self.steps, msg)
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("25:00"));
    }
}

/// An application that asks before quitting and leaves `terminating` alone.
#[derive(Default)]
struct Guarded {
    steps: Vec<String>,
}

impl App for Guarded {
    type Msg = Msg;

    fn before_quit(&self) -> Option<Msg> {
        Some(Msg::Ask)
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        record(&mut self.steps, msg)
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("unsaved"));
    }
}

fn record(steps: &mut Vec<String>, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::Ask => steps.push("asked".to_owned()),
        Msg::SaveAndQuit(cause) => {
            steps.push(format!("saved after {cause:?}"));
            return Command::quit();
        }
        Msg::StartSaving(cause) => {
            steps.push(format!("saving after {cause:?}"));
            return Command::perform(|| Msg::Saved);
        }
        Msg::Saved => {
            steps.push("saved".to_owned());
            return Command::quit();
        }
    }
    Command::none()
}

/// An application that implements none of the hooks.
struct Plain;

impl App for Plain {
    type Msg = ();

    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Text::new("plain"));
    }
}

fn harness(answer: fn(Termination) -> Msg) -> Harness<Timer> {
    Harness::new(Timer { steps: Vec::new(), answer }, 30, 4)
}

#[test]
fn an_application_that_implements_nothing_quits_on_either_signal() {
    for cause in [Termination::Terminate, Termination::Hangup] {
        let mut h = Harness::new(Plain, 20, 3);
        h.terminate(cause);
        assert!(h.quit_requested(), "{cause:?} quits at once");
    }
}

#[test]
fn a_terminate_reaches_before_quit_by_default_and_a_hangup_does_not() {
    let mut h = Harness::new(Guarded::default(), 30, 4);
    h.terminate(Termination::Terminate);
    assert_eq!(h.app().steps, ["asked"], "the terminal is there: the same question as ctrl q");
    assert!(!h.quit_requested(), "the question keeps it running");

    let mut h = Harness::new(Guarded::default(), 30, 4);
    h.terminate(Termination::Hangup);
    assert!(h.app().steps.is_empty(), "nobody could answer the question after a hangup");
    assert!(h.quit_requested());
}

#[test]
fn each_signal_reaches_the_answer_of_terminating() {
    for cause in [Termination::Terminate, Termination::Hangup] {
        let mut h = harness(Msg::SaveAndQuit);
        h.terminate(cause);
        assert_eq!(h.app().steps, [format!("saved after {cause:?}")]);
        assert!(h.quit_requested(), "the application quit on its own after saving");
    }
}

#[test]
fn background_work_finishes_within_the_grace() {
    let mut h = harness(Msg::StartSaving);
    h.terminate(Termination::Hangup);
    // The harness runs perform work at the next step, as one pass of the loop would.
    h.render();
    assert_eq!(h.app().steps, ["saving after Hangup", "saved"]);
    assert!(h.quit_requested());
}

#[test]
fn the_grace_ends_a_run_the_application_keeps_open() {
    for cause in [Termination::Terminate, Termination::Hangup] {
        let mut h = harness(|_| Msg::Ask);
        h.terminate(cause);
        assert!(!h.quit_requested(), "{cause:?}: the answer keeps it running for now");
        h.advance(cause.grace() - Duration::from_millis(1));
        assert!(!h.quit_requested(), "{cause:?}: still within the grace");
        h.advance(Duration::from_millis(1));
        assert!(h.quit_requested(), "{cause:?}: the grace is over");
    }
}

#[test]
fn a_second_terminate_ends_the_run() {
    let mut h = Harness::new(Guarded::default(), 30, 4);
    h.terminate(Termination::Terminate);
    assert!(!h.quit_requested());
    h.terminate(Termination::Terminate);
    assert!(h.quit_requested(), "the second signal is not swallowed");
    assert_eq!(h.app().steps, ["asked"], "and the application is not asked again");

    let mut h = harness(|_| Msg::Ask);
    h.terminate(Termination::Hangup).terminate(Termination::Terminate);
    assert!(h.quit_requested(), "a terminate after a hangup ends it too");
}

#[test]
fn a_repeated_hangup_is_the_same_hangup() {
    let mut h = harness(|_| Msg::Ask);
    h.terminate(Termination::Hangup);
    h.advance(Duration::from_secs(1)).terminate(Termination::Hangup);
    assert!(!h.quit_requested(), "the shell and the system both send one when a connection drops");
    assert_eq!(h.app().steps, ["asked"], "told once");
    h.advance(Termination::Hangup.grace() - Duration::from_secs(1));
    assert!(h.quit_requested(), "the first hangup's grace still counts");
}

#[test]
fn a_hangup_during_a_terminate_is_told_and_shortens_the_grace() {
    let mut h = harness(|_| Msg::Ask);
    h.terminate(Termination::Terminate);
    h.advance(Duration::from_secs(1));
    h.terminate(Termination::Hangup);
    assert_eq!(h.app().steps, ["asked", "asked"], "the question the terminate opened can no longer be answered");
    h.advance(Termination::Hangup.grace());
    assert!(h.quit_requested(), "the hangup's grace, shorter than what was left");
}

#[test]
fn a_quit_the_application_decided_needs_no_signal_bookkeeping() {
    let mut h = harness(Msg::SaveAndQuit);
    h.terminate(Termination::Terminate);
    assert!(h.quit_requested());
    h.terminate(Termination::Terminate);
    assert_eq!(h.app().steps.len(), 1, "a run that already quit hears nothing more");
}
