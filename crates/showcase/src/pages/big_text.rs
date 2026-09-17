//! Big text: a clock, a counter and a title drawn from block elements.

use qframe::prelude::*;
use qframe::widgets::{BigText, Select};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "big-text";

/// Words the playground can show.
const WORDS: [&str; 4] = ["14:32", "99.9%", "DEPLOYED", "QUVYTA"];

/// The deploy counter and the playground settings.
#[derive(Debug, Default)]
pub struct State {
    deploys: u32,
    word: usize,
    accent: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Deploy,
    Word(usize),
    Accent(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::BigText(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Deploy => {
            state.deploys += 1;
            log.push(PAGE, "Button#deploy", format!("deploys = {}", state.deploys));
        }
        Msg::Word(index) => {
            state.word = index;
            log.push(PAGE, "Playground", format!("text = {}", WORDS[index]));
        }
        Msg::Accent(on) => {
            state.accent = on;
            log.push(PAGE, "Playground", format!("accent = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("big-text.figures")).gap(0), |ui| {
        ui.add(Text::new(t!("big-text.figures-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.column(|ui| {
                ui.add(Text::new(t!("big-text.uptime")).role("faint"));
                // region: counter
                ui.add(BigText::new("99.98%").variant("accent"));
                // endregion
            });
            ui.column(|ui| {
                ui.add(Text::new(t!("big-text.deploys")).role("faint"));
                // region: live-counter
                ui.add(BigText::new(state.deploys.to_string()));
                // endregion
            });
        })
        .gap(6);
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("big-text.deploy")).on_press(send(Msg::Deploy))).id("deploy");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let mut big = BigText::new(WORDS[state.word]);
        if state.accent {
            big = big.variant("accent");
        }
        ui.add(big).id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("big-text.text"), |ui| {
            ui.add(Select::new(WORDS).selected(Some(state.word)).on_select(|i| send(Msg::Word(i))))
                .width(Length::Cells(16))
                .id("word");
        });
        setting(ui, t!("big-text.accent"), |ui| {
            ui.add(toggle(state.accent, |on| send(Msg::Accent(on)))).id("accent");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn counter_grows_in_big_digits() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains('▀'), "{}", h.screen());
        h.click_text("Record a deploy");
        h.click_text("Record a deploy");
        assert_eq!(h.app().pages.big_text.deploys, 2);
        h.send(send(Msg::Word(2)));
        assert_eq!(h.app().pages.big_text.word, 2);
    }
}
