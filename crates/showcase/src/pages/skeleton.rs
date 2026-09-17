//! Skeleton: placeholders in the shape of loading content, with the sweep light.

use qframe::prelude::*;
use qframe::widgets::{Badge, Segmented, Skeleton};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "skeleton";

/// Containers shown once the list has loaded: name, image and state.
const CONTAINERS: [(&str, &str); 3] =
    [("api-gateway", "ghcr.io/acme/gateway:2.4"), ("postgres-16", "postgres:16-alpine"), ("redis-cache", "redis:7.2")];

/// Playground settings.
#[derive(Debug)]
pub struct State {
    loaded: bool,
    lines: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { loaded: false, lines: 2 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Loaded(bool),
    Lines(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Skeleton(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Loaded(on) => {
            state.loaded = on;
            log.push(PAGE, "Playground", format!("loaded = {on}"));
        }
        Msg::Lines(index) => {
            state.lines = index;
            log.push(PAGE, "Playground", format!("lines = {}", index + 1));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("skeleton.list")), |ui| {
        ui.add(Text::new(t!("skeleton.list-hint")).role("secondary"));
        ui.column(|ui| {
            for (name, image) in CONTAINERS {
                if state.loaded {
                    ui.row(|ui| {
                        ui.add(Badge::new(t!("badge.running")).variant("success"));
                        ui.column(|ui| {
                            ui.add(Text::new(name).no_wrap());
                            ui.add(Text::new(image).role("faint").no_wrap());
                        });
                    })
                    .gap(2)
                    .fill_width();
                } else {
                    // region: list-row
                    ui.row(|ui| {
                        ui.add(Skeleton::avatar());
                        ui.add(Skeleton::lines(2)).width(Length::Cells(36));
                    })
                    .gap(2)
                    .fill_width();
                    // endregion
                }
            }
        })
        .gap(1)
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("skeleton.card")), |ui| {
        ui.add(Text::new(t!("skeleton.card-hint")).role("secondary"));
        // region: skeleton-card
        ui.add(Skeleton::block()).width(Length::Cells(48)).height(Length::Cells(4));
        ui.add(Skeleton::lines(3)).width(Length::Cells(48));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let count = u16::try_from(state.lines).unwrap_or(0) + 1;
        ui.add(Skeleton::lines(count)).width(Length::Cells(40)).id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("skeleton.lines"), |ui| {
            ui.add(Segmented::new(["1", "2", "3", "4"]).selected(state.lines).on_select(|i| send(Msg::Lines(i))))
                .id("lines");
        });
        setting(ui, t!("skeleton.loaded"), |ui| {
            ui.add(toggle(state.loaded, |on| send(Msg::Loaded(on)))).id("loaded");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn placeholders_give_way_to_content() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("▀▀▀▀"), "{}", h.screen());
        assert!(!h.screen().contains("postgres-16"));
        h.send(send(Msg::Loaded(true)));
        assert!(h.screen().contains("postgres-16"), "{}", h.screen());
        h.send(send(Msg::Lines(3)));
        assert_eq!(h.app().pages.skeleton.lines, 3);
    }
}
