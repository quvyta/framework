//! Rules for [`Command::map`]: every kind of work a command holds delivers its messages through
//! the conversion, also the messages background work produces later on its own thread.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::command::Action;
use super::handoff::{self, HandoffScreen};
use super::{App, Command, Confirm, Handoff, HandoffOutcome, Harness, Task, TaskEvent, TaskOutcome};
use crate::icons::IconMode;
use crate::widget::View;
use crate::widgets::{Corner, Text, TextInput, Toast};

/// A screen's own messages, as the screen knows them.
#[derive(Debug, Clone, PartialEq)]
enum Inner {
    Performed(&'static str),
    Line(&'static str),
    Built(&'static str),
    Event(TaskEvent),
    Confirmed,
    Cancelled,
    Undo,
    Opened,
    Read(Option<String>),
    Back(HandoffOutcome),
}

/// The application's messages: the screen's wrapped, and the triggers the tests send.
#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Screen(Inner),
    Run(Kind),
}

/// Which screen command a trigger returns, mapped with `Msg::Screen`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Perform,
    Task,
    CancelTask,
    Confirm,
    Toast,
    DismissToast,
    ReadClipboard,
    Handoff,
    Settings,
}

#[derive(Default)]
struct Host {
    received: Vec<Inner>,
    task: Option<super::TaskId>,
}

/// The screen's `update` for each trigger: a command of screen messages.
fn screen_command(kind: Kind, host: &mut Host) -> Command<Inner> {
    match kind {
        Kind::Perform => Command::perform(|| Inner::Performed("on a thread")),
        Kind::Task => {
            let task = Task::new("Build", |cx| {
                cx.send(Inner::Line("layer 0"));
                cx.progress(0.5);
                if !cx.sleep(Duration::from_millis(100)) {
                    return Err("stopped".into());
                }
                cx.send(Inner::Line("layer 1"));
                Ok(Inner::Built("sha256:4f2a"))
            })
            .on_event(Inner::Event);
            host.task = Some(task.id());
            Command::task(task)
        }
        Kind::CancelTask => host.task.map_or_else(Command::none, Command::cancel_task),
        Kind::Confirm => Command::confirm(
            Confirm::new("Remove container?", Inner::Confirmed).confirm_label("Delete").on_cancel(Inner::Cancelled),
        ),
        Kind::Toast => {
            Command::toast(Toast::success("Deployed").action("Undo", Inner::Undo).on_press(Inner::Opened).key("deploy"))
        }
        Kind::DismissToast => Command::dismiss_toast("deploy"),
        Kind::ReadClipboard => Command::read_clipboard(Inner::Read),
        Kind::Handoff => Command::handoff(Handoff::new("sudo", Inner::Back).arg("-v").notice("Authorizing")),
        Kind::Settings => Command::batch([
            Command::set_theme("amber"),
            Command::set_icon_mode(IconMode::Ascii),
            Command::copy("copied through the screen"),
            Command::toast_corner(Corner::TopLeft),
            Command::focus("name"),
        ]),
    }
}

impl App for Host {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Screen(inner) => {
                self.received.push(inner);
                Command::none()
            }
            Msg::Run(kind) => screen_command(kind, self).map(Msg::Screen),
        }
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("host"));
        ui.add(TextInput::new("")).id("name");
    }
}

fn harness() -> Harness<Host> {
    let mut h = Harness::new(Host::default(), 60, 12);
    h.set_reduced_motion(true);
    h
}

#[test]
fn the_message_of_perform_arrives_mapped() {
    let mut h = harness();
    h.send(Msg::Run(Kind::Perform));
    assert_eq!(h.app().received, [Inner::Performed("on a thread")]);
}

#[test]
fn a_task_sends_its_lines_events_and_result_later_through_the_map() {
    let mut h = harness();
    h.send(Msg::Run(Kind::Task));
    let id = h.app().task.expect("the task started");
    assert_eq!(
        h.app().received,
        [
            Inner::Event(TaskEvent::Started { id, label: "Build".into() }),
            Inner::Line("layer 0"),
            Inner::Event(TaskEvent::Progress { id, fraction: Some(0.5), note: None }),
        ],
        "the start, and what the work sent before its first sleep"
    );
    h.advance(Duration::from_millis(100));
    assert_eq!(
        h.app().received[3..],
        [
            Inner::Line("layer 1"),
            Inner::Built("sha256:4f2a"),
            Inner::Event(TaskEvent::Finished { id, outcome: TaskOutcome::Done }),
        ],
        "what the task delivered after the clock moved"
    );
}

#[test]
fn a_mapped_cancel_stops_the_mapped_task() {
    let mut h = harness();
    h.send(Msg::Run(Kind::Task)).send(Msg::Run(Kind::CancelTask));
    let id = h.app().task.expect("the task started");
    assert_eq!(
        h.app().received.last(),
        Some(&Inner::Event(TaskEvent::Finished { id, outcome: TaskOutcome::Cancelled }))
    );
    assert!(!h.app().received.contains(&Inner::Built("sha256:4f2a")), "a cancelled task drops its result");
}

#[test]
fn both_answers_of_a_confirm_arrive_mapped() {
    let mut h = harness();
    h.send(Msg::Run(Kind::Confirm)).click_text("Delete");
    h.send(Msg::Run(Kind::Confirm)).press("esc");
    assert_eq!(h.app().received, [Inner::Confirmed, Inner::Cancelled]);
}

#[test]
fn a_toast_action_and_press_arrive_mapped_and_its_key_still_dismisses() {
    let mut h = harness();
    h.send(Msg::Run(Kind::Toast)).click_text("Deployed").click_text("Undo");
    assert_eq!(h.app().received, [Inner::Opened, Inner::Undo]);
    assert!(!h.screen().contains("Deployed"), "the action dismissed the toast");
    h.send(Msg::Run(Kind::Toast));
    assert!(h.screen().contains("Deployed"));
    h.send(Msg::Run(Kind::DismissToast));
    assert!(!h.screen().contains("Deployed"), "the mapped dismiss found the toast by its key");
}

#[test]
fn the_clipboard_text_arrives_mapped() {
    let mut h = harness();
    h.set_system_clipboard(Some("from another program")).send(Msg::Run(Kind::ReadClipboard));
    assert_eq!(h.app().received, [Inner::Read(Some("from another program".into()))]);
}

#[test]
fn a_handoff_is_recorded_unchanged_and_its_outcome_arrives_mapped() {
    let mut h = harness();
    h.set_handoff_outcome(HandoffOutcome::Finished { code: Some(1) }).send(Msg::Run(Kind::Handoff));
    let request = &h.handoffs()[0];
    assert_eq!((request.program.to_str(), request.notice.as_deref()), (Some("sudo"), Some("Authorizing")));
    assert_eq!(h.app().received, [Inner::Back(HandoffOutcome::Finished { code: Some(1) })]);
}

#[test]
fn a_mapped_handoff_keeps_its_directory_environment_notice_and_pause() {
    let command = Command::handoff(
        Handoff::new("sh", Inner::Back)
            .args(["-c", r#"test "$(pwd)" = / && test "$QUVYTA_MAP_TEST" = ok"#])
            .dir("/")
            .env("QUVYTA_MAP_TEST", "ok")
            .notice("Checking")
            .pause(true),
    )
    .map(Msg::Screen);
    let Some(Action::Handoff(handoff)) = command.actions.into_iter().next() else {
        panic!("the mapped command still holds the handoff");
    };
    let steps = std::cell::RefCell::new(Vec::new());
    let mut release = |notice: Option<&str>| -> io::Result<()> {
        steps.borrow_mut().push(format!("release {}", notice.unwrap_or_default()));
        Ok(())
    };
    let mut take = || -> io::Result<()> {
        steps.borrow_mut().push("take".to_owned());
        Ok(())
    };
    let mut wait_for_key = || -> io::Result<()> {
        steps.borrow_mut().push("key".to_owned());
        Ok(())
    };
    let message = handoff::run(
        handoff,
        &mut HandoffScreen { release: &mut release, take: &mut take, wait_for_key: &mut wait_for_key },
    );
    assert_eq!(message, Msg::Screen(Inner::Back(HandoffOutcome::Finished { code: Some(0) })), "dir and env reached sh");
    assert_eq!(steps.into_inner(), ["release Checking", "key", "take"], "the notice and the pause survived");
}

#[test]
fn work_without_messages_is_carried_over_unchanged() {
    let mut h = harness();
    h.send(Msg::Run(Kind::Settings));
    assert_eq!(h.env().theme().id(), "amber");
    assert_eq!(h.copied(), ["copied through the screen"]);
    assert!(h.is_focused("name"), "the mapped focus request found the field");
    assert!(h.app().received.is_empty(), "none of it sends a message");
}

#[test]
fn a_mapping_closure_may_hold_shared_state_and_runs_on_the_task_thread() {
    // The conversion is `Send + Sync`: here it counts, from the task's own thread, how many
    // messages went through it.
    let count = Arc::new(Mutex::new(0));
    let seen = Arc::clone(&count);
    let command = Command::perform(|| Inner::Performed("counted")).map(move |inner| {
        *seen.lock().expect("unpoisoned") += 1;
        Msg::Screen(inner)
    });
    let Some(Action::Perform(work)) = command.actions.into_iter().next() else {
        panic!("the mapped command still holds the work");
    };
    let message = std::thread::spawn(work).join().expect("the work ran");
    assert_eq!(message, Msg::Screen(Inner::Performed("counted")));
    assert_eq!(*count.lock().expect("unpoisoned"), 1);
}

#[test]
fn a_command_mapped_twice_nests_the_conversions() {
    #[derive(Debug, PartialEq)]
    enum Outer {
        Host(Msg),
    }
    let command = Command::perform(|| Inner::Performed("deep")).map(Msg::Screen).map(Outer::Host);
    let Some(Action::Perform(work)) = command.actions.into_iter().next() else {
        panic!("one perform");
    };
    assert_eq!(work(), Outer::Host(Msg::Screen(Inner::Performed("deep"))));
}
