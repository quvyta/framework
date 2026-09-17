//! Mouse text selection: a deploy log that asks to be selectable, drags, double and triple
//! presses, releasing without copying, the copy key and the Copy and Raw copy menu, controls
//! that keep the mouse, and text that only becomes selectable when asked.

use qframe::prelude::*;
use qframe::runtime::ClipboardEvent;
use qframe::widgets::ScrollView;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "text-selection";

/// The deploy log shown in the demo.
const LOG: [&str; 14] = [
    "12:04:01  deploy-api   pulling ghcr.io/quvyta/deploy-api:2026.9.1",
    "12:04:03  deploy-api   image sha256:4f2a9c1e verified",
    "12:04:04  migrations   1 of 5: add deploys table",
    "12:04:06  migrations   5 of 5: drop legacy jobs",
    "12:04:07  web-7c9f     readiness probe passed on :8080",
    "12:04:07  web-51d2     readiness probe passed on :8080",
    "12:04:08  router       shifting 25% of traffic to 2026.9.1",
    "12:04:38  router       shifting 50% of traffic to 2026.9.1",
    "12:05:08  router       shifting 100% of traffic to 2026.9.1",
    "12:05:09  web-3aa0     draining old replica",
    "12:05:12  web-3aa0     stopped",
    "12:05:12  deploy-api   rollout complete in 71 s",
    "12:05:13  notify       posted summary to #deploys",
    "12:05:13  deploy-api   https://deploys.quvyta.dev/runs/8841",
];

/// What was copied last and the playground.
#[derive(Debug)]
pub struct State {
    copied: Option<String>,
    log_selectable: bool,
    token_selectable: bool,
    restarts: u32,
}

impl Default for State {
    fn default() -> Self {
        Self { copied: None, log_selectable: true, token_selectable: false, restarts: 0 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Restart,
    LogSelectable(bool),
    TokenSelectable(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::TextSelection(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Restart => {
            state.restarts += 1;
            log.push(PAGE, "Button#restart", "pressed; the button kept the mouse");
        }
        Msg::LogSelectable(on) => {
            log.push(PAGE, "Playground", format!("log selectable = {on}"));
            state.log_selectable = on;
        }
        Msg::TokenSelectable(on) => {
            log.push(PAGE, "Playground", format!("token selectable = {on}"));
            state.token_selectable = on;
        }
    }
    Command::none()
}

// region: selection-heard
/// Nothing is copied on release. The copy key, Copy and Raw copy reach the application as
/// `ClipboardEvent::Copied`.
pub fn heard(state: &mut State, event: &ClipboardEvent, log: &mut EventLog) {
    if let ClipboardEvent::Copied(text) = event {
        log.push(PAGE, "App::clipboard", format!("copied {} characters", text.chars().count()));
        state.copied = Some(text.clone());
    }
}
// endregion

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("text-selection.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("text-selection.restart")).on_press(send(Msg::Restart))).id("restart");
            // region: selection-opt-in
            ui.add(
                Text::rich([
                    Span::new(t!("text-selection.token")).role("faint"),
                    Span::new("  qv_live_8f3a61c2d94b7e05"),
                ])
                .no_wrap(),
            )
            .selectable(state.token_selectable)
            .id("token");
            // endregion
        })
        .gap(3);
        ui.spacer().height(Length::Cells(1));
        // region: selection-log
        ui.add_with(ScrollView::new(), |ui| {
            for line in LOG {
                ui.add(Text::new(line).role("body").no_wrap());
            }
        })
        .height(Length::Cells(8))
        .fill_width()
        .selectable(state.log_selectable)
        .id("log");
        // endregion
        ui.spacer().height(Length::Cells(1));
        let (text, role) = match &state.copied {
            Some(text) => (t!("text-selection.copied", text = text.replace('\n', " ⏎ ")), "body"),
            None => (t!("text-selection.nothing"), "faint"),
        };
        ui.add(Text::new(text).role(role).no_wrap());
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("text-selection.log-selectable"), |ui| {
            ui.add(toggle(state.log_selectable, |on| send(Msg::LogSelectable(on)))).id("log-selectable");
        });
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("text-selection.token-selectable"), |ui| {
            ui.add(toggle(state.token_selectable, |on| send(Msg::TokenSelectable(on)))).id("token-selectable");
        });
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("text-selection.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{click_text_below, right_click, showcase_on};

    #[test]
    fn selecting_copies_nothing_until_asked_and_controls_keep_the_mouse() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("pulling").expect("log on screen");
        h.drag((x, y), (x + 6, y));
        assert_eq!(h.clipboard(), None, "releasing only selects");
        h.press("ctrl+c");
        assert_eq!(h.clipboard(), Some("pulling"));
        assert_eq!(h.app().pages.text_selection.copied.as_deref(), Some("pulling"));
        let (x, y) = h.find("sha256:4f2a9c1e").expect("digest on screen");
        h.click(x + 3, y).click(x + 3, y).press("ctrl+c");
        assert_eq!(h.clipboard(), Some("sha256:4f2a9c1e"));
        let (x, y) = h.find("qv_live").expect("token on screen");
        h.drag((x, y), (x + 6, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), Some("sha256:4f2a9c1e"), "the token is not selectable until asked");
        h.click_text("Restart");
        assert_eq!(h.app().pages.text_selection.restarts, 1);
        let (x, y) = h.find("LIVE").expect("panel title on screen");
        h.drag((x, y), (x + 3, y)).press("ctrl+c");
        let (menu_x, menu_y) = h.find("Getting started").expect("menu on screen");
        h.drag((menu_x, menu_y), (menu_x + 6, menu_y)).press("ctrl+c");
        assert_eq!(h.copied().len(), 2, "titles and the menu never start a selection");
    }

    #[test]
    fn a_right_click_on_the_selection_copies_clean_or_raw() {
        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("legacy").expect("log on screen");
        h.click(x, y).click(x, y).click(x, y);
        right_click(&mut h, x, y);
        // The hint names the entries too; the menu's are the ones under the selection.
        click_text_below(&mut h, "Raw copy", y);
        let raw = h.clipboard().map(str::to_owned).unwrap_or_default();
        assert!(
            raw.starts_with("12:04:06  migrations   5 of 5: drop legacy jobs   "),
            "raw keeps the padding: {raw:?}"
        );
        right_click(&mut h, x, y);
        click_text_below(&mut h, "Copy", y);
        assert_eq!(h.clipboard(), Some("12:04:06  migrations   5 of 5: drop legacy jobs"), "clean drops it");
        let log = h.app().log.recent(PAGE, 10);
        let raw_entry = format!("copied {} characters", raw.chars().count());
        assert!(log.iter().any(|entry| entry.message == raw_entry), "{log:?}");
        assert!(log.iter().any(|entry| entry.message == "copied 47 characters"), "{log:?}");
    }

    #[test]
    fn the_playground_turns_selection_on_and_off() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::LogSelectable(false)));
        let (x, y) = h.find("pulling").expect("log on screen");
        h.drag((x, y), (x + 6, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), None, "the log is no longer a selection region");
        h.send(send(Msg::TokenSelectable(true)));
        let (x, y) = h.find("qv_live").expect("token on screen");
        h.drag((x, y), (x + 6, y)).press("ctrl+c");
        assert_eq!(h.clipboard(), Some("qv_live"));
    }
}
