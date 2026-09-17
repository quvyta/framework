//! Widget dock: an IDE-like side stack of widgets that open, close and are dragged into a new
//! order, with the order and open state kept in the application so they could be saved.

use qframe::prelude::*;
use qframe::widgets::{Section, WidgetDock};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "widget-dock";

/// The widgets, by id: key and icon.
const WIDGETS: [(&str, &str); 4] =
    [("git", "info"), ("containers", "dot"), ("ports", "arrow-right"), ("activity", "file")];

/// Demo containers: name, status colour and status key.
const CONTAINERS: [(&str, &str, &str); 7] = [
    ("quvyta-dev", "success", "running"),
    ("postgres", "success", "running"),
    ("redis", "success", "running"),
    ("cache-builder", "warning", "paused"),
    ("docs-preview", "success", "running"),
    ("legacy-api", "muted", "stopped"),
    ("nightly-tests", "danger", "failed"),
];

/// Recent activity lines.
const ACTIVITY: [&str; 8] = [
    "10:42  deploy billing",
    "10:40  build web-frontend",
    "10:31  restart postgres",
    "10:12  pull quvyta/dev:1.4",
    "09:58  deploy auth",
    "09:47  prune volumes",
    "09:30  build search-index",
    "09:02  start quvyta-dev",
];

// region: dock-state
/// What the application saves: the widget ids in display order and which widgets are open.
#[derive(Debug)]
pub struct State {
    order: Vec<usize>,
    open: [bool; 4],
    reorderable: bool,
    icons: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { order: vec![0, 1, 2, 3], open: [true, true, false, false], reorderable: true, icons: true }
    }
}
// endregion

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Toggle(usize, bool),
    Move(usize, usize),
    Reorderable(bool),
    Icons(bool),
    Reset,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::WidgetDock(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: dock-update
        Msg::Toggle(position, open) => {
            let id = state.order[position];
            state.open[id] = open;
            let verb = if open { "opened" } else { "closed" };
            log.push(PAGE, "WidgetDock#side", format!("{verb} {}", WIDGETS[id].0));
        }
        Msg::Move(from, to) => {
            let id = state.order.remove(from);
            state.order.insert(to, id);
            log.push(PAGE, "WidgetDock#side", format!("moved {} from {from} to {to}", WIDGETS[id].0));
        }
        // endregion
        Msg::Reorderable(on) => {
            state.reorderable = on;
            log.push(PAGE, "Playground", format!("reorderable = {on}"));
        }
        Msg::Icons(on) => {
            state.icons = on;
            log.push(PAGE, "Playground", format!("icons = {on}"));
        }
        Msg::Reset => {
            let State { reorderable, icons, .. } = *state;
            *state = State { reorderable, icons, ..State::default() };
            log.push(PAGE, "Button#reset", "pressed");
        }
    }
    Command::none()
}

/// The body of widget `id`.
fn body(ui: &mut View<'_, AppMsg>, id: usize) {
    match WIDGETS[id].0 {
        "git" => {
            ui.column(|ui| {
                ui.add(Text::rich([Span::new("main").color("accent").bold(), Span::new("  ↑2").role("faint")]));
                ui.add(Text::new(t!("widget-dock.changed", n = 3)).role("secondary"));
            });
        }
        "containers" => {
            let rows = CONTAINERS.iter().map(|(name, color, status)| {
                ListItem::new(*name).icon("dot", Some(color)).detail(t!(&format!("list.status.{status}")))
            });
            ui.add(List::new(rows)).fill().id("containers-list");
        }
        "ports" => {
            ui.column(|ui| {
                for (port, service) in [("8080 → 80", "http"), ("5432 → 5432", "postgres"), ("6379 → 6379", "redis")]
                {
                    ui.add(Text::rich([Span::new(format!("{port}   ")), Span::new(service).role("faint")]).no_wrap());
                }
            });
        }
        _ => {
            ui.add(List::new(ACTIVITY.iter().map(|line| ListItem::new(*line)))).fill().id("activity-list");
        }
    }
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("widget-dock.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            // region: dock
            let sections = state.order.iter().map(|id| {
                let (key, icon) = WIDGETS[*id];
                let section = Section::new(t!(&format!("widget-dock.{key}")));
                if state.icons { section.icon(icon) } else { section }
            });
            let open: Vec<bool> = state.order.iter().map(|id| state.open[*id]).collect();
            let mut dock =
                WidgetDock::new(sections).open(open).on_toggle(|position, open| send(Msg::Toggle(position, open)));
            if state.reorderable {
                dock = dock.on_move(|from, to| send(Msg::Move(from, to)));
            }
            ui.add_with(dock, |ui| {
                for id in &state.order {
                    ui.column(|ui| body(ui, *id)).fill().id(WIDGETS[*id].0);
                }
            })
            .width(Length::Cells(44))
            .height(Length::Cells(24))
            .id("side");
            // endregion
            ui.column(|ui| {
                ui.add(Text::new(t!("widget-dock.saved")).role("faint"));
                let names: Vec<String> =
                    state.order.iter().map(|id| t!(&format!("widget-dock.{}", WIDGETS[*id].0))).collect();
                ui.add(Text::new(names.join(", ")).role("secondary"));
                let open: Vec<String> = state
                    .order
                    .iter()
                    .filter(|id| state.open[**id])
                    .map(|id| t!(&format!("widget-dock.{}", WIDGETS[*id].0)))
                    .collect();
                let open = if open.is_empty() { t!("widget-dock.none") } else { open.join(", ") };
                ui.add(Text::new(t!("widget-dock.open", names = open)).role("secondary"));
            })
            .gap(1)
            .fill_width();
        })
        .gap(4)
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("widget-dock.reorderable"), |ui| {
            ui.add(toggle(state.reorderable, |on| send(Msg::Reorderable(on)))).id("reorderable");
        });
        setting(ui, t!("widget-dock.icons"), |ui| {
            ui.add(toggle(state.icons, |on| send(Msg::Icons(on)))).id("icons");
        });
        setting(ui, t!("widget-dock.reset-label"), |ui| {
            ui.add(Button::new(t!("widget-dock.reset")).on_press(send(Msg::Reset))).id("reset");
        });
        ui.add(Text::new(t!("widget-dock.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::event::{MouseButton, MouseKind};

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn widgets_toggle_move_and_reset() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("quvyta-dev"), "{}", h.screen());
        h.click_text("→ Ports");
        assert_eq!(h.app().pages.widget_dock.open, [true, true, true, false], "{}", h.screen());
        let (x, y) = h.find("▪ Activity").expect("activity title");
        let (_, top) = h.find("ℹ Source control").expect("git title");
        h.mouse(MouseKind::Down(MouseButton::Left), x, y);
        h.mouse(MouseKind::Drag(MouseButton::Left), x, top);
        h.mouse(MouseKind::Up(MouseButton::Left), x, top);
        assert_eq!(h.app().pages.widget_dock.order, vec![3, 0, 1, 2]);
        h.send(send(Msg::Reset));
        assert_eq!(h.app().pages.widget_dock.order, vec![0, 1, 2, 3]);
        h.send(send(Msg::Reorderable(false)));
        let (x, y) = h.find("▪ Activity").expect("activity title");
        h.mouse(MouseKind::Down(MouseButton::Left), x, y);
        h.mouse(MouseKind::Drag(MouseButton::Left), x, top);
        h.mouse(MouseKind::Up(MouseButton::Left), x, top);
        assert_eq!(h.app().pages.widget_dock.order, vec![0, 1, 2, 3]);
    }
}
