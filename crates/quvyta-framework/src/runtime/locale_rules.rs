//! Rules for reading the active language outside drawing: what `update` reads with
//! [`i18n::active_code`] is what the view sees, also after the language is switched at runtime.

use std::cell::RefCell;

use super::{App, Command, Harness};
use crate::i18n;
use crate::widget::View;
use crate::widgets::Button;

#[derive(Default)]
struct Reader {
    /// The codes `update` read, one per `Note`.
    noted: Vec<String>,
    /// The code the view saw the last time it was built.
    seen: RefCell<String>,
}

#[derive(Debug, Clone)]
enum Msg {
    Switch(&'static str),
    Note,
}

impl App for Reader {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Switch(code) => Command::set_locale(code),
            Msg::Note => {
                self.noted.push(i18n::active_code());
                Command::none()
            }
        }
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.env().i18n().active().clone_into(&mut self.seen.borrow_mut());
        ui.column(|ui| {
            ui.add(Button::new("Turkish").on_press(Msg::Switch("tr")));
            ui.add(Button::new("Brazilian").on_press(Msg::Switch("pt-BR")));
            ui.add(Button::new("Note").on_press(Msg::Note));
        });
    }
}

#[test]
fn update_reads_the_language_the_view_sees_after_a_switch() {
    let mut h = Harness::new(Reader::default(), 40, 6);
    h.set_locale("en");
    h.click_text("Note");
    assert_eq!(h.app().noted, ["en"]);
    assert_eq!(*h.app().seen.borrow(), "en");

    h.click_text("Turkish");
    h.click_text("Note");
    assert_eq!(h.app().noted, ["en", "tr"], "the switch reaches update");
    assert_eq!(*h.app().seen.borrow(), "tr", "and the view agrees");

    h.click_text("Brazilian");
    h.click_text("Note");
    assert_eq!(h.app().noted, ["en", "tr", "pt-BR"]);
    assert_eq!(*h.app().seen.borrow(), "pt-BR");
}

#[test]
fn outside_the_runtime_the_code_is_english() {
    assert_eq!(i18n::active_code(), "en");
}
