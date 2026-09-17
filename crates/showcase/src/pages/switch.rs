//! Switch: the capsule, the rail and the labeled capsule, with labels and disabled switches.

use qframe::prelude::*;
use qframe::widgets::{Switch, SwitchStyle};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "switch";

/// Settings shown in every style.
const SETTINGS: [&str; 2] = ["animations", "sounds"];

/// The switches' states and the playground.
#[derive(Debug)]
pub struct State {
    on: [bool; 2],
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { on: [true, false], disabled: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Set(usize, bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Switch(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Set(index, on) => {
            state.on[index] = on;
            log.push(PAGE, format!("Switch#{}", SETTINGS[index]), format!("toggled {on}"));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let styles = [("capsule", SwitchStyle::Capsule), ("rail", SwitchStyle::Rail), ("labeled", SwitchStyle::Labeled)];
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("switch.hint")).role("secondary"));
        ui.row(|ui| {
            for (name, style) in styles {
                ui.column(|ui| {
                    ui.spacer().height(Length::Cells(1));
                    ui.add(Text::new(t!(&format!("switch.{name}"))).role("faint"));
                    for (index, setting) in SETTINGS.iter().enumerate() {
                        // region: switch-styles
                        ui.add(
                            Switch::new(state.on[index])
                                .style(style)
                                .label(t!(&format!("switch.{setting}")))
                                .disabled(state.disabled)
                                .on_toggle(move |on| send(Msg::Set(index, on))),
                        )
                        .id(format!("{name}-{setting}"));
                        // endregion
                    }
                })
                .width(Length::Cells(30));
            }
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn every_style_toggles_the_same_setting() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("ON"), "{}", h.screen());
        h.send(send(Msg::Set(1, true)));
        assert_eq!(h.app().pages.switch.on, [true, true]);
        h.send(send(Msg::Disabled(true)));
        h.click_text("Animations");
        assert_eq!(h.app().pages.switch.on, [true, true]);
    }
}
