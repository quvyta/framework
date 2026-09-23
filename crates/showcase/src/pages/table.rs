//! Table: containers with sortable columns, multiple selection, sideways scrolling and a very
//! long build history.

use std::cmp::Ordering;
use std::sync::Arc;

use qframe::icons::{Glyph, GlyphMode};
use qframe::prelude::*;
use qframe::widgets::{Column, ColumnWidth, ContextItem, Select, SortDirection, Table, TableCell, TableRow};

use super::{PageMsg, setting, slide_setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "table";

/// A running container: name, the Nerd Font glyph of its program, status, CPU per mille, memory
/// in MiB, image, port.
#[derive(Debug, Clone, Copy)]
struct Container {
    name: &'static str,
    nerd: char,
    status: &'static str,
    cpu: u32,
    memory: u32,
    image: &'static str,
    port: &'static str,
}

const CONTAINERS: [Container; 8] = [
    Container {
        name: "quvyta-api",
        nerd: '\u{f109b}',
        status: "running",
        cpu: 124,
        memory: 512,
        image: "quvyta/api:2.4",
        port: "8080/tcp",
    },
    Container {
        name: "postgres",
        nerd: '\u{e76e}',
        status: "running",
        cpu: 38,
        memory: 1024,
        image: "postgres:16",
        port: "5432/tcp",
    },
    Container {
        name: "cache",
        nerd: '\u{e76d}',
        status: "running",
        cpu: 9,
        memory: 256,
        image: "valkey:8",
        port: "6379/tcp",
    },
    Container {
        name: "worker-emails",
        nerd: '\u{f01ee}',
        status: "paused",
        cpu: 0,
        memory: 96,
        image: "quvyta/worker:2.4",
        port: "none",
    },
    Container {
        name: "docs-preview",
        nerd: '\u{f059f}',
        status: "running",
        cpu: 21,
        memory: 128,
        image: "caddy:2",
        port: "3000/tcp",
    },
    Container {
        name: "search-index",
        nerd: '\u{f0349}',
        status: "failed",
        cpu: 0,
        memory: 0,
        image: "meilisearch:1.9",
        port: "7700/tcp",
    },
    Container {
        name: "nightly-tests",
        nerd: '\u{f0668}',
        status: "stopped",
        cpu: 0,
        memory: 0,
        image: "quvyta/ci:2.4",
        port: "none",
    },
    Container {
        name: "metrics",
        nerd: '\u{f0238}',
        status: "running",
        cpu: 57,
        memory: 384,
        image: "prometheus:3",
        port: "9090/tcp",
    },
];

/// How many builds the long table shows.
const BUILDS: usize = 100_000;

/// Contents the playground offers.
const CONTENTS: [&str; 3] = ["containers", "builds", "nothing"];

/// Rows, sort order, selection and playground settings.
#[derive(Debug)]
pub struct State {
    containers: Vec<Container>,
    builds: Arc<[TableRow]>,
    /// The name in each build row, so a row's menu can name the row it opens on without the
    /// table handing its cells back.
    build_names: Arc<[String]>,
    build_numbers: Vec<usize>,
    sort: Option<(usize, SortDirection)>,
    selected: Option<usize>,
    checked: Vec<bool>,
    contents: usize,
    multi: bool,
    sortable: bool,
    narrow: bool,
    icons: bool,
    /// Whether every row carries a menu of its own.
    menu: bool,
    /// Whether Enter and a click open a row's menu instead of the row.
    menu_on_activate: bool,
    /// What the row menu was last asked for, empty before it is used.
    asked: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            containers: CONTAINERS.to_vec(),
            builds: Arc::from(Vec::new()),
            build_names: Arc::from(Vec::new()),
            build_numbers: Vec::new(),
            sort: None,
            selected: Some(0),
            checked: vec![false; CONTAINERS.len()],
            contents: 0,
            multi: false,
            sortable: true,
            narrow: false,
            icons: true,
            menu: true,
            menu_on_activate: false,
            asked: String::new(),
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Select(usize),
    Activate(usize),
    Toggle(usize),
    Sort(usize, SortDirection),
    Contents(usize),
    Multi(bool),
    Sortable(bool),
    Narrow(bool),
    Icons(bool),
    Menu(bool),
    MenuOnActivate(bool),
    /// A row's own menu was used on the row of this index and name, for this action.
    RowAction(usize, String, &'static str),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Table(message))
}

/// Build rows are plain numbers and times, so they are made once and shared with the table.
fn build_rows(numbers: &[usize]) -> Arc<[TableRow]> {
    numbers
        .iter()
        .map(|n| {
            let seconds = 40 + (n * 37) % 380;
            TableRow::new([
                format!("#{n}"),
                format!("{}:{:02}", seconds / 60, seconds % 60),
                format!("{}", 120 + n % 90),
            ])
        })
        .collect()
}

/// The name each build row shows, made with the rows and kept beside them.
fn build_names(numbers: &[usize]) -> Arc<[String]> {
    numbers.iter().map(|n| format!("#{n}")).collect()
}

// region: table-sorting
fn sort_containers(containers: &mut [Container], column: usize, direction: SortDirection) {
    containers.sort_by(|a, b| {
        let order = match column {
            0 => a.name.cmp(b.name),
            2 => a.cpu.cmp(&b.cpu),
            3 => a.memory.cmp(&b.memory),
            _ => Ordering::Equal,
        };
        if direction == SortDirection::Descending { order.reverse() } else { order }
    });
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Select(index) => {
            state.selected = Some(index);
            log.push(PAGE, "Table#rows", format!("selected {index}"));
        }
        Msg::Activate(index) => log.push(PAGE, "Table#rows", format!("activated {index}")),
        Msg::Toggle(index) => {
            if let Some(checked) = state.checked.get_mut(index) {
                *checked = !*checked;
            }
            log.push(PAGE, "Table#rows", format!("toggled {index}"));
        }
        Msg::Sort(column, direction) => {
            state.sort = Some((column, direction));
            if state.contents == 1 {
                state.build_numbers.sort_by(|a, b| {
                    let order = a.cmp(b);
                    if direction == SortDirection::Descending { order.reverse() } else { order }
                });
                state.builds = build_rows(&state.build_numbers);
                state.build_names = build_names(&state.build_numbers);
            } else {
                sort_containers(&mut state.containers, column, direction);
            }
            log.push(PAGE, "Table#rows", format!("sort column {column} {direction:?}"));
        }
        Msg::Contents(index) => {
            state.contents = index;
            state.selected = Some(0);
            state.sort = None;
            if index == 1 && state.build_numbers.is_empty() {
                state.build_numbers = (1..=BUILDS).rev().collect();
                state.builds = build_rows(&state.build_numbers);
                state.build_names = build_names(&state.build_numbers);
            }
            log.push(PAGE, "Playground", format!("contents = {}", CONTENTS[index]));
        }
        Msg::Multi(on) => {
            state.multi = on;
            log.push(PAGE, "Playground", format!("multi = {on}"));
        }
        Msg::Sortable(on) => {
            state.sortable = on;
            log.push(PAGE, "Playground", format!("sortable = {on}"));
        }
        Msg::Narrow(on) => {
            state.narrow = on;
            log.push(PAGE, "Playground", format!("narrow = {on}"));
        }
        Msg::Icons(on) => {
            state.icons = on;
            log.push(PAGE, "Playground", format!("icons = {on}"));
        }
        Msg::Menu(on) => {
            state.menu = on;
            state.asked.clear();
            log.push(PAGE, "Playground", format!("row menu = {on}"));
        }
        Msg::MenuOnActivate(on) => {
            state.menu_on_activate = on;
            log.push(PAGE, "Playground", format!("menu on activate = {on}"));
        }
        // region: table-menu-update
        Msg::RowAction(index, name, action) => {
            // The menu says which row it was opened on, so the action never lands on the row the
            // cursor happens to rest on.
            state.asked = t!(&format!("table.menu-{action}-done"), name = name.as_str());
            log.push(PAGE, "Table#rows", format!("{action} on row {index}"));
        } // endregion
    }
    Command::none()
}

// region: table-icons
/// The glyph before a container's name: its program's own glyph where the terminal has a Nerd
/// Font, and the icon set's project icon in the other glyph modes.
fn program_glyph(container: &Container, mode: GlyphMode) -> Glyph {
    if mode == GlyphMode::Nerd { Glyph::literal(container.nerd) } else { Glyph::key("project") }
}
// endregion

// region: table-menu
/// What the menu of row `index` offers. It is built when the menu opens, for that row alone, so a
/// hundred thousand rows cost a hundred thousand nothing.
fn row_menu(name: &str, index: usize) -> Vec<ContextItem<AppMsg>> {
    vec![
        ContextItem::new(
            t!("table.menu-restart", name = name),
            send(Msg::RowAction(index, name.to_owned(), "restart")),
        ),
        ContextItem::gap(),
        ContextItem::new(t!("table.menu-stop", name = name), send(Msg::RowAction(index, name.to_owned(), "stop")))
            .danger(true),
    ]
}
// endregion

// region: table-rows
fn container_rows(containers: &[Container], icons: Option<GlyphMode>) -> Vec<TableRow> {
    containers
        .iter()
        .map(|c| {
            let mut name = TableCell::new(c.name);
            if let Some(mode) = icons {
                name = name.icon(program_glyph(c, mode), None);
            }
            let tone = match c.status {
                "running" => "success",
                "paused" => "warning",
                "failed" => "danger",
                _ => "muted",
            };
            let status = TableCell::new(t!(&format!("table.status.{}", c.status))).icon("dot", Some(tone));
            let cpu = format!("{}.{}%", c.cpu / 10, c.cpu % 10);
            TableRow::new([
                name,
                status,
                TableCell::new(cpu),
                TableCell::new(format!("{} MiB", c.memory)),
                TableCell::new(c.image),
                TableCell::new(c.port),
            ])
            .faint(c.status == "stopped")
        })
        .collect()
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        let sortable = state.sortable;
        let (columns, rows): (Vec<Column>, Arc<[TableRow]>) = match state.contents {
            // region: table-columns
            0 => (
                vec![
                    Column::new(t!("table.name")).min(12).sortable(sortable),
                    Column::new(t!("table.status")).width(ColumnWidth::Fit),
                    Column::new(t!("table.cpu")).width(ColumnWidth::Fit).align(Align::End).sortable(sortable),
                    Column::new(t!("table.memory")).width(ColumnWidth::Fit).align(Align::End).sortable(sortable),
                    Column::new(t!("table.image")).min(16),
                    Column::new(t!("table.port")).width(ColumnWidth::Fixed(9)),
                ],
                container_rows(&state.containers, state.icons.then(|| ui.env().glyph_mode())).into(),
            ),
            // endregion
            1 => (
                vec![
                    Column::new(t!("table.build")).sortable(sortable),
                    Column::new(t!("table.duration")).width(ColumnWidth::Fixed(10)).align(Align::End),
                    Column::new(t!("table.tests")).width(ColumnWidth::Fixed(8)).align(Align::End),
                ],
                Arc::clone(&state.builds),
            ),
            _ => (vec![Column::new(t!("table.name")), Column::new(t!("table.status"))], Arc::from(Vec::new())),
        };
        // region: table-options
        let names: Arc<[String]> = match state.contents {
            0 => state.containers.iter().map(|c| c.name.to_owned()).collect(),
            1 => Arc::clone(&state.build_names),
            _ => Arc::from(Vec::new()),
        };
        let mut table = Table::new(columns, rows)
            .selected(state.selected)
            .empty_text(t!("table.empty"))
            .on_select(|index| send(Msg::Select(index)))
            .on_activate(|index| send(Msg::Activate(index)));
        if state.sortable {
            table = table.on_sort(|column, direction| send(Msg::Sort(column, direction)));
            if let Some((column, direction)) = state.sort {
                table = table.sort(column, direction);
            }
        }
        if state.multi && state.contents == 0 {
            table = table.checked(state.checked.clone()).on_toggle(|index| send(Msg::Toggle(index)));
        }
        if state.menu {
            // The rows are already shared, so the menu reads the name of the row it opens on
            // rather than being given every row's menu in advance.
            table = table
                .context_menu(move |index| match names.get(index) {
                    Some(name) => row_menu(name, index),
                    None => Vec::new(),
                })
                .menu_on_activate(state.menu_on_activate);
        }
        // endregion
        let width = if state.narrow { Length::Cells(46) } else { Length::Fill(1) };
        ui.add(table).width(width).height(Length::Cells(10)).id("rows");
        if !state.asked.is_empty() {
            ui.add(Text::new(state.asked.clone()).role("faint").no_wrap());
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("table.contents"), |ui| {
            let names = CONTENTS.map(|name| t!(&format!("table.{name}")));
            ui.add(Select::new(names).selected(Some(state.contents)).on_select(|i| send(Msg::Contents(i))))
                .width(Length::Cells(22))
                .id("contents");
        });
        setting(ui, t!("table.sortable"), |ui| {
            ui.add(toggle(state.sortable, |on| send(Msg::Sortable(on)))).id("sortable");
        });
        setting(ui, t!("table.multi"), |ui| {
            ui.add(toggle(state.multi, |on| send(Msg::Multi(on)))).id("multi");
        });
        setting(ui, t!("table.narrow"), |ui| {
            ui.add(toggle(state.narrow, |on| send(Msg::Narrow(on)))).id("narrow");
        });
        setting(ui, t!("table.icons"), |ui| {
            ui.add(toggle(state.icons, |on| send(Msg::Icons(on)))).id("icons");
        });
        setting(ui, t!("table.menu"), |ui| {
            ui.add(toggle(state.menu, |on| send(Msg::Menu(on)))).id("menu");
        });
        setting(ui, t!("table.menu-on-activate"), |ui| {
            ui.add(toggle(state.menu_on_activate, |on| send(Msg::MenuOnActivate(on)))).id("menu-on-activate");
        });
        slide_setting(ui, PAGE);
        ui.add(Text::new(t!("table.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn with_the_menu_as_the_action_a_click_on_a_row_opens_its_menu() {
        let mut h = showcase_on(PAGE);
        // The switch stands right after its label's column of 24 cells.
        let (x, y) = h.find("Menu on Enter and click").expect("the playground row is on screen");
        h.click(x + 25, y);
        assert!(h.app().pages.table.menu_on_activate, "the switch turned it on");
        h.set_reduced_motion(true).click_text("postgres").render();
        assert_eq!(h.app().pages.table.selected, Some(1));
        let restart = "Restart postgres";
        assert!(h.screen().contains(restart), "the click opened the row's menu:\n{}", h.screen());
        h.click_text(restart).render();
        assert!(!h.app().pages.table.asked.is_empty(), "the entry acted on the clicked row");
    }

    #[test]
    fn sorts_selects_and_scrolls_long_tables() {
        let mut h = showcase_on(PAGE);
        h.click_text("postgres");
        assert_eq!(h.app().pages.table.selected, Some(1));
        h.click_text("CPU");
        assert_eq!(h.app().pages.table.sort, Some((2, SortDirection::Ascending)));
        assert_eq!(h.app().pages.table.containers.last().map(|c| c.name), Some("quvyta-api"));
        h.send(send(Msg::Contents(1)));
        assert!(h.screen().contains("#100000"), "{}", h.screen());
        h.send(send(Msg::Sort(0, SortDirection::Ascending)));
        assert!(h.screen().contains("#1 "), "{}", h.screen());
        h.send(send(Msg::Contents(2)));
        assert!(h.screen().contains("No rows to show"));
    }

    #[test]
    fn names_carry_a_quiet_glyph_that_stays_when_the_name_is_cut() {
        let mut h = showcase_on(PAGE);
        h.set_glyph_mode(GlyphMode::Unicode);
        let project = h.env().icons().glyph("project").into_owned();
        let (x, y) = h.find(&format!("{project} postgres")).unwrap_or_else(|| panic!("{}", h.screen()));
        let cell = |v: i32| u16::try_from(v).unwrap_or(0);
        assert_eq!(h.fg(cell(x), cell(y)), h.env().theme().color("muted"), "an unselected row's glyph is quiet");
        h.set_glyph_mode(GlyphMode::Nerd);
        assert!(h.screen().contains("\u{e76e} postgres"), "the program's own glyph in Nerd mode:\n{}", h.screen());
        h.set_glyph_mode(GlyphMode::Ascii);
        let project = h.env().icons().glyph("project").into_owned();
        h.send(send(Msg::Narrow(true)));
        assert!(h.screen().contains(&format!("{project} worker-e…")), "{}", h.screen());
        h.send(send(Msg::Icons(false)));
        assert!(h.screen().contains("  postgres"), "{}", h.screen());
    }

    #[test]
    fn a_rows_own_menu_acts_on_the_row_it_was_opened_on() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Select(0)));
        let (x, y) = h.find("cache").expect("the third container's row");
        h.mouse(qframe::event::MouseKind::Down(qframe::event::MouseButton::Right), x, y);
        h.mouse(qframe::event::MouseKind::Up(qframe::event::MouseButton::Right), x, y);
        h.advance(std::time::Duration::from_millis(400));
        assert!(h.screen().contains("Restart cache"), "the menu is that row's own:\n{}", h.screen());
        assert_eq!(h.app().pages.table.selected, Some(2), "and the row became the selection");
        h.click_text("Stop cache");
        assert!(h.screen().contains("cache was asked to stop"), "{}", h.screen());
    }

    #[test]
    fn check_marks_stay_put_and_the_header_arrows_scroll() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Icons(false)));
        h.send(send(Msg::Multi(true)));
        let (mark, label) = super::super::mark_and_label(&h, '☐', "postgres");
        assert_eq!(label, mark + 2);
        let (x, y) = h.find("postgres").expect("postgres row");
        h.hover(x + 2, y);
        assert_eq!(super::super::mark_and_label(&h, '☐', "postgres"), (mark, mark + 3), "only the name slides");
        h.click(i32::try_from(mark).unwrap_or(0), y);
        assert!(h.app().pages.table.checked[1], "a click on the mark checks the row");

        h.send(send(Msg::Narrow(true)));
        let (x, y) = h.find("▶").expect("a narrow table shows the forward arrow");
        assert!(!h.screen().contains("Port "), "{}", h.screen());
        for _ in 0..4 {
            h.click(x, y);
        }
        assert!(h.screen().contains("Port"), "the arrow scrolled to the last column:\n{}", h.screen());
        assert!(h.find("◀").is_some(), "{}", h.screen());
    }
}
