//! Scroll view: scrolling long content and following focus.

use qframe::prelude::*;
use qframe::widgets::ScrollView;

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "scroll-view";

/// Nothing to remember: scroll position lives in the runtime.
#[derive(Debug, Default)]
pub struct State;

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Pressed(&'static str),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ScrollView(message))
}

/// Applies a demo message.
pub fn update(_state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let Msg::Pressed(id) = message;
    log.push(PAGE, format!("Button#{id}"), "pressed");
    Command::none()
}

/// The live demo.
pub fn view(_state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")), |ui| {
        ui.add(Text::new(t!("scroll-view.hint")).role("secondary"));
        // region: scroll
        ui.add_with(ScrollView::new(), |ui| {
            for chapter in 1..=12 {
                ui.add(Text::new(t!("scroll-view.chapter", n = chapter)).role("title"));
                ui.add(Text::new(t!("scroll-view.body")).role("secondary"));
                if chapter == 6 {
                    ui.add(Button::new(t!("scroll-view.middle")).on_press(send(Msg::Pressed("middle")))).id("middle");
                }
            }
            ui.add(Button::new(t!("scroll-view.end")).variant("primary").on_press(send(Msg::Pressed("end")))).id("end");
        })
        .width(Length::Fill(1))
        .height(Length::Cells(14))
        .id("chapters");
        // endregion
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn focus_scrolls_the_button_into_view() {
        let mut h = showcase_on(PAGE);
        assert!(!h.screen().contains("You reached the end"));
        h.send(AppMsg::Section(0));
        for _ in 0..20 {
            h.press("tab");
            if h.is_focused("end") {
                break;
            }
        }
        assert!(h.is_focused("end"), "{}", h.screen());
        assert!(h.screen().contains("You reached the end"));
    }
}
