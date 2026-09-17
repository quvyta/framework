//! Tabs: switching views, numbers and overflow.

use qframe::prelude::*;
use qframe::widgets::Segmented;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "tabs";

/// Files of the overflow demo.
const FILES: [&str; 10] = [
    "main.rs",
    "app.rs",
    "catalog.rs",
    "log.rs",
    "regions.rs",
    "tests.rs",
    "theme.toml",
    "en.toml",
    "tr.toml",
    "keymap.toml",
];

/// Widths of the overflow strip the playground offers: narrow, the default and roomy.
const WIDTHS: [u16; 3] = [24, 48, 72];

/// Open tabs and playground settings.
#[derive(Debug)]
pub struct State {
    view: usize,
    file: usize,
    numbered: bool,
    width: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { view: 0, file: 0, numbered: true, width: 1 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    View(usize),
    File(usize),
    Numbered(bool),
    Width(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Tabs(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::View(index) => {
            state.view = index;
            log.push(PAGE, "Tabs#views", format!("selected {index}"));
        }
        Msg::File(index) => {
            state.file = index;
            log.push(PAGE, "Tabs#files", format!("selected {}", FILES[index]));
        }
        Msg::Numbered(on) => {
            state.numbered = on;
            log.push(PAGE, "Playground", format!("numbered = {on}"));
        }
        Msg::Width(index) => {
            state.width = index;
            log.push(PAGE, "Playground", format!("strip width = {}", WIDTHS[index]));
        }
    }
    Command::none()
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        // region: basic
        let labels = [t!("tabs.overview"), t!("tabs.activity"), t!("tabs.settings")];
        ui.add(Tabs::new(labels).numbered(state.numbered).active(state.view).on_select(|i| send(Msg::View(i))))
            .id("views");
        let body = [t!("tabs.overview-text"), t!("tabs.activity-text"), t!("tabs.settings-text")];
        ui.add(Text::new(body[state.view].clone()).role("secondary"));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("tabs.overflow")), |ui| {
        ui.add(Text::new(t!("tabs.overflow-hint", n = WIDTHS[state.width])).role("secondary"));
        // region: overflow
        ui.add(Tabs::new(FILES).active(state.file).on_select(|i| send(Msg::File(i))))
            .width(Length::Cells(WIDTHS[state.width]))
            .id("files");
        // endregion
        ui.add(Text::new(t!("tabs.overflow-keys")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("tabs.numbered"), |ui| {
            ui.add(toggle(state.numbered, |on| send(Msg::Numbered(on)))).id("numbered");
        });
        setting(ui, t!("tabs.width"), |ui| {
            let names = WIDTHS.map(|width| width.to_string());
            ui.add(Segmented::new(names).selected(state.width).on_select(|i| send(Msg::Width(i)))).id("width");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn switches_views_and_scrolls_overflowing_tabs() {
        let mut h = showcase_on(PAGE);
        h.click_text("Activity");
        assert_eq!(h.app().pages.tabs.view, 1);
        h.send(send(Msg::File(9)));
        assert!(h.screen().contains("keymap.toml"));
    }

    #[test]
    fn overflow_arrows_scroll_with_a_click_and_open_nothing() {
        let mut h = showcase_on(PAGE);
        let (_, y) = h.find("main.rs").expect("the overflow strip");
        let row = |h: &qframe::runtime::Harness<crate::app::Showcase>| {
            h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned()
        };
        let before = row(&h);
        assert!(before.contains('◀') && !before.contains("keymap.toml"), "{before}");
        let forward = before.chars().position(|c| c == '▶').expect("forward arrow on the strip's row");
        let forward = i32::try_from(forward).expect("on screen");
        for _ in 0..FILES.len() {
            h.click(forward, y);
        }
        let after = row(&h);
        assert!(after.contains("keymap.toml") && !after.contains("main.rs"), "{after}");
        assert_eq!(h.app().pages.tabs.file, 0, "the arrows scroll without opening a tab");
        h.send(send(Msg::Width(0)));
        assert!(h.screen().contains("strip width = 24"), "{}", h.screen());
    }
}
