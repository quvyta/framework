//! Text input: editing, validation, passwords, limits, suggestions, submitting and a rename field
//! that opens with the name selected.

use qframe::prelude::*;
use qframe::text::fuzzy;
use qframe::widgets::{Suggestion, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "text-input";

/// The most characters a project code may have.
const CODE_LIMIT: usize = 8;

/// The packages the search offers, with the icon before the name and the note on its right.
const PACKAGES: [(&str, &str, &str); 4] = [
    ("quvyta-config", "settings in one file", "folder-config"),
    ("quvyta-tools", "shell helpers, 12 files", "folder-source"),
    ("quvyta-themes", "colour schemes, 4 files", "folder-packages"),
    ("quvyta-showcase", "the component gallery", "workspace"),
];

/// Field values and playground settings.
#[derive(Debug)]
pub struct State {
    name: String,
    password: String,
    code: String,
    file: String,
    package: String,
    disabled: bool,
    submitted: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            name: String::new(),
            password: String::new(),
            code: String::new(),
            file: "quarterly-report.md".to_owned(),
            package: String::new(),
            disabled: false,
            submitted: None,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Name(String),
    Password(String),
    Code(String),
    File(String),
    Package(String),
    PackageChosen(usize),
    Submit(String),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::TextInput(message))
}

// region: rename-range
/// The characters of `file` before its extension, so a rename starts on the name. A name with no
/// extension, or one that only starts with a dot such as `.bashrc`, is selected whole.
fn name_part(file: &str) -> std::ops::Range<usize> {
    match file.rfind('.') {
        Some(dot) if dot > 0 => 0..file[..dot].chars().count(),
        _ => 0..file.chars().count(),
    }
}
// endregion

// region: validation
/// A project name needs at least three characters; an empty field is not an error yet.
fn name_problem(name: &str) -> bool {
    let length = name.chars().count();
    length > 0 && length < 3
}
// endregion

// region: suggestions
/// The packages whose name holds what has been typed, the best match first, as a package search
/// would rank them. The field draws the list and says which row was chosen; what belongs in the
/// list, and how it is found, is the application's own work.
fn packages_for(typed: &str) -> Vec<Suggestion> {
    let mut found: Vec<(i32, Suggestion)> = PACKAGES
        .iter()
        .filter_map(|(name, note, icon)| {
            let found = fuzzy(typed, name)?;
            Some((-found.score(), Suggestion::new(*name).detail(*note).icon(*icon)))
        })
        .collect();
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, row)| row).collect()
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
        Msg::File(value) => {
            log.push(PAGE, "TextInput#rename", format!("changed {value:?}"));
            state.file = value;
        }
        Msg::Package(value) => {
            log.push(PAGE, "TextInput#package", format!("changed {value:?}"));
            state.package = value;
        }
        Msg::PackageChosen(index) => {
            let offered = packages_for(&state.package);
            let Some(name) = offered.get(index).map(|row| row.label().to_owned()) else {
                return Command::none();
            };
            log.push(PAGE, "TextInput#package", format!("chose {name}"));
            state.package = name;
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
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("text-input.rename")).role("secondary"));
        // region: rename
        ui.add(
            TextInput::new(&state.file)
                .select_on_focus(name_part(&state.file))
                .disabled(state.disabled)
                .on_change(|value| send(Msg::File(value))),
        )
        .width(Length::Cells(40))
        .id("rename");
        // endregion
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("text-input.package")).role("secondary"));
        // region: suggestion-field
        ui.add(
            TextInput::new(&state.package)
                .placeholder(t!("text-input.package-placeholder"))
                .suggestions(packages_for(&state.package))
                .disabled(state.disabled)
                .on_change(|value| send(Msg::Package(value)))
                .on_suggestion(|index| send(Msg::PackageChosen(index))),
        )
        .width(Length::Cells(40))
        .id("package");
        // endregion
        // The list has a row for every package, and opens over what is under the field.
        ui.spacer().height(Length::Cells(PACKAGES.len() as u16));
        ui.add(Text::new(t!("text-input.suggestion-keys")).role("faint"));
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

    #[test]
    fn the_rename_field_opens_with_the_name_selected() {
        let mut h = showcase_on(PAGE);
        h.click_text("QVT-2026").press("tab").type_text("summary");
        assert_eq!(h.app().pages.text_input.file, "summary.md", "Tab into the field selects the name only");
        assert_eq!(name_part("şğü.txt"), 0..3, "characters, not bytes");
        assert_eq!(name_part(".bashrc"), 0..7);
        assert_eq!(name_part("Makefile"), 0..8);
    }

    #[test]
    fn the_package_search_offers_what_was_typed_and_takes_the_chosen_row() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("a package name");
        assert!(!h.screen().contains("quvyta-tools"), "the list waits for a keystroke:\n{}", h.screen());
        h.type_text("t");
        let screen = h.screen();
        assert!(
            screen.contains("quvyta-tools") && screen.contains("quvyta-themes"),
            "every package with a t is on offer: {screen}"
        );
        // The search ranks what it found, so the second row is read off the screen.
        let mut rows: Vec<(i32, &str)> =
            PACKAGES.iter().filter_map(|(name, _, _)| h.find(name).map(|(_, y)| (y, *name))).collect();
        rows.sort_unstable();
        let second = rows.get(1).map(|(_, name)| *name).unwrap_or_else(|| panic!("two rows on offer: {screen}"));
        h.press("down").press("down").press("enter");
        assert_eq!(h.app().pages.text_input.package, second, "the second row of the list on offer");
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == format!("chose {second}")), "{log:?}");
    }

    #[test]
    fn esc_closes_the_package_list_and_keeps_what_was_typed() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("a package name").type_text("t");
        assert!(h.screen().contains("quvyta-tools"), "{}", h.screen());
        h.press("esc");
        assert!(!h.screen().contains("quvyta-tools"), "the list is closed:\n{}", h.screen());
        assert_eq!(h.app().pages.text_input.package, "t", "and the text is still there");
        h.type_text("h");
        assert!(h.screen().contains("quvyta-themes"), "typing opens the list again:\n{}", h.screen());
    }
}
