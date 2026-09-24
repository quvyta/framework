//! Context menu: actions for the row under the pointer.

use qframe::prelude::*;
use qframe::widgets::{ContextItem, ContextMenu};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "context-menu";

/// Containers in the demo list.
const CONTAINERS: [&str; 4] = ["api-gateway", "postgres-primary", "redis-cache", "worker-emails"];

/// The selected container and the playground.
#[derive(Debug)]
pub struct State {
    selected: Option<usize>,
    icons: bool,
    shortcuts: bool,
    submenu: bool,
    paused: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { selected: Some(0), icons: true, shortcuts: true, submenu: true, paused: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Select(usize),
    Action(&'static str),
    Icons(bool),
    Shortcuts(bool),
    Submenu(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ContextMenu(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Select(index) => state.selected = Some(index),
        Msg::Action(action) => {
            if matches!(action, "pause" | "resume") {
                state.paused = action == "pause";
            }
            let container = state.selected.map_or("", |index| CONTAINERS[index]);
            log.push(PAGE, "ContextMenu#containers", format!("{action} {container}"));
        }
        Msg::Icons(on) => {
            state.icons = on;
            log.push(PAGE, "Playground", format!("icons = {on}"));
        }
        Msg::Shortcuts(on) => {
            state.shortcuts = on;
            log.push(PAGE, "Playground", format!("shortcuts = {on}"));
        }
        Msg::Submenu(on) => {
            state.submenu = on;
            log.push(PAGE, "Playground", format!("submenu = {on}"));
        }
    }
    Command::none()
}

// region: context-items
/// The menu for a container, built from the playground settings.
fn items(state: &State) -> Vec<ContextItem<AppMsg>> {
    let action = |key: &'static str, icon: &str, shortcut: &str| {
        let mut item = ContextItem::new(t!(&format!("context-menu.{key}")), send(Msg::Action(key)));
        if state.icons {
            item = item.icon(icon);
        }
        if state.shortcuts && !shortcut.is_empty() {
            item = item.shortcut(shortcut);
        }
        item
    };
    let mut items = vec![
        action("logs", "file", "enter"),
        action("restart", "arrow-right", "ctrl r"),
        if state.paused { action("resume", "arrow-right", "") } else { action("pause", "dot-outline", "") },
        action("attach", "prompt", "").detail(t!("context-menu.attach-detail")).disabled(true),
    ];
    if state.submenu {
        items.push(ContextItem::submenu(
            t!("context-menu.move"),
            [
                ContextItem::new("staging", send(Msg::Action("move-staging"))),
                ContextItem::new("production", send(Msg::Action("move-production"))),
            ],
        ));
    }
    items.push(ContextItem::gap());
    items.push(action("copy", "check", "ctrl c"));
    items.push(ContextItem::gap());
    items.push(action("delete", "error", "delete").danger(true));
    items
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        // region: context-list
        ui.add_with(ContextMenu::new(items(state)), |ui| {
            let rows = CONTAINERS.map(|name| ListItem::new(name).detail(t!("context-menu.running")));
            ui.add(List::new(rows).selected(state.selected).on_select(|index| send(Msg::Select(index))))
                .fill_width()
                .id("containers");
        })
        .fill_width()
        .height(Length::Cells(12));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("context-menu.icons"), |ui| {
            ui.add(toggle(state.icons, |on| send(Msg::Icons(on)))).id("icons");
        });
        setting(ui, t!("context-menu.shortcuts"), |ui| {
            ui.add(toggle(state.shortcuts, |on| send(Msg::Shortcuts(on)))).id("shortcuts");
        });
        setting(ui, t!("context-menu.submenu"), |ui| {
            ui.add(toggle(state.submenu, |on| send(Msg::Submenu(on)))).id("submenu");
        });
        ui.add(Text::new(t!("context-menu.hint")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::event::{MouseButton, MouseKind};

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn right_click_chooses_an_action_for_the_row() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("redis-cache").expect("list on screen");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y);
        h.mouse(MouseKind::Up(MouseButton::Right), x, y);
        h.click_text("Restart");
        assert!(h.screen().contains("restart"), "{}", h.screen());
    }

    #[test]
    fn the_disabled_row_says_why_in_a_quiet_note() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("redis-cache").expect("list on screen");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y);
        h.mouse(MouseKind::Up(MouseButton::Right), x, y);
        let (x, y) = h.find("no shell in image").expect("the note is drawn");
        let (label_x, label_y) = h.find("Attach terminal").expect("the label is drawn");
        assert_eq!(y, label_y);
        assert!(x > label_x, "the note sits right of the label");
        let fg = h.fg(u16::try_from(x).unwrap(), u16::try_from(y).unwrap());
        assert_eq!(fg, h.env().theme().color("muted"));
    }

    #[test]
    fn keyboard_opens_the_menu_and_deletes() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("worker-emails");
        h.press("shift+f10").press("end").press("enter");
        assert!(h.screen().contains("delete worker-emails"), "{}", h.screen());
    }
}
