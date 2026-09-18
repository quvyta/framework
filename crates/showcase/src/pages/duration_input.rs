//! Duration input: a focus session and its break, a daily target that says why a paste was not
//! read, and a reminder that starts empty.

use std::time::Duration;

use qframe::prelude::*;
use qframe::widgets::{DurationError, DurationInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "duration-input";

/// The focus plan and the playground.
#[derive(Debug)]
pub struct State {
    session: Duration,
    rest: Duration,
    target: Duration,
    rejected: Option<DurationError>,
    reminder: Duration,
    seconds: bool,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            session: Duration::from_secs(50 * 60),
            rest: Duration::from_secs(10 * 60),
            target: Duration::from_secs(4 * 3600 + 30 * 60),
            rejected: None,
            reminder: Duration::ZERO,
            seconds: false,
            disabled: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Session(Duration),
    Rest(Duration),
    Target(Duration),
    Rejected(DurationError),
    Reminder(Duration),
    Seconds(bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::DurationInput(message))
}

fn written(length: Duration) -> String {
    let total = length.as_secs();
    format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Session(length) => {
            state.session = length;
            ("DurationInput#session", format!("changed {}", written(length)))
        }
        Msg::Rest(length) => {
            state.rest = length;
            ("DurationInput#break", format!("changed {}", written(length)))
        }
        // region: target-update
        Msg::Target(length) => {
            state.target = length;
            state.rejected = None;
            ("DurationInput#target", format!("changed {}", written(length)))
        }
        Msg::Rejected(error) => {
            let text = format!("rejected a paste: {error:?}");
            state.rejected = Some(error);
            ("DurationInput#target", text)
        }
        // endregion
        Msg::Reminder(length) => {
            state.reminder = length;
            ("DurationInput#reminder", format!("changed {}", written(length)))
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

/// A line under a field, lined up with it: a danger reason or a faint hint.
fn below(ui: &mut View<'_, AppMsg>, text: Text) {
    ui.row(|ui| {
        ui.spacer().width(Length::Cells(24));
        ui.add(text);
    });
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("duration-input.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("duration-input.session"), |ui| {
            // region: field
            ui.add(DurationInput::new(state.session).on_change(|length| send(Msg::Session(length)))).id("session");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        // region: rest-check
        let too_long = state.rest >= state.session;
        setting(ui, t!("duration-input.break"), |ui| {
            ui.add(DurationInput::new(state.rest).invalid(too_long).on_change(|length| send(Msg::Rest(length))))
                .id("break");
        });
        if too_long {
            below(ui, Text::new(t!("duration-input.too-long")).color("danger"));
        }
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("duration-input.target"), |ui| {
            // region: target
            ui.add(
                DurationInput::new(state.target)
                    .on_change(|length| send(Msg::Target(length)))
                    .on_reject(|error| send(Msg::Rejected(error))),
            )
            .id("target");
            // endregion
        });
        match &state.rejected {
            Some(error) => {
                let reason = error.message(ui.env().i18n());
                below(ui, Text::new(reason).color("danger"));
            }
            None => below(ui, Text::new(t!("duration-input.paste-hint")).role("faint")),
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.add(Text::new(t!("duration-input.playground")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("duration-input.reminder"), |ui| {
            // region: configured
            ui.add(
                DurationInput::new(state.reminder)
                    .seconds(state.seconds)
                    .disabled(state.disabled)
                    .on_change(|length| send(Msg::Reminder(length))),
            )
            .id("reminder");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("time-input.seconds"), |ui| {
            ui.add(toggle(state.seconds, |on| send(Msg::Seconds(on)))).id("seconds");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("duration-input.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;
    use qframe::event::MouseKind;

    fn minutes(n: u64) -> Duration {
        Duration::from_secs(n * 60)
    }

    #[test]
    fn typing_the_break_past_the_session_flags_it() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("  0 h 50 min"), "{}", h.screen());
        let (x, y) = h.find("  0 h 10 min").expect("the break field");
        h.click(x + 7, y).type_text("75");
        assert_eq!(h.app().pages.duration_input.rest, minutes(75), "75 minutes carry into an hour");
        assert!(h.screen().contains("▌ 1 h 15 min"), "{}", h.screen());
        assert!(h.screen().contains("The break is not shorter than the session"), "{}", h.screen());
        h.press("left").press("backspace");
        assert_eq!(h.app().pages.duration_input.rest, minutes(15));
        assert!(!h.screen().contains("not shorter"));
    }

    #[test]
    fn a_paste_it_cannot_read_is_explained_under_the_field_in_the_active_language() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("  4 h 30 min").expect("the target field");
        h.click(x + 2, y).paste("3 days");
        assert_eq!(h.app().pages.duration_input.target, minutes(270));
        assert!(h.screen().contains("“days” is not a unit of time"), "{}", h.screen());
        h.set_locale("tr");
        assert!(h.screen().contains("“days” bir zaman birimi değil"), "{}", h.screen());
        h.paste("1 saat 45 dk");
        assert_eq!(h.app().pages.duration_input.target, minutes(105));
        assert!(!h.screen().contains("“days”"), "a good paste clears the reason");
    }

    #[test]
    fn the_wheel_changes_the_segment_under_the_pointer_without_a_click() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("  0 h 50 min").expect("the session field");
        h.hover(x + 7, y).mouse(MouseKind::ScrollUp, x + 7, y).mouse(MouseKind::ScrollUp, x + 7, y);
        assert_eq!(h.app().pages.duration_input.session, minutes(52), "the minutes under the pointer");
        h.hover(x + 2, y).mouse(MouseKind::ScrollUp, x + 2, y);
        assert_eq!(h.app().pages.duration_input.session, minutes(112), "the hours under the pointer");
    }

    #[test]
    fn the_empty_reminder_takes_seconds_when_they_are_on() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("  0 h 00 min"), "the reminder starts empty: {}", h.screen());
        h.send(send(Msg::Seconds(true)));
        let (x, y) = h.find("  0 h 00 min 00 s").expect("the reminder with seconds");
        h.click(x + 14, y).type_text("30");
        assert_eq!(h.app().pages.duration_input.reminder, Duration::from_secs(30));
    }
}
