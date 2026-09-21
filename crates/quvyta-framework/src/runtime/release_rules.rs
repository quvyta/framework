//! Rules for the release of a mouse press: it acts only where the press began.

use crate::event::{MouseButton, MouseKind};
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, SettingRow, SettingsList, Text};

/// A settings page whose row opens a setup page on the press, and that page's Finish button,
/// which covers the cells the row stood on.
#[derive(Default)]
struct Setup {
    open: bool,
    finished: bool,
}

#[derive(Clone)]
enum Msg {
    Open,
    Finish,
}

impl App for Setup {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open => self.open = true,
            Msg::Finish => self.finished = true,
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        if self.open {
            ui.add(Button::new("Finish setup").on_press(Msg::Finish)).fill_width();
        } else {
            SettingsList::show(ui, |list| {
                list.row(SettingRow::new("Location").on_activate(Msg::Open), |ui| {
                    ui.add(Text::new("Change"));
                });
            })
            .fill_width();
        }
    }
}

#[test]
fn a_release_does_not_press_a_button_that_appeared_under_it() {
    // The row acts on the press and the page it opens puts a button where the row was. The
    // release of that same click lands on the button, which was not there when the mouse button
    // went down; pressing it would finish a setup the user never looked at.
    let mut h = Harness::new(Setup::default(), 40, 6);
    let (x, y) = h.find("Location").expect("the settings row");
    h.mouse(MouseKind::Down(MouseButton::Left), x, y);
    assert!(h.app().open, "the row acts on the press");
    assert_eq!(h.find("Finish setup").map(|(_, row)| row), Some(y), "the button stands where the row was");
    h.mouse(MouseKind::Up(MouseButton::Left), x, y);
    assert!(!h.app().finished, "the release of the click that opened the page presses nothing");
}

#[test]
fn a_press_and_release_on_the_same_button_still_press_it() {
    let mut h = Harness::new(Setup { open: true, finished: false }, 40, 6);
    h.click_text("Finish setup");
    assert!(h.app().finished);
}
