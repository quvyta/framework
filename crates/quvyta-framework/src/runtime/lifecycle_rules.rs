//! The application's lifecycle: the start-up command, the size in `update` and the question
//! before quitting, driven through the harness the way the terminal runtime drives them.

use std::time::Duration;

use crate::geometry::Size;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, CommandPalette, List, ListItem, Text};

/// Records every lifecycle step it hears, in order.
#[derive(Default)]
struct Journal {
    /// What happened, oldest first: `init`, `size WxH`, `asked`, `key`.
    steps: Vec<String>,
    /// The size the application knows in `update`.
    size: Option<Size>,
    /// The row the list shows selected.
    selected: usize,
    /// Whether quitting asks first.
    guard: bool,
    /// What `init` returns.
    first: Option<fn() -> Command<Msg>>,
    palette: bool,
}

#[derive(Clone)]
enum Msg {
    Resized(Size),
    AskBeforeQuit,
    Select(usize),
    Pressed,
    Quit,
    ClosePalette,
    OpenPalette,
}

impl App for Journal {
    type Msg = Msg;

    fn init(&mut self) -> Command<Msg> {
        self.steps.push("init".to_owned());
        self.first.map_or_else(Command::none, |first| first())
    }

    fn resized(&self, size: Size) -> Option<Msg> {
        Some(Msg::Resized(size))
    }

    fn before_quit(&self) -> Option<Msg> {
        self.guard.then_some(Msg::AskBeforeQuit)
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Resized(size) => {
                self.steps.push(format!("size {}x{}", size.width, size.height));
                self.size = Some(size);
            }
            Msg::AskBeforeQuit => self.steps.push("asked".to_owned()),
            Msg::Select(row) => self.selected = row,
            Msg::Pressed => self.steps.push("key".to_owned()),
            Msg::Quit => return Command::quit(),
            Msg::ClosePalette => self.palette = false,
            Msg::OpenPalette => self.palette = true,
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Button::new("Refresh").on_press(Msg::Pressed)).id("refresh");
        let rows = ["alpha", "beta", "gamma"].map(ListItem::new);
        ui.add(List::new(rows).selected(Some(self.selected)).on_select(Msg::Select)).id("files");
        if self.palette {
            ui.add(CommandPalette::new(Vec::new(), Msg::ClosePalette).keymap(true));
        }
    }

    fn action(&self, name: &str) -> Option<Msg> {
        (name == "palette").then_some(Msg::OpenPalette)
    }
}

fn focus_files() -> Command<Msg> {
    Command::focus("files")
}

fn focus_refresh() -> Command<Msg> {
    Command::focus("refresh")
}

fn quit_at_once() -> Command<Msg> {
    Command::quit()
}

#[test]
fn the_start_up_command_runs_once_after_the_first_size() {
    let mut h = Harness::new(Journal::default(), 40, 6);
    assert_eq!(h.app().steps, ["size 40x6", "init"], "the size is known when init runs");
    h.press("tab").press("enter").advance(Duration::from_secs(2)).render();
    h.resize(30, 5).set_theme("amber").set_locale("tr");
    let inits = h.app().steps.iter().filter(|step| *step == "init").count();
    assert_eq!(inits, 1, "no later frame, input, resize or setting runs it again: {:?}", h.app().steps);
}

#[test]
fn the_first_key_reaches_the_control_the_start_up_command_focused() {
    let mut h = Harness::new(Journal { first: Some(focus_files), ..Journal::default() }, 40, 6);
    assert!(h.is_focused("files"), "focused before any input");
    h.press("down");
    assert_eq!(h.app().selected, 1, "the very first arrow key moved the list");
}

#[test]
fn the_first_frame_is_drawn_with_the_start_up_focus() {
    let focused = Harness::new(Journal { first: Some(focus_refresh), ..Journal::default() }, 40, 6);
    let plain = Harness::new(Journal::default(), 40, 6);
    let mut tabbed = Harness::new(Journal::default(), 40, 6);
    tabbed.press("tab");
    assert!(focused.is_focused("refresh") && tabbed.is_focused("refresh"));
    assert_ne!(focused.bg(2, 0), plain.bg(2, 0), "the focus shows without another event");
    assert_eq!(focused.bg(2, 0), tabbed.bg(2, 0), "the button looks focused, as after reaching it with tab");
}

#[test]
fn a_start_up_quit_quits() {
    let h = Harness::new(Journal { first: Some(quit_at_once), guard: true, ..Journal::default() }, 40, 6);
    assert!(h.quit_requested());
    assert!(!h.app().steps.contains(&"asked".to_owned()), "the application's own quit is not asked about");
}

#[test]
fn update_knows_the_size_at_start_and_after_every_resize() {
    let mut h = Harness::new(Journal::default(), 120, 30);
    assert_eq!(h.app().size, Some(Size::new(120, 30)));
    h.resize(60, 20);
    assert_eq!(h.app().size, Some(Size::new(60, 20)));
    h.resize(60, 20).press("tab").advance(Duration::from_secs(1));
    let reports = h.app().steps.iter().filter(|step| step.starts_with("size")).count();
    assert_eq!(reports, 2, "an unchanged size is not reported again: {:?}", h.app().steps);
}

/// Shows the size it knows from `update` next to the size its view is drawn at.
#[derive(Default)]
struct Sizes {
    known: Size,
}

impl App for Sizes {
    type Msg = Size;

    fn resized(&self, size: Size) -> Option<Size> {
        Some(size)
    }

    fn update(&mut self, size: Size) -> Command<Size> {
        self.known = size;
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Size>) {
        let drawn = ui.size();
        ui.add(Text::new(format!("{}x{} {}x{}", self.known.width, self.known.height, drawn.width, drawn.height)));
    }
}

#[test]
fn the_size_in_update_is_the_size_of_the_frame_being_drawn() {
    let mut h = Harness::new(Sizes::default(), 30, 2);
    assert_eq!(h.screen().lines().next(), Some("30x2 30x2"), "the very first frame agrees");
    h.resize(24, 3);
    assert_eq!(h.screen().lines().next(), Some("24x3 24x3"), "so does the first frame after a resize");
}

#[test]
fn the_quit_key_asks_first_and_the_answer_quits_without_asking_again() {
    let mut h = Harness::new(Journal { guard: true, ..Journal::default() }, 40, 6);
    h.press("ctrl+q");
    assert!(!h.quit_requested(), "the application answered with a message, so it stays");
    assert_eq!(h.app().steps.last().map(String::as_str), Some("asked"));
    h.press("ctrl+q");
    assert_eq!(h.app().steps.iter().filter(|step| *step == "asked").count(), 2, "asking again asks again");
    h.send(Msg::Quit);
    assert!(h.quit_requested(), "Command::quit is the application's decision");
    assert_eq!(h.app().steps.iter().filter(|step| *step == "asked").count(), 2, "and it is not asked about");
}

#[test]
fn the_quit_action_run_from_the_command_palette_asks_first() {
    let mut h = Harness::new(Journal { guard: true, ..Journal::default() }, 60, 20);
    h.press("ctrl+p").advance(Duration::from_millis(200));
    h.type_text("quit").press("enter");
    assert!(!h.quit_requested());
    assert_eq!(h.app().steps.last().map(String::as_str), Some("asked"), "{}", h.screen());
    let mut h = Harness::new(Journal::default(), 60, 20);
    h.press("ctrl+p").advance(Duration::from_millis(200));
    h.type_text("quit").press("enter");
    assert!(h.quit_requested(), "without an answer the palette's quit quits");
}

#[test]
fn a_quit_the_application_lets_through_quits_at_once() {
    let mut h = Harness::new(Journal::default(), 40, 6);
    h.press("ctrl+q");
    assert!(h.quit_requested());
    assert!(!h.app().steps.contains(&"asked".to_owned()));
}

/// An application that implements none of the lifecycle hooks.
#[derive(Default)]
struct Plain {
    presses: u32,
}

impl App for Plain {
    type Msg = ();

    fn update(&mut self, (): ()) -> Command<()> {
        self.presses += 1;
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Button::new("Press").on_press(())).id("press");
        ui.add(Button::new("Other").on_press(())).id("other");
    }
}

#[test]
fn an_application_without_hooks_behaves_as_before() {
    let mut h = Harness::new(Plain::default(), 30, 4);
    assert!(!h.is_focused("press") && !h.is_focused("other"), "nothing is focused at start");
    assert_eq!(h.app().presses, 0, "no message reached update on its own");
    h.resize(20, 3);
    assert_eq!(h.app().presses, 0, "a resize sends nothing either");
    h.press("tab");
    assert!(h.is_focused("press"), "the first tab reaches the first control");
    h.press("enter");
    assert_eq!(h.app().presses, 1);
    h.press("ctrl+q");
    assert!(h.quit_requested(), "the quit key quits at once");
}
