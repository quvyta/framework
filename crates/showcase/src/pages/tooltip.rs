//! Tooltip: a short explanation next to the widget the pointer rests on.

use qframe::prelude::*;
use qframe::text;
use qframe::widgets::{Placement, Segmented, Tooltip};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "tooltip";

/// Toolbar actions, each with a tooltip.
const ACTIONS: [&str; 3] = ["restart", "stop", "logs"];

/// The playground.
#[derive(Debug, Default)]
pub struct State {
    placement: usize,
    on_focus: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Action(usize),
    Placement(usize),
    OnFocus(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Tooltip(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Action(index) => log.push(PAGE, format!("Button#{}", ACTIONS[index]), "pressed"),
        Msg::Placement(index) => {
            state.placement = index;
            log.push(PAGE, "Playground", format!("placement = {}", Placement::ALL[index].name()));
        }
        Msg::OnFocus(on) => {
            state.on_focus = on;
            log.push(PAGE, "Playground", format!("on_focus = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let placement = Placement::ALL[state.placement];
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            for (index, action) in ACTIONS.iter().enumerate() {
                // region: tooltip-toolbar
                ui.add_with(
                    Tooltip::new(t!(&format!("tooltip.{action}-tip"))).placement(placement).on_focus(state.on_focus),
                    |ui| {
                        ui.add(Button::new(t!(&format!("tooltip.{action}"))).on_press(send(Msg::Action(index))))
                            .id(*action);
                    },
                );
                // endregion
            }
        })
        .gap(2);
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Text::new(t!("tooltip.container")).role("secondary").no_wrap()).width(Length::Cells(12));
            // region: tooltip-truncated
            let name = "api-gateway-production-eu-central-7f9c4";
            ui.add_with(Tooltip::new(name).placement(placement), |ui| {
                ui.add(Text::new(text::truncate(name, 20).into_owned()).role("body").no_wrap());
            })
            .width(Length::Cells(20));
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("popover.placement"), |ui| {
            let names = Placement::ALL.map(|placement| t!(&format!("popover.{}", placement.name())));
            ui.add(Segmented::new(names).selected(state.placement).on_select(|index| send(Msg::Placement(index))))
                .id("placement");
        });
        setting(ui, t!("tooltip.on-focus"), |ui| {
            ui.add(toggle(state.on_focus, |on| send(Msg::OnFocus(on)))).id("on-focus");
        });
        ui.add(Text::new(t!("tooltip.hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn resting_on_a_button_explains_it() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("Restart").expect("toolbar on screen");
        h.hover(x + 1, y).advance(Duration::from_secs(1));
        assert!(h.screen().contains("Restart every container of the stack"), "{}", h.screen());
        h.hover(x + 1, y + 6).advance(Duration::from_millis(50));
        assert!(!h.screen().contains("Restart every container of the stack"));
    }

    #[test]
    fn full_name_of_a_truncated_container() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("api-gateway").expect("name on screen");
        h.hover(x, y).advance(Duration::from_secs(1));
        assert!(h.screen().contains("api-gateway-production-eu-central-7f9c4"));
    }
}
