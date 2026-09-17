//! Divider: separation by space, captions and tone bands, never by a line.

use qframe::prelude::*;
use qframe::widgets::{Divider, Segmented};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "divider";

/// Services of the demo host, running first.
const RUNNING: [&str; 3] = ["api-gateway", "postgres-16", "redis-cache"];
const STOPPED: [&str; 2] = ["image-resizer", "nightly-backup"];

/// Playground settings.
#[derive(Debug)]
pub struct State {
    space: usize,
    label: bool,
    band: bool,
    vertical: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { space: 0, label: true, band: false, vertical: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Space(usize),
    Label(bool),
    Band(bool),
    Vertical(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Divider(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Space(index) => {
            state.space = index;
            log.push(PAGE, "Playground", format!("space = {}", index + 1));
        }
        Msg::Label(on) => {
            state.label = on;
            log.push(PAGE, "Playground", format!("label = {on}"));
        }
        Msg::Band(on) => {
            state.band = on;
            log.push(PAGE, "Playground", format!("band = {on}"));
        }
        Msg::Vertical(on) => {
            state.vertical = on;
            log.push(PAGE, "Playground", format!("vertical = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("divider.space")).gap(0), |ui| {
        ui.add(Text::new(t!("divider.space-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        for name in RUNNING {
            ui.add(Text::new(name));
        }
        // region: caption
        ui.add(Divider::new().label(t!("divider.stopped")));
        // endregion
        for name in STOPPED {
            ui.add(Text::new(name).role("secondary"));
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("divider.band")).gap(0), |ui| {
        ui.add(Text::new(t!("divider.band-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.column(|ui| {
                ui.add(Text::new(t!("divider.cpu")).role("faint"));
                ui.add(Text::new("37%").role("title"));
            })
            .width(Length::Fill(1));
            // region: band
            ui.add(Divider::new().vertical().band()).fill_height();
            // endregion
            ui.column(|ui| {
                ui.add(Text::new(t!("divider.memory")).role("faint"));
                ui.add(Text::new("6.2 GiB").role("title"));
            })
            .width(Length::Fill(1))
            .padding(Padding { left: 3, ..Padding::default() });
        })
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let mut divider = Divider::new().space(u16::try_from(state.space).unwrap_or(0) + 1);
        if state.label {
            divider = divider.label(t!("divider.stopped"));
        }
        if state.band {
            divider = divider.band();
        }
        if state.vertical {
            ui.row(|ui| {
                ui.add(Text::new(RUNNING[0]).no_wrap());
                ui.add(divider.vertical()).fill_height().id("configured");
                ui.add(Text::new(STOPPED[0]).role("secondary").no_wrap());
            });
        } else {
            ui.add(Text::new(RUNNING[0]));
            ui.add(divider).id("configured");
            ui.add(Text::new(STOPPED[0]).role("secondary"));
        }
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("divider.space-setting"), |ui| {
            ui.add(Segmented::new(["1", "2", "3"]).selected(state.space).on_select(|i| send(Msg::Space(i))))
                .id("space");
        });
        setting(ui, t!("divider.label"), |ui| {
            ui.add(toggle(state.label, |on| send(Msg::Label(on)))).id("label");
        });
        setting(ui, t!("divider.band-setting"), |ui| {
            ui.add(toggle(state.band, |on| send(Msg::Band(on)))).id("band");
        });
        setting(ui, t!("divider.vertical"), |ui| {
            ui.add(toggle(state.vertical, |on| send(Msg::Vertical(on)))).id("vertical");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn captions_and_playground() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("STOPPED"), "{}", h.screen());
        h.send(send(Msg::Vertical(true)));
        h.send(send(Msg::Space(2)));
        assert!(h.screen().contains("api-gateway   image-resizer"), "{}", h.screen());
        assert!(h.app().pages.divider.vertical);
    }
}
