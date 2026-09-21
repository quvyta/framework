//! Rules for events that arrive together: several keys in one read of the terminal.

use crate::event::{Event, KeyEvent};
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, TextInput};

/// A new-workspace form: a controlled name field and a Create button.
#[derive(Default)]
struct NewWorkspace {
    name: String,
    created: Vec<String>,
}

#[derive(Clone)]
enum Msg {
    Name(String),
    Create,
}

impl App for NewWorkspace {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Name(name) => self.name = name,
            Msg::Create => self.created.push(std::mem::take(&mut self.name)),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(
            TextInput::new(&self.name).placeholder("Workspace name").on_change(Msg::Name).on_submit(|_| Msg::Create),
        )
        .fill_width()
        .id("name");
        ui.add(Button::new("Create").on_press(Msg::Create));
    }
}

/// The keys of `text` as one burst, the way a terminal multiplexer, a slow connection or a paste
/// without bracketed paste hands them over.
fn burst(text: &str) -> Vec<Event> {
    text.chars()
        .map(|c| match c {
            '\n' => "enter".to_owned(),
            c => c.to_string(),
        })
        .map(|chord| Event::Key(KeyEvent::press(&chord)))
        .collect()
}

/// The form with the name field clicked, the way a person starts typing into it.
fn form() -> Harness<NewWorkspace> {
    let mut h = Harness::new(NewWorkspace::default(), 40, 4);
    let (x, y) = h.find("Workspace name").expect("the field shows its placeholder");
    h.click(x, y);
    h
}

#[test]
fn every_key_of_a_burst_reaches_a_controlled_field() {
    // A controlled field computes its new value from the value its node was built with. When
    // four keys arrive in one read and the view is not rebuilt between them, each starts from
    // the same empty value and only the last survives: `demo` becomes `o`.
    let mut h = form();
    h.events(&burst("demo"));
    assert_eq!(h.app().name, "demo");
    assert!(h.screen().contains("demo"), "{}", h.screen());
    h.events(&burst("-api"));
    assert_eq!(h.app().name, "demo-api", "a second burst continues where the first ended");
}

#[test]
fn enter_in_the_same_burst_submits_everything_typed_before_it() {
    let mut h = form();
    h.events(&burst("demo\n"));
    assert_eq!(h.app().created, ["demo"]);
}
