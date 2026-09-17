//! Example application: setting up a new Quvyta project, built only from framework components.

use std::time::Duration;

use qframe::prelude::*;
use qframe::widgets::{
    Checkbox, CodeView, Field, Form, FormErrors, Language, ProgressBar, RadioGroup, Select, SettingRow, SettingsList,
    Steps, Switch, TextInput, Wizard,
};

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "example-setup-wizard";

/// The wizard's steps, as locale keys under `setup.`.
const STEPS: [&str; 5] = ["step-project", "step-engine", "step-look", "step-features", "step-summary"];

/// Container engines, as locale keys and config values.
const ENGINES: [&str; 3] = ["podman", "docker", "none"];

/// Built-in themes.
const THEMES: [&str; 4] = ["Monochrome", "Iris", "Nordic", "Amber"];

/// Icon modes a project can start with.
const ICONS: [&str; 3] = ["auto", "nerd", "ascii"];

/// Optional framework features.
const FEATURES: [&str; 4] = ["router", "tasks", "clipboard", "keymap"];

/// The work Create does, one stage at a time.
const STAGES: [&str; 4] = ["stage-files", "stage-theme", "stage-engine", "stage-git"];

/// Rows every wizard page gets, so the buttons never move.
const PAGE_ROWS: u16 = 15;

/// How long each creation stage takes in this example.
const STAGE_TIME: Duration = Duration::from_millis(350);

// region: setup-state
/// Everything the user answers, and where the flow is.
#[derive(Debug)]
pub struct State {
    step: usize,
    name: String,
    location: String,
    engine: Option<usize>,
    theme: usize,
    icons: usize,
    animations: bool,
    features: [bool; 4],
    errors: FormErrors,
    /// While creating: how many stages are done.
    creating: Option<usize>,
    created: bool,
}
// endregion

impl Default for State {
    fn default() -> Self {
        Self {
            step: 0,
            name: String::new(),
            location: "~/projects".to_owned(),
            engine: None,
            theme: 0,
            icons: 0,
            animations: true,
            features: [true, false, false, true],
            errors: FormErrors::new(),
            creating: None,
            created: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Name(String),
    Location(String),
    Engine(usize),
    Theme(usize),
    Icons(usize),
    Animations(bool),
    Feature(usize, bool),
    Back,
    Next,
    Step(usize),
    Create,
    StageDone(usize),
    Cancel,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ExampleSetupWizard(message))
}

// region: setup-validate
/// Problems of `step`; error names are control ids.
fn validate(state: &State, step: usize) -> FormErrors {
    let mut errors = FormErrors::new();
    match step {
        0 => {
            let name = state.name.trim();
            let valid = name.chars().count() >= 2
                && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !name.starts_with('-');
            errors.check("setup-name", valid, t!("setup.name-invalid"));
            errors.check("setup-location", !state.location.trim().is_empty(), t!("setup.location-missing"));
        }
        1 => errors.check("setup-engine", state.engine.is_some(), t!("setup.engine-missing")),
        _ => {}
    }
    errors
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let command = match message {
        Msg::Name(value) => {
            state.name = value;
            Command::none()
        }
        Msg::Location(value) => {
            state.location = value;
            Command::none()
        }
        Msg::Engine(index) => {
            state.engine = Some(index);
            log.push(PAGE, "RadioGroup#engine", format!("selected {}", ENGINES[index]));
            Command::none()
        }
        Msg::Theme(index) => {
            state.theme = index;
            log.push(PAGE, "Select#theme", format!("selected {}", THEMES[index]));
            Command::none()
        }
        Msg::Icons(index) => {
            state.icons = index;
            log.push(PAGE, "Segmented#icons", format!("selected {}", ICONS[index]));
            Command::none()
        }
        Msg::Animations(on) => {
            state.animations = on;
            log.push(PAGE, "Switch#animations", format!("toggled {on}"));
            Command::none()
        }
        Msg::Feature(index, on) => {
            state.features[index] = on;
            log.push(PAGE, format!("Checkbox#{}", FEATURES[index]), format!("toggled {on}"));
            Command::none()
        }
        Msg::Back => {
            state.errors.clear();
            state.step = state.step.saturating_sub(1);
            log.push(PAGE, "Wizard#back", t!(&format!("setup.{}", STEPS[state.step])));
            Command::none()
        }
        Msg::Step(index) => {
            state.errors.clear();
            state.step = index;
            log.push(PAGE, "Wizard#step", t!(&format!("setup.{}", STEPS[index])));
            Command::none()
        }
        // region: setup-next
        Msg::Next => {
            state.errors = validate(state, state.step);
            if state.errors.is_empty() {
                state.step += 1;
                log.push(PAGE, "Wizard#next", t!(&format!("setup.{}", STEPS[state.step])));
                // The first control of the new step takes focus.
                let first = ["setup-name", "setup-engine", "setup-look", "setup-features", "wizard-next"];
                Command::focus(first[state.step])
            } else {
                log.push(PAGE, "Wizard#next", format!("blocked on {}", state.errors.first().unwrap_or_default()));
                state.errors.focus_first()
            }
        }
        // endregion
        // region: setup-create
        Msg::Create => {
            log.push(PAGE, "Wizard#finish", format!("creating {}", state.name));
            state.creating = Some(0);
            next_stage(0)
        }
        Msg::StageDone(stage) => {
            log.push(PAGE, "Task#create", t!(&format!("setup.{}", STAGES[stage])));
            state.creating = Some(stage + 1);
            if stage + 1 < STAGES.len() {
                next_stage(stage + 1)
            } else {
                state.creating = None;
                state.created = true;
                Command::none()
            }
        }
        // endregion
        Msg::Cancel => {
            log.push(PAGE, "Wizard#cancel", "started over");
            *state = State::default();
            Command::focus("setup-name")
        }
    };
    if !state.errors.is_empty() {
        state.errors = validate(state, state.step);
    }
    command
}

/// Runs creation stage `stage` in the background; this example only waits.
fn next_stage(stage: usize) -> Command<AppMsg> {
    Command::perform(move || {
        std::thread::sleep(STAGE_TIME);
        send(Msg::StageDone(stage))
    })
}

/// The example application.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().gap(0), |ui| {
        ui.row(|ui| {
            ui.add(
                Text::rich([Span::new("quvyta").color("accent").bold(), Span::new("  new").role("faint")]).no_wrap(),
            );
            ui.spacer();
            ui.add(Text::new(t!("setup.tagline")).role("faint").no_wrap());
        })
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        if state.created {
            done_view(state, ui);
        } else if let Some(done) = state.creating {
            creating_view(done, ui);
        } else {
            wizard_view(state, ui);
        }
    })
    .fill_width();
}

// region: setup-wizard
fn wizard_view(state: &State, ui: &mut View<'_, AppMsg>) {
    Wizard::new(STEPS.map(|step| t!(&format!("setup.{step}"))))
        .current(state.step)
        .on_back(send(Msg::Back))
        .on_next(send(Msg::Next))
        .on_finish(send(Msg::Create))
        .on_cancel(send(Msg::Cancel))
        .on_step(|index| send(Msg::Step(index)))
        .page_height(PAGE_ROWS)
        .show(ui, |ui| match state.step {
            0 => project_page(state, ui),
            1 => engine_page(state, ui),
            2 => look_page(state, ui),
            3 => features_page(state, ui),
            _ => summary_page(state, ui),
        });
}
// endregion

fn project_page(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add(Text::new(t!("setup.project-intro")).role("secondary"));
    ui.spacer().height(Length::Cells(1));
    // region: setup-project
    Form::new().label_width(18).show(ui, |form| {
        let name = Field::new(t!("setup.name"))
            .required(true)
            .hint(t!("setup.name-hint"))
            .error(state.errors.get("setup-name"));
        form.field(name, |ui| {
            ui.add(
                TextInput::new(&state.name)
                    .placeholder("container-deck")
                    .max_length(40)
                    .invalid(state.errors.has("setup-name"))
                    .on_change(|value| send(Msg::Name(value))),
            )
            .width(Length::Cells(32))
            .id("setup-name");
        });
        let location = Field::new(t!("setup.location"))
            .hint(t!("setup.location-hint", path = project_path(state)))
            .error(state.errors.get("setup-location"));
        form.field(location, |ui| {
            ui.add(
                TextInput::new(&state.location)
                    .invalid(state.errors.has("setup-location"))
                    .on_change(|value| send(Msg::Location(value))),
            )
            .width(Length::Cells(32))
            .id("setup-location");
        });
    });
    // endregion
}

fn engine_page(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add(Text::new(t!("setup.engine-intro")).role("secondary"));
    ui.spacer().height(Length::Cells(1));
    Form::new().show(ui, |form| {
        let field = Field::new(t!("setup.engine")).required(true).error(state.errors.get("setup-engine"));
        form.field(field, |ui| {
            let options = ENGINES.map(|engine| t!(&format!("setup.engine-{engine}")));
            ui.add(RadioGroup::new(options).selected(state.engine).on_select(|index| send(Msg::Engine(index))))
                .id("setup-engine");
        });
    });
    if let Some(engine) = state.engine {
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!(&format!("setup.engine-{}-note", ENGINES[engine]))).role("faint"));
    }
}

// region: setup-look
fn look_page(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add(Text::new(t!("setup.look-intro")).role("secondary"));
    ui.spacer().height(Length::Cells(1));
    SettingsList::show(ui, |list| {
        list.row(SettingRow::new(t!("setup.theme")).description(t!("setup.theme-text")), |ui| {
            ui.add(Select::new(THEMES).selected(Some(state.theme)).on_select(|i| send(Msg::Theme(i))))
                .width(Length::Cells(16));
        });
        list.row(SettingRow::new(t!("setup.icons")).description(t!("setup.icons-text")), |ui| {
            let options = ICONS.map(|mode| t!(&format!("setup.icons-{mode}")));
            ui.add(qframe::widgets::Segmented::new(options).selected(state.icons).on_select(|i| send(Msg::Icons(i))));
        });
        list.row(SettingRow::new(t!("setup.animations")).description(t!("setup.animations-text")), |ui| {
            ui.add(Switch::new(state.animations).on_toggle(|on| send(Msg::Animations(on))));
        });
    })
    .id("setup-look");
}
// endregion

fn features_page(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add(Text::new(t!("setup.features-intro")).role("secondary"));
    ui.spacer().height(Length::Cells(1));
    for (index, feature) in FEATURES.iter().enumerate() {
        let node = ui.add(
            Checkbox::new(state.features[index])
                .label(t!(&format!("setup.feature-{feature}")))
                .on_toggle(move |on| send(Msg::Feature(index, on))),
        );
        if index == 0 {
            node.id("setup-features");
        } else {
            node.id(format!("setup-feature-{feature}"));
        }
        ui.add(Text::new(t!(&format!("setup.feature-{feature}-text"))).role("faint"))
            .padding(Padding { left: 5, ..Padding::default() });
    }
}

fn summary_page(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add(Text::new(t!("setup.summary-intro", path = project_path(state))).role("secondary"));
    ui.spacer().height(Length::Cells(1));
    ui.add(CodeView::new(config(state), Language::Toml).line_numbers(false)).fill_width();
}

/// Where the project will be created.
fn project_path(state: &State) -> String {
    let name = if state.name.trim().is_empty() { "…" } else { state.name.trim() };
    format!("{}/{name}", state.location.trim().trim_end_matches('/'))
}

/// The project file the summary previews.
fn config(state: &State) -> String {
    let features: Vec<String> =
        FEATURES.iter().zip(state.features).filter(|(_, on)| *on).map(|(name, _)| format!("\"{name}\"")).collect();
    format!(
        "[project]\nname = \"{}\"\nengine = \"{}\"\n\n[look]\ntheme = \"{}\"\nicons = \"{}\"\nanimations = {}\n\n[features]\nenabled = [{}]",
        state.name.trim(),
        state.engine.map_or("none", |engine| ENGINES[engine]),
        THEMES[state.theme].to_lowercase(),
        ICONS[state.icons],
        state.animations,
        features.join(", ")
    )
}

// region: setup-creating
fn creating_view(done: usize, ui: &mut View<'_, AppMsg>) {
    ui.add(Text::new(t!("setup.creating")).role("title"));
    ui.spacer().height(Length::Cells(1));
    ui.add(Steps::new(STAGES.map(|stage| t!(&format!("setup.{stage}")))).current(done).vertical(true).running(true));
    ui.spacer().height(Length::Cells(1));
    let progress = done as f32 / STAGES.len() as f32;
    ui.add(ProgressBar::new(progress).percent(true)).width(Length::Cells(40));
}
// endregion

fn done_view(state: &State, ui: &mut View<'_, AppMsg>) {
    let marker = ui.env().icons().glyph("success").into_owned();
    ui.add(
        Text::rich([
            Span::new(format!("{marker}  ")).color("success"),
            Span::new(t!("setup.created", name = state.name.trim().to_owned())).bold(),
        ])
        .no_wrap(),
    );
    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(t!("setup.next-steps")).role("secondary"));
    ui.spacer().height(Length::Cells(1));
    let commands = format!("cd {}\ncargo run", project_path(state));
    ui.add(CodeView::new(commands, Language::Plain).line_numbers(false)).fill_width();
    ui.spacer().height(Length::Cells(1));
    ui.add(Button::new(t!("setup.again")).variant("primary").on_press(send(Msg::Cancel))).id("setup-again");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn a_project_is_set_up_from_start_to_finish() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Next));
        assert!(h.is_focused("setup-name"), "an empty name blocks the first step");
        h.type_text("deck");
        h.send(send(Msg::Next));
        assert_eq!(h.app().pages.example_setup_wizard.step, 1);
        assert!(h.is_focused("setup-engine"), "the new step's first control takes focus");
        h.send(send(Msg::Next));
        assert_eq!(h.app().pages.example_setup_wizard.step, 1, "an engine must be chosen");
        h.press("down");
        assert_eq!(h.app().pages.example_setup_wizard.engine, Some(0));
        h.send(send(Msg::Next)).send(send(Msg::Next)).send(send(Msg::Next));
        assert!(h.screen().contains("name = \"deck\""), "{}", h.screen());
        h.send(send(Msg::Create));
        // Each stage is a perform that starts the next; the harness runs one per frame.
        for _ in 0..=STAGES.len() {
            if h.app().pages.example_setup_wizard.creating.is_none() {
                break;
            }
            h.render();
        }
        assert!(h.app().pages.example_setup_wizard.created);
        assert!(h.screen().contains("cargo run"), "{}", h.screen());
    }
}
