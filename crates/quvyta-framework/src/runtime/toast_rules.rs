//! Toasts and modal layers: a toast never covers an open dialog. It keeps to the rows between
//! its corner and the dialog, and waits, with its time stopped, when there is no room there.

use std::time::Duration;

use crate::runtime::{App, Command, Harness};
use crate::widget::View;
use crate::widgets::{Button, Corner, Modal, Text, Toast};

/// A counter's recovery dialog over a dashboard, as on a 20-row terminal.
#[derive(Default)]
struct Recovery {
    open: bool,
    /// Lines of explanation in the dialog, which set its height.
    lines: usize,
}

#[derive(Clone)]
enum Msg {
    Open(usize),
    Close,
    Saved,
    Top,
}

impl App for Recovery {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open(lines) => {
                self.open = true;
                self.lines = lines;
            }
            Msg::Close => self.open = false,
            Msg::Saved => return Command::toast(Toast::success("Session saved")),
            Msg::Top => return Command::toast_corner(Corner::TopRight),
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("dashboard"));
        if self.open {
            let dialog = Modal::new()
                .title("A counter was left running")
                .action(Button::new("Discard").on_press(Msg::Close))
                .action(Button::new("Resume").variant("primary").on_press(Msg::Close));
            ui.add_with(dialog, |ui| {
                for line in 0..self.lines {
                    ui.add(Text::new(format!("detail line {line}")));
                }
            });
        }
    }
}

/// The rows the dialog's surface covers: from the row above its title to the row below its
/// buttons, with the default padding of one row.
fn dialog_rows(h: &Harness<Recovery>) -> (i32, i32) {
    let title = h.find("A counter was left running").expect("dialog title").1;
    let buttons = h.find("Resume").expect("dialog buttons").1;
    (title - 1, buttons + 1)
}

fn opened(lines: usize) -> Harness<Recovery> {
    let mut h = Harness::new(Recovery::default(), 80, 20);
    h.set_reduced_motion(true).send(Msg::Open(lines));
    h
}

#[test]
fn a_toast_keeps_clear_of_an_open_dialog_at_80_by_20() {
    let mut h = opened(2);
    h.send(Msg::Saved).advance(Duration::from_millis(300));
    let screen = h.screen();
    let (top, bottom) = dialog_rows(&h);
    let buttons = screen.lines().nth(usize::try_from(bottom - 1).unwrap_or(0)).unwrap_or_default();
    assert!(buttons.contains("Discard") && buttons.contains("Resume"), "the button row is whole:\n{screen}");
    let toast = h.find("Session saved").unwrap_or_else(|| panic!("there is room below the dialog:\n{screen}")).1;
    assert!(toast > bottom + 1, "the toast sits below the dialog with a free row between:\n{screen}");
    assert!(top > 0);
}

#[test]
fn toasts_in_a_top_corner_keep_above_the_dialog() {
    let mut h = opened(2);
    h.send(Msg::Top).send(Msg::Saved).advance(Duration::from_millis(300));
    let screen = h.screen();
    let (top, _) = dialog_rows(&h);
    let toast = h.find("Session saved").unwrap_or_else(|| panic!("there is room above the dialog:\n{screen}")).1;
    assert!(toast < top - 1, "the toast sits above the dialog:\n{screen}");
}

#[test]
fn a_toast_without_room_waits_for_the_dialog_with_its_time_intact() {
    let mut h = opened(10);
    h.send(Msg::Saved).advance(Duration::from_millis(300));
    assert!(h.find("Session saved").is_none(), "no room beside the dialog:\n{}", h.screen());
    let buttons = h.find("Resume").expect("buttons").1;
    let screen = h.screen();
    let row = screen.lines().nth(usize::try_from(buttons).unwrap_or(0)).unwrap_or_default();
    assert!(row.contains("Discard"), "nothing covers the button row:\n{screen}");
    h.advance(Duration::from_secs(20));
    h.send(Msg::Close);
    assert!(h.find("Session saved").is_some(), "the waiting toast shows once the dialog closed:\n{}", h.screen());
    h.advance(Duration::from_secs(4));
    assert!(h.find("Session saved").is_some(), "its five seconds start when it is seen");
    h.advance(Duration::from_secs(2));
    assert!(h.find("Session saved").is_none(), "and then it goes:\n{}", h.screen());
}

#[test]
fn a_shown_toast_steps_aside_for_a_dialog_opening_over_it_and_comes_back() {
    let mut h = Harness::new(Recovery::default(), 80, 20);
    h.set_reduced_motion(true).send(Msg::Saved).advance(Duration::from_secs(1));
    assert!(h.find("Session saved").is_some());
    h.send(Msg::Open(10));
    assert!(h.find("Session saved").is_none(), "the dialog is not covered:\n{}", h.screen());
    h.advance(Duration::from_secs(30)).send(Msg::Close);
    assert!(h.find("Session saved").is_some(), "the toast is back after the dialog:\n{}", h.screen());
}
