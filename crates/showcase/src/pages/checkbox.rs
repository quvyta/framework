//! Checkbox: on and off, partly checked parents, labels and disabled boxes.

use qframe::prelude::*;
use qframe::widgets::{Checkbox, CheckboxStyle, Segmented};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "checkbox";

/// The notification kinds grouped under one parent box.
const KINDS: [&str; 3] = ["builds", "deploys", "mentions"];

/// Styles the playground offers, the default first, with their locale keys.
const STYLES: [(CheckboxStyle, &str); 2] = [(CheckboxStyle::Box, "box"), (CheckboxStyle::Check, "check")];

/// Settings of the demo form and the playground.
#[derive(Debug)]
pub struct State {
    autosave: bool,
    telemetry: bool,
    notify: [bool; 3],
    style: usize,
    disabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { autosave: true, telemetry: false, notify: [true, false, true], style: 0, disabled: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Autosave(bool),
    Telemetry(bool),
    AllNotifications(bool),
    Notification(usize, bool),
    Style(usize),
    Disabled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Checkbox(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Autosave(on) => {
            state.autosave = on;
            log.push(PAGE, "Checkbox#autosave", format!("toggled {on}"));
        }
        Msg::Telemetry(on) => {
            state.telemetry = on;
            log.push(PAGE, "Checkbox#telemetry", format!("toggled {on}"));
        }
        // region: parent-update
        Msg::AllNotifications(on) => {
            state.notify = [on; 3];
            log.push(PAGE, "Checkbox#notifications", format!("toggled {on}"));
        }
        Msg::Notification(index, on) => {
            state.notify[index] = on;
            log.push(PAGE, format!("Checkbox#{}", KINDS[index]), format!("toggled {on}"));
        }
        // endregion
        Msg::Style(index) => {
            state.style = index;
            log.push(PAGE, "Playground", format!("style = {}", STYLES[index].1));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let style = STYLES[state.style].0;
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("checkbox.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: basic
        ui.add(
            Checkbox::new(state.autosave)
                .style(style)
                .label(t!("checkbox.autosave"))
                .disabled(state.disabled)
                .on_toggle(|on| send(Msg::Autosave(on))),
        )
        .id("autosave");
        // endregion
        ui.add(
            Checkbox::new(state.telemetry)
                .style(style)
                .label(t!("checkbox.telemetry"))
                .disabled(state.disabled)
                .on_toggle(|on| send(Msg::Telemetry(on))),
        )
        .id("telemetry");
        ui.spacer().height(Length::Cells(1));
        // region: parent
        let checked = state.notify.iter().filter(|on| **on).count();
        let all = checked == KINDS.len();
        ui.add(
            Checkbox::new(all)
                .style(style)
                .partial(checked > 0 && !all)
                .label(t!("checkbox.notifications", n = checked, total = KINDS.len()))
                .disabled(state.disabled)
                .on_toggle(|on| send(Msg::AllNotifications(on))),
        )
        .id("notifications");
        for (index, kind) in KINDS.iter().enumerate() {
            ui.add(
                Checkbox::new(state.notify[index])
                    .style(style)
                    .label(t!(&format!("checkbox.{kind}")))
                    .disabled(state.disabled)
                    .on_toggle(move |on| send(Msg::Notification(index, on))),
            )
            .padding(Padding { left: 4, ..Padding::default() })
            .id(*kind);
        }
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("checkbox.style"), |ui| {
            let names = STYLES.map(|(_, name)| t!(&format!("checkbox.style-{name}")));
            ui.add(Segmented::new(names).selected(state.style).on_select(|i| send(Msg::Style(i)))).id("style");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn parent_box_checks_every_child_and_shows_partial() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("Notifications, 2 of 3").expect("parent box");
        let (x, y) = (u16::try_from(x - 4).unwrap_or(0), u16::try_from(y).unwrap_or(0));
        let theme = h.env().theme().clone();
        assert_eq!((h.bg(x, y), h.bg(x + 1, y)), (theme.color("accent"), theme.color("raised")), "partly filled");
        h.send(send(Msg::Style(1)));
        assert!(h.screen().contains("–   Notifications, 2 of 3"), "{}", h.screen());
        assert!(h.screen().contains("style = check"));
        h.send(send(Msg::Style(0)));
        h.click_text("Notifications");
        assert_eq!(h.app().pages.checkbox.notify, [true; 3]);
        h.click_text("Deploys");
        assert_eq!(h.app().pages.checkbox.notify, [true, false, true]);
        h.click_text("Autosave");
        assert!(!h.app().pages.checkbox.autosave);
    }
}
