//! Hold to confirm: a control, a chord held anywhere, a floating card and the options.

use std::time::Duration;

use qframe::prelude::*;
use qframe::widgets::{HoldToConfirm, Segmented, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "hold-to-confirm";

/// Hold durations the playground offers, in milliseconds.
const DURATIONS: [u64; 3] = [600, 1200, 2000];

/// Colour choices of the playground: the locale key of the label and the theme colour. `None` is
/// the theme's own `hold.to`; the last choice is the custom text field.
const COLORS: [(&str, Option<&str>); 5] = [
    ("hold-to-confirm.color-warning", None),
    ("hold-to-confirm.color-danger", Some("$danger")),
    ("hold-to-confirm.color-success", Some("$success")),
    ("hold-to-confirm.color-accent", Some("$accent")),
    ("hold-to-confirm.color-custom", None),
];

/// Index of the custom choice in [`COLORS`].
const CUSTOM: usize = COLORS.len() - 1;

/// Volumes of the demo and the playground.
#[derive(Debug)]
pub struct State {
    volumes: u32,
    purged: u32,
    duration: usize,
    disabled: bool,
    key: bool,
    floating: bool,
    configured: u32,
    color: usize,
    custom: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            volumes: 3,
            purged: 0,
            duration: 1,
            disabled: false,
            key: false,
            floating: false,
            configured: 0,
            color: 0,
            custom: "#38BDF8".to_owned(),
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    DeleteVolume,
    Purge,
    Configured,
    Duration(usize),
    Disabled(bool),
    Key(bool),
    Floating(bool),
    Color(usize),
    Custom(String),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::HoldToConfirm(message))
}

impl State {
    /// The colour expression the configured control uses, `None` for the theme's default.
    fn color(&self) -> Option<&str> {
        if self.color == CUSTOM { Some(&self.custom) } else { COLORS[self.color].1 }
    }
}

/// Describes the chosen colour for the event log.
fn color_log(state: &State) -> String {
    match state.color() {
        None => "color = theme default, $warning".to_owned(),
        Some(paint) => format!("color = {paint}"),
    }
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::DeleteVolume => {
            state.volumes = state.volumes.saturating_sub(1);
            log.push(PAGE, "HoldToConfirm#delete", format!("confirmed, {} volumes left", state.volumes));
        }
        Msg::Purge => {
            state.purged += 1;
            log.push(PAGE, "HoldToConfirm#purge", "confirmed with ctrl e");
        }
        Msg::Configured => {
            state.configured += 1;
            log.push(PAGE, "HoldToConfirm#configured", "confirmed");
        }
        Msg::Duration(index) => {
            state.duration = index;
            log.push(PAGE, "Playground", format!("duration = {} ms", DURATIONS[index]));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
        Msg::Key(on) => {
            state.key = on;
            log.push(PAGE, "Playground", format!("key = {on}"));
        }
        Msg::Floating(on) => {
            state.floating = on;
            log.push(PAGE, "Playground", format!("floating = {on}"));
        }
        Msg::Color(index) => {
            state.color = index;
            log.push(PAGE, "Playground", color_log(state));
        }
        Msg::Custom(text) => {
            state.custom = text;
            log.push(PAGE, "Playground", color_log(state));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("hold-to-confirm.hint")).role("secondary"));
        ui.row(|ui| {
            ui.add(Text::new(t!("hold-to-confirm.volumes", n = state.volumes)).role("faint").no_wrap())
                .width(Length::Cells(24));
            // region: control
            ui.add(
                HoldToConfirm::new(t!("hold-to-confirm.delete"))
                    .disabled(state.volumes == 0)
                    .on_confirm(send(Msg::DeleteVolume)),
            )
            .id("delete");
            // endregion
        })
        .gap(2);
        ui.row(|ui| {
            ui.add(Text::new(t!("hold-to-confirm.purged", n = state.purged)).role("faint").no_wrap())
                .width(Length::Cells(24));
            ui.add(Text::new(t!("hold-to-confirm.purge-hint")).role("secondary").no_wrap());
            // region: floating
            ui.add(
                HoldToConfirm::new(t!("hold-to-confirm.purge"))
                    .key("ctrl+e")
                    .floating(true)
                    .on_confirm(send(Msg::Purge)),
            )
            .id("purge");
            // endregion
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("hold-to-confirm.duration"), |ui| {
            let labels = DURATIONS.map(|ms| format!("{:.1} s", ms as f32 / 1000.0));
            ui.add(Segmented::new(labels).selected(state.duration).on_select(|i| send(Msg::Duration(i))))
                .id("duration");
        });
        setting(ui, t!("hold-to-confirm.disabled"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        setting(ui, t!("hold-to-confirm.key"), |ui| {
            ui.add(toggle(state.key, |on| send(Msg::Key(on)))).id("key");
        });
        setting(ui, t!("hold-to-confirm.floating"), |ui| {
            ui.add(toggle(state.floating, |on| send(Msg::Floating(on)))).id("floating");
        });
        setting(ui, t!("hold-to-confirm.color"), |ui| {
            let labels = COLORS.map(|(key, _)| t!(key));
            ui.add(Segmented::new(labels).selected(state.color).on_select(|i| send(Msg::Color(i)))).id("color");
        });
        if state.color == CUSTOM {
            // An expression the theme cannot resolve falls back to the warning tone; the field says so.
            let valid = ui.env().theme().solid(&state.custom).is_ok();
            setting(ui, t!("hold-to-confirm.custom"), |ui| {
                ui.row(|ui| {
                    ui.add(
                        TextInput::new(&state.custom)
                            .placeholder("#RRGGBB")
                            .invalid(!valid)
                            .on_change(|text| send(Msg::Custom(text))),
                    )
                    .id("custom")
                    .width(Length::Cells(28));
                    if !valid {
                        ui.add(Text::new(t!("hold-to-confirm.custom-invalid")).role("faint").no_wrap());
                    }
                })
                .gap(2);
            });
        }
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            // region: configured
            let mut hold = HoldToConfirm::new(t!("hold-to-confirm.configured"))
                .duration(Duration::from_millis(DURATIONS[state.duration]))
                .disabled(state.disabled)
                .floating(state.floating);
            if state.key {
                hold = hold.key("ctrl+g");
            }
            if let Some(color) = state.color() {
                // A theme token such as "$danger", or a fixed "#RRGGBB".
                hold = hold.color(color);
            }
            ui.add(hold.on_confirm(send(Msg::Configured))).id("configured");
            // endregion
            ui.add(Text::new(t!("hold-to-confirm.confirmed", n = state.configured)).role("faint").no_wrap());
        })
        .gap(2);
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::event::{KeyEvent, KeyKind};

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn holding_enter_deletes_a_volume_and_ctrl_e_purges_from_anywhere() {
        let mut h = showcase_on(PAGE);
        h.click_text("Hold to delete volume");
        assert_eq!(h.app().pages.hold_to_confirm.volumes, 3, "a click is not a hold");
        h.key(KeyEvent::press("enter"));
        for _ in 0..45 {
            h.advance(Duration::from_millis(30));
            h.key(KeyEvent { kind: KeyKind::Repeat, ..KeyEvent::press("enter") });
        }
        assert_eq!(h.app().pages.hold_to_confirm.volumes, 2);
        h.key(KeyEvent::press("ctrl+e"));
        for _ in 0..45 {
            h.advance(Duration::from_millis(30));
            h.key(KeyEvent::press("ctrl+e"));
        }
        assert_eq!(h.app().pages.hold_to_confirm.purged, 1);
        assert!(h.screen().contains("HoldToConfirm#purge"));
    }

    #[test]
    fn the_playground_colour_applies_live_and_a_bad_custom_value_falls_back() {
        let mut h = showcase_on(PAGE);
        h.click_text("Danger");
        assert_eq!(h.app().pages.hold_to_confirm.color(), Some("$danger"));
        assert!(h.screen().contains("color = $danger"));
        h.click_text("Custom");
        assert_eq!(h.app().pages.hold_to_confirm.color(), Some("#38BDF8"));
        assert!(h.screen().contains("color = #38BDF8"));
        h.click_text("#38BDF8").press("end").press("backspace");
        assert_eq!(h.app().pages.hold_to_confirm.color(), Some("#38BDF"));
        assert!(h.screen().contains("falls back"), "{}", h.screen());
    }
}
