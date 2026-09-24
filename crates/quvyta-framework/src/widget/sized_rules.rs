//! Rules for a child given a width of its own in cells: it is measured at that width, the width it
//! is drawn at, so text that wraps inside it takes as many lines in the measure as on the screen.

use crate::runtime::{App, Command, Harness};
use crate::widget::{Align, Length, View};
use crate::widgets::{Button, Text};

/// A page centred both ways, holding a column 80 cells wide with a long paragraph and a button
/// under it, as a settings page centred on a wide screen is built.
struct Centred {
    pressed: usize,
}

const PARAGRAPH: &str = "The engines this machine offers are listed below. Choose the one your projects run in; \
    a container keeps everything a project installs inside it and leaves this machine as it was, \
    while running on the machine itself is faster to start and needs nothing installed first. \
    You can change this later for every project, or for one project alone.";

impl App for Centred {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        self.pressed += 1;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.column(|ui| {
            ui.row(|ui| {
                ui.column(|ui| {
                    ui.add(Text::new(PARAGRAPH));
                    ui.row(|ui| {
                        ui.add(Button::new("Save").on_press(()));
                    });
                })
                .width(Length::Cells(80));
            })
            .fill_width()
            .justify(Align::Center);
        })
        .fill()
        .justify(Align::Center);
    }
}

#[test]
fn a_column_of_a_set_width_is_measured_as_tall_as_it_is_drawn() {
    for (width, height) in [(120, 30), (80, 30), (100, 12)] {
        let mut h = Harness::new(Centred { pressed: 0 }, width, height);
        let screen = h.screen();
        assert!(screen.contains("Save"), "{width}x{height}: the button under the paragraph is drawn:\n{screen}");
        assert!(screen.contains("one project alone."), "{width}x{height}: every line of the paragraph:\n{screen}");
        h.click_text("Save");
        assert_eq!(h.app().pressed, 1, "{width}x{height}: and a click presses it");
    }
}
