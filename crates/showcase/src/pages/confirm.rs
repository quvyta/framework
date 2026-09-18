//! Confirmation: asking "are you sure?" with a command, no dialog state in the application.

use qframe::prelude::*;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "confirm";

/// Containers of the demo.
const CONTAINERS: [&str; 3] = ["web", "worker", "postgres"];

/// Which containers still run, and the playground.
#[derive(Debug)]
pub struct State {
    running: [bool; 3],
    danger: bool,
    message: bool,
    labels: bool,
    cancel_message: bool,
    dismissable: bool,
    alternative: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            running: [true; 3],
            danger: true,
            message: true,
            labels: true,
            cancel_message: true,
            dismissable: true,
            alternative: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    AskStop(usize),
    Stop(usize),
    Kept(usize),
    Restarted(usize),
    StartAll,
    Danger(bool),
    Message(bool),
    Labels(bool),
    CancelMessage(bool),
    Dismissable(bool),
    Alternative(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Confirm(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::AskStop(index) => {
            log.push(PAGE, "Command::confirm", format!("asked to stop {}", CONTAINERS[index]));
            return ask(state, index);
        }
        Msg::Stop(index) => {
            state.running[index] = false;
            log.push(PAGE, "Confirm", format!("confirmed, stopped {}", CONTAINERS[index]));
        }
        Msg::Kept(index) => {
            log.push(
                PAGE,
                "Confirm",
                format!("cancelled with the button, Esc or ×, {} keeps running", CONTAINERS[index]),
            );
        }
        Msg::Restarted(index) => {
            log.push(PAGE, "Confirm", format!("third way, restarted {}", CONTAINERS[index]));
        }
        Msg::StartAll => {
            state.running = [true; 3];
            log.push(PAGE, "Button#start-all", "started all");
        }
        Msg::Danger(on) => set(log, &mut state.danger, "danger", on),
        Msg::Message(on) => set(log, &mut state.message, "message", on),
        Msg::Labels(on) => set(log, &mut state.labels, "confirm_label", on),
        Msg::CancelMessage(on) => set(log, &mut state.cancel_message, "on_cancel", on),
        Msg::Dismissable(on) => set(log, &mut state.dismissable, "dismissable", on),
        Msg::Alternative(on) => set(log, &mut state.alternative, "alternative", on),
    }
    Command::none()
}

fn set(log: &mut EventLog, field: &mut bool, name: &str, on: bool) {
    *field = on;
    log.push(PAGE, "Playground", format!("{name} = {on}"));
}

fn ask(state: &State, index: usize) -> Command<AppMsg> {
    let name = CONTAINERS[index];
    // region: ask
    let mut question = Confirm::new(t!("confirm.title", name = name), send(Msg::Stop(index)));
    if state.message {
        question = question.message(t!("confirm.body"));
    }
    if state.danger {
        question = question.danger();
    }
    if state.labels {
        question = question.confirm_label(t!("confirm.stop")).cancel_label(t!("confirm.keep"));
    }
    if state.cancel_message {
        question = question.on_cancel(send(Msg::Kept(index)));
    }
    // Esc and × cancel together; without them only the buttons answer.
    question = question.dismissable(state.dismissable);
    // A third way between Cancel and the confirm button; Tab visits it second.
    if state.alternative {
        question = question.alternative(t!("confirm.restart"), send(Msg::Restarted(index)));
    }
    Command::confirm(question)
    // endregion
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("confirm.hint")).role("secondary"));
        for (index, name) in CONTAINERS.iter().enumerate() {
            ui.row(|ui| {
                let (status, color) = if state.running[index] {
                    (t!("confirm.running"), "success")
                } else {
                    (t!("confirm.stopped"), "muted")
                };
                ui.add(
                    Text::rich([
                        Span::new("● ").color(color),
                        Span::new(format!("{name:<10}")).bold(),
                        Span::new(format!("{status:<10}")).role("secondary"),
                    ])
                    .no_wrap(),
                );
                if state.running[index] {
                    // region: button
                    ui.add(Button::new(t!("confirm.stop")).variant("danger").on_press(send(Msg::AskStop(index))))
                        .id(format!("stop-{name}"));
                    // endregion
                }
            })
            .gap(2);
        }
        ui.add(Button::new(t!("confirm.start-all")).on_press(send(Msg::StartAll))).id("start-all");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("confirm.danger"), |ui| {
            ui.add(toggle(state.danger, |on| send(Msg::Danger(on)))).id("danger");
        });
        setting(ui, t!("confirm.message"), |ui| {
            ui.add(toggle(state.message, |on| send(Msg::Message(on)))).id("message");
        });
        setting(ui, t!("confirm.labels"), |ui| {
            ui.add(toggle(state.labels, |on| send(Msg::Labels(on)))).id("labels");
        });
        setting(ui, t!("confirm.cancel-message"), |ui| {
            ui.add(toggle(state.cancel_message, |on| send(Msg::CancelMessage(on)))).id("cancel-message");
        });
        setting(ui, t!("confirm.dismissable"), |ui| {
            ui.add(toggle(state.dismissable, |on| send(Msg::Dismissable(on)))).id("dismissable");
        });
        setting(ui, t!("confirm.alternative"), |ui| {
            ui.add(toggle(state.alternative, |on| send(Msg::Alternative(on)))).id("alternative");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn cancel_is_the_default_and_confirming_stops() {
        let mut h = showcase_on(PAGE);
        h.click_text("Stop").advance(Duration::from_millis(200));
        assert!(h.screen().contains("Stop web?"), "{}", h.screen());
        h.press("enter");
        assert!(h.app().pages.confirm.running[0]);
        assert!(h.screen().contains("keeps running"));
        h.click_text("Stop").advance(Duration::from_millis(200));
        h.press("tab").press("enter");
        assert!(!h.app().pages.confirm.running[0]);
        assert!(h.screen().contains("stopped web"));
    }

    #[test]
    fn the_close_mark_cancels_unless_the_question_is_not_dismissable() {
        let mut h = showcase_on(PAGE);
        h.click_text("Stop").advance(Duration::from_millis(200));
        let (x, y) = crate::tests::close_mark_beside(&h, "Stop web?");
        h.click(x.expect("a close mark on the title row"), y);
        assert!(h.app().pages.confirm.running[0]);
        assert!(h.screen().contains("cancelled with the button, Esc or ×"), "{}", h.screen());
        h.send(send(Msg::Dismissable(false)));
        h.click_text("Stop").advance(Duration::from_millis(200));
        assert!(crate::tests::close_mark_beside(&h, "Stop web?").0.is_none(), "{}", h.screen());
        h.press("esc");
        assert!(h.screen().contains("Stop web?"), "Esc does not answer: {}", h.screen());
        h.press("enter");
        assert!(h.app().pages.confirm.running[0], "Cancel answered");
    }

    #[test]
    fn a_third_way_restarts_and_sits_between_the_answers() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Alternative(true)));
        h.click_text("Stop").advance(Duration::from_millis(200));
        let screen = h.screen();
        let row = screen.lines().find(|line| line.contains("Restart")).unwrap_or_else(|| panic!("{screen}"));
        let at = |label: &str| row.find(label).unwrap_or_else(|| panic!("`{label}` in {row}"));
        assert!(at("Keep running") < at("Restart") && at("Restart") < at("Stop"), "{row}");
        h.press("tab").press("enter");
        assert!(h.app().pages.confirm.running[0], "restarting keeps it running");
        assert!(h.screen().contains("third way, restarted web"), "{}", h.screen());
        h.click_text("Stop").advance(Duration::from_millis(200));
        h.press("tab").press("tab").press("enter");
        assert!(!h.app().pages.confirm.running[0], "the confirm button is third in the tab order");
    }
}
