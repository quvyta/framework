//! Keymap actions answered by the node that holds focus (`NodeMut::on_action`): where focus is
//! decides the message, however focus got there.

use crate::env::Env;
use crate::keymap::{KeyChord, Scope};
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, TextInput};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Inner,
    Outer,
    Screen(Inner),
    FromApp,
    Next,
    Typed(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Inner {
    Answered,
}

#[derive(Default)]
struct Demo {
    heard: Vec<Msg>,
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        self.heard.push(msg);
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            ui.add(TextInput::new("inner").on_change(Msg::Typed)).id("inner").on_action(
                Scope::App,
                "switch",
                Msg::Inner,
            );
            ui.add(TextInput::new("sibling").on_change(Msg::Typed)).id("sibling");
        })
        .on_action(Scope::App, "switch", Msg::Outer)
        .on_action(Scope::Global, "focus-next", Msg::Next)
        .on_action(Scope::App, "letter", Msg::Outer);
        ui.map(Msg::Screen, |ui| {
            ui.add(TextInput::new("mapped")).id("mapped").on_action(Scope::App, "switch", Inner::Answered);
        });
        ui.add(Button::new("outside").on_press(Msg::FromApp)).id("outside");
    }
    fn action(&self, name: &str) -> Option<Msg> {
        matches!(name, "switch" | "letter").then_some(Msg::FromApp)
    }
}

fn chord(text: &str) -> KeyChord {
    text.parse().expect("valid chord")
}

fn harness() -> Harness<Demo> {
    let mut env = Env::builtin();
    env.keymap_mut().bind(Scope::App, "switch", &[chord("alt+s")]);
    env.keymap_mut().bind(Scope::App, "letter", &[chord("x")]);
    Harness::with_env(Demo::default(), env, 40, 8)
}

fn focus(h: &mut Harness<Demo>, name: &str) {
    for _ in 0..6 {
        if h.is_focused(name) {
            return;
        }
        h.press("tab");
    }
    panic!("{name} never took focus");
}

#[test]
fn the_innermost_node_around_focus_answers_and_outside_it_the_application_does() {
    let mut h = harness();
    h.press("alt+s");
    assert_eq!(h.app().heard, [Msg::FromApp], "nothing focused: the action reaches App::action");
    focus(&mut h, "inner");
    h.press("alt+s");
    focus(&mut h, "sibling");
    h.press("alt+s");
    focus(&mut h, "outside");
    h.press("alt+s");
    assert_eq!(h.app().heard[1..], [Msg::Inner, Msg::Outer, Msg::FromApp]);
}

#[test]
fn focus_given_by_a_click_counts_like_any_other() {
    let mut h = harness();
    h.click_text("sibling").press("alt+s");
    assert!(h.is_focused("sibling"));
    assert_eq!(h.app().heard, [Msg::Outer]);
}

#[test]
fn a_node_inside_a_mapped_screen_answers_converted() {
    let mut h = harness();
    focus(&mut h, "mapped");
    h.press("alt+s");
    assert_eq!(h.app().heard, [Msg::Screen(Inner::Answered)]);
}

#[test]
fn the_runtime_actions_and_keys_the_widget_uses_are_not_answered() {
    let mut h = harness();
    focus(&mut h, "inner");
    h.press("tab");
    assert!(h.is_focused("sibling"), "tab still moves focus");
    assert!(!h.app().heard.contains(&Msg::Next));
    h.press("x");
    assert_eq!(h.app().heard, [Msg::Typed("siblingx".into())], "the field typed x, the answer was not sent");
}
