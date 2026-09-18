//! Rules for a detached handoff in the [`Harness`]: the request is recorded, the test answers
//! it, and a stand-in child plays the program for the rest of the test.

use std::ffi::OsString;
use std::sync::{Arc, Mutex};

use super::{App, ChildLine, Command, DetachedHandoff, DetachedOutcome, Harness, LiveChild, TestChild};
use crate::widget::View;

/// A screen that starts a helper and talks to it, as an application's own module would.
mod helper {
    use crate::runtime::{ChildLine, Command, DetachedHandoff, DetachedOutcome, LiveChild};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Msg {
        Start,
        Started(DetachedOutcome),
        Said(ChildLine),
        Ask(&'static str),
        Stop,
    }

    #[derive(Default)]
    pub struct State {
        pub child: Option<LiveChild>,
        pub heard: Vec<String>,
    }

    pub fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Start => {
                return Command::handoff_detached(
                    DetachedHandoff::new("pkexec", Msg::Started)
                        .args(["/usr/lib/quvyta/helper", "--serve"])
                        .notice("Asking for permission")
                        .on_line(Msg::Said),
                );
            }
            Msg::Started(DetachedOutcome::Detached { child, first_line }) => {
                state.heard.push(format!("first {first_line}"));
                state.child = Some(child);
            }
            Msg::Started(DetachedOutcome::Finished { code }) => state.heard.push(format!("finished {code:?}")),
            Msg::Started(DetachedOutcome::Failed(reason)) => state.heard.push(format!("failed {reason}")),
            Msg::Said(ChildLine::Line(line)) => state.heard.push(line),
            Msg::Said(ChildLine::Ended { code }) => {
                state.heard.push(format!("ended {code:?}"));
                state.child = None;
            }
            Msg::Ask(line) => {
                if let Some(child) = &state.child {
                    child.write_line(line).expect("the stand-in's input is open");
                }
            }
            Msg::Stop => {
                if let Some(child) = state.child.take() {
                    child.close_stdin();
                }
            }
        }
        Command::none()
    }
}

/// The application: the helper screen, placed with `Command::map`.
#[derive(Default)]
struct Host {
    helper: helper::State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Msg {
    Helper(helper::Msg),
}

impl App for Host {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Helper(msg) => helper::update(&mut self.helper, msg).map(Msg::Helper),
        }
    }
    fn view(&self, _ui: &mut View<'_, Msg>) {}
}

fn send(msg: helper::Msg) -> Msg {
    Msg::Helper(msg)
}

/// A harness whose detached handoffs detach with a stand-in child that said `ready`.
fn detaching() -> (Harness<Host>, TestChild, LiveChild) {
    let mut h = Harness::new(Host::default(), 20, 2);
    let (child, program) = LiveChild::for_tests();
    h.set_detached_outcome(DetachedOutcome::Detached { child: child.clone(), first_line: "ready".to_owned() });
    (h, program, child)
}

#[test]
fn a_detached_handoff_is_recorded_and_answered_with_the_outcome_the_test_set() {
    let mut h = Harness::new(Host::default(), 20, 2);
    assert!(h.detached_handoffs().is_empty());
    h.send(send(helper::Msg::Start));
    let asked = h.detached_handoffs();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].program, OsString::from("pkexec"));
    assert_eq!(asked[0].args, ["/usr/lib/quvyta/helper", "--serve"].map(OsString::from));
    assert_eq!(asked[0].notice.as_deref(), Some("Asking for permission"));
    assert!(h.handoffs().is_empty(), "handoffs that wait are recorded apart");
    assert_eq!(h.app().helper.heard, ["finished Some(0)"], "without an outcome the program simply ends");
    h.set_detached_outcome(DetachedOutcome::Finished { code: Some(126) });
    h.send(send(helper::Msg::Start));
    h.set_detached_outcome(DetachedOutcome::Failed("pkexec is not installed".to_owned()));
    h.send(send(helper::Msg::Start));
    assert_eq!(h.app().helper.heard[1..], ["finished Some(126)", "failed pkexec is not installed"]);
    assert_eq!(h.detached_handoffs().len(), 3);
}

#[test]
fn a_stand_in_child_hears_the_application_and_answers_it_through_mapped_messages() {
    let (mut h, program, child) = detaching();
    // Said before the application had it: kept, as a pipe keeps it.
    program.say("early");
    h.send(send(helper::Msg::Start));
    assert_eq!(h.app().helper.child.as_ref(), Some(&child), "the application holds the test's child");
    assert_eq!(h.app().helper.heard, ["first ready", "early"], "the first line comes with the outcome");
    h.send(send(helper::Msg::Ask("list-updates")));
    assert_eq!(program.written(), ["list-updates"]);
    program.say("3 updates");
    assert_eq!(h.app().helper.heard.len(), 2, "a line waits for the next step");
    h.render();
    assert_eq!(h.app().helper.heard[2..], ["3 updates"]);
    program.exit(Some(0));
    h.render();
    assert_eq!(h.app().helper.heard[3..], ["ended Some(0)"]);
    assert_eq!(child.try_wait().expect("the stand-in's state"), Some(Some(0)));
    assert!(!program.killed());
}

#[test]
fn the_childs_input_closes_when_the_application_says_so_or_goes() {
    let (mut h, program, child) = detaching();
    h.send(send(helper::Msg::Start));
    assert!(program.stdin_open());
    h.send(send(helper::Msg::Stop));
    assert!(!program.stdin_open(), "closing reaches every clone");
    assert!(child.write_line("late").is_err());

    let (mut h, program, child) = detaching();
    drop(child);
    h.send(send(helper::Msg::Start));
    assert!(program.stdin_open(), "the application and the harness hold the child");
    drop(h);
    assert!(!program.stdin_open(), "the child's input closed with the application");
}

#[test]
fn killing_a_stand_in_child_ends_it_by_a_signal_and_it_says_nothing_after() {
    let (child, program) = LiveChild::for_tests();
    assert_eq!(child.id(), None, "the stand-in has no process");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    program.say("one");
    child.attach(Box::new(move |line| sink.lock().expect("lines").push(line)));
    child.kill().expect("the stand-in can always be killed");
    assert!(program.killed());
    assert_eq!(child.try_wait().expect("its state"), Some(None));
    program.say("after the end");
    program.exit(Some(0));
    assert_eq!(
        *seen.lock().expect("lines"),
        [ChildLine::Line("one".to_owned()), ChildLine::Ended { code: None }],
        "the line said before the sink came first, and nothing follows the end"
    );
}

#[test]
fn a_mapped_detached_handoff_delivers_both_of_its_messages_mapped() {
    let command: Command<Msg> = Command::handoff_detached(
        DetachedHandoff::new("sh", helper::Msg::Started).arg("-c").arg("echo ready; cat").on_line(helper::Msg::Said),
    )
    .map(Msg::Helper);
    let Some(super::command::Action::HandoffDetached(handoff)) = command.actions.into_iter().next() else {
        panic!("the mapped command still holds the detached handoff");
    };
    let mut release = |_: Option<&str>| Ok(());
    let mut take = || Ok(());
    let mut wait_for_key = || Ok(());
    let (deliveries, lines) = std::sync::mpsc::channel();
    let message = super::detached::run(
        handoff,
        &mut super::handoff::HandoffScreen { release: &mut release, take: &mut take, wait_for_key: &mut wait_for_key },
        &deliveries,
    );
    let Msg::Helper(helper::Msg::Started(DetachedOutcome::Detached { child, first_line })) = message else {
        panic!("the outcome arrives mapped: {message:?}");
    };
    assert_eq!(first_line, "ready");
    child.write_line("echo").expect("the child reads");
    let line = lines.recv_timeout(std::time::Duration::from_secs(30)).expect("a line");
    let super::task::Delivery::Message(line) = line else {
        panic!("a line is a message");
    };
    assert_eq!(line, Msg::Helper(helper::Msg::Said(ChildLine::Line("echo".to_owned()))));
}
