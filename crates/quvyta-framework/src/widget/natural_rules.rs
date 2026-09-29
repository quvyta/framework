//! `natural_size` answers the cells a widget covers when it is drawn: each widget stands in a
//! row with a marker right after it, and the marker's column is the width the layout gave it.

use crate::env::Env;
use crate::geometry::Size;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::{View, natural_size};
use crate::widgets::{Badge, Button, Checkbox, CopyValue, Tabs, Text};

/// One widget of each kind, each followed by `END` on its own row.
struct Measured;

const LABEL: &str = "Grüße 日本";

/// Adds one widget to a row.
type Adds = Box<dyn Fn(&mut View<'_, ()>)>;

fn widgets() -> Vec<Adds> {
    vec![
        Box::new(|ui| {
            ui.add(Button::new(LABEL).icon("check").shortcut("⏎").on_press(()));
        }),
        Box::new(|ui| {
            ui.add(Badge::new(LABEL));
        }),
        Box::new(|ui| {
            ui.add(Tabs::new(["Genel", LABEL, "Ağ"]).on_select(|_| ()));
        }),
        Box::new(|ui| {
            ui.add(Checkbox::new(true).label(LABEL).on_toggle(|_| ()));
        }),
        Box::new(|ui| {
            ui.add(CopyValue::new(LABEL));
        }),
    ]
}

fn widths(env: &Env) -> Vec<u16> {
    let size = |widget: &dyn Fn(&Env) -> Size| widget(env).width;
    vec![
        size(&|env| natural_size(&Button::<()>::new(LABEL).icon("check").shortcut("⏎").on_press(()), env, Size::MAX)),
        size(&|env| natural_size::<()>(&Badge::new(LABEL), env, Size::MAX)),
        size(&|env| natural_size(&Tabs::new(["Genel", LABEL, "Ağ"]).on_select(|_| ()), env, Size::MAX)),
        size(&|env| natural_size(&Checkbox::new(true).label(LABEL).on_toggle(|_| ()), env, Size::MAX)),
        size(&|env| natural_size::<()>(&CopyValue::new(LABEL), env, Size::MAX)),
    ]
}

impl App for Measured {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        for widget in widgets() {
            ui.row(|ui| {
                widget(ui);
                ui.add(Text::new("END"));
            });
        }
    }
}

fn drawn(h: &Harness<Measured>) -> Vec<u16> {
    h.screen().lines().filter_map(|line| line.find("END").map(|byte| crate::text::width(&line[..byte]))).collect()
}

#[test]
fn the_natural_size_of_a_widget_is_the_width_it_is_drawn_at() {
    for mode in [GlyphMode::Unicode, GlyphMode::Ascii] {
        let mut h = Harness::new(Measured, 120, 10);
        h.set_glyph_mode(mode);
        assert_eq!(drawn(&h), widths(h.env()), "{mode:?}:\n{}", h.screen());
    }
}

#[test]
fn a_widget_that_fills_answers_with_what_it_is_given() {
    let env = Env::builtin();
    let given = Size::new(37, 3);
    let bar = natural_size::<()>(&crate::widgets::KeyHints::new().hint("q", "quit"), &env, given);
    assert_eq!(bar, Size::new(37, 1), "a key hint bar takes the row it is given");
}
