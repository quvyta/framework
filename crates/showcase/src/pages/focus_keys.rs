//! Focus and keys: focus order, the keymap and its labels, and the debug layer.

use qframe::keymap::Scope;
use qframe::prelude::*;
use qframe::widgets::TextInput;

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "focus-keys";

/// The focus order demo fields.
#[derive(Debug, Default)]
pub struct State {
    first: String,
    second: String,
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    First(String),
    Second(String),
    Pressed,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::FocusKeys(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::First(text) => state.first = text,
        Msg::Second(text) => state.second = text,
        Msg::Pressed => {
            log.push(PAGE, "Button#focus-demo", "pressed");
            // region: focus-command
            return Command::focus("first-field");
            // endregion
        }
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("focus-keys.order")), |ui| {
        ui.add(Text::new(t!("focus-keys.order-hint")).role("secondary"));
        // region: order
        ui.row(|ui| {
            ui.add(TextInput::new(&state.first).placeholder(t!("focus-keys.first")).on_change(|s| send(Msg::First(s))))
                .width(Length::Fill(1))
                .id("first-field");
            ui.add(
                TextInput::new(&state.second).placeholder(t!("focus-keys.second")).on_change(|s| send(Msg::Second(s))),
            )
            .width(Length::Fill(1))
            .id("second-field");
            ui.add(Button::new(t!("focus-keys.back-to-first")).on_press(send(Msg::Pressed))).id("focus-demo");
        })
        .gap(2)
        .fill_width();
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("focus-keys.keymap")), |ui| {
        let env = ui.env();
        let bindings: Vec<(Scope, String, String)> = env
            .keymap()
            .iter()
            .map(|(scope, action, chords)| {
                let keys = chords.iter().map(qframe::keymap::KeyChord::label).collect::<Vec<_>>().join("  ");
                (scope, action.to_owned(), keys)
            })
            .collect();
        let items = bindings.into_iter().map(|(scope, action, keys)| {
            // region: labels
            let label = env.i18n().translate(&scope.label_key(&action), &[]);
            // endregion
            let scope_name = if scope == Scope::Global { "global" } else { "app" };
            ListItem::new(format!("{keys}   {label}")).detail(format!("{scope_name} · {action}"))
        });
        ui.add(List::new(items.collect::<Vec<_>>())).width(Length::Fill(1)).height(Length::Cells(12)).id("bindings");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("focus-keys.debug")), |ui| {
        ui.add(Text::new(t!("focus-keys.debug-hint")).role("secondary"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn button_moves_focus_back_and_keymap_is_listed() {
        let mut h = showcase_on(PAGE);
        h.click_text("Back to the first field");
        assert!(h.is_focused("first-field"));
        h.type_text("hi");
        assert_eq!(h.app().pages.focus_keys.first, "hi");
        assert!(h.screen().contains("ctrl q"));
    }
}
