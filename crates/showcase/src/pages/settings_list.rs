//! Settings list: workspace preferences with switches, choices and values anchored on the right.

use qframe::prelude::*;
use qframe::widgets::{Segmented, Select, SettingRow, SettingsList, Switch};

use super::{PageMsg, setting, slide_setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "settings-list";

/// Container engines to choose from.
const ENGINES: [&str; 2] = ["Podman", "Docker"];

/// How often unused images are removed.
const PRUNE: [&str; 3] = ["daily", "weekly", "never"];

/// Width of the list when the playground narrows it.
const NARROW: u16 = 44;

/// The workspace preferences and the playground.
#[derive(Debug)]
pub struct State {
    theme: usize,
    density: usize,
    animations: bool,
    engine: usize,
    autostart: bool,
    prune: Option<usize>,
    telemetry: bool,
    opened: usize,
    descriptions: bool,
    locked: bool,
    narrow: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            theme: 0,
            density: 0,
            animations: true,
            engine: 0,
            autostart: false,
            prune: Some(1),
            telemetry: false,
            opened: 0,
            descriptions: true,
            locked: true,
            narrow: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Theme(usize),
    Density(usize),
    Animations(bool),
    Engine(usize),
    Autostart(bool),
    Prune(usize),
    Telemetry(bool),
    OpenStorage,
    Descriptions(bool),
    Locked(bool),
    Narrow(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::SettingsList(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Theme(index) => {
            state.theme = index;
            log.push(PAGE, "Select#theme", format!("selected {index}"));
        }
        Msg::Density(index) => {
            state.density = index;
            log.push(PAGE, "Segmented#density", format!("selected {index}"));
        }
        Msg::Animations(on) => {
            state.animations = on;
            log.push(PAGE, "Switch#animations", format!("toggled {on}"));
        }
        Msg::Engine(index) => {
            state.engine = index;
            log.push(PAGE, "Segmented#engine", format!("selected {}", ENGINES[index]));
        }
        Msg::Autostart(on) => {
            state.autostart = on;
            log.push(PAGE, "Switch#autostart", format!("toggled {on}"));
        }
        Msg::Prune(index) => {
            state.prune = Some(index);
            log.push(PAGE, "Select#prune", format!("selected {}", PRUNE[index]));
        }
        Msg::Telemetry(on) => {
            state.telemetry = on;
            log.push(PAGE, "Switch#telemetry", format!("toggled {on}"));
        }
        Msg::OpenStorage => {
            state.opened += 1;
            log.push(PAGE, "SettingRow#storage", "activated");
        }
        Msg::Descriptions(on) => {
            state.descriptions = on;
            log.push(PAGE, "Playground", format!("descriptions = {on}"));
        }
        Msg::Locked(on) => {
            state.locked = on;
            log.push(PAGE, "Playground", format!("telemetry disabled = {on}"));
        }
        Msg::Narrow(on) => {
            state.narrow = on;
            log.push(PAGE, "Playground", format!("narrow = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let describe = |row: SettingRow<AppMsg>, key: &str| {
        if state.descriptions { row.description(t!(key)) } else { row }
    };
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("settings-list.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let list = SettingsList::show(ui, |list| {
            list.heading(t!("settings-list.appearance"));
            list.row(SettingRow::new(t!("settings-list.theme")), |ui| {
                let themes = ["Monochrome", "Iris", "Nordic", "Amber"];
                ui.add(Select::new(themes).selected(Some(state.theme)).on_select(|i| send(Msg::Theme(i))))
                    .width(Length::Cells(16));
            });
            // region: settings-rows
            let density = [t!("settings-list.cozy"), t!("settings-list.compact")];
            list.row(SettingRow::new(t!("settings-list.density")), |ui| {
                ui.add(Segmented::new(density).selected(state.density).on_select(|i| send(Msg::Density(i))));
            });
            let animations = describe(SettingRow::new(t!("settings-list.animations")), "settings-list.animations-text");
            list.row(animations, |ui| {
                ui.add(Switch::new(state.animations).on_toggle(|on| send(Msg::Animations(on))));
            });
            // endregion
            list.heading(t!("settings-list.containers"));
            list.row(SettingRow::new(t!("settings-list.engine")), |ui| {
                ui.add(Segmented::new(ENGINES).selected(state.engine).on_select(|i| send(Msg::Engine(i))));
            });
            let autostart = describe(SettingRow::new(t!("settings-list.autostart")), "settings-list.autostart-text");
            list.row(autostart, |ui| {
                ui.add(Switch::new(state.autostart).on_toggle(|on| send(Msg::Autostart(on))));
            });
            list.row(SettingRow::new(t!("settings-list.prune")), |ui| {
                let options = PRUNE.map(|p| t!(&format!("settings-list.prune-{p}")));
                ui.add(Select::new(options).selected(state.prune).on_select(|i| send(Msg::Prune(i))))
                    .width(Length::Cells(16));
            });
            list.heading(t!("settings-list.privacy"));
            // region: settings-extras
            let storage = describe(SettingRow::new(t!("settings-list.storage")), "settings-list.storage-text")
                .on_activate(send(Msg::OpenStorage));
            list.row(storage, |ui| {
                ui.add(Text::new("2.4 GB").role("secondary").no_wrap());
            });
            let telemetry = describe(SettingRow::new(t!("settings-list.telemetry")), "settings-list.telemetry-text")
                .disabled(state.locked);
            list.row(telemetry, |ui| {
                ui.add(Switch::new(state.telemetry).disabled(state.locked).on_toggle(|on| send(Msg::Telemetry(on))));
            });
            // endregion
        })
        .id("preferences");
        if state.narrow {
            list.width(Length::Cells(NARROW));
        }
        if state.opened > 0 {
            ui.spacer().height(Length::Cells(1));
            ui.add(Text::new(t!("settings-list.opened", n = state.opened)).role("faint"));
        }
        // Room for a dropdown opened on the last rows.
        ui.spacer().height(Length::Cells(3));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("settings-list.descriptions"), |ui| {
            ui.add(toggle(state.descriptions, |on| send(Msg::Descriptions(on)))).id("descriptions");
        });
        setting(ui, t!("settings-list.locked"), |ui| {
            ui.add(toggle(state.locked, |on| send(Msg::Locked(on)))).id("locked");
        });
        setting(ui, t!("settings-list.narrow"), |ui| {
            ui.add(toggle(state.narrow, |on| send(Msg::Narrow(on)))).id("narrow");
        });
        slide_setting(ui, PAGE);
        ui.add(Text::new(t!("settings-list.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn keyboard_walks_rows_and_drives_their_controls() {
        let mut h = showcase_on(PAGE);
        h.click_text("Density");
        assert!(h.is_focused("preferences"));
        h.press("right");
        assert_eq!(h.app().pages.settings_list.density, 1);
        h.press("down").press("space");
        assert!(!h.app().pages.settings_list.animations);
        h.press("down").press("down").press("down").press("down").press("enter");
        assert_eq!(h.app().pages.settings_list.opened, 1);
        h.press("down").press("space");
        assert!(!h.app().pages.settings_list.telemetry, "the locked row is skipped");
        h.send(send(Msg::Narrow(true)));
        assert!(h.screen().contains("…"), "{}", h.screen());
    }
}
