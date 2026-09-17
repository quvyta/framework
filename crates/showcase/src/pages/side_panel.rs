//! Side panel: an explorer and a search view docked beside an editor, switched from an activity
//! bar, opened and closed from its edge or a shortcut, resized by dragging, and either collapsing
//! to the bar or hiding completely.

use qframe::prelude::*;
use qframe::widgets::{Closed, Segmented, Side, SidePanel, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "side-panel";

/// Views the panel can show, by locale key and icon.
const VIEWS: [(&str, &str); 2] = [("explorer", "folder"), ("search", "search")];

/// Files of the explorer view.
const FILES: [&str; 7] = ["src", "main.rs", "app.rs", "pages", "Cargo.toml", "CATALOG.toml", "README.md"];

/// Lines of the project the search view looks through: file, line number, text.
const LINES: [(&str, u16, &str); 9] = [
    ("main.rs", 2, "mod app;"),
    ("main.rs", 4, "use showcase::app::Showcase;"),
    ("app.rs", 12, "pub struct Showcase"),
    ("app.rs", 58, "use qframe::widgets::SidePanel;"),
    ("app.rs", 60, "panel_width: u16,"),
    ("Cargo.toml", 2, "name = \"showcase\""),
    ("CATALOG.toml", 929, "id = \"side-panel\""),
    ("README.md", 1, "# Quvyta showcase"),
    ("README.md", 7, "The side panel keeps the explorer and search one click away."),
];

/// Width limits the playground offers: none above the minimum, then two ranges.
const LIMITS: [(u16, Option<u16>); 3] = [(8, None), (18, Some(48)), (30, Some(60))];

/// Panel state owned by the application, and the playground.
#[derive(Debug)]
pub struct State {
    open: bool,
    width: u16,
    view: usize,
    query: String,
    right: bool,
    collapsible: bool,
    hide: bool,
    resizable: bool,
    strip: bool,
    /// Index into [`LIMITS`].
    limits: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: true,
            width: 30,
            view: 0,
            query: "panel".to_owned(),
            right: false,
            collapsible: true,
            hide: false,
            resizable: true,
            strip: true,
            limits: 0,
        }
    }
}

/// How a limit choice reads: the range, or the word for no upper limit.
fn limits_name(index: usize) -> String {
    match LIMITS[index] {
        (_, None) => t!("side-panel.unlimited"),
        (min, Some(max)) => format!("{min}–{max}"),
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Toggle(bool),
    Resize(u16),
    View(u16),
    Query(String),
    Side(usize),
    Collapsible(bool),
    Closing(usize),
    Resizable(bool),
    ShowStrip(bool),
    Limits(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::SidePanel(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: side-panel-update
        Msg::Toggle(open) => {
            state.open = open;
            log.push(PAGE, "SidePanel#explorer", if open { "opened" } else { "closed" });
        }
        Msg::Resize(width) => {
            state.width = width;
            log.push(PAGE, "SidePanel#explorer", format!("resized to {width}"));
        }
        // A strip icon means "show this view": switch to it, opening the panel if needed. The
        // shown view's icon closes the panel through Toggle instead.
        Msg::View(index) => {
            state.view = usize::from(index).min(VIEWS.len() - 1);
            let view = VIEWS[state.view].0;
            let message = if state.open { format!("switched to {view}") } else { format!("opened on {view}") };
            state.open = true;
            log.push(PAGE, "SidePanel#explorer", message);
        }
        // endregion
        Msg::Query(query) => state.query = query,
        Msg::Side(index) => {
            state.right = index == 1;
            log.push(PAGE, "Playground", format!("side = {}", if state.right { "right" } else { "left" }));
        }
        Msg::Collapsible(on) => {
            state.collapsible = on;
            state.open |= !on;
            log.push(PAGE, "Playground", format!("collapsible = {on}"));
        }
        Msg::Closing(index) => {
            state.hide = index == 1;
            log.push(PAGE, "Playground", format!("closing = {}", if state.hide { "hide" } else { "collapse" }));
        }
        Msg::Resizable(on) => {
            state.resizable = on;
            log.push(PAGE, "Playground", format!("resizable = {on}"));
        }
        Msg::ShowStrip(on) => {
            state.strip = on;
            log.push(PAGE, "Playground", format!("strip = {on}"));
        }
        Msg::Limits(index) => {
            state.limits = index;
            // The panel never shows a width outside its limits; keep the state honest too.
            let (min, max) = LIMITS[index];
            state.width = state.width.max(min).min(max.unwrap_or(u16::MAX));
            log.push(PAGE, "Playground", format!("limits = {}, width = {}", limits_name(index), state.width));
        }
    }
    Command::none()
}

/// The explorer view: the project's files.
fn explorer(ui: &mut View<'_, AppMsg>) {
    let rows = FILES.iter().map(|name| {
        let icon = if name.contains('.') { "file" } else { "folder" };
        ListItem::new(*name).icon(icon, None)
    });
    ui.add(List::new(rows).selected(Some(1))).fill().id("tree");
}

/// The search view: a query and the project lines that contain it.
fn search(ui: &mut View<'_, AppMsg>, query: &str) {
    ui.add(TextInput::new(query).placeholder(t!("side-panel.search-placeholder")).on_change(|q| send(Msg::Query(q))))
        .fill_width()
        .id("query");
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        ui.add(Text::new(t!("side-panel.search-empty")).role("faint"));
        return;
    }
    let found: Vec<_> = LINES.iter().filter(|(_, _, text)| text.to_lowercase().contains(&needle)).collect();
    if found.is_empty() {
        ui.add(Text::new(t!("side-panel.search-none", query = query.trim())).role("faint"));
        return;
    }
    let files = {
        let mut names: Vec<&str> = found.iter().map(|(file, _, _)| *file).collect();
        names.dedup();
        names.len()
    };
    let files = t!("side-panel.search-files", n = files);
    ui.add(Text::new(t!("side-panel.search-count", n = found.len(), files = files)).role("faint").no_wrap());
    let rows = found.iter().map(|(file, line, text)| ListItem::new(text.trim()).detail(format!("{file}:{line}")));
    ui.add(List::new(rows)).fill().id("results");
}

/// The panel's content for the chosen view.
fn panel(ui: &mut View<'_, AppMsg>, state: &State) {
    ui.column(|ui| {
        ui.add(Text::new(t!(&format!("side-panel.{}", VIEWS[state.view].0))).role("faint").no_wrap());
        if state.view == 0 {
            explorer(ui);
        } else {
            search(ui, &state.query);
        }
    })
    .gap(1)
    .padding(Padding { top: 1, right: 1, bottom: 0, left: 1 })
    .fill();
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("side-panel.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.column(|ui| {
            // region: side-panel
            let side = if state.right { Side::Right } else { Side::Left };
            let closed = if state.hide { Closed::Hide } else { Closed::Collapse };
            let (min, max) = LIMITS[state.limits];
            let mut side_panel =
                SidePanel::new(state.width).side(side).open(state.open).closed(closed).limits(min, max);
            if state.collapsible {
                side_panel = side_panel.on_toggle(|open| send(Msg::Toggle(open)));
            }
            if state.resizable {
                side_panel = side_panel.on_resize(|width| send(Msg::Resize(width)));
            }
            if state.strip {
                let active = u16::try_from(state.view).unwrap_or(0);
                side_panel =
                    side_panel.strip(VIEWS.map(|(_, icon)| icon), |index| send(Msg::View(index))).active_view(active);
            }
            side_panel
                .panel(|ui| panel(ui, state))
                .body(|ui| {
                    ui.column(|ui| {
                        ui.add(Text::new("app.rs").role("title").no_wrap());
                        ui.add(Text::new(t!("side-panel.body")).role("secondary"));
                        ui.add(Button::new(t!("side-panel.run"))).id("run");
                    })
                    .gap(1)
                    .padding(Padding::symmetric(1, 3))
                    .fill();
                })
                .show(ui)
                .id("explorer");
            // endregion
        })
        .height(Length::Cells(16))
        .fill_width();
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("side-panel.side"), |ui| {
            let options = [t!("side-panel.left"), t!("side-panel.right")];
            ui.add(Segmented::new(options).selected(usize::from(state.right)).on_select(|i| send(Msg::Side(i))))
                .id("side");
        });
        setting(ui, t!("side-panel.collapsible"), |ui| {
            ui.add(toggle(state.collapsible, |on| send(Msg::Collapsible(on)))).id("collapsible");
        });
        setting(ui, t!("side-panel.closing"), |ui| {
            let options = [t!("side-panel.collapse"), t!("side-panel.hide")];
            ui.add(Segmented::new(options).selected(usize::from(state.hide)).on_select(|i| send(Msg::Closing(i))))
                .id("closing");
        });
        setting(ui, t!("side-panel.resizable"), |ui| {
            ui.add(toggle(state.resizable, |on| send(Msg::Resizable(on)))).id("resizable");
        });
        setting(ui, t!("side-panel.strip"), |ui| {
            ui.add(toggle(state.strip, |on| send(Msg::ShowStrip(on)))).id("strip");
        });
        setting(ui, t!("side-panel.limits"), |ui| {
            let names: Vec<String> = (0..LIMITS.len()).map(limits_name).collect();
            ui.add(Segmented::new(names).selected(state.limits).on_select(|i| send(Msg::Limits(i)))).id("limits");
        });
        ui.add(Text::new(t!("side-panel.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;
    use qframe::runtime::Harness;

    type Showcase = Harness<crate::app::Showcase>;

    fn state(h: &Showcase) -> &State {
        &h.app().pages.side_panel
    }

    fn last_log(h: &Showcase) -> String {
        h.app().log.recent(PAGE, 1)[0].message.clone()
    }

    #[test]
    fn the_strip_switches_views_while_open_and_closes_on_the_shown_view() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        assert!(h.screen().contains("CATALOG.toml"), "{}", h.screen());
        let (x, y) = h.find("⌕").expect("search icon beside the open panel");
        h.click(x, y);
        assert_eq!((state(&h).view, state(&h).open), (1, true));
        assert_eq!(last_log(&h), "switched to search");
        assert!(h.screen().contains("4 results in 3 files"), "{}", h.screen());
        assert!(h.screen().contains("app.rs:58"), "{}", h.screen());
        h.click(x, y);
        assert!(!state(&h).open, "the shown view's icon closes the panel");
        assert_eq!(last_log(&h), "closed");
        assert_eq!(h.find("⌕"), Some((x, y)), "collapsed: the strip stays");
        let (x, y) = h.find("■").expect("explorer icon");
        h.click(x, y);
        assert_eq!((state(&h).view, state(&h).open), (0, true));
        assert_eq!(last_log(&h), "opened on explorer");
    }

    #[test]
    fn hide_closes_to_an_edge_and_reopens_on_the_last_view() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.click_text("Hide");
        assert_eq!(last_log(&h), "closing = hide");
        let (x, y) = h.find("⌕").expect("search icon");
        h.click(x, y);
        h.send(send(Msg::Toggle(false)));
        assert!(h.find("⌕").is_none() && h.find("■").is_none(), "hidden: no strip:\n{}", h.screen());
        h.send(send(Msg::Query("showcase".to_owned())));
        h.send(send(Msg::Toggle(true)));
        assert_eq!(state(&h).view, 1);
        assert!(h.screen().contains("4 results in 4 files"), "{}", h.screen());
    }

    #[test]
    fn limit_choices_bound_the_width_and_are_logged() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        assert_eq!(state(&h).limits, 0, "no upper limit unless the app sets one");
        h.send(send(Msg::Resize(70)));
        assert_eq!(state(&h).width, 70);
        h.click_text("30–60");
        assert_eq!((state(&h).limits, state(&h).width), (2, 60));
        assert_eq!(last_log(&h), "limits = 30–60, width = 60");
        h.click_text("18–48");
        assert_eq!(state(&h).width, 48);
        h.click_text("Unlimited");
        assert_eq!(state(&h).width, 48, "lifting the limit keeps the width");
    }

    #[test]
    fn the_edge_toggle_answers_the_pointer() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("EXPLORER").expect("panel content");
        // The panel is 30 columns wide and 16 rows tall: its edge is its last column.
        let edge = x - 1 + 29;
        h.hover(edge, y);
        let middle = y - 1 + 8;
        let button_cells = |h: &Showcase| -> String {
            let screen = h.screen();
            let row = screen.lines().nth(usize::try_from(middle).unwrap_or(0)).unwrap_or_default();
            row.chars().skip(usize::try_from(edge).unwrap_or(0)).take(2).collect()
        };
        assert_eq!(button_cells(&h), " ‹", "the lit edge raises the toggle without its pillar:\n{}", h.screen());
        h.hover(edge, middle);
        assert_eq!(button_cells(&h), "▌‹", "on the toggle the pillar appears:\n{}", h.screen());
        h.click(edge + 1, middle);
        assert!(!state(&h).open);
        assert_eq!(last_log(&h), "closed");
    }
}
