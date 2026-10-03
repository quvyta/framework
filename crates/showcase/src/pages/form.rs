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

/// The registry the company pulls its own images from; an image from anywhere else still works,
/// only slower.
const SHARED_REGISTRY: &str = "registry.internal/";

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
    warnings: bool,
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
            warnings: true,
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
    Warnings(bool),
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

// region: form-warning
/// What the image is about to cost, which is not what is wrong with it. It stays out of
/// `FormErrors`, so Create sends the container either way.
fn image_warning(state: &State) -> Option<String> {
    let image = state.image.trim();
    (!image.is_empty() && !image.starts_with(SHARED_REGISTRY)).then(|| t!("form.image-warning"))
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
            *state = State {
                beside: state.beside,
                summary: state.summary,
                hints: state.hints,
                warnings: state.warnings,
                ..State::default()
            };
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
        Msg::Warnings(on) => {
            log.push(PAGE, "Playground", format!("warnings = {on}"));
            state.warnings = on;
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
    let warning = if state.warnings { image_warning(state) } else { None };
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
            // The warning is in the `form-warning` region above; the field it belongs to is here,
            // inside the form's own layout.
            let image = Field::new(t!("form.image"))
                .required(true)
                .error(errors.get("image"))
                .warning(warning)
                .disabled(state.disabled);
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
        setting(ui, t!("form.warnings"), |ui| {
            ui.add(toggle(state.warnings, |on| send(Msg::Warnings(on)))).id("warnings");
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
    use qframe::runtime::Harness;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_on;

    /// Clicks the switch of the playground row labelled `label`; the switch stands after the
    /// label's column of 24 cells.
    fn click_setting(h: &mut Harness<Showcase>, label: &str) {
        let (x, y) = h.find(label).unwrap_or_else(|| panic!("the {label} row:\n{}", h.screen()));
        h.click(x + 25, y);
    }

    /// Clicks the Create button. The demo's own sentence above spells the word out as well, so
    /// the button is the last row carrying it, the one Reset shares.
    fn click_create(h: &mut Harness<Showcase>) {
        let screen = h.screen();
        let row = screen
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains("Create"))
            .map(|(row, _)| row)
            .last()
            .unwrap_or_else(|| panic!("the Create button:\n{screen}"));
        let column = screen.lines().nth(row).unwrap_or_default().find("Create").unwrap_or(0);
        h.click(i32::try_from(column).unwrap_or(0), i32::try_from(row).unwrap_or(0));
    }

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

    #[test]
    fn a_warning_is_signed_and_the_container_is_created_anyway() {
        let mut h = showcase_on(PAGE);
        h.click_text("web-api").type_text("web");
        h.send(send(Msg::Image("nginx:1.27".into()))).send(send(Msg::Port("8080".into()))).send(send(Msg::Policy(2)));
        let (x, y) = h.find("Not from the shared").expect("the image is from outside the shared registry");
        let (x, y) = (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0));
        let row = h.screen().lines().nth(usize::from(y)).unwrap_or_default().to_owned();
        let sign = h.env().icons().glyph("warning");
        assert!(row.contains(&format!("{sign} Not from the shared")), "a sign and a space stand in front of it: {row}");
        let warning = h.env().theme().color("warning").expect("every theme has a warning colour");
        assert_eq!(h.fg(x, y), Some(warning), "the sentence in the warning tone: {row}");
        assert_eq!(h.fg(x - 2, y), Some(warning), "and so does the sign: {row}");
        click_create(&mut h);
        assert_eq!(h.app().pages.form.created.as_deref(), Some("web"), "a warning is not a problem");
    }

    #[test]
    fn an_error_takes_the_place_from_the_warning_and_stops_the_create() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Image("nginx".into())));
        click_create(&mut h);
        let screen = h.screen();
        assert!(!screen.contains("pull may be slow"), "the error has the one place: {screen}");
        assert!(screen.contains("Add a tag after a colon"), "{screen}");
        assert!(h.app().pages.form.created.is_none(), "an error is a problem");
        h.send(send(Msg::Image("nginx:1.27".into())));
        assert!(h.screen().contains("pull may be slow"), "and the warning is back in its place: {}", h.screen());
        click_setting(&mut h, "Warnings");
        assert!(!h.screen().contains("pull may be slow"), "the playground can take the warning away: {}", h.screen());
    }
}
