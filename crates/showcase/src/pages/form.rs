//! Form and field: labels, hints, required words, errors, a summary and focus on the first
//! problem, around ordinary controls.

use qframe::prelude::*;
use qframe::widgets::{Field, Form, FormErrors, Select, Switch, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "form";

/// Restart policies a container can have.
const POLICIES: [&str; 4] = ["no", "on-failure", "always", "unless-stopped"];

/// Width of the label column when labels sit beside controls.
const LABEL_COLUMN: u16 = 16;

/// The container being created and the playground.
#[derive(Debug)]
pub struct State {
    name: String,
    image: String,
    port: String,
    policy: Option<usize>,
    start: bool,
    errors: FormErrors,
    /// Once the user tried to create, errors follow every edit.
    attempted: bool,
    created: Option<String>,
    beside: bool,
    summary: bool,
    hints: bool,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            name: String::new(),
            image: String::new(),
            port: String::new(),
            policy: None,
            start: true,
            errors: FormErrors::new(),
            attempted: false,
            created: None,
            beside: false,
            summary: false,
            hints: true,
            disabled: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Name(String),
    Image(String),
    Port(String),
    Policy(usize),
    Start(bool),
    Create,
    Reset,
    Beside(bool),
    Summary(bool),
    Hints(bool),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Form(message))
}

// region: form-validate
/// Checks every value in form order; the error names are the controls' ids.
fn validate(state: &State) -> FormErrors {
    let mut errors = FormErrors::new();
    let name = state.name.trim();
    if name.is_empty() {
        errors.set("name", t!("form.name-missing"));
    } else if name.chars().count() < 3
        || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        errors.set("name", t!("form.name-invalid"));
    }
    errors.check("image", state.image.contains(':'), t!("form.image-invalid"));
    let port = state.port.trim().parse::<u32>();
    errors.check("port", port.is_ok_and(|port| (1024..=65535).contains(&port)), t!("form.port-invalid"));
    errors.check("policy", state.policy.is_some(), t!("form.policy-missing"));
    errors
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Name(value) => {
            log.push(PAGE, "TextInput#name", format!("changed {value:?}"));
            state.name = value;
        }
        Msg::Image(value) => {
            log.push(PAGE, "TextInput#image", format!("changed {value:?}"));
            state.image = value;
        }
        Msg::Port(value) => {
            log.push(PAGE, "TextInput#port", format!("changed {value:?}"));
            state.port = value;
        }
        Msg::Policy(index) => {
            log.push(PAGE, "Select#policy", format!("selected {}", POLICIES[index]));
            state.policy = Some(index);
        }
        Msg::Start(on) => {
            log.push(PAGE, "Switch#start", format!("toggled {on}"));
            state.start = on;
        }
        // region: form-submit
        Msg::Create => {
            state.attempted = true;
            state.errors = validate(state);
            if state.errors.is_empty() {
                log.push(PAGE, "Button#create", format!("created {}", state.name));
                state.created = Some(state.name.clone());
                return Command::none();
            }
            log.push(
                PAGE,
                "Button#create",
                format!("{} problems, focus {}", state.errors.len(), state.errors.first().unwrap_or_default()),
            );
            state.created = None;
            return state.errors.focus_first();
        }
        // endregion
        Msg::Reset => {
            log.push(PAGE, "Button#reset", "cleared the form");
            *state = State { beside: state.beside, summary: state.summary, hints: state.hints, ..State::default() };
            return Command::focus("name");
        }
        Msg::Beside(on) => {
            log.push(PAGE, "Playground", format!("label_width = {}", if on { "16" } else { "none" }));
            state.beside = on;
        }
        Msg::Summary(on) => {
            log.push(PAGE, "Playground", format!("summary = {on}"));
            state.summary = on;
        }
        Msg::Hints(on) => {
            log.push(PAGE, "Playground", format!("hints = {on}"));
            state.hints = on;
        }
        Msg::Disabled(on) => {
            log.push(PAGE, "Playground", format!("disabled = {on}"));
            state.disabled = on;
        }
    }
    if state.attempted {
        state.errors = validate(state);
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let hint = |key: &str| if state.hints { Some(t!(key)) } else { None };
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("form.intro")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: form-layout
        let mut form = Form::new();
        if state.beside {
            form = form.label_width(LABEL_COLUMN);
        }
        if state.summary {
            form = form.summary(&state.errors);
        }
        let errors = &state.errors;
        form.show(ui, |form| {
            let name = Field::new(t!("form.name")).required(true).error(errors.get("name")).disabled(state.disabled);
            form.field(with_hint(name, hint("form.name-hint")), |ui| {
                ui.add(
                    TextInput::new(&state.name)
                        .placeholder("web-api")
                        .invalid(errors.has("name"))
                        .disabled(state.disabled)
                        .on_change(|value| send(Msg::Name(value))),
                )
                .width(Length::Cells(36))
                .id("name");
            });
            let image = Field::new(t!("form.image")).required(true).error(errors.get("image")).disabled(state.disabled);
            form.field(with_hint(image, hint("form.image-hint")), |ui| {
                ui.add(
                    TextInput::new(&state.image)
                        .placeholder("docker.io/library/nginx:1.27")
                        .invalid(errors.has("image"))
                        .disabled(state.disabled)
                        .on_change(|value| send(Msg::Image(value))),
                )
                .width(Length::Cells(36))
                .id("image");
            });
            let port = Field::new(t!("form.port")).required(true).error(errors.get("port")).disabled(state.disabled);
            form.field(with_hint(port, hint("form.port-hint")), |ui| {
                ui.add(
                    TextInput::new(&state.port)
                        .placeholder("8080")
                        .max_length(5)
                        .invalid(errors.has("port"))
                        .disabled(state.disabled)
                        .on_change(|value| send(Msg::Port(value))),
                )
                .width(Length::Cells(14))
                .id("port");
            });
            let policy = Field::new(t!("form.policy")).error(errors.get("policy")).disabled(state.disabled);
            form.field(policy, |ui| {
                ui.add(
                    Select::new(POLICIES.map(|policy| t!(&format!("form.policy-{policy}"))))
                        .selected(state.policy)
                        .placeholder(t!("form.policy-placeholder"))
                        .disabled(state.disabled)
                        .on_select(|index| send(Msg::Policy(index))),
                )
                .width(Length::Cells(24))
                .id("policy");
            });
            form.field(Field::new(t!("form.start")).disabled(state.disabled), |ui| {
                ui.add(
                    Switch::new(state.start)
                        .label(t!("form.start-label"))
                        .disabled(state.disabled)
                        .on_toggle(|on| send(Msg::Start(on))),
                )
                .id("start");
            });
        });
        // endregion
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(
                Button::new(t!("form.create")).variant("primary").disabled(state.disabled).on_press(send(Msg::Create)),
            )
            .id("create");
            ui.add(Button::new(t!("form.reset")).disabled(state.disabled).on_press(send(Msg::Reset))).id("reset");
            if let Some(created) = &state.created {
                let marker = ui.env().icons().glyph("success").into_owned();
                ui.add(
                    Text::rich([
                        Span::new(format!("{marker}  ")).color("success"),
                        Span::new(t!("form.created", name = created.clone())).role("secondary"),
                    ])
                    .no_wrap(),
                );
            }
        })
        .gap(2);
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("form.beside"), |ui| {
            ui.add(toggle(state.beside, |on| send(Msg::Beside(on)))).id("beside");
        });
        setting(ui, t!("form.summary"), |ui| {
            ui.add(toggle(state.summary, |on| send(Msg::Summary(on)))).id("summary");
        });
        setting(ui, t!("form.hints"), |ui| {
            ui.add(toggle(state.hints, |on| send(Msg::Hints(on)))).id("hints");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("form.keys")).role("faint"));
    })
    .fill_width();
}

/// Adds the hint when the playground shows hints.
fn with_hint(field: Field<AppMsg>, hint: Option<String>) -> Field<AppMsg> {
    match hint {
        Some(hint) => field.hint(hint),
        None => field,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn create_reports_problems_focuses_the_first_and_follows_edits() {
        let mut h = showcase_on(PAGE);
        h.click_text("web-api");
        h.type_text("Web");
        h.send(send(Msg::Image("nginx:1.27".into())));
        h.send(send(Msg::Create));
        let screen = h.screen();
        assert!(screen.contains("Use lowercase letters"), "{screen}");
        assert!(h.is_focused("name"));
        h.press("backspace").press("backspace").press("backspace").type_text("web");
        assert!(!h.screen().contains("Use lowercase letters"), "errors follow edits after the first try");
        h.send(send(Msg::Port("8080".into()))).send(send(Msg::Policy(2))).send(send(Msg::Create));
        assert_eq!(h.app().pages.form.created.as_deref(), Some("web"));
        assert!(h.app().log.recent(PAGE, 1).iter().any(|entry| entry.message == "created web"));
    }

    #[test]
    fn playground_moves_labels_beside_and_shows_the_summary() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Beside(true))).send(send(Msg::Summary(true))).send(send(Msg::Create));
        let screen = h.screen();
        assert!(screen.contains("4 fields need attention"), "{screen}");
        assert!(h.is_focused("name"));
    }
}
