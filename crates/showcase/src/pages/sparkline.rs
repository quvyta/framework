//! Sparkline: trends of a container host in eighth-cell columns.

use qframe::prelude::*;
use qframe::widgets::{Segmented, Sparkline};

use super::{PageMsg, samples, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "sparkline";

/// Samples kept per series.
const HISTORY: u32 = 90;

/// Series of the host: locale key and sample series.
const SERIES: [(&str, u32); 3] = [("cpu", 0), ("network", 1), ("requests", 2)];

/// Where the demo clock is and the playground settings.
#[derive(Debug)]
pub struct State {
    tick: u32,
    extremes: bool,
    baseline: bool,
    height: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { tick: 120, extremes: false, baseline: false, height: 1 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Sample,
    Extremes(bool),
    Baseline(bool),
    Height(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Sparkline(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Sample => {
            state.tick += 1;
            log.push(PAGE, "Button#sample", format!("cpu = {:.0}%", samples::load(0, state.tick - 1)));
        }
        Msg::Extremes(on) => {
            state.extremes = on;
            log.push(PAGE, "Playground", format!("extremes = {on}"));
        }
        Msg::Baseline(on) => {
            state.baseline = on;
            log.push(PAGE, "Playground", format!("baseline = {on}"));
        }
        Msg::Height(index) => {
            state.height = index;
            log.push(PAGE, "Playground", format!("height = {}", index + 1));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("sparkline.host")).gap(0), |ui| {
        ui.add(Text::new(t!("sparkline.host-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.column(|ui| {
            for (key, series) in SERIES {
                let values = samples::history(series, state.tick, HISTORY);
                let last = values.last().copied().unwrap_or_default();
                ui.row(|ui| {
                    ui.add(Text::new(t!(&format!("sparkline.{key}"))).role("secondary").no_wrap())
                        .width(Length::Cells(14));
                    // region: basic
                    ui.add(Sparkline::new(values).range(0.0, 100.0)).width(Length::Fill(1));
                    // endregion
                    ui.add(Text::new(format!("{last:>3.0}%")).bold().no_wrap()).width(Length::Cells(5));
                })
                .gap(2)
                .fill_width();
            }
        })
        .gap(1)
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("sparkline.sample")).on_press(send(Msg::Sample))).id("sample");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let mut spark = Sparkline::new(samples::history(0, state.tick, HISTORY)).range(0.0, 100.0);
        if state.extremes {
            spark = spark.highlight_extremes();
        }
        if state.baseline {
            spark = spark.baseline(80.0);
        }
        let rows = u16::try_from(state.height).unwrap_or(0) + 1;
        ui.add(spark).width(Length::Cells(60)).height(Length::Cells(rows)).id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("sparkline.height"), |ui| {
            ui.add(Segmented::new(["1", "2", "3"]).selected(state.height).on_select(|i| send(Msg::Height(i))))
                .id("height");
        });
        setting(ui, t!("sparkline.extremes"), |ui| {
            ui.add(toggle(state.extremes, |on| send(Msg::Extremes(on)))).id("extremes");
        });
        setting(ui, t!("sparkline.baseline"), |ui| {
            ui.add(toggle(state.baseline, |on| send(Msg::Baseline(on)))).id("baseline");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn samples_move_the_series_and_playground_configures() {
        let mut h = showcase_on(PAGE);
        let before = h.screen();
        assert!(before.contains('▁') || before.contains('▄'), "{before}");
        h.click_text("Take a sample");
        assert_eq!(h.app().pages.sparkline.tick, 121);
        assert_ne!(h.screen(), before);
        h.send(send(Msg::Height(2)));
        h.send(send(Msg::Extremes(true)));
        assert!(h.app().pages.sparkline.extremes);
    }
}
