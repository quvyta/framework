//! Rules for a row that wraps (`ui.row(..).wrap(true)`): children that do not fit move to the
//! next line instead of running off the edge, and everything the user touches follows them.
//!
//! The buttons used here are `Alpha`, `Bravo` and `Delta` (9 cells each: two cells of padding on
//! each side of a five-letter label) and `Charlie` (11 cells). The label of a button starts two
//! cells after the button does.

use crate::runtime::{App, Command, Harness};
use crate::widget::{Align, View};
use crate::widgets::{Button, Text};

/// A spacer among the buttons.
const SPACER: &str = "|";

/// A wrapping row of buttons with a line of text under it, in a column.
struct Buttons {
    labels: Vec<&'static str>,
    wrap: bool,
    justify: Align,
    line_gap: Option<u16>,
    pressed: Vec<String>,
}

impl Buttons {
    fn new(labels: &[&'static str]) -> Self {
        Self { labels: labels.to_vec(), wrap: true, justify: Align::Start, line_gap: None, pressed: Vec::new() }
    }

    fn unwrapped(mut self) -> Self {
        self.wrap = false;
        self
    }

    fn justify(mut self, justify: Align) -> Self {
        self.justify = justify;
        self
    }

    fn line_gap(mut self, rows: u16) -> Self {
        self.line_gap = Some(rows);
        self
    }
}

impl App for Buttons {
    type Msg = String;

    fn update(&mut self, pressed: String) -> Command<String> {
        self.pressed.push(pressed);
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, String>) {
        ui.column(|ui| {
            let row = ui
                .row(|ui| {
                    for label in &self.labels {
                        if *label == SPACER {
                            ui.spacer();
                        } else {
                            ui.add(Button::new(*label).on_press((*label).to_string())).id(label.to_lowercase());
                        }
                    }
                })
                .gap(1)
                .justify(self.justify)
                .fill_width();
            let row = if self.wrap { row.wrap(true) } else { row };
            if let Some(rows) = self.line_gap {
                row.line_gap(rows);
            }
            ui.add(Text::new("below"));
        })
        .fill();
    }
}

const FOUR: [&str; 4] = ["Alpha", "Bravo", "Charlie", "Delta"];

fn at(h: &Harness<Buttons>, text: &str) -> (i32, i32) {
    h.find(text).unwrap_or_else(|| panic!("`{text}` is not on screen:\n{}", h.screen()))
}

#[test]
fn a_row_that_fits_draws_exactly_as_a_row_without_wrap() {
    for width in [41, 60] {
        let wrapped = Harness::new(Buttons::new(&FOUR), width, 5);
        let plain = Harness::new(Buttons::new(&FOUR).unwrapped(), width, 5);
        assert_eq!(wrapped.html("row"), plain.html("row"), "at {width} columns:\n{}", wrapped.screen());
        assert_eq!(at(&wrapped, "Delta").1, 0, "{}", wrapped.screen());
        assert_eq!(at(&wrapped, "below").1, 1, "{}", wrapped.screen());
    }
}

#[test]
fn buttons_that_do_not_fit_move_to_the_next_line_and_push_what_is_below() {
    let plain = Harness::new(Buttons::new(&FOUR).unwrapped(), 25, 5);
    assert!(plain.find("Delta").is_none(), "without wrap the last button runs off the edge:\n{}", plain.screen());

    let h = Harness::new(Buttons::new(&FOUR), 25, 5);
    assert_eq!(at(&h, "Alpha"), (2, 0), "{}", h.screen());
    assert_eq!(at(&h, "Bravo"), (2 + 9 + 1, 0), "one cell of gap between buttons on a line:\n{}", h.screen());
    assert_eq!(at(&h, "Charlie"), (2, 1), "the second line starts at the edge, with no gap before it:\n{}", h.screen());
    assert_eq!(at(&h, "Delta"), (2 + 11 + 1, 1), "the gap holds on the second line too:\n{}", h.screen());
    assert_eq!(at(&h, "below"), (0, 2), "the text under the row moves down, not drawn over:\n{}", h.screen());
}

#[test]
fn justify_places_every_line_on_its_own() {
    let h = Harness::new(Buttons::new(&FOUR).justify(Align::End), 25, 5);
    assert_eq!(at(&h, "Bravo"), (25 - 9 + 2, 0), "the first line ends at the right edge:\n{}", h.screen());
    assert_eq!(at(&h, "Alpha"), (25 - 9 - 1 - 9 + 2, 0), "{}", h.screen());
    assert_eq!(at(&h, "Delta"), (25 - 9 + 2, 1), "the second line ends at the right edge too:\n{}", h.screen());
    assert_eq!(at(&h, "Charlie"), (25 - 9 - 1 - 11 + 2, 1), "{}", h.screen());
}

#[test]
fn a_spacer_takes_the_room_left_on_its_own_line() {
    let h = Harness::new(Buttons::new(&["Alpha", SPACER, "Bravo", "Charlie", SPACER, "Delta"]), 25, 5);
    assert_eq!(at(&h, "Alpha"), (2, 0), "{}", h.screen());
    assert_eq!(at(&h, "Bravo"), (25 - 9 + 2, 0), "the first spacer pushes Bravo to the right edge:\n{}", h.screen());
    assert_eq!(at(&h, "Charlie"), (2, 1), "{}", h.screen());
    assert_eq!(at(&h, "Delta"), (25 - 9 + 2, 1), "the second spacer works on the second line:\n{}", h.screen());
}

#[test]
fn a_spacer_at_a_line_break_does_not_push_the_next_line() {
    // Alpha, gap and Bravo fill all 19 cells: the gap before the spacer no longer fits.
    let h = Harness::new(Buttons::new(&["Alpha", "Bravo", SPACER, "Charlie"]), 19, 5);
    assert_eq!(at(&h, "Bravo"), (12, 0), "{}", h.screen());
    assert_eq!(at(&h, "Charlie"), (2, 1), "the next line starts at the edge:\n{}", h.screen());
    assert_eq!(at(&h, "below"), (0, 2), "{}", h.screen());
}

#[test]
fn a_click_on_a_button_that_moved_down_presses_it() {
    let mut h = Harness::new(Buttons::new(&FOUR), 25, 5);
    h.click_text("Delta");
    h.click_text("Charlie");
    h.click_text("Alpha");
    assert_eq!(h.app().pressed, ["Delta", "Charlie", "Alpha"]);
}

#[test]
fn tab_goes_through_the_buttons_in_order_across_lines() {
    let mut h = Harness::new(Buttons::new(&FOUR), 25, 5);
    for (name, label) in [("alpha", "Alpha"), ("bravo", "Bravo"), ("charlie", "Charlie"), ("delta", "Delta")] {
        let (x, y) = at(&h, label);
        let (x, y) = (u16::try_from(x).unwrap(), u16::try_from(y).unwrap());
        let resting = h.bg(x, y);
        h.press("tab");
        assert!(h.is_focused(name), "{name} should have focus:\n{}", h.screen());
        assert_ne!(h.bg(x, y), resting, "focus shows on {label} where it is drawn:\n{}", h.screen());
    }
    h.press("enter");
    assert_eq!(h.app().pressed, ["Delta"]);
}

#[test]
fn a_child_wider_than_the_row_gets_a_line_of_its_own() {
    let h = Harness::new(Buttons::new(&["Alpha", "An extraordinarily long label", "Bravo"]), 20, 5);
    assert_eq!(at(&h, "Alpha"), (2, 0), "{}", h.screen());
    assert_eq!(at(&h, "An extra").1, 1, "{}", h.screen());
    assert_eq!(at(&h, "Bravo"), (2, 2), "{}", h.screen());
    assert_eq!(at(&h, "below"), (0, 3), "{}", h.screen());
    assert!(h.find("label").is_none(), "the long button is cut at the row's width:\n{}", h.screen());
}

#[test]
fn line_gap_leaves_empty_rows_between_lines() {
    let h = Harness::new(Buttons::new(&FOUR).line_gap(1), 25, 6);
    assert_eq!(at(&h, "Alpha"), (2, 0), "{}", h.screen());
    assert_eq!(h.screen().lines().nth(1).map(str::trim), Some(""), "{}", h.screen());
    assert_eq!(at(&h, "Charlie"), (2, 2), "{}", h.screen());
    assert_eq!(at(&h, "below"), (0, 3), "{}", h.screen());

    let wide = Harness::new(Buttons::new(&FOUR).line_gap(1), 60, 6);
    assert_eq!(at(&wide, "below"), (0, 1), "one line has no gap after it:\n{}", wide.screen());
}

#[test]
fn a_row_narrowed_after_a_resize_wraps_and_unwraps() {
    let mut h = Harness::new(Buttons::new(&FOUR), 60, 5);
    assert_eq!(at(&h, "Delta").1, 0);
    h.resize(25, 5);
    assert_eq!(at(&h, "Delta").1, 1, "{}", h.screen());
    h.resize(60, 5);
    assert_eq!(at(&h, "Delta").1, 0, "{}", h.screen());
    assert_eq!(at(&h, "below"), (0, 1), "{}", h.screen());
}
