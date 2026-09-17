//! Select: choosing one option from a dropdown layer.

use qframe::prelude::*;
use qframe::widgets::Select;

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "select";

/// Regions of the long list.
const REGIONS: [&str; 24] = [
    "Amsterdam",
    "Ankara",
    "Athens",
    "Berlin",
    "Bogotá",
    "Cairo",
    "Cape Town",
    "Dublin",
    "Helsinki",
    "Istanbul",
    "Jakarta",
    "Lagos",
    "Lisbon",
    "London",
    "Madrid",
    "Mexico City",
    "Montréal",
    "Oslo",
    "Paris",
    "São Paulo",
    "Seoul",
    "Sydney",
    "Tokyo",
    "Vienna",
];

/// Row counts the playground offers.
const VISIBLE: [usize; 3] = [3, 5, 8];

/// The chosen options and playground settings.
#[derive(Debug)]
pub struct State {
    runtime: Option<usize>,
    region: Option<usize>,
    visible: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { runtime: None, region: Some(9), visible: 2 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Runtime(usize),
    Region(usize),
    Visible(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Select(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Runtime(index) => {
            state.runtime = Some(index);
            let name = if index == 0 { t!("select.podman") } else { t!("select.docker") };
            log.push(PAGE, "Select#runtime", format!("selected {name}"));
        }
        Msg::Region(index) => {
            state.region = Some(index);
            log.push(PAGE, "Select#region", format!("selected {}", REGIONS[index]));
        }
        Msg::Visible(index) => {
            state.visible = index;
            log.push(PAGE, "Playground", format!("max_visible = {}", VISIBLE[index]));
        }
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        setting(ui, t!("select.runtime"), |ui| {
            // region: basic
            let options = [t!("select.podman"), t!("select.docker")];
            ui.add(
                Select::new(options)
                    .selected(state.runtime)
                    .placeholder(t!("select.choose"))
                    .on_select(|index| send(Msg::Runtime(index))),
            )
            .width(Length::Cells(24))
            .id("runtime");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("select.region"), |ui| {
            // region: long
            ui.add(
                Select::new(REGIONS)
                    .selected(state.region)
                    .max_visible(VISIBLE[state.visible])
                    .on_select(|index| send(Msg::Region(index))),
            )
            .width(Length::Cells(24))
            .id("region");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("select.disabled"), |ui| {
            ui.add(Select::new([t!("select.podman")]).selected(Some(0)).disabled(true)).width(Length::Cells(24));
        });
        ui.spacer().height(Length::Cells(VISIBLE[state.visible] as u16));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("select.visible"), |ui| {
            ui.add(
                Select::new(VISIBLE.map(|v| v.to_string()))
                    .selected(Some(state.visible))
                    .on_select(|i| send(Msg::Visible(i))),
            )
            .width(Length::Cells(16))
            .id("visible");
        });
        ui.add(Text::new(t!("select.keys")).role("faint"));
        ui.add(Text::new(t!("select.through")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn opens_chooses_and_logs() {
        let mut h = showcase_on(PAGE);
        h.click_text("Choose a runtime").advance(std::time::Duration::from_millis(200));
        h.click_text("Docker");
        assert_eq!(h.app().pages.select.runtime, Some(1));
        assert!(h.screen().contains("Select#runtime"));
    }
}
