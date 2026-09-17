//! Gauge: meters for a host's resources with warning and danger limits.

use qframe::prelude::*;
use qframe::widgets::Gauge;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "gauge";

/// How far one press moves the memory gauge, in gibibytes.
const STEP: f32 = 0.4;

/// Memory in use and the playground settings.
#[derive(Debug)]
pub struct State {
    memory: f32,
    label: bool,
    thresholds: bool,
    text: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { memory: 5.2, label: true, thresholds: true, text: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Less,
    More,
    Label(bool),
    Thresholds(bool),
    Text(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Gauge(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Less => {
            state.memory = (state.memory - STEP).max(0.0);
            ("Button#less", format!("memory = {:.1} GiB", state.memory))
        }
        Msg::More => {
            state.memory = (state.memory + STEP).min(8.0);
            ("Button#more", format!("memory = {:.1} GiB", state.memory))
        }
        Msg::Label(on) => {
            state.label = on;
            ("Playground", format!("label = {on}"))
        }
        Msg::Thresholds(on) => {
            state.thresholds = on;
            ("Playground", format!("thresholds = {on}"))
        }
        Msg::Text(on) => {
            state.text = on;
            ("Playground", format!("value text = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("gauge.resources")).gap(0), |ui| {
        ui.add(Text::new(t!("gauge.resources-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: thresholds
        ui.add(Gauge::new(34.0).label(t!("gauge.cpu")).label_width(8).thresholds(75.0, 90.0)).width(Length::Cells(64));
        ui.add(Gauge::new(81.0).label(t!("gauge.disk")).label_width(8).thresholds(75.0, 90.0)).width(Length::Cells(64));
        ui.add(Gauge::new(96.0).label(t!("gauge.inodes")).label_width(8).thresholds(75.0, 90.0))
            .width(Length::Cells(64));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.row(|ui| {
            ui.add(Button::new("−").on_press(send(Msg::Less))).id("less");
            ui.add(Button::new("+").on_press(send(Msg::More))).id("more");
            // region: configured
            let mut gauge = Gauge::new(state.memory).range(0.0, 8.0);
            if state.label {
                gauge = gauge.label(t!("gauge.memory"));
            }
            if state.thresholds {
                gauge = gauge.thresholds(6.0, 7.2);
            }
            if state.text {
                gauge = gauge.value_text(t!("gauge.of", used = format!("{:.1}", state.memory)));
            }
            ui.add(gauge).width(Length::Cells(56)).id("configured");
            // endregion
        })
        .gap(2);
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("gauge.label"), |ui| {
            ui.add(toggle(state.label, |on| send(Msg::Label(on)))).id("label");
        });
        setting(ui, t!("gauge.thresholds"), |ui| {
            ui.add(toggle(state.thresholds, |on| send(Msg::Thresholds(on)))).id("thresholds");
        });
        setting(ui, t!("gauge.text"), |ui| {
            ui.add(toggle(state.text, |on| send(Msg::Text(on)))).id("text");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn memory_climbs_into_the_warning_zone() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("5.2 of 8 GiB"), "{}", h.screen());
        for _ in 0..3 {
            h.click_text("+");
        }
        assert!(h.screen().contains("▲ 6.4 of 8 GiB"), "{}", h.screen());
        h.send(send(Msg::Text(false)));
        assert!(h.screen().contains("▲ 80%"), "{}", h.screen());
    }
}
