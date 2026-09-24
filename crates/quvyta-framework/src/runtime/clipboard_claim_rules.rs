//! Clipboard keys claimed by the node that holds focus (`NodeMut::on_clipboard`): the focused
//! widget and text selected with the mouse keep them first, and outside the node they do what
//! they do without it.

use crate::env::Env;
use crate::keymap::{KeyChord, Scope};
use crate::runtime::{App, Command, Harness};
use crate::widget::{ClipboardKey, View};
use crate::widgets::{List, ListItem, TextInput};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Cut,
    Copy,
    Paste,
    Screen(Inner),
    Typed(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Inner {
    Copy,
}

#[derive(Default)]
struct Demo {
    heard: Vec<Msg>,
    field: String,
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        if let Msg::Typed(text) = &msg {
            self.field.clone_from(text);
        }
        self.heard.push(msg);
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(TextInput::new(self.field.clone()).on_change(Msg::Typed)).id("outside");
        ui.column(|ui| {
            ui.add(List::new(["a.txt", "b.txt"].map(ListItem::new))).id("rows");
            ui.add(TextInput::new(self.field.clone()).on_change(Msg::Typed)).id("inside");
        })
        .on_clipboard(ClipboardKey::Cut, Msg::Cut)
        .on_clipboard(ClipboardKey::Copy, Msg::Copy)
        .on_clipboard(ClipboardKey::Paste, Msg::Paste);
        ui.map(Msg::Screen, |ui| {
            ui.add(List::new(["c.txt"].map(ListItem::new))).id("mapped").on_clipboard(ClipboardKey::Copy, Inner::Copy);
        });
    }
}

fn harness(env: Env) -> Harness<Demo> {
    Harness::with_env(Demo::default(), env, 40, 8)
}

fn focus(h: &mut Harness<Demo>, name: &str) {
    for _ in 0..8 {
        if h.is_focused(name) {
            return;
        }
        h.press("tab");
    }
    panic!("{name} never took focus");
}

fn heard(h: &Harness<Demo>) -> Vec<Msg> {
    h.app().heard.iter().filter(|msg| !matches!(msg, Msg::Typed(_))).cloned().collect()
}

#[test]
fn a_focused_node_inside_the_claim_gets_the_three_keys() {
    let mut h = harness(Env::builtin());
    focus(&mut h, "rows");
    h.press("ctrl+x").press("ctrl+c").press("ctrl+v");
    assert_eq!(heard(&h), [Msg::Cut, Msg::Copy, Msg::Paste]);
    assert!(h.copied().is_empty(), "no text was copied");
}

#[test]
fn nothing_is_claimed_while_focus_is_outside_the_node_or_nowhere() {
    let mut h = harness(Env::builtin());
    h.press("ctrl+x").press("ctrl+c").press("ctrl+v");
    focus(&mut h, "outside");
    h.type_text("words").press("ctrl+a").press("ctrl+c").press("ctrl+x").press("ctrl+v");
    assert!(heard(&h).is_empty(), "{:?}", h.app().heard);
    assert_eq!(h.copied(), ["words", "words"], "the field copied and cut its own text");
    assert_eq!(h.app().field, "words", "and pasted it back");
}

#[test]
fn a_text_field_inside_the_node_keeps_cutting_and_copying_its_own_text() {
    let mut h = harness(Env::builtin());
    focus(&mut h, "inside");
    h.type_text("inner").press("ctrl+a").press("ctrl+c").press("ctrl+x");
    assert!(heard(&h).is_empty(), "{:?}", h.app().heard);
    assert_eq!(h.copied(), ["inner", "inner"]);
    assert_eq!(h.app().field, "");
}

#[test]
fn a_claim_inside_a_mapped_part_answers_with_the_screens_message() {
    let mut h = harness(Env::builtin());
    focus(&mut h, "mapped");
    h.press("ctrl+c").press("ctrl+x");
    assert_eq!(heard(&h), [Msg::Screen(Inner::Copy)], "Ctrl+X was not claimed there");
}

#[test]
fn copy_and_paste_follow_the_keymap() {
    let mut env = Env::builtin();
    let chord = |text: &str| text.parse::<KeyChord>().expect("a chord");
    env.keymap_mut().bind(Scope::Global, "copy", &[chord("ctrl+shift+c")]);
    let mut h = harness(env);
    focus(&mut h, "rows");
    h.press("ctrl+c");
    assert!(heard(&h).is_empty(), "ctrl+c no longer copies");
    h.press("ctrl+shift+c");
    assert_eq!(heard(&h), [Msg::Copy]);
}
