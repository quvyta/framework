//! Text input: editing, validation, passwords, limits and submitting.

use qframe::prelude::*;
use qframe::widgets::TextInput;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "text-input";

/// The most characters a project code may have.
const CODE_LIMIT: usize = 8;

/// Field values and playground settings.
#[derive(Debug, Default)]
pub struct State {
    name: String,
    password: String,
    code: String,
    disabled: bool,
    submitted: Option<String>,
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Name(String),
    Password(String),
    Code(String),
    Submit(String),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::TextInput(message))
}

// region: validation
/// A project name needs at least three characters; an empty field is not an error yet.
fn name_problem(name: &str) -> bool {
    let length = name.chars().count();
    length > 0 && length < 3
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Name(value) => {
            log.push(PAGE, "TextInput#name", format!("changed {value:?}"));
            state.name = value;
        }
        Msg::Password(value) => {
            log.push(PAGE, "TextInput#password", format!("changed, {} characters", value.chars().count()));
            state.password = value;
        }
        Msg::Code(value) => {
            log.push(PAGE, "TextInput#code", format!("changed {value:?}"));
            state.code = value;
        }
        Msg::Submit(value) => {
            log.push(PAGE, "TextInput#name", format!("submitted {value:?}"));
            state.submitted = Some(value);
        }
        Msg::Disabled(on) => {
            log.push(PAGE, "Playground", format!("disabled = {on}"));
            state.disabled = on;
        }
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("text-input.name")).role("secondary"));
        // region: field
        let invalid = name_problem(&state.name);
        ui.add(
            TextInput::new(&state.name)
                .placeholder(t!("text-input.name-placeholder"))
                .invalid(invalid)
                .disabled(state.disabled)
                .on_change(|value| send(Msg::Name(value)))
                .on_submit(|value| send(Msg::Submit(value))),
        )
        .width(Length::Cells(40))
        .id("name");
        if invalid {
            ui.add(Text::new(t!("text-input.name-error")).color("danger"));
        }
        // endregion
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("text-input.password")).role("secondary"));
        ui.add(
            TextInput::new(&state.password)
                .password(true)
                .placeholder(t!("text-input.password-placeholder"))
                .disabled(state.disabled)
                .on_change(|value| send(Msg::Password(value))),
        )
        .width(Length::Cells(40))
        .id("password");
        ui.spacer().height(Length::Cells(1));
        ui.add(
            Text::new(t!("text-input.code", count = state.code.chars().count(), limit = CODE_LIMIT)).role("secondary"),
        );
        // region: limit
        ui.add(
            TextInput::new(&state.code)
                .max_length(CODE_LIMIT)
                .placeholder("QVT-2026")
                .disabled(state.disabled)
                .on_change(|value| send(Msg::Code(value))),
        )
        .width(Length::Cells(20))
        .id("code");
        // endregion
        if let Some(submitted) = &state.submitted {
            ui.spacer().height(Length::Cells(1));
            ui.add(Text::new(t!("text-input.submitted", value = submitted.clone())).color("success"));
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        ui.add(Text::new(t!("text-input.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod edit_menu_tests {
    use qframe::event::{MouseButton, MouseKind};

    use crate::tests::{click_text_below, showcase_on};

    #[test]
    fn the_edit_menu_cuts_and_pastes_and_the_log_shows_it() {
        let mut h = showcase_on(super::PAGE);
        h.set_reduced_motion(true);
        h.click_text("My project").type_text("aurora");
        let (x, y) = h.find("aurora").expect("typed name on screen");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        click_text_below(&mut h, "Select all", y);
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        click_text_below(&mut h, "Cut", y);
        assert_eq!(h.app().pages.text_input.name, "");
        assert_eq!(h.clipboard(), Some("aurora"));
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        click_text_below(&mut h, "Paste", y);
        assert_eq!(h.app().pages.text_input.name, "aurora");
        let log = h.app().log.recent(super::PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "copied 6 characters"), "{log:?}");
        assert!(log.iter().any(|entry| entry.message == "changed \"\""), "{log:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn validates_limits_and_submits() {
        let mut h = showcase_on(PAGE);
        h.click_text("My project");
        h.type_text("ab");
        assert!(h.screen().contains("at least 3"));
        h.type_text("c").press("enter");
        assert_eq!(h.app().pages.text_input.submitted.as_deref(), Some("abc"));
        h.click_text("QVT-2026");
        h.type_text("ABCDEFGHIJ");
        assert_eq!(h.app().pages.text_input.code, "ABCDEFGH");
    }
}
