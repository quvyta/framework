//! Slider: a value along a range, with steps, suffixes, custom labels and disabled sliders.

use qframe::prelude::*;
use qframe::widgets::Slider;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "slider";

/// Megabytes in a gigabyte.
const MB_PER_GB: f64 = 1024.0;

/// Slider values and the playground.
#[derive(Debug)]
pub struct State {
    canary: f64,
    cpu: f64,
    memory: f64,
    playground: f64,
    stepped: bool,
    suffix: bool,
    formatted: bool,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            canary: 25.0,
            cpu: 2.0,
            memory: 2048.0,
            playground: 40.0,
            stepped: false,
            suffix: false,
            formatted: false,
            disabled: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Canary(f64),
    Cpu(f64),
    Memory(f64),
    Playground(f64),
    Stepped(bool),
    Suffix(bool),
    Formatted(bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Slider(message))
}

// region: format
/// Writes a memory limit in megabytes below a gigabyte and in gigabytes above.
fn memory_label(megabytes: f64) -> String {
    if megabytes < MB_PER_GB { format!("{megabytes} MB") } else { format!("{:.2} GB", megabytes / MB_PER_GB) }
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Canary(value) => {
            state.canary = value;
            log.push(PAGE, "Slider#canary", format!("changed {value}"));
        }
        Msg::Cpu(value) => {
            state.cpu = value;
            log.push(PAGE, "Slider#cpu", format!("changed {value}"));
        }
        Msg::Memory(value) => {
            state.memory = value;
            log.push(PAGE, "Slider#memory", format!("changed {value}"));
        }
        Msg::Playground(value) => {
            state.playground = value;
            log.push(PAGE, "Slider#playground", format!("changed {value}"));
        }
        Msg::Stepped(on) => {
            state.stepped = on;
            log.push(PAGE, "Playground", format!("step 5 = {on}"));
        }
        Msg::Suffix(on) => {
            state.suffix = on;
            log.push(PAGE, "Playground", format!("suffix = {on}"));
        }
        Msg::Formatted(on) => {
            state.formatted = on;
            log.push(PAGE, "Playground", format!("format = {on}"));
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
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("slider.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("slider.canary"), |ui| {
            // region: basic
            ui.add(Slider::new(state.canary).on_change(|value| send(Msg::Canary(value))))
                .width(Length::Cells(48))
                .id("canary");
            // endregion
        });
        setting(ui, t!("slider.cpu"), |ui| {
            // region: range
            ui.add(
                Slider::new(state.cpu)
                    .range(0.5, 8.0)
                    .step(0.5)
                    .suffix(t!("slider.cores"))
                    .on_change(|value| send(Msg::Cpu(value))),
            )
            .width(Length::Cells(48))
            .id("cpu");
            // endregion
        });
        setting(ui, t!("slider.memory"), |ui| {
            ui.add(
                Slider::new(state.memory)
                    .range(256.0, 8192.0)
                    .step(256.0)
                    .format(memory_label)
                    .on_change(|value| send(Msg::Memory(value))),
            )
            .width(Length::Cells(48))
            .id("memory");
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.add(Text::new(t!("slider.playground")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("slider.retention"), |ui| {
            // region: configured
            let mut slider = Slider::new(state.playground).range(0.0, 90.0).disabled(state.disabled);
            if state.stepped {
                slider = slider.step(5.0);
            }
            if state.suffix {
                slider = slider.suffix(t!("slider.days"));
            }
            if state.formatted {
                slider = slider.format(|days| if days == 0.0 { "∞".to_owned() } else { days.to_string() });
            }
            ui.add(slider.on_change(|value| send(Msg::Playground(value)))).width(Length::Cells(48)).id("playground");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("slider.step"), |ui| {
            ui.add(toggle(state.stepped, |on| send(Msg::Stepped(on)))).id("stepped");
        });
        setting(ui, t!("slider.suffix"), |ui| {
            ui.add(toggle(state.suffix, |on| send(Msg::Suffix(on)))).id("suffix");
        });
        setting(ui, t!("slider.format"), |ui| {
            ui.add(toggle(state.formatted, |on| send(Msg::Formatted(on)))).id("formatted");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("slider.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn the_wheel_over_a_slider_changes_it_and_is_logged() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("2.0 cores").expect("cpu slider");
        let before = h.screen().find("Traffic to canary");
        h.mouse(qframe::event::MouseKind::ScrollUp, x + 20, y);
        assert_eq!(h.app().pages.slider.cpu, 2.5);
        h.mouse(qframe::event::MouseKind::ScrollDown, x + 20, y);
        h.mouse(qframe::event::MouseKind::ScrollDown, x + 20, y);
        assert_eq!(h.app().pages.slider.cpu, 1.5);
        assert_eq!(h.screen().find("Traffic to canary"), before, "the page did not scroll");
        assert!(h.screen().contains("Slider#cpu"), "{}", h.screen());
    }

    #[test]
    fn sliders_move_by_keys_and_clicks_and_label_their_values() {
        let mut h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("25"), "{screen}");
        assert!(screen.contains("2.0 cores"), "{screen}");
        assert!(screen.contains("2.00 GB"), "{screen}");
        let (x, y) = h.find("2.0 cores").expect("cpu slider");
        h.click(x, y).press("right");
        assert_eq!(h.app().pages.slider.cpu, 2.5);
        h.press("end");
        assert_eq!(h.app().pages.slider.cpu, 8.0);
        h.send(send(Msg::Formatted(true))).send(send(Msg::Playground(0.0)));
        assert!(h.screen().contains('∞'));
        h.send(send(Msg::Disabled(true)));
        let (x, y) = h.find("∞").expect("playground slider");
        h.click(x + 30, y);
        assert_eq!(h.app().pages.slider.playground, 0.0);
    }
}
