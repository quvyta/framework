//! Markdown: rendering documents with headings, lists, quotes and code, and selecting them with
//! the mouse.

use qframe::prelude::*;
use qframe::widgets::Markdown;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "markdown";

/// The sample document in English.
const SAMPLE_EN: &str = include_str!("../../assets/pages/markdown/sample.en.md");
/// The sample document in Turkish.
const SAMPLE_TR: &str = include_str!("../../assets/pages/markdown/sample.tr.md");

/// Playground settings.
#[derive(Debug)]
pub struct State {
    selectable: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { selectable: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Selectable(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Markdown(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let Msg::Selectable(on) = message;
    state.selectable = on;
    log.push(PAGE, "Playground", format!("selectable = {on}"));
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let sample = if ui.env().i18n().active() == "tr" { SAMPLE_TR } else { SAMPLE_EN };
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("markdown.hint")).role("secondary"));
        // region: render
        // A document is selectable by itself; `.selectable(false)` turns that off.
        ui.add(Markdown::new(sample)).fill_width().selectable(state.selectable);
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("text.selectable"), |ui| {
            ui.add(toggle(state.selectable, |on| send(Msg::Selectable(on)))).id("selectable");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn the_document_is_selectable_and_clean_copies_skip_heading_pillars() {
        let mut h = showcase_on("markdown");
        let (x, y) = h.find("Release notes").expect("heading on screen");
        h.drag((x - 2, y), (x + 12, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), Some("Release notes"));
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "copied 13 characters"), "{log:?}");
        h.send(send(Msg::Selectable(false)));
        h.drag((x, y), (x + 6, y)).press("ctrl+c");
        assert_eq!(h.copied().len(), 1, "selection turned off");
    }

    #[test]
    fn renders_the_sample() {
        let h = showcase_on("markdown");
        let screen = h.screen();
        assert!(screen.contains("Release notes"), "{screen}");
        assert!(!screen.contains("##"));
    }
}
