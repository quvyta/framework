//! Badge: status pills in every tone, counts, and narrow labels.

use qframe::prelude::*;
use qframe::widgets::{Badge, Select};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "badge";

/// Tones the playground offers; the first is neutral, which has no variant.
const TONES: [&str; 6] = ["neutral", "success", "warning", "danger", "info", "accent"];

/// Containers of the demo host with their state and tone.
const CONTAINERS: [(&str, &str, &str); 4] = [
    ("api-gateway", "running", "success"),
    ("postgres-16", "restarting", "warning"),
    ("image-resizer", "exited", "danger"),
    ("nightly-backup", "paused", "neutral"),
];

/// Playground settings.
#[derive(Debug)]
pub struct State {
    tone: usize,
    count: bool,
    narrow: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { tone: 1, count: false, narrow: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Tone(usize),
    Count(bool),
    Narrow(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Badge(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Tone(index) => {
            state.tone = index;
            log.push(PAGE, "Playground", format!("tone = {}", TONES[index]));
        }
        Msg::Count(on) => {
            state.count = on;
            log.push(PAGE, "Playground", format!("count = {on}"));
        }
        Msg::Narrow(on) => {
            state.narrow = on;
            log.push(PAGE, "Playground", format!("narrow = {on}"));
        }
    }
    Command::none()
}

/// A badge in `tone`; neutral badges have no variant.
fn toned(label: String, tone: &str) -> Badge {
    let badge = Badge::new(label);
    if tone == "neutral" { badge } else { badge.variant(tone) }
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("badge.tones")), |ui| {
        ui.add(Text::new(t!("badge.tones-hint")).role("secondary"));
        ui.row(|ui| {
            // region: tones
            ui.add(Badge::new(t!("badge.paused")));
            ui.add(Badge::new(t!("badge.running")).variant("success"));
            ui.add(Badge::new(t!("badge.restarting")).variant("warning"));
            ui.add(Badge::new(t!("badge.exited")).variant("danger"));
            ui.add(Badge::new(t!("badge.pulling")).variant("info"));
            ui.add(Badge::new(t!("badge.pinned")).variant("accent"));
            // endregion
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("badge.containers")).gap(0), |ui| {
        for (name, status, tone) in CONTAINERS {
            ui.row(|ui| {
                ui.add(Text::new(name).no_wrap()).width(Length::Cells(20));
                // region: in-rows
                ui.add(toned(t!(&format!("badge.{status}")), tone));
                // endregion
            })
            .fill_width();
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("badge.counts")), |ui| {
        ui.add(Text::new(t!("badge.counts-hint")).role("secondary"));
        ui.row(|ui| {
            // region: counts
            ui.add(Badge::new(t!("badge.alerts")).variant("danger").count(3));
            ui.add(Badge::new(t!("badge.updates")).variant("info").count(12));
            ui.add(Badge::new(t!("badge.log-lines")).count(1_284));
            // endregion
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.row(|ui| {
            // region: configured
            let label = if state.narrow { t!("badge.long") } else { t!("badge.running") };
            let mut badge = toned(label, TONES[state.tone]);
            if state.count {
                badge = badge.count(7);
            }
            let width = if state.narrow { Length::Cells(16) } else { Length::Auto };
            ui.add(badge).width(width).id("configured");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("badge.tone"), |ui| {
            ui.add(Select::new(TONES).selected(Some(state.tone)).on_select(|i| send(Msg::Tone(i))))
                .width(Length::Cells(16))
                .id("tone");
        });
        setting(ui, t!("badge.count"), |ui| {
            ui.add(toggle(state.count, |on| send(Msg::Count(on)))).id("count");
        });
        setting(ui, t!("badge.narrow"), |ui| {
            ui.add(toggle(state.narrow, |on| send(Msg::Narrow(on)))).id("narrow");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn tones_counts_and_playground() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("● Restarting"), "{screen}");
        assert!(screen.contains("99+"), "{screen}");
        h.send(send(Msg::Count(true)));
        h.send(send(Msg::Narrow(true)));
        assert!(h.screen().contains("● Waiting …  7"), "{}", h.screen());
        h.send(send(Msg::Tone(3)));
        assert_eq!(h.app().pages.badge.tone, 3);
    }
}
