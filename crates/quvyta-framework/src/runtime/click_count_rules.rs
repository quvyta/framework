//! The runtime's count of presses in a row, read by widgets through `EventCx::clicks`: the same
//! button on the same cell within the interval counts on, anything else starts over.

use std::time::Duration;

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::runtime::{App, Command, Harness};
use crate::widget::{EventCx, MeasureCx, PaintCx, View, Widget};

/// A strip that reports the count it read on every mouse event it gets.
struct Strip;

impl Widget<(MouseKind, u8)> for Strip {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(20, 1).min(available)
    }
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
    }
    fn event(&self, cx: &mut EventCx<'_, (MouseKind, u8)>, event: &Event) -> bool {
        let Event::Mouse(mouse) = event else {
            return false;
        };
        if matches!(mouse.kind, MouseKind::Down(_)) {
            cx.capture_pointer();
        }
        let clicks = cx.clicks();
        cx.emit((mouse.kind, clicks));
        true
    }
}

/// Shows what the strip heard last on the second row.
#[derive(Default)]
struct Counter {
    heard: Vec<(MouseKind, u8)>,
}

impl App for Counter {
    type Msg = (MouseKind, u8);
    fn update(&mut self, heard: (MouseKind, u8)) -> Command<(MouseKind, u8)> {
        self.heard.push(heard);
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, (MouseKind, u8)>) {
        ui.add(Strip);
        let last = self.heard.last().map_or(0, |(_, clicks)| *clicks);
        ui.add(crate::widgets::Text::new(format!("clicks {last}")));
    }
}

fn counter() -> Harness<Counter> {
    Harness::new(Counter::default(), 20, 2)
}

/// The counts the press and the release of the last click read.
fn last_click(h: &Harness<Counter>) -> (u8, u8) {
    let heard = &h.app().heard;
    let press = heard.iter().rev().find(|(kind, _)| matches!(kind, MouseKind::Down(_))).map_or(0, |(_, n)| *n);
    let release = heard.iter().rev().find(|(kind, _)| matches!(kind, MouseKind::Up(_))).map_or(0, |(_, n)| *n);
    (press, release)
}

#[test]
fn clicks_on_one_cell_without_time_between_count_on() {
    let mut h = counter();
    h.click(3, 0);
    assert_eq!(last_click(&h), (1, 1));
    h.click(3, 0);
    assert_eq!(last_click(&h), (2, 2), "a double click: press and release both read 2");
    assert!(h.screen().contains("clicks 2"), "{}", h.screen());
    h.click(3, 0).click(3, 0);
    assert_eq!(last_click(&h), (4, 4), "a fourth press counts on");
}

#[test]
fn time_another_cell_or_another_button_start_over() {
    let mut h = counter();
    h.click(3, 0).advance(Duration::from_millis(399)).click(3, 0);
    assert_eq!(last_click(&h), (2, 2), "399 ms later is still the same series");
    h.advance(Duration::from_millis(400)).click(3, 0);
    assert_eq!(last_click(&h), (1, 1), "400 ms is too slow");
    h.click(4, 0);
    assert_eq!(last_click(&h), (1, 1), "another cell");
    h.mouse(MouseKind::Down(MouseButton::Right), 4, 0).mouse(MouseKind::Up(MouseButton::Right), 4, 0);
    assert_eq!(last_click(&h), (1, 1), "another button on the same cell");
    h.mouse(MouseKind::Down(MouseButton::Right), 4, 0).mouse(MouseKind::Up(MouseButton::Right), 4, 0);
    assert_eq!(last_click(&h), (2, 2), "a double right click");
}

#[test]
fn drags_and_scrolling_count_nothing() {
    let mut h = counter();
    h.click(3, 0).mouse(MouseKind::Down(MouseButton::Left), 3, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 6, 0);
    assert_eq!(h.app().heard.last(), Some(&(MouseKind::Drag(MouseButton::Left), 0)));
    h.mouse(MouseKind::Up(MouseButton::Left), 6, 0);
    assert_eq!(h.app().heard.last(), Some(&(MouseKind::Up(MouseButton::Left), 2)), "the release ends the second press");
    h.mouse(MouseKind::ScrollDown, 3, 0);
    assert_eq!(h.app().heard.last(), Some(&(MouseKind::ScrollDown, 0)));
}
