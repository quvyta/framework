//! Steps: where a release pipeline stands, in a row or a column, with finished steps to go
//! back to.

use qframe::prelude::*;
use qframe::widgets::Steps;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "steps";

/// The stages of the release pipeline.
const STAGES: [&str; 5] = ["build", "test", "package", "deploy", "verify"];

/// Width of the demo when the playground narrows it.
const NARROW: u16 = 26;

/// The pipeline and the playground.
#[derive(Debug, Default)]
pub struct State {
    current: usize,
    vertical: bool,
    running: bool,
    failed: bool,
    choosable: bool,
    narrow: bool,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Back,
    Next,
    Choose(usize),
    Vertical(bool),
    Running(bool),
    Failed(bool),
    Choosable(bool),
    Narrow(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Steps(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let flag = |log: &mut EventLog, name: &str, on: bool| log.push(PAGE, "Playground", format!("{name} = {on}"));
    match message {
        Msg::Back => {
            state.current = state.current.saturating_sub(1);
            log.push(PAGE, "Button#back", format!("current = {}", state.current));
        }
        Msg::Next => {
            state.current = (state.current + 1).min(STAGES.len());
            log.push(PAGE, "Button#next", format!("current = {}", state.current));
        }
        // region: steps-choose
        Msg::Choose(index) => {
            state.current = index;
            log.push(PAGE, "Steps#pipeline", format!("chose {}", STAGES[index]));
        }
        // endregion
        Msg::Vertical(on) => {
            state.vertical = on;
            flag(log, "vertical", on);
        }
        Msg::Running(on) => {
            state.running = on;
            flag(log, "running", on);
        }
        Msg::Failed(on) => {
            state.failed = on;
            flag(log, "failed", on);
        }
        Msg::Choosable(on) => {
            state.choosable = on;
            flag(log, "on_select", on);
        }
        Msg::Narrow(on) => {
            state.narrow = on;
            flag(log, "narrow", on);
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("steps.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: steps-basic
        let labels = STAGES.map(|stage| t!(&format!("steps.{stage}")));
        let mut steps = Steps::new(labels)
            .current(state.current)
            .vertical(state.vertical)
            .running(state.running)
            .failed(state.failed);
        if state.choosable {
            steps = steps.on_select(|index| send(Msg::Choose(index)));
        }
        let node = ui.add(steps).id("pipeline");
        // endregion
        if state.narrow {
            node.width(Length::Cells(NARROW));
        }
        ui.spacer().height(Length::Cells(1));
        let status = if state.current >= STAGES.len() {
            t!("steps.released")
        } else {
            t!("steps.status", n = state.current + 1, total = STAGES.len())
        };
        ui.add(Text::new(status).role("faint"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("steps.back")).disabled(state.current == 0).on_press(send(Msg::Back))).id("back");
            ui.add(
                Button::new(t!("steps.next"))
                    .variant("primary")
                    .disabled(state.current >= STAGES.len())
                    .on_press(send(Msg::Next)),
            )
            .id("next");
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("steps.vertical"), |ui| {
            ui.add(toggle(state.vertical, |on| send(Msg::Vertical(on)))).id("vertical");
        });
        setting(ui, t!("steps.running"), |ui| {
            ui.add(toggle(state.running, |on| send(Msg::Running(on)))).id("running");
        });
        setting(ui, t!("steps.failed"), |ui| {
            ui.add(toggle(state.failed, |on| send(Msg::Failed(on)))).id("failed");
        });
        setting(ui, t!("steps.choosable"), |ui| {
            ui.add(toggle(state.choosable, |on| send(Msg::Choosable(on)))).id("choosable");
        });
        setting(ui, t!("steps.narrow"), |ui| {
            ui.add(toggle(state.narrow, |on| send(Msg::Narrow(on)))).id("narrow");
        });
        ui.add(Text::new(t!("steps.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn advances_goes_back_by_choosing_and_narrows() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Next)).send(send(Msg::Next)).send(send(Msg::Next));
        assert!(h.screen().contains("✓  Build   ✓  Test   ✓  Package   ●  Deploy"), "{}", h.screen());
        h.send(send(Msg::Choosable(true)));
        h.click_text("Test");
        assert_eq!(h.app().pages.steps.current, 1);
        h.send(send(Msg::Narrow(true)));
        assert!(h.screen().contains("✓   ●   ○   ○   ○   Test"), "{}", h.screen());
    }
}
