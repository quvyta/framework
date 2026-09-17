//! Empty state: an icon, a title, an explanation and a way out, adapting to small areas.

use qframe::prelude::*;
use qframe::widgets::EmptyState;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "empty-state";

/// Playground settings.
#[derive(Debug)]
pub struct State {
    icon: bool,
    message: bool,
    action: bool,
    small: bool,
    created: u32,
}

impl Default for State {
    fn default() -> Self {
        Self { icon: true, message: true, action: true, small: false, created: 0 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Create,
    Icon(bool),
    Message(bool),
    Action(bool),
    Small(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::EmptyState(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Create => {
            state.created += 1;
            ("Button#run", "pressed".to_owned())
        }
        Msg::Icon(on) => {
            state.icon = on;
            ("Playground", format!("icon = {on}"))
        }
        Msg::Message(on) => {
            state.message = on;
            ("Playground", format!("message = {on}"))
        }
        Msg::Action(on) => {
            state.action = on;
            ("Playground", format!("action = {on}"))
        }
        Msg::Small(on) => {
            state.small = on;
            ("Playground", format!("small area = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("empty-state.containers")).gap(0), |ui| {
        // region: basic
        ui.add(
            EmptyState::new(t!("empty-state.title"))
                .icon("inbox")
                .message(t!("empty-state.message"))
                .action(Button::new(t!("empty-state.run")).variant("primary").on_press(send(Msg::Create))),
        )
        .fill_width()
        .height(Length::Cells(10))
        .id("containers");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("empty-state.search")).gap(0), |ui| {
        // region: title-only
        ui.add(EmptyState::new(t!("empty-state.no-match")).message(t!("empty-state.no-match-hint")))
            .fill_width()
            .height(Length::Cells(4));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.row(|ui| {
            // region: configured
            let mut empty = EmptyState::new(t!("empty-state.title"));
            if state.icon {
                empty = empty.icon("inbox");
            }
            if state.message {
                empty = empty.message(t!("empty-state.message"));
            }
            if state.action {
                empty = empty.action(Button::new(t!("empty-state.run")).on_press(send(Msg::Create)));
            }
            let (width, height) = if state.small { (24, 4) } else { (56, 9) };
            ui.add(empty).width(Length::Cells(width)).height(Length::Cells(height)).id("configured");
            // endregion
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("empty-state.icon"), |ui| {
            ui.add(toggle(state.icon, |on| send(Msg::Icon(on)))).id("icon");
        });
        setting(ui, t!("empty-state.message-setting"), |ui| {
            ui.add(toggle(state.message, |on| send(Msg::Message(on)))).id("message");
        });
        setting(ui, t!("empty-state.action"), |ui| {
            ui.add(toggle(state.action, |on| send(Msg::Action(on)))).id("action");
        });
        setting(ui, t!("empty-state.small"), |ui| {
            ui.add(toggle(state.small, |on| send(Msg::Small(on)))).id("small");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn action_runs_and_small_areas_keep_the_title() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("No containers yet"), "{}", h.screen());
        h.click_text("Run a container");
        assert_eq!(h.app().pages.empty_state.created, 1);
        h.send(send(Msg::Small(true)));
        assert!(h.screen().matches("No containers yet").count() >= 2);
    }
}
