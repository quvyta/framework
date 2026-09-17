//! Tab rail: vertical project tabs with icons, badges, a collapsed mode, closing, reordering, tall
//! blocks, markers of the collapsed strip and a right-click menu.

use qframe::prelude::*;
use qframe::widgets::{CollapsedMarker, ContextItem, NumberInput, RailTab, Segmented, TabEdit, TabRail};

use super::tab_menu::{self, OpenTab, TabAction};
use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "tab-rail";

/// Demo projects: name, icon, status colour and key, running containers.
const PROJECTS: [(&str, &str, &str, &str, u32); 9] = [
    ("quvyta", "folder", "success", "running", 3),
    ("qcode", "folder", "warning", "paused", 1),
    ("homelab", "folder", "success", "running", 7),
    ("website", "file", "danger", "failed", 0),
    ("dotfiles", "file", "muted", "stopped", 0),
    ("notes", "file", "muted", "stopped", 0),
    ("registry", "folder", "success", "running", 2),
    ("backups", "file", "warning", "paused", 1),
    ("monitoring", "folder", "success", "running", 4),
];

/// The row heights the playground offers, in lines.
const ROW_HEIGHT: (u16, u16) = (1, 9);

/// The gaps the playground offers, in lines.
const GAP: (u16, u16) = (0, 5);

/// The rail-wide markers of the playground, with their locale keys.
const RAIL_MARKERS: [(CollapsedMarker, &str); 3] = [
    (CollapsedMarker::Icon, "tab-rail.marker-icon"),
    (CollapsedMarker::Initial, "tab-rail.marker-letter"),
    (CollapsedMarker::Number, "tab-rail.marker-number"),
];

/// The icons a project can pick in its tab menu, with their locale keys.
const MARKER_ICONS: [(&str, &str); 3] =
    [("folder", "tab-rail.marker-folder"), ("file", "tab-rail.marker-file"), ("inbox", "tab-rail.marker-inbox")];

/// The marker a project picked from its tab menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// An icon key of the icon set, shown in the strip and before the name.
    Icon(&'static str),
    Letter,
    Number,
}

impl Marker {
    /// The event log word of the marker.
    fn describe(self) -> String {
        match self {
            Self::Icon(key) => format!("{key} icon"),
            Self::Letter => "letter".to_owned(),
            Self::Number => "number".to_owned(),
        }
    }
}

/// Width of the playground's number fields: a digit, the prompt and both steppers.
const FIELD: Length = Length::Cells(16);

/// Open projects and the options the playground turned on.
#[derive(Debug)]
pub struct State {
    projects: Vec<OpenTab>,
    active: usize,
    collapsed: bool,
    badges: bool,
    closable: bool,
    reorderable: bool,
    add: bool,
    row_height: u16,
    /// The gap set in the playground; `None` leaves the rail's own default.
    gap: Option<u16>,
    menu: bool,
    /// Index into [`RAIL_MARKERS`]: what the collapsed strip shows for projects without their own.
    rail_marker: usize,
    /// The marker each project picked in its tab menu, by project; `None` follows the rail.
    markers: [Option<Marker>; PROJECTS.len()],
}

impl Default for State {
    fn default() -> Self {
        Self {
            projects: OpenTab::all(PROJECTS.len()),
            active: 0,
            collapsed: false,
            badges: true,
            closable: false,
            reorderable: false,
            add: false,
            row_height: 1,
            gap: None,
            menu: false,
            rail_marker: 0,
            markers: [None; PROJECTS.len()],
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Open(usize),
    Edit(TabEdit),
    Menu(TabAction),
    Reopen,
    Collapsed(bool),
    Badges(bool),
    Closable(bool),
    Reorderable(bool),
    AddRow(bool),
    RowHeight(u16),
    /// A gap in lines, or `None` for the rail's default.
    Gap(Option<u16>),
    ContextMenu(bool),
    /// A dragged tab scrolled the rail; the first row now in view.
    DragScroll(usize),
    RailMarker(usize),
    /// The marker for the project at a tab, or `None` to follow the rail again.
    Marker(usize, Option<Marker>),
    Add,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::TabRail(message))
}

/// The gap the rail draws: the one set, or the rail's default of one line between tall rows.
fn shown_gap(state: &State) -> u16 {
    state.gap.unwrap_or(u16::from(state.row_height > 1))
}

/// A number field's value as whole lines within `min..=max`.
fn lines(value: f64, (min, max): (u16, u16)) -> u16 {
    (min..=max).find(|line| f64::from(*line) >= value.round()).unwrap_or(max)
}

/// The name of the open project at tab `index`.
fn name(state: &State, index: usize) -> &'static str {
    state.projects.get(index).map_or("", |tab| PROJECTS[tab.item].0)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Open(index) => {
            state.active = index;
            log.push(PAGE, "TabRail#projects", format!("selected {}", name(state, index)));
        }
        Msg::Edit(edit) => {
            let entry = match edit {
                TabEdit::Close(index) => format!("close {}", name(state, index)),
                TabEdit::Move { from, to } => format!("move {} from {from} to {to}", name(state, from)),
            };
            edit.apply(&mut state.projects, &mut state.active);
            log.push(PAGE, "TabRail#projects", entry);
        }
        Msg::Menu(action) => {
            let entry = action.describe(&state.projects, name(state, action.index()));
            tab_menu::apply(action, &mut state.projects, &mut state.active);
            log.push(PAGE, "TabRail#projects", entry);
        }
        Msg::Reopen => {
            let active = state.projects.get(state.active).map_or(0, |tab| tab.item);
            state.projects = OpenTab::all(PROJECTS.len());
            state.active = active;
            log.push(PAGE, "Button#reopen", "reopened every project");
        }
        Msg::Collapsed(on) => {
            state.collapsed = on;
            log.push(PAGE, "Playground", format!("collapsed = {on}"));
        }
        Msg::Badges(on) => {
            state.badges = on;
            log.push(PAGE, "Playground", format!("badges = {on}"));
        }
        Msg::Closable(on) => {
            state.closable = on;
            log.push(PAGE, "Playground", format!("closable = {on}"));
        }
        Msg::Reorderable(on) => {
            state.reorderable = on;
            log.push(PAGE, "Playground", format!("reorderable = {on}"));
        }
        Msg::AddRow(on) => {
            state.add = on;
            log.push(PAGE, "Playground", format!("add row = {on}"));
        }
        Msg::RowHeight(lines) => {
            state.row_height = lines;
            log.push(PAGE, "Playground", format!("row_height = {lines}"));
        }
        Msg::Gap(gap) => {
            state.gap = gap;
            let entry = match gap {
                Some(lines) => format!("gap = {lines}"),
                None => format!("gap = auto ({})", shown_gap(state)),
            };
            log.push(PAGE, "Playground", entry);
        }
        Msg::ContextMenu(on) => {
            state.menu = on;
            log.push(PAGE, "Playground", format!("context menu = {on}"));
        }
        Msg::DragScroll(first) => {
            let shown = if first < state.projects.len() { name(state, first) } else { "the add row" };
            log.push(PAGE, "TabRail#projects", format!("drag scroll: {shown} first in view"));
        }
        Msg::RailMarker(index) => {
            state.rail_marker = index;
            log.push(PAGE, "Playground", format!("collapsed_marker = {:?}", RAIL_MARKERS[index].0));
        }
        Msg::Marker(index, marker) => {
            if let Some(tab) = state.projects.get(index) {
                state.markers[tab.item] = marker;
                let shown = marker.map_or_else(|| "the rail's".to_owned(), Marker::describe);
                log.push(PAGE, "TabRail#projects", format!("menu: {} marker = {shown}", name(state, index)));
            }
        }
        // region: tab-rail-add
        Msg::Add => match (0..PROJECTS.len()).find(|project| !state.projects.iter().any(|tab| tab.item == *project)) {
            Some(project) => {
                state.projects.push(OpenTab { item: project, pinned: false });
                state.active = state.projects.len() - 1;
                log.push(PAGE, "TabRail#projects", format!("added {}", PROJECTS[project].0));
            }
            None => log.push(PAGE, "TabRail#projects", "add: every project is already open"),
        },
        // endregion
    }
    Command::none()
}

// region: tab-rail-marker
/// `tab` with the project's `icon` and its own `marker`, if it picked one: an icon replaces the
/// project's icon, a letter or a number becomes the tab's collapsed marker.
fn with_marker(tab: RailTab, icon: &str, marker: Option<Marker>) -> RailTab {
    match marker {
        None => tab.icon(icon),
        Some(Marker::Icon(key)) => tab.icon(key).marker(CollapsedMarker::Icon),
        Some(Marker::Letter) => tab.icon(icon).marker(CollapsedMarker::Initial),
        Some(Marker::Number) => tab.icon(icon).marker(CollapsedMarker::Number),
    }
}

/// The Marker submenu of the tab at `index`, whose project picked `current`; the entry already in
/// use is disabled.
fn marker_menu(index: usize, current: Option<Marker>) -> ContextItem<AppMsg> {
    let entry = |key: &str, marker: Option<Marker>| {
        ContextItem::new(t!(key), send(Msg::Marker(index, marker))).disabled(current == marker)
    };
    let icons = MARKER_ICONS.map(|(icon, key)| entry(key, Some(Marker::Icon(icon))).icon(icon));
    let items = icons.into_iter().chain([
        ContextItem::gap(),
        entry("tab-rail.marker-letter", Some(Marker::Letter)),
        entry("tab-rail.marker-number", Some(Marker::Number)),
        ContextItem::gap(),
        entry("tab-rail.marker-rail", None),
    ]);
    ContextItem::submenu(t!("tab-rail.marker"), items)
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("tab-rail.hint")).role("secondary"));
        ui.row(|ui| {
            // region: tab-rail-tabs
            let tabs = state.projects.iter().map(|tab| {
                let (name, icon, color, _, running) = PROJECTS[tab.item];
                let tab = with_marker(RailTab::new(name), icon, state.markers[tab.item]);
                if state.badges && running > 0 { tab.status(color).badge(running.to_string()) } else { tab }
            });
            // endregion
            // region: tab-rail-options
            let mut rail = TabRail::new(tabs)
                .active(state.active)
                .collapsed(state.collapsed)
                .collapsed_marker(RAIL_MARKERS[state.rail_marker].0)
                .row_height(state.row_height)
                .pinned(tab_menu::pinned(&state.projects))
                .on_select(|index| send(Msg::Open(index)));
            if let Some(gap) = state.gap {
                rail = rail.gap(gap);
            }
            if state.closable {
                rail = rail.closable(|index| send(Msg::Edit(TabEdit::Close(index))));
            }
            if state.reorderable {
                rail = rail
                    .reorderable(|from, to| send(Msg::Edit(TabEdit::Move { from, to })))
                    .on_drag_scroll(|first| send(Msg::DragScroll(first)));
            }
            if state.add {
                rail = rail.on_add(|| send(Msg::Add));
            }
            // endregion
            // region: tab-rail-menu
            if state.menu {
                let (projects, markers) = (state.projects.clone(), state.markers);
                rail = rail.context_menu(move |index| {
                    let mut items = tab_menu::items(&projects, index, |action| send(Msg::Menu(action)));
                    let current = projects.get(index).and_then(|tab| markers[tab.item]);
                    items.extend([ContextItem::gap(), marker_menu(index, current)]);
                    items
                });
            }
            // endregion
            let width = if state.collapsed { 4 } else { 24 };
            // Room for a few tabs, and always for two whole blocks with the gap between them.
            let few = if state.row_height > 1 { 16 } else { 8 };
            let height = few.max(state.row_height * 2 + shown_gap(state));
            ui.add(rail).width(Length::Cells(width)).height(Length::Cells(height)).id("projects");
            ui.column(|ui| match state.projects.get(state.active) {
                Some(tab) => {
                    let (name, _, color, status, running) = PROJECTS[tab.item];
                    ui.add(Text::new(name).role("title"));
                    ui.add(Text::new(t!(&format!("tab-rail.{status}"))).color(color));
                    ui.add(Text::new(t!("tab-rail.containers", n = running)).role("secondary"));
                }
                None => {
                    ui.add(Text::new(t!("tab-rail.empty")).role("faint"));
                }
            })
            .padding(Padding::symmetric(0, 3))
            .fill_width();
        })
        .fill_width();
        ui.add(Button::new(t!("tab-rail.reopen")).on_press(send(Msg::Reopen))).id("reopen");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("tab-rail.add"), |ui| {
            ui.add(toggle(state.add, |on| send(Msg::AddRow(on)))).id("add");
        });
        setting(ui, t!("tab-rail.collapsed"), |ui| {
            ui.add(toggle(state.collapsed, |on| send(Msg::Collapsed(on)))).id("collapsed");
        });
        setting(ui, t!("tab-rail.badges"), |ui| {
            ui.add(toggle(state.badges, |on| send(Msg::Badges(on)))).id("badges");
        });
        setting(ui, t!("tab-rail.closable"), |ui| {
            ui.add(toggle(state.closable, |on| send(Msg::Closable(on)))).id("closable");
        });
        setting(ui, t!("tab-rail.reorderable"), |ui| {
            ui.add(toggle(state.reorderable, |on| send(Msg::Reorderable(on)))).id("reorderable");
        });
        // region: tab-rail-thickness
        setting(ui, t!("tab-rail.row-height"), |ui| {
            let field = NumberInput::new(f64::from(state.row_height))
                .range(f64::from(ROW_HEIGHT.0), f64::from(ROW_HEIGHT.1))
                .steppers(true)
                .on_change(|value| send(Msg::RowHeight(lines(value, ROW_HEIGHT))));
            ui.add(field).width(FIELD).id("row-height");
        });
        // Until a gap is set the rail picks it: the switch says so, and the field shows the gap the
        // rail draws, read-only until the switch is turned off.
        setting(ui, t!("tab-rail.gap-auto"), |ui| {
            let shown = shown_gap(state);
            ui.add(toggle(state.gap.is_none(), move |auto| send(Msg::Gap((!auto).then_some(shown))))).id("gap-auto");
        });
        setting(ui, t!("tab-rail.gap"), |ui| {
            let field = NumberInput::new(f64::from(shown_gap(state)))
                .range(f64::from(GAP.0), f64::from(GAP.1))
                .steppers(true)
                .disabled(state.gap.is_none())
                .on_change(|value| send(Msg::Gap(Some(lines(value, GAP)))));
            ui.add(field).width(FIELD).id("gap");
        });
        // endregion
        setting(ui, t!("tab-rail.collapsed-marker"), |ui| {
            let names = RAIL_MARKERS.map(|(_, key)| t!(key));
            ui.add(Segmented::new(names).selected(state.rail_marker).on_select(|i| send(Msg::RailMarker(i))))
                .id("collapsed-marker");
        });
        setting(ui, t!("tab-rail.menu"), |ui| {
            ui.add(toggle(state.menu, |on| send(Msg::ContextMenu(on)))).id("menu");
        });
        ui.add(Text::new(t!("tab-rail.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;
    use qframe::event::{MouseButton, MouseKind};

    /// Whether the event log of the page has an entry `message`; the log panel may be scrolled
    /// out of view on a tall page.
    fn logged(h: &qframe::runtime::Harness<crate::app::Showcase>, message: &str) -> bool {
        h.app().log.recent(PAGE, 20).iter().any(|entry| entry.message == message)
    }

    fn items(h: &qframe::runtime::Harness<crate::app::Showcase>) -> Vec<usize> {
        h.app().pages.tab_rail.projects.iter().map(|tab| tab.item).collect()
    }

    #[test]
    fn opens_collapses_closes_and_moves_projects() {
        let mut h = showcase_on(PAGE);
        h.click_text("homelab");
        assert_eq!(h.app().pages.tab_rail.active, 2);
        assert!(h.screen().contains("7 running containers"), "{}", h.screen());
        h.send(send(Msg::Collapsed(true)));
        assert!(!h.screen().contains("dotfiles"), "{}", h.screen());
        h.send(send(Msg::Closable(true)));
        h.send(send(Msg::Edit(TabEdit::Close(0))));
        assert_eq!(h.app().pages.tab_rail.active, 1);
        h.send(send(Msg::Edit(TabEdit::Move { from: 1, to: 0 })));
        assert_eq!(h.app().pages.tab_rail.projects[0].item, 2);
        assert_eq!(h.app().pages.tab_rail.active, 0);
    }

    #[test]
    fn a_collapsed_rail_opens_and_closes_projects_from_the_name_card() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Collapsed(true))).send(send(Msg::Closable(true)));
        let (title, third) = h.find("3 running containers").expect("the open project's details");
        let top = third - 2;
        // The rail is four cells wide and the project column after it keeps three cells of padding.
        let rail = title - 3 - 4;
        let row = |h: &qframe::runtime::Harness<crate::app::Showcase>, y: i32| {
            h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned()
        };
        h.hover(rail + 1, top + 2);
        assert!(row(&h, top + 2).contains("homelab  7"), "hovering a tab shows its card:\n{}", h.screen());
        h.hover(rail + 6, top + 2).click(rail + 6, top + 2);
        assert_eq!(h.app().pages.tab_rail.active, 2, "a click on the card opens the project");
        assert!(logged(&h, "selected homelab"));
        let line = row(&h, top + 2);
        let mark = line.chars().position(|c| c == '×').expect("the card's close mark");
        h.click(i32::try_from(mark).expect("on screen"), top + 2);
        assert!(logged(&h, "close homelab"));
        assert_eq!(h.app().pages.tab_rail.projects.len(), PROJECTS.len() - 1);
    }

    #[test]
    fn tall_rows_are_a_line_apart_until_the_playground_picks_a_gap() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::RowHeight(3)));
        let (_, y) = h.find("qcode").expect("the tall rail");
        let line = |h: &qframe::runtime::Harness<crate::app::Showcase>, y: i32| {
            h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned()
        };
        assert!(line(&h, y - 4).contains("quvyta"), "one line apart by default:\n{}", h.screen());
        h.send(send(Msg::Gap(Some(0))));
        assert!(logged(&h, "gap = 0"));
        let (_, y) = h.find("qcode").expect("the stacked rail");
        assert!(line(&h, y - 3).contains("quvyta"), "gap(0) stacks the blocks:\n{}", h.screen());
    }

    #[test]
    fn number_fields_set_any_row_height_and_gap_and_the_gap_can_go_back_to_auto() {
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::new(), PAGE, 120);
        h.set_reduced_motion(true);
        let field = |h: &qframe::runtime::Harness<crate::app::Showcase>, label: &str| {
            let (x, y) = h.find(label).expect("a playground row");
            let line = h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned();
            (x + 27, y, line)
        };
        let (x, y, line) = field(&h, "Gap between rows");
        assert!(line.contains(" 0 "), "the automatic gap of one-line rows shows as 0:\n{line}");
        h.click(x, y).press("up");
        assert_eq!(h.app().pages.tab_rail.gap, None, "the field is read-only while the gap is automatic");

        let (x, y, _) = field(&h, "Row height");
        h.click(x, y).press("end").press("backspace").type_text("9");
        assert_eq!(h.app().pages.tab_rail.row_height, 9);
        assert!(logged(&h, "row_height = 9"));
        let (_, _, line) = field(&h, "Gap between rows");
        assert!(line.contains(" 1 "), "tall rows get the rail's gap of one line:\n{line}");
        h.press("up");
        assert_eq!(h.app().pages.tab_rail.row_height, 9, "nine is the most the field offers");

        let (x, y, _) = field(&h, "Automatic gap");
        h.click(x - 3, y);
        assert_eq!(h.app().pages.tab_rail.gap, Some(1), "turning automatic off keeps the gap that showed");
        assert!(logged(&h, "gap = 1"));
        let (x, y, _) = field(&h, "Gap between rows");
        h.click(x, y).press("pgup");
        assert_eq!(h.app().pages.tab_rail.gap, Some(5), "and the field takes up to five lines");
        let (_, next) = h.find("qcode").expect("the second tab");
        let above = h.screen().lines().nth(usize::try_from(next - 9 - 5).unwrap_or(0)).unwrap_or_default().to_owned();
        assert!(above.contains("quvyta"), "blocks nine lines tall, five apart:\n{}", h.screen());

        let (x, y, _) = field(&h, "Automatic gap");
        h.click(x - 3, y);
        assert_eq!(h.app().pages.tab_rail.gap, None);
        assert!(logged(&h, "gap = auto (1)"));
    }

    #[test]
    fn the_marker_submenu_changes_one_projects_marker_and_the_playground_sets_the_rail_default() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        h.send(send(Msg::Collapsed(true))).send(send(Msg::ContextMenu(true)));
        let (title, third) = h.find("3 running containers").expect("the open project's details");
        let (rail, top) = (title - 3 - 4, third - 2);
        let strip = |h: &qframe::runtime::Harness<crate::app::Showcase>| -> String {
            let lines: Vec<String> = h.screen().lines().map(str::to_owned).collect();
            (0..6)
                .map(|row| {
                    lines[usize::try_from(top + row).unwrap_or(0)]
                        .chars()
                        .nth(usize::try_from(rail + 1).unwrap_or(0))
                        .unwrap_or(' ')
                })
                .collect()
        };
        assert_eq!(strip(&h), "■■■▪▪▪", "projects start with their own icons:\n{}", h.screen());

        crate::tests::right_click(&mut h, rail + 1, top + 1);
        crate::tests::click_text_below(&mut h, "Marker", top);
        crate::tests::click_text_below(&mut h, "Number", top);
        assert_eq!(strip(&h), "■2■▪▪▪", "only qcode shows its position:\n{}", h.screen());
        assert!(logged(&h, "menu: qcode marker = number"));

        crate::tests::right_click(&mut h, rail + 1, top + 3);
        crate::tests::click_text_below(&mut h, "Marker", top);
        crate::tests::click_text_below(&mut h, "Inbox", top);
        assert_eq!(strip(&h), "■2■◌▪▪", "website picks the inbox icon:\n{}", h.screen());
        assert!(logged(&h, "menu: website marker = inbox icon"));

        h.send(send(Msg::RailMarker(1)));
        assert!(logged(&h, "collapsed_marker = Initial"));
        assert_eq!(
            strip(&h),
            "Q2H◌DN",
            "the rest follow the rail's letters; tabs with their own marker keep it:\n{}",
            h.screen()
        );

        h.send(send(Msg::Edit(TabEdit::Move { from: 1, to: 4 })));
        assert_eq!(strip(&h), "QH◌D5N", "a number follows its tab to its new position");

        crate::tests::right_click(&mut h, rail + 1, top + 4);
        crate::tests::click_text_below(&mut h, "Marker", top);
        crate::tests::click_text_below(&mut h, "Same as the rail", top);
        assert_eq!(strip(&h), "QH◌DQN", "and back to the rail's marker");
        assert!(logged(&h, "menu: qcode marker = the rail's"));
    }

    #[test]
    fn a_dragged_project_held_on_the_last_row_scrolls_the_rail_and_logs_each_step() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Reorderable(true)));
        let (x, top) = h.find("■ quvyta").expect("the rail");
        let last = top + 7;
        assert!(
            h.screen().lines().nth(usize::try_from(last).unwrap_or(0)).is_some_and(|line| line.contains("backups")),
            "{}",
            h.screen()
        );
        h.mouse(MouseKind::Down(MouseButton::Left), x, top);
        h.mouse(MouseKind::Drag(MouseButton::Left), x, last);
        h.advance(std::time::Duration::from_millis(399));
        assert!(!logged(&h, "drag scroll: qcode first in view"), "nothing before the delay");
        h.advance(std::time::Duration::from_millis(1));
        assert!(logged(&h, "drag scroll: qcode first in view"), "one row at the delay");
        h.mouse(MouseKind::Up(MouseButton::Left), x, last);
        assert_eq!(items(&h), vec![1, 2, 3, 4, 5, 6, 7, 8, 0], "dropped on the last row, it lands last");
        assert!(logged(&h, "move quvyta from 0 to 8"));
    }

    #[test]
    fn tall_blocks_and_the_right_click_menu_run_real_actions() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("qcode").expect("the rail");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y);
        assert!(!h.screen().contains("Close others"), "no menu until it is turned on:\n{}", h.screen());

        h.send(send(Msg::RowHeight(3))).send(send(Msg::Gap(Some(1)))).send(send(Msg::ContextMenu(true)));
        assert!(logged(&h, "row_height = 3"));
        let (x, y) = h.find("qcode").expect("the tall rail");
        let line = |h: &qframe::runtime::Harness<crate::app::Showcase>, y: i32| {
            h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned()
        };
        assert!(line(&h, y - 4).contains("quvyta"), "three lines and a gap apart:\n{}", h.screen());

        h.mouse(MouseKind::Down(MouseButton::Right), x, y - 1).mouse(MouseKind::Up(MouseButton::Right), x, y - 1);
        h.click_text("Duplicate");
        assert_eq!(items(&h), vec![0, 1, 1, 2, 3, 4, 5, 6, 7, 8], "a click above the name still hits the block");
        assert!(logged(&h, "menu: duplicate qcode"));

        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        h.click_text("Pin");
        assert!(h.app().pages.tab_rail.projects[1].pinned);
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        assert!(h.screen().contains("Unpin"), "{}", h.screen());
        h.click_text("Close others");
        assert_eq!(items(&h), vec![1], "pinned or not, the menu's own tab stays");
        assert_eq!(h.app().pages.tab_rail.active, 0);
    }
}
