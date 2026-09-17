//! Button: variants, states, shortcut segment, icon and loading.

use qframe::prelude::*;
use qframe::widgets::Select;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "button";

/// Variants the playground offers.
const VARIANTS: [&str; 3] = ["neutral", "primary", "danger"];

/// Playground settings.
#[derive(Debug)]
pub struct State {
    variant: usize,
    disabled: bool,
    loading: bool,
    shortcut: bool,
    icon: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { variant: 1, disabled: false, loading: false, shortcut: true, icon: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Pressed(&'static str),
    Variant(usize),
    Disabled(bool),
    Loading(bool),
    Shortcut(bool),
    Icon(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Button(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Pressed(id) => log.push(PAGE, format!("Button#{id}"), "pressed"),
        Msg::Variant(index) => {
            state.variant = index;
            log.push(PAGE, "Playground", format!("variant = {}", VARIANTS[index]));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
        Msg::Loading(on) => {
            state.loading = on;
            log.push(PAGE, "Playground", format!("loading = {on}"));
        }
        Msg::Shortcut(on) => {
            state.shortcut = on;
            log.push(PAGE, "Playground", format!("shortcut = {on}"));
        }
        Msg::Icon(on) => {
            state.icon = on;
            log.push(PAGE, "Playground", format!("icon = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.row(|ui| {
            // region: variants
            ui.add(
                Button::new(t!("button.save")).variant("primary").shortcut("⏎").on_press(send(Msg::Pressed("save"))),
            )
            .id("save");
            ui.add(Button::new(t!("button.cancel")).on_press(send(Msg::Pressed("cancel")))).id("cancel");
            ui.add(Button::new(t!("button.delete")).variant("danger").on_press(send(Msg::Pressed("delete"))))
                .id("delete");
            ui.add(Button::new(t!("button.disabled")).disabled(true).on_press(send(Msg::Pressed("disabled"))));
            // endregion
        })
        .gap(2);
        ui.add(Text::new(t!("button.configured")).role("faint"));
        ui.row(|ui| {
            // region: configured
            let mut button = Button::new(t!("button.publish")).on_press(send(Msg::Pressed("publish")));
            if VARIANTS[state.variant] != "neutral" {
                button = button.variant(VARIANTS[state.variant]);
            }
            if state.shortcut {
                button = button.shortcut("ctrl s");
            }
            if state.icon {
                button = button.icon("check");
            }
            ui.add(button.disabled(state.disabled).loading(state.loading)).id("publish");
            // endregion
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        let variants = VARIANTS.map(|v| t!(&format!("button.variant.{v}")));
        setting(ui, t!("button.variant"), |ui| {
            ui.add(Select::new(variants).selected(Some(state.variant)).on_select(|i| send(Msg::Variant(i))))
                .width(Length::Cells(16))
                .id("variant");
        });
        setting(ui, t!("button.disabled-label"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        setting(ui, t!("button.loading"), |ui| {
            ui.add(toggle(state.loading, |on| send(Msg::Loading(on)))).id("loading");
        });
        setting(ui, t!("button.shortcut"), |ui| {
            ui.add(toggle(state.shortcut, |on| send(Msg::Shortcut(on)))).id("shortcut");
        });
        setting(ui, t!("button.icon"), |ui| {
            ui.add(toggle(state.icon, |on| send(Msg::Icon(on)))).id("icon");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn presses_are_logged_and_playground_changes_the_button() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("⏎").expect("save shortcut");
        h.hover(x + 5, y);
        let row = h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned();
        let left = usize::try_from(x).unwrap_or(0) - 2;
        assert!(row.chars().skip(left).collect::<String>().starts_with("▌ ⏎   Save"), "the pillar comes first: {row}");
        h.click_text("Save");
        assert!(h.screen().contains("Button#save"));
        h.click_text("Primary").advance(std::time::Duration::from_millis(200));
        h.click_text("Danger");
        assert_eq!(h.app().pages.button.variant, 2);
        assert!(h.screen().contains("variant = danger"));
    }
}
