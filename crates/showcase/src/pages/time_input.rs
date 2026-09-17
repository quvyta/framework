//! Time input: a maintenance window edited segment by segment, validation, seconds and disabled
//! fields.

use qframe::prelude::*;
use qframe::widgets::{TimeInput, TimeOfDay};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "time-input";

/// The maintenance window and the playground.
#[derive(Debug)]
pub struct State {
    starts: TimeOfDay,
    ends: TimeOfDay,
    timeout: TimeOfDay,
    seconds: bool,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            starts: TimeOfDay::new(2, 30, 0),
            ends: TimeOfDay::new(4, 0, 0),
            timeout: TimeOfDay::new(0, 5, 30),
            seconds: true,
            disabled: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Starts(TimeOfDay),
    Ends(TimeOfDay),
    Timeout(TimeOfDay),
    Seconds(bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::TimeInput(message))
}

fn written(time: TimeOfDay) -> String {
    format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Starts(time) => {
            state.starts = time;
            ("TimeInput#starts", format!("changed {}", written(time)))
        }
        Msg::Ends(time) => {
            state.ends = time;
            ("TimeInput#ends", format!("changed {}", written(time)))
        }
        Msg::Timeout(time) => {
            state.timeout = time;
            ("TimeInput#timeout", format!("changed {}", written(time)))
        }
        Msg::Seconds(on) => {
            state.seconds = on;
            ("Playground", format!("seconds = {on}"))
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            ("Playground", format!("disabled = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("time-input.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("time-input.starts"), |ui| {
            // region: time-field
            ui.add(TimeInput::new(state.starts).on_change(|time| send(Msg::Starts(time)))).id("starts");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        // region: window-check
        let backwards = state.ends <= state.starts;
        setting(ui, t!("time-input.ends"), |ui| {
            ui.add(TimeInput::new(state.ends).invalid(backwards).on_change(|time| send(Msg::Ends(time)))).id("ends");
        });
        if backwards {
            ui.row(|ui| {
                ui.spacer().width(Length::Cells(24));
                ui.add(Text::new(t!("time-input.backwards")).color("danger"));
            });
        }
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.add(Text::new(t!("time-input.playground")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("time-input.timeout"), |ui| {
            // region: configured
            ui.add(
                TimeInput::new(state.timeout)
                    .seconds(state.seconds)
                    .disabled(state.disabled)
                    .on_change(|time| send(Msg::Timeout(time))),
            )
            .id("timeout");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("time-input.seconds"), |ui| {
            ui.add(toggle(state.seconds, |on| send(Msg::Seconds(on)))).id("seconds");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("time-input.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;
    use qframe::event::MouseKind;

    #[test]
    fn typing_and_arrows_edit_the_window_and_backwards_windows_are_flagged() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains(" 02 : 30"), "{}", h.screen());
        let (x, y) = h.find(" 02 : 30").expect("start field");
        h.click(x + 1, y).type_text("05");
        assert_eq!(h.app().pages.time_input.starts, TimeOfDay::new(5, 30, 0));
        assert!(h.screen().contains("ends before it starts"), "{}", h.screen());
        h.press("down").press("down");
        assert_eq!(h.app().pages.time_input.starts, TimeOfDay::new(5, 28, 0));
        h.send(send(Msg::Starts(TimeOfDay::new(1, 0, 0))));
        assert!(!h.screen().contains("ends before it starts"));
        h.send(send(Msg::Seconds(false)));
        assert!(h.screen().contains(" 00 : 05\n") || h.screen().contains(" 00 : 05 "), "{}", h.screen());
    }

    #[test]
    fn the_wheel_changes_the_segment_under_the_pointer_without_a_click() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find(" 02 : 30").expect("start field");
        h.hover(x + 6, y).mouse(MouseKind::ScrollUp, x + 6, y);
        assert_eq!(h.app().pages.time_input.starts, TimeOfDay::new(2, 31, 0), "the minutes under the pointer");
        h.hover(x + 4, y).mouse(MouseKind::ScrollDown, x + 4, y).mouse(MouseKind::ScrollDown, x + 4, y);
        assert_eq!(h.app().pages.time_input.starts, TimeOfDay::new(2, 29, 0), "the colon keeps the minutes");
        h.hover(x + 1, y).mouse(MouseKind::ScrollUp, x + 1, y);
        assert_eq!(h.app().pages.time_input.starts, TimeOfDay::new(3, 29, 0), "the hours under the pointer");
    }
}
