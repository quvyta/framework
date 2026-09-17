//! Progress bar: eighth-cell determinate progress, tones and the indeterminate sweep.

use qframe::prelude::*;
use qframe::widgets::ProgressBar;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "progress-bar";

/// How far one press of a step button moves the bar, in percent.
const STEP: u8 = 5;

/// The upload demo's progress and playground settings.
#[derive(Debug)]
pub struct State {
    percent: u8,
    show_percent: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { percent: 45, show_percent: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Less,
    More,
    ShowPercent(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ProgressBar(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Less => state.percent = state.percent.saturating_sub(STEP),
        Msg::More => state.percent = (state.percent + STEP).min(100),
        Msg::ShowPercent(on) => {
            state.show_percent = on;
            log.push(PAGE, "Playground", format!("percent = {on}"));
            return Command::none();
        }
    }
    log.push(PAGE, "Button", format!("progress = {}%", state.percent));
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("progress-bar.determinate")), |ui| {
        ui.add(Text::new(t!("progress-bar.determinate-hint")).role("secondary"));
        ui.row(|ui| {
            ui.add(Button::new("−").on_press(send(Msg::Less))).id("less");
            ui.add(Button::new("+").on_press(send(Msg::More))).id("more");
            // region: determinate
            let value = f32::from(state.percent) / 100.0;
            let mut bar = ProgressBar::new(value).percent(state.show_percent);
            if state.percent == 100 {
                bar = bar.variant("success");
            }
            ui.add(bar).width(Length::Fill(1)).id("upload");
            // endregion
        })
        .gap(2)
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("progress-bar.tones")).gap(0), |ui| {
        for (key, value, variant) in [("disk", 0.82, "warning"), ("tests", 1.0, "success"), ("quota", 0.97, "danger")] {
            ui.row(|ui| {
                ui.add(Text::new(t!(&format!("progress-bar.{key}"))).role("secondary").no_wrap())
                    .width(Length::Cells(18));
                // region: tones
                ui.add(ProgressBar::new(value).variant(variant)).width(Length::Fill(1));
                // endregion
            })
            .fill_width();
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("progress-bar.indeterminate")).gap(0), |ui| {
        ui.add(Text::new(t!("progress-bar.indeterminate-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: indeterminate
        ui.add(ProgressBar::indeterminate()).width(Length::Fill(1));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("progress-bar.show-percent"), |ui| {
            ui.add(toggle(state.show_percent, |on| send(Msg::ShowPercent(on)))).id("percent");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn steps_and_percent_label() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("45%"));
        h.click_text("+");
        let shown = h.screen().matches(" 50%").count();
        assert!(shown >= 2, "the bar and the event log show 50%");
        h.send(send(Msg::ShowPercent(false)));
        assert_eq!(h.screen().matches(" 50%").count(), shown - 1);
    }
}
