//! Following the end of growing content: every test drives the view from where the person
//! acts (wheel, keys, the scrollbar, a click) and grows the content with the application's own
//! message, which stands for output arriving.

use std::time::Duration;

use super::*;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, Text};

/// A conversation that grows by the number of rows each message carries.
struct Chat {
    rows: usize,
    follow: bool,
    /// A button above the rows and one below them, like a reply field at the end.
    buttons: bool,
}

impl App for Chat {
    type Msg = usize;
    fn update(&mut self, grow: usize) -> Command<usize> {
        self.rows += grow;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, usize>) {
        ui.add_with(ScrollView::new().follow_end(self.follow), |ui| {
            if self.buttons {
                ui.add(Button::new("Top").on_press(0)).id("top");
            }
            for i in 0..self.rows {
                ui.add(Text::new(format!("row {i}")));
            }
            if self.buttons {
                ui.add(Button::new("Send").on_press(0)).id("send");
            }
        })
        .fill();
    }
}

fn chat(follow: bool) -> Harness<Chat> {
    Harness::new(Chat { rows: 20, follow, buttons: false }, 40, 5)
}

/// The numbers of the rows on screen, top to bottom.
fn rows(h: &Harness<Chat>) -> Vec<usize> {
    h.screen()
        .lines()
        .filter_map(|line| line.trim().strip_prefix("row ")?.split_whitespace().next()?.parse().ok())
        .collect()
}

fn first_row(h: &Harness<Chat>) -> Option<usize> {
    rows(h).first().copied()
}

fn last_row(h: &Harness<Chat>) -> Option<usize> {
    rows(h).last().copied()
}

/// Lets any glide finish.
const SETTLE: Duration = Duration::from_secs(2);

#[test]
fn the_first_frame_opens_at_the_bottom() {
    let h = chat(true);
    assert_eq!((first_row(&h), last_row(&h)), (Some(15), Some(19)), "{}", h.screen());
}

#[test]
fn growing_content_keeps_the_last_row_in_view() {
    let mut h = chat(true);
    h.send(1).advance(SETTLE);
    assert_eq!(last_row(&h), Some(20), "{}", h.screen());
    h.send(5).advance(SETTLE);
    assert_eq!(last_row(&h), Some(25), "{}", h.screen());
}

#[test]
fn growth_glides_to_the_new_end() {
    let mut h = chat(true);
    h.send(10);
    assert_eq!(last_row(&h), Some(19), "the glide starts where the view was:\n{}", h.screen());
    h.advance(Duration::from_millis(40));
    let between = last_row(&h).expect("rows on screen");
    assert!(between > 19 && between < 29, "on its way: {}", h.screen());
    h.advance(SETTLE);
    assert_eq!(last_row(&h), Some(29), "{}", h.screen());
}

#[test]
fn reduced_motion_jumps_to_the_end_without_a_frame_between() {
    let mut h = chat(true);
    h.set_reduced_motion(true);
    h.send(10);
    assert_eq!(last_row(&h), Some(29), "{}", h.screen());
}

#[test]
fn scrolling_up_with_the_wheel_stops_following() {
    let mut h = chat(true);
    h.mouse(MouseKind::ScrollUp, 2, 2);
    let held = first_row(&h);
    assert_eq!(held, Some(12));
    h.send(3).advance(SETTLE);
    assert_eq!(first_row(&h), held, "growth leaves the view where the person put it:\n{}", h.screen());
    assert!(h.screen().contains("6 lines below"), "{}", h.screen());
}

#[test]
fn up_stops_following_and_end_resumes_it() {
    let mut h = chat(true);
    h.press("tab").press("up");
    h.send(2).advance(SETTLE);
    assert_eq!(last_row(&h), Some(18), "{}", h.screen());
    h.press("end");
    assert_eq!(last_row(&h), Some(21), "{}", h.screen());
    h.send(1).advance(SETTLE);
    assert_eq!(last_row(&h), Some(22), "{}", h.screen());
    assert!(!h.screen().contains("below"), "no note while following:\n{}", h.screen());
}

#[test]
fn wheeling_back_to_the_bottom_resumes_following() {
    let mut h = chat(true);
    h.mouse(MouseKind::ScrollUp, 2, 2).mouse(MouseKind::ScrollDown, 2, 2);
    h.send(2).advance(SETTLE);
    assert_eq!(last_row(&h), Some(21), "{}", h.screen());
}

#[test]
fn the_scrollbar_stops_and_resumes_following() {
    let mut h = chat(true);
    h.drag((39, 4), (39, 0));
    assert_eq!(first_row(&h), Some(0), "{}", h.screen());
    h.send(1).advance(SETTLE);
    assert_eq!(first_row(&h), Some(0), "{}", h.screen());
    h.drag((39, 0), (39, 4));
    h.send(1).advance(SETTLE);
    assert_eq!(last_row(&h), Some(21), "{}", h.screen());
}

#[test]
fn a_click_on_the_note_resumes_following() {
    let mut h = chat(true);
    h.mouse(MouseKind::ScrollUp, 2, 2);
    h.send(3).advance(SETTLE);
    h.click_text("lines below");
    assert_eq!(last_row(&h), Some(22), "{}", h.screen());
    h.send(1).advance(SETTLE);
    assert_eq!(last_row(&h), Some(23), "{}", h.screen());
}

#[test]
fn without_the_option_nothing_follows() {
    let mut h = chat(false);
    assert_eq!(first_row(&h), Some(0), "{}", h.screen());
    h.send(5).advance(SETTLE);
    assert_eq!(first_row(&h), Some(0), "{}", h.screen());
    h.press("tab").press("end");
    assert_eq!(last_row(&h), Some(24));
    h.send(3).advance(SETTLE);
    assert_eq!(last_row(&h), Some(24), "{}", h.screen());
    h.mouse(MouseKind::ScrollUp, 2, 2);
    assert!(!h.screen().contains("below"), "{}", h.screen());
}

#[test]
fn focus_higher_up_holds_the_view_and_focus_at_the_end_keeps_following() {
    let mut h = Harness::new(Chat { rows: 20, follow: true, buttons: true }, 40, 5);
    assert!(h.screen().contains("Send"), "{}", h.screen());
    h.press("tab").press("tab");
    assert!(h.is_focused("top"));
    h.send(3).advance(SETTLE);
    assert!(h.screen().contains("Top"), "focus higher up holds the view:\n{}", h.screen());
    h.press("tab");
    assert!(h.is_focused("send"));
    h.send(2).advance(SETTLE);
    let screen = h.screen();
    assert!(screen.contains("Send") && last_row(&h) == Some(24), "the focused end keeps following:\n{screen}");
}
