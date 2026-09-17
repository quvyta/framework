//! Menu: a sidebar of grouped destinations with icons, badges and folding groups.

use qframe::prelude::*;
use qframe::widgets::{Menu, MenuGroup, MenuItem};

use super::{PageMsg, setting, slide_setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "menu";

/// One destination of the demo console: group, key, icon, icon colour, badge.
type Destination = (&'static str, &'static str, &'static str, Option<&'static str>, Option<&'static str>);

/// Destinations of the demo console.
const ITEMS: [Destination; 9] = [
    ("workspace", "overview", "info", None, None),
    ("workspace", "deploys", "success", Some("success"), Some("3")),
    ("workspace", "logs", "file", None, None),
    ("infrastructure", "containers", "dot", Some("success"), Some("12")),
    ("infrastructure", "volumes", "folder", None, None),
    ("infrastructure", "alerts", "warning", Some("warning"), Some("2")),
    ("settings", "members", "dot-outline", None, None),
    ("settings", "tokens", "dot-outline", None, None),
    ("settings", "billing", "dot-outline", None, None),
];

/// Groups in order.
const GROUPS: [&str; 3] = ["workspace", "infrastructure", "settings"];

/// The current page, the closed groups and the playground.
#[derive(Debug)]
pub struct State {
    page: String,
    closed: Vec<String>,
    icons: bool,
    badges: bool,
    collapsible: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { page: "deploys".to_owned(), closed: Vec::new(), icons: true, badges: true, collapsible: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Go(String),
    Group(String, bool),
    Icons(bool),
    Badges(bool),
    Collapsible(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Menu(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Go(page) => {
            log.push(PAGE, "Menu#console", format!("selected {page}"));
            state.page = page;
        }
        // region: menu-toggle
        Msg::Group(group, open) => {
            state.closed.retain(|closed| *closed != group);
            if !open {
                state.closed.push(group.clone());
            }
            log.push(PAGE, "Menu#console", format!("{group} open = {open}"));
        }
        // endregion
        Msg::Icons(on) => {
            state.icons = on;
            log.push(PAGE, "Playground", format!("icons = {on}"));
        }
        Msg::Badges(on) => {
            state.badges = on;
            log.push(PAGE, "Playground", format!("badges = {on}"));
        }
        Msg::Collapsible(on) => {
            state.collapsible = on;
            log.push(PAGE, "Playground", format!("collapsible = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("menu.hint")).role("secondary"));
        ui.row(|ui| {
            // region: menu-groups
            let groups = GROUPS.map(|group| {
                let items = ITEMS.iter().filter(|item| item.0 == group).map(|(_, key, icon, color, badge)| {
                    let mut item = MenuItem::new(*key, t!(&format!("menu.{key}")));
                    if state.icons {
                        item = item.icon(*icon, *color);
                    }
                    match badge {
                        Some(badge) if state.badges => item.badge(*badge),
                        _ => item,
                    }
                });
                MenuGroup::new(group, items).title(t!(&format!("menu.{group}")))
            });
            // endregion
            // region: menu-menu
            let mut menu = Menu::new(groups).selected(Some(&state.page)).on_select(|key| send(Msg::Go(key.to_owned())));
            if state.collapsible {
                menu = menu
                    .collapsible(|group, open| send(Msg::Group(group.to_owned(), open)))
                    .collapsed(state.closed.clone());
            }
            ui.add(menu).width(Length::Cells(28)).height(Length::Cells(15)).id("console");
            // endregion
            ui.column(|ui| {
                ui.add(Text::new(t!(&format!("menu.{}", state.page))).role("title"));
                ui.add(Text::new(t!(&format!("menu.{}-text", state.page))).role("secondary"));
            })
            .padding(Padding::symmetric(0, 3))
            .fill_width();
        })
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("menu.icons"), |ui| {
            ui.add(toggle(state.icons, |on| send(Msg::Icons(on)))).id("icons");
        });
        setting(ui, t!("menu.badges"), |ui| {
            ui.add(toggle(state.badges, |on| send(Msg::Badges(on)))).id("badges");
        });
        setting(ui, t!("menu.collapsible"), |ui| {
            ui.add(toggle(state.collapsible, |on| send(Msg::Collapsible(on)))).id("collapsible");
        });
        slide_setting(ui, PAGE);
        ui.add(Text::new(t!("menu.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn navigates_and_folds_groups() {
        let mut h = showcase_on(PAGE);
        h.click_text("Volumes");
        assert_eq!(h.app().pages.menu.page, "volumes");
        assert!(h.screen().contains("Persistent disks"), "{}", h.screen());
        h.click_text("SETTINGS");
        assert_eq!(h.app().pages.menu.closed, Vec::<String>::new(), "plain headings do nothing");
        h.send(send(Msg::Collapsible(true)));
        h.click_text("SETTINGS");
        assert_eq!(h.app().pages.menu.closed, vec!["settings".to_owned()]);
        assert!(!h.screen().contains("Billing"), "{}", h.screen());
    }
}
