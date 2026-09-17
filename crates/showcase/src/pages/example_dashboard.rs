//! Example: a container host dashboard built only from framework components.

use qframe::prelude::*;
use qframe::widgets::{Badge, Bar, BarChart, BigText, EmptyState, Gauge, Skeleton, Sparkline};

use super::{PageMsg, samples, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "example-dashboard";

/// Samples of CPU history shown.
const HISTORY: u32 = 64;

/// Services of the host and the sample series that drives their CPU use.
const SERVICES: [(&str, u32); 5] =
    [("api-gateway", 3), ("postgres-16", 4), ("redis-cache", 5), ("image-resizer", 6), ("worker", 7)];

/// CPU share above which a service is flagged, in percent.
const HOT: f32 = 70.0;

/// Minutes after midnight when the demo clock starts.
const CLOCK_START: u32 = 14 * 60 + 32;

/// The host's clock and what the dashboard shows.
#[derive(Debug)]
pub struct State {
    tick: u32,
    loading: bool,
    alerts: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { tick: 90, loading: false, alerts: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Refresh,
    Acknowledge,
    Loading(bool),
    Alerts(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ExampleDashboard(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Refresh => {
            state.tick += 1;
            ("Button#refresh", format!("tick = {}", state.tick))
        }
        Msg::Acknowledge => {
            state.alerts = false;
            ("Button#acknowledge", "alerts cleared".to_owned())
        }
        Msg::Loading(on) => {
            state.loading = on;
            ("Playground", format!("loading = {on}"))
        }
        Msg::Alerts(on) => {
            state.alerts = on;
            ("Playground", format!("alerts = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The CPU use of every service at the current tick, in percent.
fn service_load(tick: u32) -> Vec<(&'static str, f32)> {
    SERVICES.iter().map(|(name, series)| (*name, samples::load(*series, tick))).collect()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    header(state, ui);
    ui.row(|ui| {
        cpu_panel(state, ui);
        memory_panel(state, ui);
    })
    .gap(2)
    .fill_width();
    ui.row(|ui| {
        services_panel(state, ui);
        alerts_panel(state, ui);
    })
    .gap(2)
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("example-dashboard.loading"), |ui| {
            ui.add(toggle(state.loading, |on| send(Msg::Loading(on)))).id("loading");
        });
        setting(ui, t!("example-dashboard.alerts-setting"), |ui| {
            ui.add(toggle(state.alerts, |on| send(Msg::Alerts(on)))).id("alerts");
        });
    })
    .fill_width();
}

fn header(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().gap(0), |ui| {
        ui.row(|ui| {
            ui.column(|ui| {
                ui.add(Text::new(t!("example-dashboard.region")).role("faint").no_wrap());
                ui.add(Text::new("edge-01").role("title").no_wrap());
                ui.spacer().height(Length::Cells(1));
                // region: status
                let hot = service_load(state.tick).iter().filter(|(_, load)| *load > HOT).count();
                ui.row(|ui| {
                    if state.alerts {
                        ui.add(Badge::new(t!("example-dashboard.degraded")).variant("warning"));
                    } else {
                        ui.add(Badge::new(t!("example-dashboard.healthy")).variant("success"));
                    }
                    ui.add(Badge::new(t!("example-dashboard.containers")).count(12));
                    if hot > 0 {
                        ui.add(Badge::new(t!("example-dashboard.hot")).variant("danger").count(hot as u32));
                    }
                })
                .gap(2);
                // endregion
            })
            .width(Length::Fill(1));
            ui.column(|ui| {
                ui.add(Text::new(t!("example-dashboard.local-time")).role("faint").no_wrap());
                // region: clock
                let minutes = (CLOCK_START + state.tick) % (24 * 60);
                ui.add(BigText::new(format!("{:02}:{:02}", minutes / 60, minutes % 60)).variant("accent"));
                // endregion
            });
            ui.add(Button::new(t!("example-dashboard.refresh")).shortcut("r").on_press(send(Msg::Refresh)))
                .id("refresh");
        })
        .gap(4)
        .fill_width();
    })
    .fill_width();
}

fn cpu_panel(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("example-dashboard.cpu")).gap(0), |ui| {
        if state.loading {
            ui.add(Skeleton::block()).fill_width();
            ui.spacer().height(Length::Cells(1));
            ui.add(Skeleton::lines(1)).width(Length::Cells(24));
            return;
        }
        // region: cpu
        let history = samples::history(0, state.tick, HISTORY);
        let now = history.last().copied().unwrap_or_default();
        ui.add(Sparkline::new(history).range(0.0, 100.0).highlight_extremes().baseline(80.0))
            .fill_width()
            .height(Length::Cells(3));
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::rich([
            Span::new(format!("{now:.0}%")).bold(),
            Span::new(t!("example-dashboard.cpu-detail")).role("secondary"),
        ]));
        // endregion
    })
    .width(Length::Fill(1))
    .fill_height();
}

fn memory_panel(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("example-dashboard.resources")).gap(0), |ui| {
        if state.loading {
            ui.add(Skeleton::lines(3)).fill_width();
            return;
        }
        // region: gauges
        let memory = 4.0 + samples::load(1, state.tick) / 100.0 * 3.8;
        let text = t!("example-dashboard.gib", used = format!("{memory:.1}"), total = 8);
        ui.add(
            Gauge::new(memory)
                .range(0.0, 8.0)
                .label(t!("example-dashboard.memory"))
                .label_width(8)
                .thresholds(6.0, 7.2)
                .value_text(text),
        )
        .fill_width();
        ui.add(Gauge::new(71.0).label(t!("example-dashboard.disk")).label_width(8).thresholds(80.0, 92.0)).fill_width();
        ui.add(Gauge::new(12.0).label(t!("example-dashboard.swap")).label_width(8).thresholds(50.0, 80.0)).fill_width();
        // endregion
    })
    .width(Length::Fill(1))
    .fill_height();
}

fn services_panel(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("example-dashboard.services")).gap(0), |ui| {
        if state.loading {
            for _ in SERVICES {
                ui.row(|ui| {
                    ui.add(Skeleton::lines(1)).width(Length::Cells(14));
                    ui.add(Skeleton::lines(1)).width(Length::Fill(1));
                })
                .gap(2)
                .fill_width();
                ui.spacer().height(Length::Cells(1));
            }
            return;
        }
        // region: services
        let bars = service_load(state.tick).into_iter().map(|(name, load)| {
            let bar = Bar::new(name, load).value_text(format!("{load:.0}%"));
            if load > HOT { bar.variant("danger") } else { bar }
        });
        ui.add(BarChart::new(bars).max(100.0)).fill_width();
        // endregion
    })
    .width(Length::Fill(1))
    .fill_height();
}

fn alerts_panel(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("example-dashboard.alerts")).gap(0), |ui| {
        if !state.alerts {
            // region: no-alerts
            ui.add(
                EmptyState::new(t!("example-dashboard.no-alerts"))
                    .icon("success")
                    .message(t!("example-dashboard.no-alerts-hint")),
            )
            .fill_width()
            .height(Length::Cells(9));
            // endregion
            return;
        }
        let alerts = [
            ("danger", "example-dashboard.alert-oom", "example-dashboard.alert-oom-detail"),
            ("warning", "example-dashboard.alert-restart", "example-dashboard.alert-restart-detail"),
        ];
        ui.column(|ui| {
            for (tone, title, detail) in alerts {
                ui.row(|ui| {
                    ui.add(Badge::new(t!(&format!("example-dashboard.{tone}"))).variant(tone)).width(Length::Cells(12));
                    ui.column(|ui| {
                        ui.add(Text::new(t!(title)).no_wrap());
                        ui.add(Text::new(t!(detail)).role("faint").no_wrap());
                    });
                })
                .gap(2)
                .fill_width();
            }
        })
        .gap(1)
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("example-dashboard.acknowledge")).on_press(send(Msg::Acknowledge))).id("acknowledge");
    })
    .width(Length::Fill(1))
    .fill_height();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn dashboard_refreshes_loads_and_clears_alerts() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("edge-01"), "{screen}");
        assert!(screen.contains("No alerts"), "{screen}");
        h.click_text("Refresh");
        assert_eq!(h.app().pages.example_dashboard.tick, 91);
        h.send(send(Msg::Alerts(true)));
        assert!(h.screen().contains("Degraded"), "{}", h.screen());
        h.click_text("Acknowledge all");
        assert!(!h.app().pages.example_dashboard.alerts);
        h.send(send(Msg::Loading(true)));
        assert!(h.screen().contains('▀'), "{}", h.screen());
    }
}
