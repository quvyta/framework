//! A widget that takes every key while it has focus asking which of them belong to the
//! application (`EventCx::is_reserved`), the way an embedded page, a terminal or a coding tool's
//! own screen has to, so the bindings a person expects keep working inside it.

use crate::env::{AssetDirs, Env};
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::runtime::{App, Command, Harness};
use crate::style::CellStyle;
use crate::widget::{EventCx, MeasureCx, PaintCx, View, Widget};

/// What the screen was handed: the key as a person writes it, and whether the application owns it.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    key: String,
    reserved: bool,
}

#[derive(Default)]
struct Demo {
    seen: Vec<Seen>,
    actions: Vec<String>,
}

#[derive(Clone, Debug)]
enum Msg {
    Key(Seen),
    Action(String),
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Key(seen) => self.seen.push(seen),
            Msg::Action(name) => self.actions.push(name),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Screen).fill().id("screen");
    }
    fn action(&self, name: &str) -> Option<Msg> {
        Some(Msg::Action(name.to_owned()))
    }
}

/// A screen that takes every key while it has focus and uses it, the shape an embedded page has.
struct Screen;

impl Widget<Msg> for Screen {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        cx.text(area.x, area.y, "harness screen", CellStyle::default(), area.width);
    }
    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !cx.is_focused() {
            return false;
        }
        let Event::Key(key) = event else { return false };
        let reserved = cx.is_reserved(&key.chord);
        cx.emit(Msg::Key(Seen { key: key.chord.label(), reserved }));
        // A key the application owns is not this screen's to use, so it goes on as usual.
        !reserved
    }
    fn focusable(&self) -> bool {
        true
    }
}

/// The application's own keys, as the text an application would compile into its binary.
fn screen(keys: &str) -> Harness<Demo> {
    let dirs = AssetDirs { keymap_source: Some(("keymap.toml".to_owned(), keys.to_owned())), ..AssetDirs::default() };
    let env = Env::load_with(&dirs, |_| None).expect("the keymap comes from text and nothing is read from disk");
    Harness::with_env(Demo::default(), env, 40, 8)
}

/// The screen focused by a click, the way a person starts typing into it.
fn focused(keys: &str) -> Harness<Demo> {
    let mut h = screen(keys);
    h.click_text("harness screen");
    assert!(h.is_focused("screen"), "{}", h.screen());
    h
}

fn seen(key: &str, reserved: bool) -> Seen {
    Seen { key: key.to_owned(), reserved }
}

#[test]
fn a_screen_taking_every_key_leaves_the_applications_own_to_it() {
    let mut h = focused("[app]\nrebuild = \"ctrl+l\"\n");
    h.press("ctrl+l").press("ctrl+k");
    assert_eq!(h.app().seen, [seen("ctrl l", true), seen("ctrl k", false)]);
    assert_eq!(h.app().actions, ["rebuild"], "the reserved key went on and did the usual thing");
}

#[test]
fn rebinding_an_action_in_the_keymap_moves_the_answer_to_the_new_key() {
    let mut h = focused("[app]\nrebuild = \"ctrl+g\"\n");
    h.press("ctrl+l").press("ctrl+g");
    assert_eq!(h.app().seen, [seen("ctrl l", false), seen("ctrl g", true)]);
    assert_eq!(h.app().actions, ["rebuild"]);
}

#[test]
fn the_quit_key_is_reserved_and_quits_from_inside_a_screen() {
    let mut h = focused("[app]\nrebuild = \"ctrl+l\"\n");
    h.press("ctrl+q");
    assert_eq!(h.app().seen, [seen("ctrl q", true)]);
    assert!(h.quit_requested(), "the runtime's own action was let through to it");
}
