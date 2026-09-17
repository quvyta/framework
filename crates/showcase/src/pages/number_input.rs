//! Number input: typing and stepping numbers, ranges, stepper segments and decimal steps.

use qframe::prelude::*;
use qframe::widgets::NumberInput;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "number-input";

/// Width of every demo field.
const FIELD: Length = Length::Cells(24);

/// Field values and playground settings.
#[derive(Debug)]
pub struct State {
    port: f64,
    replicas: f64,
    cpu: f64,
    workers: f64,
    ranged: bool,
    steppers: bool,
    halves: bool,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            port: 8080.0,
            replicas: 3.0,
            cpu: 1.5,
            workers: 4.0,
            ranged: false,
            steppers: false,
            halves: false,
            disabled: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Port(f64),
    Replicas(f64),
    Cpu(f64),
    Workers(f64),
    Ranged(bool),
    Steppers(bool),
    Halves(bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::NumberInput(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Port(value) => {
            state.port = value;
            ("NumberInput#port", format!("changed {value}"))
        }
        Msg::Replicas(value) => {
            state.replicas = value;
            ("NumberInput#replicas", format!("changed {value}"))
        }
        Msg::Cpu(value) => {
            state.cpu = value;
            ("NumberInput#cpu", format!("changed {value}"))
        }
        Msg::Workers(value) => {
            state.workers = value;
            ("NumberInput#workers", format!("changed {value}"))
        }
        Msg::Ranged(on) => {
            state.ranged = on;
            ("Playground", format!("range = {on}"))
        }
        Msg::Steppers(on) => {
            state.steppers = on;
            ("Playground", format!("steppers = {on}"))
        }
        Msg::Halves(on) => {
            state.halves = on;
            ("Playground", format!("step 0.5 = {on}"))
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            ("Playground", format!("disabled = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("number-input.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("number-input.port"), |ui| {
            // region: number-field
            ui.add(NumberInput::new(state.port).on_change(|value| send(Msg::Port(value)))).width(FIELD).id("port");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("number-input.replicas"), |ui| {
            // region: steppers
            ui.add(
                NumberInput::new(state.replicas)
                    .range(1.0, 12.0)
                    .steppers(true)
                    .on_change(|value| send(Msg::Replicas(value))),
            )
            .width(FIELD)
            .id("replicas");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("number-input.cpu"), |ui| {
            // region: decimals
            ui.add(NumberInput::new(state.cpu).range(0.25, 8.0).step(0.25).on_change(|value| send(Msg::Cpu(value))))
                .width(FIELD)
                .id("cpu");
            // endregion
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.add(Text::new(t!("number-input.playground")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("number-input.workers"), |ui| {
            // region: configured
            let mut field = NumberInput::new(state.workers).steppers(state.steppers).disabled(state.disabled);
            if state.ranged {
                field = field.range(1.0, 16.0);
            }
            if state.halves {
                field = field.step(0.5);
            }
            ui.add(field.on_change(|value| send(Msg::Workers(value)))).width(FIELD).id("workers");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("number-input.range"), |ui| {
            ui.add(toggle(state.ranged, |on| send(Msg::Ranged(on)))).id("ranged");
        });
        setting(ui, t!("number-input.steppers"), |ui| {
            ui.add(toggle(state.steppers, |on| send(Msg::Steppers(on)))).id("steppers");
        });
        setting(ui, t!("number-input.halves"), |ui| {
            ui.add(toggle(state.halves, |on| send(Msg::Halves(on)))).id("halves");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("number-input.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn typing_stepping_and_stepper_segments() {
        let mut h = showcase_on(PAGE);
        h.click_text("8080");
        h.press("end").press("backspace").press("backspace").type_text("43");
        assert_eq!(h.app().pages.number_input.port, 8043.0);
        h.press("up");
        assert_eq!(h.app().pages.number_input.port, 8044.0);
        let (x, y) = h.find("+").expect("replica steppers");
        h.click(x, y).click(x, y);
        assert_eq!(h.app().pages.number_input.replicas, 5.0);
        h.click_text("1.50").press("pgup");
        assert_eq!(h.app().pages.number_input.cpu, 4.0);
        h.send(send(Msg::Disabled(true)));
        let (x, y) = h.find("Worker threads").expect("playground field");
        h.click(x + 27, y).press("up");
        assert_eq!(h.app().pages.number_input.workers, 4.0);
    }
}
