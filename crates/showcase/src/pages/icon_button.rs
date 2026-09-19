//! Icon button: one glyph in three cells, for the quiet controls at the end of a header.

use qframe::prelude::*;
use qframe::widgets::{IconButton, Select};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "icon-button";

/// Icons the playground offers, by their key in the icon set.
const ICONS: [&str; 5] = ["settings", "search", "add", "close", "profile"];

/// Playground settings.
#[derive(Debug)]
pub struct State {
    icon: usize,
    disabled: bool,
    tooltip: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { icon: 0, disabled: false, tooltip: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Pressed(&'static str),
    Icon(usize),
    Disabled(bool),
    Tooltip(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::IconButton(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Pressed(id) => log.push(PAGE, format!("IconButton#{id}"), "pressed"),
        Msg::Icon(index) => {
            state.icon = index;
            log.push(PAGE, "Playground", format!("icon = {}", ICONS[index]));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
        Msg::Tooltip(on) => {
            state.tooltip = on;
            log.push(PAGE, "Playground", format!("tooltip = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("icon-button.header-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            // region: header
            ui.add(Text::new(t!("icon-button.title")).role("title")).fill_width();
            ui.add(IconButton::new("search").tooltip(t!("icon-button.search")).on_press(send(Msg::Pressed("search"))))
                .id("search");
            ui.add(IconButton::new("add").tooltip(t!("icon-button.add")).on_press(send(Msg::Pressed("add")))).id("add");
            ui.add(
                IconButton::new("settings")
                    .tooltip(t!("icon-button.settings"))
                    .on_press(send(Msg::Pressed("settings"))),
            )
            .id("settings");
            // endregion
        })
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("icon-button.configured")).role("faint"));
        ui.row(|ui| {
            // region: configured
            let icon = ICONS[state.icon];
            let mut button = IconButton::new(icon).disabled(state.disabled).on_press(send(Msg::Pressed(icon)));
            if state.tooltip {
                button = button.tooltip(t!(&format!("icon-button.icon.{icon}")));
            }
            ui.add(button).id("configured");
            // endregion
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        let icons = ICONS.map(|icon| t!(&format!("icon-button.icon.{icon}")));
        setting(ui, t!("icon-button.icon-label"), |ui| {
            ui.add(Select::new(icons).selected(Some(state.icon)).on_select(|i| send(Msg::Icon(i))))
                .width(Length::Cells(16))
                .id("icon");
        });
        setting(ui, t!("icon-button.disabled"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
        setting(ui, t!("icon-button.tooltip"), |ui| {
            ui.add(toggle(state.tooltip, |on| send(Msg::Tooltip(on)))).id("tooltip");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn the_header_buttons_press_and_name_themselves() {
        let mut h = showcase_on(PAGE);
        h.set_glyph_mode(qframe::icons::GlyphMode::Unicode);
        let (x, y) = h.find("Packages").expect("the header title");
        let row = h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned();
        let settings = row.chars().position(|c| c == '▤').expect("the settings glyph on the title's row");
        let settings = i32::try_from(settings).unwrap_or(0);
        assert!(settings > x, "{row}");
        h.hover(settings, y).advance(std::time::Duration::from_secs(1));
        let below = h.screen().lines().nth(usize::try_from(y + 1).unwrap_or(0)).unwrap_or_default().to_owned();
        assert!(below.contains("Settings"), "the tooltip names it below: {}", h.screen());
        h.click(settings, y);
        assert!(h.screen().contains("IconButton#settings"), "{}", h.screen());
    }

    #[test]
    fn the_playground_changes_the_configured_button() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Icon(1))).send(send(Msg::Disabled(true)));
        assert_eq!(h.app().pages.icon_button.icon, 1);
        assert!(h.app().pages.icon_button.disabled);
        assert!(h.screen().contains("disabled = true"), "{}", h.screen());
        h.send(send(Msg::Tooltip(false)));
        assert!(!h.app().pages.icon_button.tooltip);
    }
}
