//! Modal: dialogs over a dimmed screen, destructive dialogs, stacked dialogs and the options one
//! at a time.

use qframe::prelude::*;
use qframe::widgets::{Modal, Select, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "modal";

/// Environment variables of the demo project.
const VARIABLES: [&str; 4] = ["DATABASE_URL", "REDIS_URL", "SENTRY_DSN", "STRIPE_KEY"];

/// Widths the playground offers.
const WIDTHS: [u16; 3] = [40, 56, 72];

/// Which dialog of the live demo is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    Rename,
    Delete,
    Variables,
}

/// The demo project and the playground.
#[derive(Debug)]
pub struct State {
    project: String,
    draft: String,
    deleted: bool,
    variables: usize,
    open: Option<Dialog>,
    clearing: bool,
    playground_open: bool,
    title: bool,
    danger: bool,
    dismissable: bool,
    outside: bool,
    actions: bool,
    width: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            project: "atlas-api".to_owned(),
            draft: String::new(),
            deleted: false,
            variables: VARIABLES.len(),
            open: None,
            clearing: false,
            playground_open: false,
            title: true,
            danger: false,
            dismissable: true,
            outside: false,
            actions: true,
            width: 1,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Open(Dialog),
    Close,
    Cancel,
    Draft(String),
    Rename,
    Delete,
    Restore,
    AskClear,
    KeepVariables,
    ClearVariables,
    PlaygroundOpen(bool),
    PlaygroundDismissed,
    Title(bool),
    Danger(bool),
    Dismissable(bool),
    Outside(bool),
    Actions(bool),
    Width(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Modal(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Open(dialog) => {
            state.draft = state.project.clone();
            state.open = Some(dialog);
            log.push(PAGE, "Modal", format!("open {dialog:?}"));
        }
        Msg::Close => {
            log.push(PAGE, "Modal", format!("dismissed {:?} with Esc or ×", state.open));
            state.open = None;
        }
        Msg::Cancel => {
            log.push(PAGE, "Button", format!("closed {:?}", state.open));
            state.open = None;
        }
        Msg::Draft(text) => state.draft = text,
        Msg::Rename => {
            let name = state.draft.trim();
            if !name.is_empty() {
                state.project = name.to_owned();
            }
            state.open = None;
            log.push(PAGE, "Modal#rename", format!("renamed to {}", state.project));
        }
        Msg::Delete => {
            state.deleted = true;
            state.open = None;
            log.push(PAGE, "Modal#delete", format!("deleted {}", state.project));
        }
        Msg::Restore => {
            state.deleted = false;
            log.push(PAGE, "Button#restore", format!("restored {}", state.project));
        }
        Msg::AskClear => {
            state.clearing = true;
            log.push(PAGE, "Modal#clear", "open, stacked");
        }
        Msg::KeepVariables => {
            state.clearing = false;
            log.push(PAGE, "Modal#clear", "closed");
        }
        Msg::ClearVariables => {
            state.clearing = false;
            state.variables = 0;
            log.push(PAGE, "Modal#clear", "cleared variables");
        }
        Msg::PlaygroundOpen(open) => {
            state.playground_open = open;
            log.push(PAGE, "Modal#playground", if open { "open" } else { "closed with Done" });
        }
        Msg::PlaygroundDismissed => {
            state.playground_open = false;
            log.push(PAGE, "Modal#playground", "dismissed with Esc, × or a click outside");
        }
        Msg::Title(on) => set(log, &mut state.title, "title", on),
        Msg::Danger(on) => set(log, &mut state.danger, "variant danger", on),
        Msg::Dismissable(on) => set(log, &mut state.dismissable, "dismissable", on),
        Msg::Outside(on) => set(log, &mut state.outside, "close_on_click_outside", on),
        Msg::Actions(on) => set(log, &mut state.actions, "actions", on),
        Msg::Width(index) => {
            state.width = index;
            log.push(PAGE, "Playground", format!("width = {}", WIDTHS[index]));
        }
    }
    Command::none()
}

fn set(log: &mut EventLog, field: &mut bool, name: &str, on: bool) {
    *field = on;
    log.push(PAGE, "Playground", format!("{name} = {on}"));
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("modal.hint")).role("secondary"));
        if state.deleted {
            ui.row(|ui| {
                ui.add(Text::new(t!("modal.deleted", name = state.project.as_str())).role("faint").no_wrap());
                ui.add(Button::new(t!("modal.restore")).on_press(send(Msg::Restore))).id("restore");
            })
            .gap(2);
        } else {
            ui.add(
                Text::rich([
                    Span::new(format!("{}   ", t!("modal.project"))).role("faint"),
                    Span::new(state.project.clone()).bold(),
                    Span::new(format!("   {}", t!("modal.variables-count", n = state.variables))).role("secondary"),
                ])
                .no_wrap(),
            );
            ui.row(|ui| {
                ui.add(Button::new(t!("modal.rename")).on_press(send(Msg::Open(Dialog::Rename)))).id("rename");
                ui.add(Button::new(t!("modal.variables")).on_press(send(Msg::Open(Dialog::Variables)))).id("variables");
                ui.add(Button::new(t!("modal.delete")).variant("danger").on_press(send(Msg::Open(Dialog::Delete))))
                    .id("delete");
            })
            .gap(2);
        }
        match state.open {
            Some(Dialog::Rename) => rename(state, ui),
            Some(Dialog::Delete) => delete(state, ui),
            Some(Dialog::Variables) => variables(state, ui),
            None => {}
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("modal.title-label"), |ui| {
            ui.add(toggle(state.title, |on| send(Msg::Title(on)))).id("title");
        });
        setting(ui, t!("modal.danger"), |ui| {
            ui.add(toggle(state.danger, |on| send(Msg::Danger(on)))).id("danger");
        });
        setting(ui, t!("modal.dismissable"), |ui| {
            ui.add(toggle(state.dismissable, |on| send(Msg::Dismissable(on)))).id("dismissable");
        });
        setting(ui, t!("modal.outside"), |ui| {
            ui.add(toggle(state.outside, |on| send(Msg::Outside(on)))).id("outside");
        });
        setting(ui, t!("modal.actions"), |ui| {
            ui.add(toggle(state.actions, |on| send(Msg::Actions(on)))).id("actions");
        });
        setting(ui, t!("modal.width"), |ui| {
            ui.add(
                Select::new(WIDTHS.map(|w| w.to_string()))
                    .selected(Some(state.width))
                    .on_select(|i| send(Msg::Width(i))),
            )
            .width(Length::Cells(10))
            .id("width");
        });
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("modal.open")).variant("primary").on_press(send(Msg::PlaygroundOpen(true))))
            .id("open-playground");
        if state.playground_open {
            configured(state, ui);
        }
    })
    .fill_width();
}

fn rename(state: &State, ui: &mut View<'_, AppMsg>) {
    // region: modal-basic
    let dialog = Modal::new()
        .title(t!("modal.rename"))
        .on_close(send(Msg::Close))
        .action(Button::new(t!("modal.cancel")).on_press(send(Msg::Cancel)))
        .action(Button::new(t!("modal.rename-action")).variant("primary").on_press(send(Msg::Rename)));
    ui.add_with(dialog, |ui| {
        ui.add(Text::new(t!("modal.rename-body")).role("secondary"));
        ui.add(TextInput::new(&state.draft).on_change(|text| send(Msg::Draft(text))).on_submit(|_| send(Msg::Rename)))
            .fill_width()
            .id("project-name");
    });
    // endregion
}

fn delete(state: &State, ui: &mut View<'_, AppMsg>) {
    // region: modal-danger
    let dialog = Modal::new()
        .title(t!("modal.delete-title", name = state.project.as_str()))
        .variant("danger")
        .on_close(send(Msg::Close))
        .action(Button::new(t!("modal.cancel")).on_press(send(Msg::Cancel)))
        .action(Button::new(t!("modal.delete-action")).variant("danger").on_press(send(Msg::Delete)));
    ui.add_with(dialog, |ui| {
        ui.add(Text::new(t!("modal.delete-body")).role("secondary"));
    });
    // endregion
}

fn variables(state: &State, ui: &mut View<'_, AppMsg>) {
    // region: modal-stacked
    let dialog = Modal::new()
        .title(t!("modal.variables"))
        .on_close(send(Msg::Close))
        .action(Button::new(t!("modal.clear")).variant("danger").on_press(send(Msg::AskClear)))
        .action(Button::new(t!("modal.done")).variant("primary").on_press(send(Msg::Cancel)));
    ui.add_with(dialog, |ui| {
        ui.add(Text::new(t!("modal.variables-count", n = state.variables)).role("secondary"));
        for name in VARIABLES.iter().take(state.variables) {
            ui.add(Text::new(*name).role("faint"));
        }
        if state.clearing {
            // A dialog added inside another one stacks on top of it.
            let confirm = Modal::new()
                .title(t!("modal.clear-title"))
                .variant("danger")
                .width(44)
                .on_close(send(Msg::KeepVariables))
                .action(Button::new(t!("modal.cancel")).on_press(send(Msg::KeepVariables)))
                .action(Button::new(t!("modal.clear")).variant("danger").on_press(send(Msg::ClearVariables)));
            ui.add_with(confirm, |ui| {
                ui.add(Text::new(t!("modal.clear-body")).role("secondary"));
            });
        }
    });
    // endregion
}

fn configured(state: &State, ui: &mut View<'_, AppMsg>) {
    // region: modal-configured
    // Esc and × always come together: `dismissable(false)` turns both off, and the click
    // outside with them, while the close message stays.
    let mut dialog = Modal::new()
        .width(WIDTHS[state.width])
        .on_close(send(Msg::PlaygroundDismissed))
        .dismissable(state.dismissable)
        .close_on_click_outside(state.outside);
    if state.title {
        dialog = dialog.title(t!("modal.play-title"));
    }
    if state.danger {
        dialog = dialog.variant("danger");
    }
    if state.actions || !state.dismissable {
        // A dialog that cannot be dismissed needs a button to close it.
        dialog = dialog.action(Button::new(t!("modal.done")).on_press(send(Msg::PlaygroundOpen(false))));
    }
    ui.add_with(dialog, |ui| {
        ui.add(Text::new(t!("modal.play-body")).role("secondary"));
    });
    // endregion
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn renames_in_a_dialog_and_focus_returns() {
        let mut h = showcase_on(PAGE);
        h.click_text("Rename project").advance(Duration::from_millis(200));
        assert!(h.is_focused("project-name"), "{}", h.screen());
        h.press("ctrl+u").type_text("atlas-web").press("enter");
        assert_eq!(h.app().pages.modal.project, "atlas-web");
        assert!(h.is_focused("rename"));
        assert!(h.screen().contains("Modal#rename"));
    }

    #[test]
    fn stacked_dialog_clears_and_escape_closes_only_the_top() {
        let mut h = showcase_on(PAGE);
        h.click_text("Environment variables").advance(Duration::from_millis(200));
        h.click_text("Clear all").advance(Duration::from_millis(200));
        assert!(h.screen().contains("Clear all variables?"), "{}", h.screen());
        h.press("esc");
        assert!(!h.app().pages.modal.clearing);
        assert_eq!(h.app().pages.modal.open, Some(Dialog::Variables));
        h.press("esc");
        assert_eq!(h.app().pages.modal.open, None);
        assert_eq!(h.app().current(), PAGE, "esc inside a dialog never goes back a page");
    }

    #[test]
    fn danger_dialog_deletes_and_restores() {
        let mut h = showcase_on(PAGE);
        h.click_text("Delete project").advance(Duration::from_millis(200));
        assert!(h.screen().contains("Delete atlas-api?"));
        h.press("tab").press("enter");
        assert!(h.app().pages.modal.deleted);
        h.click_text("Restore project");
        assert!(!h.app().pages.modal.deleted);
    }

    #[test]
    fn the_close_mark_dismisses_and_the_playground_turns_esc_and_the_mark_off_together() {
        let mut h = showcase_on(PAGE);
        h.click_text("Rename project").advance(Duration::from_millis(200));
        let (x, y) = crate::tests::close_mark_beside(&h, "Rename project");
        h.click(x.expect("a close mark on the title row"), y);
        assert_eq!(h.app().pages.modal.open, None);
        assert!(h.screen().contains("dismissed Some(Rename) with Esc or ×"), "{}", h.screen());
        h.send(send(Msg::Dismissable(false)));
        h.click_text("Open the dialog").advance(Duration::from_millis(200));
        let (mark, _) = crate::tests::close_mark_beside(&h, "Maintenance window");
        assert!(mark.is_none() && !h.screen().contains("esc close"), "{}", h.screen());
        h.press("esc");
        assert!(h.app().pages.modal.playground_open, "Esc does nothing either");
        h.click_text("Done");
        assert!(!h.app().pages.modal.playground_open);
    }
}
