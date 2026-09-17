//! Bar chart: memory per service, horizontal and vertical, with a limit that colours a bar.

use qframe::prelude::*;
use qframe::widgets::{Bar, BarChart, Segmented};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "bar-chart";

/// Services with their memory in mebibytes.
const SERVICES: [(&str, f32); 5] = [
    ("api-gateway", 412.0),
    ("postgres-16", 1_536.0),
    ("redis-cache", 268.0),
    ("image-resizer", 1_890.0),
    ("worker", 734.0),
];

/// Memory limit per service; bars above it take the danger tone.
const LIMIT: f32 = 1_800.0;

/// Playground settings.
#[derive(Debug, Default)]
pub struct State {
    vertical: bool,
    limit: bool,
    width: usize,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Vertical(bool),
    Limit(bool),
    Width(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::BarChart(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Vertical(on) => {
            state.vertical = on;
            log.push(PAGE, "Playground", format!("vertical = {on}"));
        }
        Msg::Limit(on) => {
            state.limit = on;
            log.push(PAGE, "Playground", format!("limit = {on}"));
        }
        Msg::Width(index) => {
            state.width = index;
            log.push(PAGE, "Playground", format!("width = {}", WIDTHS[index]));
        }
    }
    Command::none()
}

/// Widths the playground offers, in cells.
const WIDTHS: [u16; 3] = [72, 40, 20];

/// The services as bars, the ones over the limit in the danger tone when `limit` is on.
fn bars(limit: bool) -> Vec<Bar> {
    SERVICES
        .iter()
        .map(|(name, mebibytes)| {
            // region: bars
            let bar = Bar::new(*name, *mebibytes).value_text(format!("{mebibytes:.0} MiB"));
            if limit && *mebibytes > LIMIT { bar.variant("danger") } else { bar }
            // endregion
        })
        .collect()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("bar-chart.memory")).gap(0), |ui| {
        ui.add(Text::new(t!("bar-chart.memory-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: horizontal-bars
        ui.add(BarChart::new(bars(true))).width(Length::Cells(72));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("bar-chart.week")).gap(0), |ui| {
        ui.add(Text::new(t!("bar-chart.week-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let days = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
        let deploys = [4.0, 7.0, 5.0, 9.0, 3.0, 0.0, 1.0];
        // region: vertical-bars
        let week = days.iter().zip(deploys).map(|(day, count)| Bar::new(t!(&format!("bar-chart.{day}")), count));
        ui.add(BarChart::new(week).vertical()).width(Length::Cells(56)).height(Length::Cells(9));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let mut chart = BarChart::new(bars(state.limit));
        if state.vertical {
            chart = chart.vertical();
        }
        let height = if state.vertical { Length::Cells(8) } else { Length::Auto };
        ui.add(chart).width(Length::Cells(WIDTHS[state.width])).height(height).id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("bar-chart.width"), |ui| {
            let labels = WIDTHS.map(|w| w.to_string());
            ui.add(Segmented::new(labels).selected(state.width).on_select(|i| send(Msg::Width(i)))).id("width");
        });
        setting(ui, t!("bar-chart.vertical"), |ui| {
            ui.add(toggle(state.vertical, |on| send(Msg::Vertical(on)))).id("vertical");
        });
        setting(ui, t!("bar-chart.limit"), |ui| {
            ui.add(toggle(state.limit, |on| send(Msg::Limit(on)))).id("limit");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn services_over_the_limit_carry_a_marker() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("● 1890 MiB"), "{}", h.screen());
        h.send(send(Msg::Vertical(true)));
        h.send(send(Msg::Width(2)));
        h.send(send(Msg::Limit(true)));
        assert!(h.app().pages.bar_chart.vertical);
        assert!(h.screen().contains("Thu"), "{}", h.screen());
    }
}
