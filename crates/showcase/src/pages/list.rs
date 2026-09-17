//! List: selection, activation, multi-select, empty lists and very long lists.

use std::cell::RefCell;
use std::sync::Arc;

use qframe::prelude::*;
use qframe::widgets::Select;

use super::{PageMsg, setting, slide_setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "list";

/// Demo containers: name, status icon colour, status key.
const CONTAINERS: [(&str, &str, &str); 6] = [
    ("quvyta-dev", "success", "running"),
    ("postgres", "success", "running"),
    ("cache-builder", "warning", "paused"),
    ("docs-preview", "success", "running"),
    ("legacy-api", "muted", "stopped"),
    ("nightly-tests", "danger", "failed"),
];

/// Contents the playground offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Contents {
    Containers,
    Many,
    Empty,
}

const CONTENTS: [Contents; 3] = [Contents::Containers, Contents::Many, Contents::Empty];

/// Selection, checks and playground settings.
#[derive(Debug)]
pub struct State {
    selected: Option<usize>,
    checked: Vec<bool>,
    multi: bool,
    contents: usize,
    /// The very long list, with the word its rows were labelled in.
    many: RefCell<Option<(String, Arc<[ListItem]>)>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            selected: Some(0),
            checked: vec![false; CONTAINERS.len()],
            multi: false,
            contents: 0,
            many: RefCell::new(None),
        }
    }
}

/// The 100 000 rows, built once (again only when the language changes) and shared by every frame.
fn many_rows(state: &State) -> Arc<[ListItem]> {
    let word = t!("list.row");
    let mut many = state.many.borrow_mut();
    if let Some((built_in, rows)) = &*many
        && *built_in == word
    {
        return Arc::clone(rows);
    }
    let rows: Arc<[ListItem]> = (1..=100_000).map(|n| ListItem::new(format!("{word} {n}"))).collect();
    *many = Some((word, Arc::clone(&rows)));
    rows
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Select(usize),
    Activate(usize),
    Toggle(usize),
    Multi(usize),
    Contents(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::List(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Select(index) => {
            state.selected = Some(index);
            log.push(PAGE, "List#containers", format!("selected {index}"));
        }
        Msg::Activate(index) => log.push(PAGE, "List#containers", format!("activated {index}")),
        // region: toggle
        Msg::Toggle(index) => {
            if let Some(checked) = state.checked.get_mut(index) {
                *checked = !*checked;
            }
            log.push(PAGE, "List#containers", format!("toggled {index}"));
        }
        // endregion
        Msg::Multi(index) => {
            state.multi = index == 1;
            log.push(PAGE, "Playground", format!("multi = {}", state.multi));
        }
        Msg::Contents(index) => {
            state.contents = index;
            state.selected = Some(0);
            log.push(PAGE, "Playground", format!("contents = {index}"));
        }
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        let contents = CONTENTS[state.contents];
        // region: items
        let list = match contents {
            Contents::Containers => List::new(CONTAINERS.iter().map(|(name, color, status)| {
                let status = t!(&format!("list.status.{status}"));
                ListItem::new(*name).icon("dot", Some(color)).detail(status)
            })),
            // A very long list lives in the state; the list only shares it.
            Contents::Many => List::shared(many_rows(state)),
            Contents::Empty => List::new(Vec::new()),
        };
        // endregion
        // region: list
        let mut list = list
            .selected(state.selected)
            .empty_text(t!("list.empty"))
            .on_select(|index| send(Msg::Select(index)))
            .on_activate(|index| send(Msg::Activate(index)));
        if state.multi && contents == Contents::Containers {
            list = list.checked(state.checked.clone()).on_toggle(|index| send(Msg::Toggle(index)));
        }
        ui.add(list).width(Length::Fill(1)).height(Length::Cells(8)).id("containers");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("list.mode"), |ui| {
            let modes = [t!("list.single"), t!("list.multi")];
            ui.add(Select::new(modes).selected(Some(usize::from(state.multi))).on_select(|i| send(Msg::Multi(i))))
                .width(Length::Cells(22))
                .id("mode");
        });
        setting(ui, t!("list.contents"), |ui| {
            let names = [t!("list.containers"), t!("list.many"), t!("list.nothing")];
            ui.add(Select::new(names).selected(Some(state.contents)).on_select(|i| send(Msg::Contents(i))))
                .width(Length::Cells(22))
                .id("contents");
        });
        slide_setting(ui, PAGE);
        ui.add(Text::new(t!("list.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn selects_toggles_and_handles_huge_lists() {
        let mut h = showcase_on(PAGE);
        h.click_text("postgres");
        assert_eq!(h.app().pages.list.selected, Some(1));
        h.send(send(Msg::Multi(1)));
        h.send(send(Msg::Toggle(2)));
        assert!(h.app().pages.list.checked[2]);
        h.send(send(Msg::Contents(1)));
        assert!(h.screen().contains("Row 1"));
        let rows = |h: &Harness<crate::app::Showcase>| {
            h.app().pages.list.many.borrow().as_ref().map(|(_, rows)| Arc::clone(rows))
        };
        let before = rows(&h);
        h.press("down");
        assert!(
            before.zip(rows(&h)).is_some_and(|(a, b)| Arc::ptr_eq(&a, &b)),
            "the rows are kept, not built per frame"
        );
        h.set_locale("tr");
        assert!(h.screen().contains("Satır 1"), "the rows follow the language");
        h.set_locale("en");
        h.send(send(Msg::Contents(2)));
        assert!(h.screen().contains("No containers"));
    }

    #[test]
    fn check_marks_stay_put_while_icon_and_name_slide() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Multi(1)));
        let (mark, label) = super::super::mark_and_label(&h, '☐', "● postgres");
        assert_eq!(label, mark + 2);
        let (x, y) = h.find("postgres").expect("postgres row");
        h.hover(x + 3, y);
        assert_eq!(super::super::mark_and_label(&h, '☐', "● postgres"), (mark, mark + 3), "only icon and name slide");
        h.send(AppMsg::Page(PageMsg::Slide(PAGE, false)));
        assert_eq!(super::super::mark_and_label(&h, '☐', "● postgres"), (mark, mark + 2));
        h.click(i32::try_from(mark).unwrap_or(0), y);
        assert!(h.app().pages.list.checked[1], "a click on the mark checks the row");
        assert_eq!(h.app().pages.list.selected, Some(0), "and does not open it");
    }
}
