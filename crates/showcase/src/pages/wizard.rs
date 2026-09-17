//! Wizard: adding a deploy target in three steps, with validation that blocks Next.

use qframe::prelude::*;
use qframe::widgets::{Field, Form, FormErrors, Segmented, Switch, TextInput, Wizard};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "wizard";

/// The steps, as locale keys under `wizard.`.
const STEPS: [&str; 3] = ["target", "connection", "review"];

/// Environments a target can belong to.
const ENVIRONMENTS: [&str; 2] = ["staging", "production"];

/// Rows every page gets while the playground keeps the buttons in place.
const PAGE_ROWS: u16 = 9;

/// The target being added and the playground.
#[derive(Debug, Default)]
pub struct State {
    step: usize,
    name: String,
    environment: usize,
    host: String,
    port: String,
    health_check: bool,
    errors: FormErrors,
    added: Option<String>,
    cancellable: bool,
    choosable: bool,
    fixed: bool,
    busy: bool,
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Name(String),
    Environment(usize),
    Host(String),
    Port(String),
    HealthCheck(bool),
    Back,
    Next,
    Finish,
    Cancel,
    Step(usize),
    Cancellable(bool),
    Choosable(bool),
    Fixed(bool),
    Busy(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Wizard(message))
}

// region: wizard-validate
/// The problems of one step; the names are the controls' ids.
fn validate(state: &State, step: usize) -> FormErrors {
    let mut errors = FormErrors::new();
    match step {
        0 => errors.check("target-name", state.name.trim().chars().count() >= 3, t!("wizard.name-invalid")),
        1 => {
            errors.check("target-host", state.host.contains('.'), t!("wizard.host-invalid"));
            let port = state.port.trim().parse::<u16>();
            errors.check("target-port", port.is_ok_and(|port| port > 0), t!("wizard.port-invalid"));
        }
        _ => {}
    }
    errors
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Name(value) => state.name = value,
        Msg::Environment(index) => {
            state.environment = index;
            log.push(PAGE, "Segmented#environment", format!("selected {}", ENVIRONMENTS[index]));
        }
        Msg::Host(value) => state.host = value,
        Msg::Port(value) => state.port = value,
        Msg::HealthCheck(on) => {
            state.health_check = on;
            log.push(PAGE, "Switch#health-check", format!("toggled {on}"));
        }
        // region: wizard-next
        Msg::Next => {
            state.errors = validate(state, state.step);
            if !state.errors.is_empty() {
                log.push(PAGE, "Wizard#next", format!("blocked on {}", state.errors.first().unwrap_or_default()));
                return state.errors.focus_first();
            }
            state.step += 1;
            log.push(PAGE, "Wizard#next", format!("step {}", STEPS[state.step]));
        }
        Msg::Back => {
            state.errors.clear();
            state.step = state.step.saturating_sub(1);
            log.push(PAGE, "Wizard#back", format!("step {}", STEPS[state.step]));
        }
        // endregion
        Msg::Step(index) => {
            state.errors.clear();
            state.step = index;
            log.push(PAGE, "Wizard#step", format!("step {}", STEPS[index]));
        }
        Msg::Finish => {
            log.push(PAGE, "Wizard#finish", format!("added {}", state.name));
            *state = State {
                added: Some(state.name.clone()),
                cancellable: state.cancellable,
                choosable: state.choosable,
                fixed: state.fixed,
                ..State::default()
            };
        }
        Msg::Cancel => {
            log.push(PAGE, "Wizard#cancel", "discarded the target");
            *state = State {
                cancellable: state.cancellable,
                choosable: state.choosable,
                fixed: state.fixed,
                ..State::default()
            };
        }
        Msg::Cancellable(on) => {
            state.cancellable = on;
            log.push(PAGE, "Playground", format!("on_cancel = {on}"));
        }
        Msg::Choosable(on) => {
            state.choosable = on;
            log.push(PAGE, "Playground", format!("on_step = {on}"));
        }
        Msg::Fixed(on) => {
            state.fixed = on;
            log.push(PAGE, "Playground", format!("page_height = {}", if on { "9" } else { "none" }));
        }
        Msg::Busy(on) => {
            state.busy = on;
            log.push(PAGE, "Playground", format!("busy = {on}"));
        }
    }
    if !state.errors.is_empty() {
        state.errors = validate(state, state.step);
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("wizard.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: wizard-show
        let mut wizard = Wizard::new(STEPS.map(|step| t!(&format!("wizard.{step}"))))
            .current(state.step)
            .on_back(send(Msg::Back))
            .on_next(send(Msg::Next))
            .on_finish(send(Msg::Finish))
            .busy(state.busy);
        if state.cancellable {
            wizard = wizard.on_cancel(send(Msg::Cancel));
        }
        if state.choosable {
            wizard = wizard.on_step(|index| send(Msg::Step(index)));
        }
        if state.fixed {
            wizard = wizard.page_height(PAGE_ROWS);
        }
        wizard.show(ui, |ui| match state.step {
            0 => target_page(state, ui),
            1 => connection_page(state, ui),
            _ => review_page(state, ui),
        });
        // endregion
        if let Some(added) = &state.added {
            ui.spacer().height(Length::Cells(1));
            let marker = ui.env().icons().glyph("success").into_owned();
            ui.add(
                Text::rich([
                    Span::new(format!("{marker}  ")).color("success"),
                    Span::new(t!("wizard.added", name = added.clone())).role("secondary"),
                ])
                .no_wrap(),
            );
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("wizard.cancellable"), |ui| {
            ui.add(toggle(state.cancellable, |on| send(Msg::Cancellable(on)))).id("cancellable");
        });
        setting(ui, t!("wizard.choosable"), |ui| {
            ui.add(toggle(state.choosable, |on| send(Msg::Choosable(on)))).id("choosable");
        });
        setting(ui, t!("wizard.fixed"), |ui| {
            ui.add(toggle(state.fixed, |on| send(Msg::Fixed(on)))).id("fixed");
        });
        setting(ui, t!("wizard.busy"), |ui| {
            ui.add(toggle(state.busy, |on| send(Msg::Busy(on)))).id("busy");
        });
        ui.add(Text::new(t!("wizard.keys")).role("faint"));
    })
    .fill_width();
}

// region: wizard-page
fn target_page(state: &State, ui: &mut View<'_, AppMsg>) {
    Form::new().label_width(16).show(ui, |form| {
        let name = Field::new(t!("wizard.name")).required(true).error(state.errors.get("target-name"));
        form.field(name.hint(t!("wizard.name-hint")), |ui| {
            ui.add(
                TextInput::new(&state.name)
                    .placeholder("eu-west-web")
                    .invalid(state.errors.has("target-name"))
                    .on_change(|value| send(Msg::Name(value))),
            )
            .width(Length::Cells(30))
            .id("target-name");
        });
        form.field(Field::new(t!("wizard.environment")), |ui| {
            let options = ENVIRONMENTS.map(|env| t!(&format!("wizard.{env}")));
            ui.add(Segmented::new(options).selected(state.environment).on_select(|i| send(Msg::Environment(i))))
                .id("target-environment");
        });
    });
}
// endregion

fn connection_page(state: &State, ui: &mut View<'_, AppMsg>) {
    Form::new().label_width(16).show(ui, |form| {
        let host = Field::new(t!("wizard.host")).required(true).error(state.errors.get("target-host"));
        form.field(host, |ui| {
            ui.add(
                TextInput::new(&state.host)
                    .placeholder("deploy.example.com")
                    .invalid(state.errors.has("target-host"))
                    .on_change(|value| send(Msg::Host(value))),
            )
            .width(Length::Cells(30))
            .id("target-host");
        });
        let port = Field::new(t!("wizard.port")).required(true).error(state.errors.get("target-port"));
        form.field(port, |ui| {
            ui.add(
                TextInput::new(&state.port)
                    .placeholder("22")
                    .max_length(5)
                    .invalid(state.errors.has("target-port"))
                    .on_change(|value| send(Msg::Port(value))),
            )
            .width(Length::Cells(12))
            .id("target-port");
        });
        form.field(Field::new(t!("wizard.health")), |ui| {
            ui.add(
                Switch::new(state.health_check)
                    .label(t!("wizard.health-label"))
                    .on_toggle(|on| send(Msg::HealthCheck(on))),
            )
            .id("target-health");
        });
    });
}

fn review_page(state: &State, ui: &mut View<'_, AppMsg>) {
    let rows = [
        (t!("wizard.name"), state.name.clone()),
        (t!("wizard.environment"), t!(&format!("wizard.{}", ENVIRONMENTS[state.environment]))),
        (t!("wizard.host"), format!("{}:{}", state.host, state.port)),
        (t!("wizard.health"), t!(if state.health_check { "wizard.yes" } else { "wizard.no" })),
    ];
    ui.add(Text::new(t!("wizard.review-hint")).role("faint"));
    ui.spacer().height(Length::Cells(1));
    for (label, value) in rows {
        ui.row(|ui| {
            ui.add(Text::new(label).role("secondary").no_wrap()).width(Length::Cells(18));
            ui.add(Text::new(value).no_wrap());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn next_is_blocked_until_the_step_is_valid_and_finish_adds_the_target() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Next));
        assert_eq!(h.app().pages.wizard.step, 0);
        assert!(h.is_focused("target-name"));
        assert!(h.screen().contains("at least 3"), "{}", h.screen());
        h.type_text("eu-west-web");
        assert!(!h.screen().contains("at least 3"), "the error clears as the value becomes valid");
        h.send(send(Msg::Next));
        assert_eq!(h.app().pages.wizard.step, 1);
        h.send(send(Msg::Host("deploy.example.com".into()))).send(send(Msg::Port("22".into())));
        h.send(send(Msg::Next));
        assert!(h.screen().contains("deploy.example.com:22"), "{}", h.screen());
        h.send(send(Msg::Finish));
        assert_eq!(h.app().pages.wizard.added.as_deref(), Some("eu-west-web"));
        assert_eq!(h.app().pages.wizard.step, 0);
    }

    #[test]
    fn cancel_and_choosing_steps_are_opt_in() {
        let mut h = showcase_on(PAGE);
        assert!(!h.screen().contains("Cancel  "));
        h.send(send(Msg::Cancellable(true))).send(send(Msg::Name("web".into()))).send(send(Msg::Next));
        h.send(send(Msg::Choosable(true)));
        h.click_text("Target");
        assert_eq!(h.app().pages.wizard.step, 0);
        h.click_text("Cancel  ");
        assert!(h.app().pages.wizard.name.is_empty());
    }
}
