//! Rules for events that arrive together: several keys in one read of the terminal.

use std::time::Duration;

use crate::event::{Event, KeyEvent, KeyKind};
use crate::geometry::{Rect, Size};
use crate::runtime::{App, Command, Harness};
use crate::widget::{EventCx, MeasureCx, PaintCx, View, Widget};
use crate::widgets::{Button, TextArea, TextInput};

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
            ' ' => "space".to_owned(),
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

#[test]
fn spaces_of_a_burst_reach_a_text_field() {
    // Text sent through a terminal multiplexer arrives in one read, spaces well within the
    // 100 ms a held key is guessed from. A text field takes every one of them.
    let mut h = form();
    h.events(&burst("a b c"));
    assert_eq!(h.app().name, "a b c");
    h.events(&burst("  d"));
    assert_eq!(h.app().name, "a b c  d", "two spaces in a row are two spaces");
}

#[test]
fn a_space_the_terminal_repeats_types_into_a_text_field() {
    // Holding Space in a field types spaces, the way holding a letter types letters.
    let mut h = form();
    h.type_text("a").press("space");
    h.key(KeyEvent { kind: KeyKind::Repeat, ..KeyEvent::press("space") });
    assert_eq!(h.app().name, "a  ");
}

/// Notes in a text area.
#[derive(Default)]
struct Notes {
    text: String,
}

impl App for Notes {
    type Msg = String;
    fn update(&mut self, text: String) -> Command<String> {
        self.text = text;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, String>) {
        ui.add(TextArea::new(&self.text).on_change(|text| text)).fill();
    }
}

#[test]
fn both_enters_of_a_burst_reach_a_text_area() {
    let mut h = Harness::new(Notes::default(), 30, 6);
    h.press("tab");
    h.events(&burst("one\n\ntwo"));
    assert_eq!(h.app().text, "one\n\ntwo");
}

/// A widget of the application's own that records the text of every key it is given, and does
/// not say it takes text.
struct Recorder;

impl Widget<char> for Recorder {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
    }
    fn event(&self, cx: &mut EventCx<'_, char>, event: &Event) -> bool {
        match event {
            Event::Key(key) if key.text.is_some() => {
                cx.emit(key.text.unwrap_or_default());
                true
            }
            _ => false,
        }
    }
    fn focusable(&self) -> bool {
        true
    }
}

#[derive(Default)]
struct Recorded(String);

impl App for Recorded {
    type Msg = char;
    fn update(&mut self, c: char) -> Command<char> {
        self.0.push(c);
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, char>) {
        ui.add(Recorder).fill();
    }
}

#[test]
fn a_key_between_two_spaces_shows_neither_is_held() {
    // A held key repeats alone: once another key came between them, a second Space is a new
    // press, however soon it follows. This keeps typed words apart even in a widget that does
    // not say it takes text.
    let mut h = Harness::new(Recorded::default(), 10, 1);
    h.press("tab");
    h.events(&burst("a b c"));
    assert_eq!(h.app().0, "a b c");
}

/// A button that counts its presses.
#[derive(Default)]
struct Counter(u32);

impl App for Counter {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        self.0 += 1;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Button::new("Save").on_press(()));
    }
}

#[test]
fn an_enter_the_terminal_reports_as_a_repeat_does_not_press_a_button_again() {
    let mut h = Harness::new(Counter::default(), 20, 1);
    h.press("tab").press("enter");
    assert_eq!(h.app().0, 1);
    // Long after the first press, so only the terminal's word says it is held.
    h.advance(Duration::from_secs(1));
    h.key(KeyEvent { kind: KeyKind::Repeat, ..KeyEvent::press("enter") });
    assert_eq!(h.app().0, 1, "a held Enter presses once");
    h.press("enter");
    assert_eq!(h.app().0, 2, "a new press still presses");
}
