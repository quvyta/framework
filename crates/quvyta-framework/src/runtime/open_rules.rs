//! Rules for an opening in the [`Harness`]: it is recorded rather than carried out, the test
//! answers it, and the screen is never touched.

use std::ffi::{OsStr, OsString};

use super::{App, Command, Harness, OpenOutcome};
use crate::widget::View;

/// A screen that opens things on the person's own desktop, as an application's own module would.
mod links {
    use crate::runtime::{Command, Open, OpenOutcome};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Msg {
        /// Opens the guide and says nothing about it.
        Guide,
        /// Opens an address and wants to know whether it was handed over.
        SignIn,
        Opened(OpenOutcome),
        /// Starts a program of its own instead of the desktop's opener.
        Draw,
    }

    #[derive(Default)]
    pub struct State {
        pub heard: Vec<String>,
    }

    pub fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Guide => Command::open("https://quvyta.com/guide"),
            Msg::SignIn => Command::open_with(Open::new("https://example.com/sign-in").answer(Msg::Opened)),
            Msg::Draw => Command::open_with(Open::program("gimp").arg("--new-instance").dir("/tmp")),
            Msg::Opened(outcome) => {
                state.heard.push(match outcome {
                    OpenOutcome::Opened => "opened".to_owned(),
                    OpenOutcome::Failed(reason) => format!("failed {reason}"),
                });
                Command::none()
            }
        }
    }
}

/// The application: the link screen, placed with `Command::map`.
#[derive(Default)]
struct Host {
    links: links::State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Msg {
    Links(links::Msg),
}

impl App for Host {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Links(msg) => links::update(&mut self.links, msg).map(Msg::Links),
        }
    }
    fn view(&self, _ui: &mut View<'_, Msg>) {}
}

fn send(msg: links::Msg) -> Msg {
    Msg::Links(msg)
}

#[test]
fn an_opening_is_recorded_with_what_it_was_asked_to_open_and_no_screen_is_given_away() {
    let mut h = Harness::new(Host::default(), 20, 2);
    assert!(h.opens().is_empty());
    h.send(send(links::Msg::Guide));
    let asked = h.opens();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].target.as_deref(), Some(OsStr::new("https://quvyta.com/guide")));
    assert_eq!(asked[0].args, [OsString::from("https://quvyta.com/guide")]);
    // The point of an opening: the terminal is never handed over, so nothing blinks.
    assert!(h.handoffs().is_empty(), "no handoff waits for it");
    assert!(h.detached_handoffs().is_empty(), "and none detaches either");
    assert!(h.app().links.heard.is_empty(), "this opening asked for no message");
}

#[test]
fn the_outcome_the_test_sets_reaches_the_application_through_a_mapped_message() {
    let mut h = Harness::new(Host::default(), 20, 2);
    h.send(send(links::Msg::SignIn));
    assert_eq!(h.app().links.heard, ["opened"], "without an outcome the opening simply works");
    h.set_open_outcome(OpenOutcome::Failed("no opener on this desktop".to_owned()));
    h.send(send(links::Msg::SignIn));
    assert_eq!(h.app().links.heard[1..], ["failed no opener on this desktop"]);
    assert_eq!(h.opens().len(), 2);
    assert_eq!(h.opens()[1].target.as_deref(), Some(OsStr::new("https://example.com/sign-in")));
}

#[test]
fn a_program_of_its_own_is_recorded_with_its_arguments_and_no_target() {
    let mut h = Harness::new(Host::default(), 20, 2);
    h.send(send(links::Msg::Draw));
    let asked = h.opens();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].program, OsString::from("gimp"));
    assert_eq!(asked[0].args, [OsString::from("--new-instance")]);
    assert_eq!(asked[0].target, None, "nothing was handed to an opener");
}
