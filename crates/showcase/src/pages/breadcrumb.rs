//! Breadcrumb: the path through a file tree, collapsing when narrow.

use qframe::prelude::*;
use qframe::widgets::{Breadcrumb, Segmented};

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "breadcrumb";

/// The demo tree: each folder and its subfolders.
const TREE: [(&str, &[&str]); 9] = [
    ("workspace", &["quvyta", "homelab"]),
    ("quvyta", &["crates", "docs"]),
    ("homelab", &["compose"]),
    ("crates", &["quvyta-framework", "showcase"]),
    ("quvyta-framework", &["src", "assets"]),
    ("src", &["widgets", "runtime", "theme"]),
    ("widgets", &[]),
    ("docs", &["notes"]),
    ("showcase", &["pages"]),
];

/// Widths the playground offers; 0 means the whole panel.
const WIDTHS: [u16; 3] = [0, 48, 26];

/// The open path and the playground.
#[derive(Debug)]
pub struct State {
    path: Vec<&'static str>,
    width: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { path: vec!["workspace", "quvyta", "crates", "quvyta-framework", "src"], width: 0 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Up(usize),
    Into(&'static str),
    Width(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Breadcrumb(message))
}

fn children(folder: &str) -> &'static [&'static str] {
    TREE.iter().find(|(name, _)| *name == folder).map_or(&[], |(_, children)| children)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: breadcrumb-update
        Msg::Up(index) => {
            state.path.truncate(index + 1);
            log.push(PAGE, "Breadcrumb#path", format!("selected {index}"));
        }
        // endregion
        Msg::Into(folder) => {
            state.path.push(folder);
            log.push(PAGE, "Button#folder", format!("opened {folder}"));
        }
        Msg::Width(index) => {
            state.width = index;
            log.push(PAGE, "Playground", format!("width = {}", WIDTHS[index]));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("breadcrumb.hint")).role("secondary"));
        // region: breadcrumb-path
        let crumb = ui.add(Breadcrumb::new(state.path.clone()).on_select(|index| send(Msg::Up(index)))).id("path");
        // endregion
        match WIDTHS[state.width] {
            0 => crumb.fill_width(),
            cells => crumb.width(Length::Cells(cells)),
        };
        let current = state.path.last().copied().unwrap_or_default();
        let folders = children(current);
        if folders.is_empty() {
            ui.add(Text::new(t!("breadcrumb.empty")).role("faint"));
        } else {
            ui.row(|ui| {
                for folder in folders {
                    ui.add(Button::new(*folder).icon("folder").on_press(send(Msg::Into(folder))))
                        .id(format!("folder-{folder}"));
                }
            })
            .gap(1);
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("breadcrumb.width"), |ui| {
            let names = [t!("breadcrumb.full"), t!("breadcrumb.columns", n = 48), t!("breadcrumb.columns", n = 26)];
            ui.add(Segmented::new(names).selected(state.width).on_select(|i| send(Msg::Width(i)))).id("width");
        });
        ui.add(Text::new(t!("breadcrumb.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn opens_levels_and_collapses_when_narrow() {
        let mut h = showcase_on(PAGE);
        h.click_text("crates");
        assert_eq!(h.app().pages.breadcrumb.path, vec!["workspace", "quvyta", "crates"]);
        h.send(send(Msg::Into("showcase")));
        assert_eq!(h.app().pages.breadcrumb.path.last(), Some(&"showcase"));
        h.send(send(Msg::Up(5)));
        h.send(send(Msg::Width(2)));
        assert!(h.screen().contains("…"), "{}", h.screen());
    }
}
