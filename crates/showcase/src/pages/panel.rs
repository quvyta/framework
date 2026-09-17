//! Panel: raised surfaces, titles, selection and pressable cards.

use qframe::prelude::*;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "panel";

/// The plans of the card demo.
const PLANS: [&str; 3] = ["starter", "team", "studio"];

/// Which card is chosen.
#[derive(Debug, Default)]
pub struct State {
    chosen: usize,
    disabled: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Choose(usize),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Panel(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Choose(index) => {
            state.chosen = index;
            log.push(PAGE, format!("Panel#{}", PLANS[index]), "pressed");
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("panel.cards")), |ui| {
        ui.add(Text::new(t!("panel.cards-hint")).role("secondary"));
        ui.row(|ui| {
            for (index, plan) in PLANS.iter().enumerate() {
                // region: card
                ui.add_with(
                    Panel::new()
                        .variant("inset")
                        .title(t!(&format!("panel.{plan}.name")))
                        .selected(state.chosen == index)
                        .disabled(state.disabled)
                        .on_press(send(Msg::Choose(index))),
                    |ui| {
                        ui.add(Text::new(t!(&format!("panel.{plan}.price"))).role("title"));
                        ui.add(Text::new(t!(&format!("panel.{plan}.text"))).role("secondary"));
                    },
                )
                .width(Length::Fill(1))
                .id(*plan);
                // endregion
            }
        })
        .gap(2)
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("panel.disabled"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("panel.nested")), |ui| {
        // region: nested
        ui.add_with(Panel::new().variant("inset").gap(0), |ui| {
            ui.add(Text::new(t!("panel.nested-text")).role("secondary"));
        })
        .fill_width();
        // endregion
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn cards_are_chosen_by_click_and_keyboard() {
        let mut h = showcase_on(PAGE);
        h.click_text("STUDIO");
        assert_eq!(h.app().pages.panel.chosen, 2);
        assert!(h.screen().contains("Panel#studio"));
    }

    #[test]
    fn hovering_a_card_raises_it_and_disabled_cards_stay_still() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("TEAM").expect("the team card");
        let cell = (u16::try_from(x + 4).expect("on screen"), u16::try_from(y).expect("on screen"));
        let rest = h.bg(cell.0, cell.1);
        h.hover(x + 4, y);
        let hover = h.bg(cell.0, cell.1);
        assert_ne!(hover, rest, "the card rises under the pointer");
        h.hover(0, 0);
        h.send(send(Msg::Disabled(true)));
        assert!(h.app().pages.panel.disabled);
        h.hover(x + 4, y);
        assert_eq!(h.bg(cell.0, cell.1), rest, "a disabled card does not rise");
        h.click_text("TEAM");
        assert_eq!(h.app().pages.panel.chosen, 0);
    }
}
