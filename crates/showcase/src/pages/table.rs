//! Table: containers with sortable columns, multiple selection, sideways scrolling and a very
//! long build history.

use std::cmp::Ordering;
use std::sync::Arc;

use qframe::prelude::*;
use qframe::widgets::{Column, ColumnWidth, Select, SortDirection, Table, TableCell, TableRow};

use super::{PageMsg, setting, slide_setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "table";

/// A running container: name, status, CPU per mille, memory in MiB, image, port.
#[derive(Debug, Clone, Copy)]
struct Container {
    name: &'static str,
    status: &'static str,
    cpu: u32,
    memory: u32,
    image: &'static str,
    port: &'static str,
}

const CONTAINERS: [Container; 8] = [
    Container {
        name: "quvyta-api",
        status: "running",
        cpu: 124,
        memory: 512,
        image: "quvyta/api:2.4",
        port: "8080/tcp",
    },
    Container { name: "postgres", status: "running", cpu: 38, memory: 1024, image: "postgres:16", port: "5432/tcp" },
    Container { name: "cache", status: "running", cpu: 9, memory: 256, image: "valkey:8", port: "6379/tcp" },
    Container { name: "worker-emails", status: "paused", cpu: 0, memory: 96, image: "quvyta/worker:2.4", port: "none" },
    Container { name: "docs-preview", status: "running", cpu: 21, memory: 128, image: "caddy:2", port: "3000/tcp" },
    Container { name: "search-index", status: "failed", cpu: 0, memory: 0, image: "meilisearch:1.9", port: "7700/tcp" },
    Container { name: "nightly-tests", status: "stopped", cpu: 0, memory: 0, image: "quvyta/ci:2.4", port: "none" },
    Container { name: "metrics", status: "running", cpu: 57, memory: 384, image: "prometheus:3", port: "9090/tcp" },
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
    build_numbers: Vec<usize>,
    sort: Option<(usize, SortDirection)>,
    selected: Option<usize>,
    checked: Vec<bool>,
    contents: usize,
    multi: bool,
    sortable: bool,
    narrow: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            containers: CONTAINERS.to_vec(),
            builds: Arc::from(Vec::new()),
            build_numbers: Vec::new(),
            sort: None,
            selected: Some(0),
            checked: vec![false; CONTAINERS.len()],
            contents: 0,
            multi: false,
            sortable: true,
            narrow: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Select(usize),
    Activate(usize),
    Toggle(usize),
    Sort(usize, SortDirection),
    Contents(usize),
    Multi(bool),
    Sortable(bool),
    Narrow(bool),
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
    }
    Command::none()
}

// region: table-rows
fn container_rows(containers: &[Container]) -> Vec<TableRow> {
    containers
        .iter()
        .map(|c| {
            let tone = match c.status {
                "running" => "success",
                "paused" => "warning",
                "failed" => "danger",
                _ => "muted",
            };
            let status = TableCell::new(t!(&format!("table.status.{}", c.status))).icon("dot", Some(tone));
            let cpu = format!("{}.{}%", c.cpu / 10, c.cpu % 10);
            TableRow::new([
                TableCell::new(c.name),
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
                container_rows(&state.containers).into(),
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
        // endregion
        let width = if state.narrow { Length::Cells(46) } else { Length::Fill(1) };
        ui.add(table).width(width).height(Length::Cells(10)).id("rows");
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
    fn check_marks_stay_put_and_the_header_arrows_scroll() {
        let mut h = showcase_on(PAGE);
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
