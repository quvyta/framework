//! Rules for [`View::map`]: a screen written for its own messages behaves inside the
//! application's view exactly as it would alone, and everything it sends arrives converted.

use std::time::Duration;

use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, Modal, Text};

/// A screen with messages of its own: it knows nothing of the application around it.
mod screen {
    use crate::runtime::Command;
    use crate::widget::{Length, View};
    use crate::widgets::{Button, Modal, Panel, Select, TextInput};

    #[derive(Debug, Clone, PartialEq)]
    pub enum Msg {
        Save,
        Typed(String),
        Picked(usize),
        Open,
        Confirm,
        Close,
        FocusName,
    }

    #[derive(Default)]
    pub struct Screen {
        pub name: String,
        pub dialog: bool,
    }

    impl Screen {
        pub fn update(&mut self, msg: &Msg) -> Command<Msg> {
            match msg {
                Msg::Typed(text) => self.name.clone_from(text),
                Msg::Open => self.dialog = true,
                Msg::Close | Msg::Confirm => self.dialog = false,
                Msg::FocusName => return Command::focus("name"),
                Msg::Save | Msg::Picked(_) => {}
            }
            Command::none()
        }

        pub fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add_with(Panel::new().title("Profile"), |ui| {
                ui.row(|ui| {
                    ui.add(TextInput::new(&self.name).on_change(Msg::Typed)).id("name").width(Length::Cells(20));
                    ui.column(|ui| {
                        ui.add(Button::new("Save").on_press(Msg::Save)).id("save");
                    });
                })
                .gap(1);
                ui.add(Select::new(["Monochrome", "Amber"]).selected(Some(0)).on_select(Msg::Picked)).id("theme");
                ui.add(Button::new("Open dialog").on_press(Msg::Open));
            });
            if self.dialog {
                ui.add_with(Modal::new().title("Delete profile?").on_close(Msg::Close), |ui| {
                    ui.add(Button::new("Delete it").on_press(Msg::Confirm)).id("confirm");
                });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    /// Messages of the screen in tab `usize`.
    Screen(usize, screen::Msg),
    Quit,
}

#[derive(Default)]
struct Host {
    screen: screen::Screen,
    received: Vec<Msg>,
}

impl App for Host {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        self.received.push(msg.clone());
        match msg {
            Msg::Screen(tab, inner) => self.screen.update(&inner).map(move |inner| Msg::Screen(tab, inner)),
            Msg::Quit => Command::none(),
        }
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Button::new("Quit").on_press(Msg::Quit));
        // The conversion captures state: the tab the screen sits in.
        let tab = 7;
        ui.map(move |inner| Msg::Screen(tab, inner), |ui| self.screen.view(ui)).fill();
    }
}

fn harness() -> Harness<Host> {
    let mut h = Harness::new(Host::default(), 70, 20);
    h.set_reduced_motion(true);
    h
}

#[test]
fn a_button_of_the_screen_sends_its_message_through_the_map() {
    let mut h = harness();
    h.click_text("Save");
    assert_eq!(h.app().received, [Msg::Screen(7, screen::Msg::Save)]);
    h.click_text("Quit");
    assert_eq!(h.app().received.last(), Some(&Msg::Quit), "the application's own widgets are untouched");
}

#[test]
fn handlers_inside_nested_containers_arrive_mapped() {
    let mut h = harness();
    h.click_text("Save").press("shift+tab").type_text("ada");
    assert!(h.is_focused("name"), "focus moves through the screen's widgets like any others");
    assert_eq!(h.app().screen.name, "ada");
    assert_eq!(h.app().received.last(), Some(&Msg::Screen(7, screen::Msg::Typed("ada".into()))));
}

#[test]
fn a_focus_request_of_the_screen_finds_its_widget() {
    let mut h = harness();
    h.send(Msg::Screen(7, screen::Msg::FocusName));
    assert!(h.is_focused("name"));
    h.type_text("x");
    assert_eq!(h.app().screen.name, "x");
}

#[test]
fn an_open_dropdown_of_the_screen_chooses_through_the_map() {
    let mut h = harness();
    h.click_text("Monochrome").advance(Duration::from_millis(300));
    h.press("down").press("enter");
    assert_eq!(h.app().received.last(), Some(&Msg::Screen(7, screen::Msg::Picked(1))), "{}", h.screen());
}

#[test]
fn a_modal_of_the_screen_is_a_layer_whose_buttons_and_close_arrive_mapped() {
    let mut h = harness();
    h.click_text("Open dialog").advance(Duration::from_millis(300));
    assert!(h.screen().contains("Delete profile?"), "{}", h.screen());
    h.click_text("Save");
    assert!(!h.app().received.contains(&Msg::Screen(7, screen::Msg::Save)), "the layer is modal");
    h.click_text("Delete it").advance(Duration::from_millis(300));
    assert_eq!(h.app().received.last(), Some(&Msg::Screen(7, screen::Msg::Confirm)));
    assert!(!h.screen().contains("Delete profile?"), "{}", h.screen());

    h.click_text("Open dialog").advance(Duration::from_millis(300));
    h.press("esc").advance(Duration::from_millis(300));
    assert_eq!(h.app().received.last(), Some(&Msg::Screen(7, screen::Msg::Close)));
    assert!(!h.screen().contains("Delete profile?"));
}

#[derive(Debug, Clone, PartialEq)]
enum Middle {
    Inner(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
enum Outer {
    Middle(Middle),
}

/// A screen placed inside a screen: both conversions apply, innermost first.
struct Nested {
    received: Vec<Outer>,
}

impl App for Nested {
    type Msg = Outer;

    fn update(&mut self, msg: Outer) -> Command<Outer> {
        self.received.push(msg);
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Outer>) {
        ui.map(Outer::Middle, |ui| {
            ui.add(Text::new("middle"));
            ui.map(Middle::Inner, |ui| {
                ui.add(Button::new("Deep").on_press("deep"));
            });
        });
    }
}

#[test]
fn a_screen_inside_a_screen_converts_twice() {
    let mut h = Harness::new(Nested { received: Vec::new() }, 30, 4);
    assert_eq!(h.screen().lines().next(), Some("middle"));
    h.click_text("Deep");
    assert_eq!(h.app().received, [Outer::Middle(Middle::Inner("deep"))]);
}

/// A modal of the application whose body is a screen with two buttons.
struct Dialog;

impl App for Dialog {
    type Msg = Msg;

    fn update(&mut self, _msg: Msg) -> Command<Msg> {
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add_with(Modal::new().title("Two buttons"), |ui| {
            ui.map(
                |inner| Msg::Screen(0, inner),
                |ui| {
                    ui.add(Button::new("One").on_press(screen::Msg::Save));
                    ui.add(Button::new("Two").on_press(screen::Msg::Open));
                },
            );
        });
    }
}

#[test]
fn a_layer_counts_the_focusable_widgets_of_a_screen_inside_it() {
    let mut h = Harness::new(Dialog, 50, 12);
    h.set_reduced_motion(true).render();
    assert!(h.screen().contains("tab switch"), "two buttons in the screen offer switching:\n{}", h.screen());
}

/// A screen that watches the silence: its watch reaches the runtime through the mapped part,
/// and the message it sends arrives converted.
#[test]
fn a_screen_watching_idleness_sends_its_message_converted() {
    #[derive(Debug, Clone, PartialEq)]
    enum Inner {
        Away(bool),
    }

    #[derive(Default)]
    struct Idle {
        seen: Vec<(usize, bool)>,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Outer {
        Screen(usize, Inner),
    }

    impl App for Idle {
        type Msg = Outer;

        fn update(&mut self, msg: Outer) -> Command<Outer> {
            let Outer::Screen(tab, Inner::Away(away)) = msg;
            self.seen.push((tab, away));
            Command::none()
        }

        fn view(&self, ui: &mut View<'_, Outer>) {
            ui.map(
                |inner| Outer::Screen(3, inner),
                |ui| {
                    ui.on_idle(Duration::from_secs(60), Inner::Away);
                    let silent = ui.idle_for();
                    ui.add(Text::new(format!("silent {}", silent.as_secs())));
                },
            );
        }
    }

    let mut h = Harness::new(Idle::default(), 30, 3);
    assert!(h.app().seen.is_empty());
    h.advance(Duration::from_secs(61));
    assert_eq!(h.app().seen, vec![(3, true)], "the watch fired once, converted");
    assert!(h.screen().contains("silent 61"), "the screen read the silence and was drawn again:\n{}", h.screen());
    h.press("tab");
    assert_eq!(h.app().seen, vec![(3, true), (3, false)], "input ends the silence, converted");
}
