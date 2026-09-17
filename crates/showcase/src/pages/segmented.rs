//! Segmented: a few short choices on one surface.

use qframe::prelude::*;
use qframe::widgets::Segmented;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "segmented";

/// View modes of the first control.
const VIEWS: [&str; 3] = ["list", "grid", "tree"];

/// Time ranges of the second control.
const RANGES: [&str; 4] = ["1h", "24h", "7d", "30d"];

/// Choices and the playground.
#[derive(Debug, Default)]
pub struct State {
    view: usize,
    range: usize,
    disabled: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    View(usize),
    Range(usize),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Segmented(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::View(index) => {
            state.view = index;
            log.push(PAGE, "Segmented#view", format!("selected {}", VIEWS[index]));
        }
        Msg::Range(index) => {
            state.range = index;
            log.push(PAGE, "Segmented#range", format!("selected {}", RANGES[index]));
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
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.row(|ui| {
            ui.add(Text::new(t!("segmented.view")).role("secondary").no_wrap()).width(Length::Cells(12));
            // region: basic
            let views = VIEWS.map(|view| t!(&format!("segmented.{view}")));
            ui.add(
                Segmented::new(views)
                    .selected(state.view)
                    .disabled(state.disabled)
                    .on_select(|index| send(Msg::View(index))),
            )
            .id("view");
            // endregion
        });
        ui.add(Text::new(t!(&format!("segmented.{}-text", VIEWS[state.view]))).role("faint"));
        ui.row(|ui| {
            ui.add(Text::new(t!("segmented.range")).role("secondary").no_wrap()).width(Length::Cells(12));
            ui.add(
                Segmented::new(RANGES)
                    .selected(state.range)
                    .disabled(state.disabled)
                    .on_select(|index| send(Msg::Range(index))),
            )
            .id("range");
        });
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
    fn clicks_and_arrows_choose_a_segment() {
        let mut h = showcase_on(PAGE);
        h.click_text("Grid");
        assert_eq!(h.app().pages.segmented.view, 1);
        h.press("right");
        assert_eq!(h.app().pages.segmented.view, 2);
        h.click_text("7d");
        assert_eq!(h.app().pages.segmented.range, 2);
    }
}
