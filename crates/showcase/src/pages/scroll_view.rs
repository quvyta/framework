//! Scroll view: scrolling long content, following focus and following the end of growing
//! content.

use std::time::Duration;

use qframe::prelude::*;
use qframe::widgets::ScrollView;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "scroll-view";

/// Lines the growing view starts with, enough to overflow it.
const FIRST_LINES: usize = 12;

/// Lines added at once by the burst button.
const BURST: usize = 8;

/// Time between written lines.
const INTERVAL: Duration = Duration::from_millis(400);

/// The growing view's lines and its writer; scroll position lives in the runtime.
#[derive(Debug)]
pub struct State {
    lines: usize,
    writing: bool,
    /// Bumped whenever writing stops, so a tick from an older writer ends it.
    generation: u64,
}

impl Default for State {
    fn default() -> Self {
        Self { lines: FIRST_LINES, writing: false, generation: 0 }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Pressed(&'static str),
    Writing(bool),
    Tick(u64),
    Burst,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::ScrollView(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Pressed(id) => log.push(PAGE, format!("Button#{id}"), "pressed"),
        // region: follow-write
        Msg::Tick(generation) if state.writing && generation == state.generation => {
            state.lines += 1;
            return next_tick(generation);
        }
        Msg::Tick(_) => {}
        Msg::Writing(on) => {
            state.writing = on;
            state.generation += 1;
            log.push(PAGE, "Switch#writing", format!("writing = {on}"));
            if on {
                return next_tick(state.generation);
            }
        }
        // endregion
        Msg::Burst => {
            state.lines += BURST;
            log.push(PAGE, "Button#burst", format!("{} lines", state.lines));
        }
    }
    Command::none()
}

/// Waits one interval on a background thread, then asks for the next line.
fn next_tick(generation: u64) -> Command<AppMsg> {
    Command::perform(move || {
        std::thread::sleep(INTERVAL);
        send(Msg::Tick(generation))
    })
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
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

    ui.add_with(Panel::new().title(t!("scroll-view.follow-title")), |ui| {
        ui.add(Text::new(t!("scroll-view.follow-hint")).role("secondary"));
        // region: follow-end
        ui.add_with(ScrollView::new().follow_end(true), |ui| {
            for n in 1..=state.lines {
                ui.add(Text::new(t!("scroll-view.line", n = n)));
            }
        })
        .width(Length::Fill(1))
        .height(Length::Cells(8))
        .id("growing");
        // endregion
        setting(ui, t!("scroll-view.writing"), |ui| {
            ui.row(|ui| {
                ui.add(toggle(state.writing, |on| send(Msg::Writing(on)))).id("writing");
                ui.add(Button::new(t!("scroll-view.burst")).on_press(send(Msg::Burst))).id("burst");
            })
            .gap(2);
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    /// The number of the last written line on screen.
    fn last_line(h: &qframe::runtime::Harness<crate::app::Showcase>) -> Option<usize> {
        h.screen().lines().filter_map(|line| line.split("Line ").nth(1)?.split(' ').next()?.parse().ok()).next_back()
    }

    #[test]
    fn the_growing_view_opens_at_its_end_and_follows_a_burst() {
        // Tall enough to show the second panel below the chapters.
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::new(), PAGE, 70);
        h.set_reduced_motion(true);
        assert_eq!(last_line(&h), Some(FIRST_LINES), "{}", h.screen());
        h.click_text("A burst arrives");
        assert_eq!(last_line(&h), Some(FIRST_LINES + BURST), "{}", h.screen());
    }

    #[test]
    fn the_writing_switch_adds_lines_that_the_view_follows_until_it_is_switched_off() {
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::new(), PAGE, 70);
        h.set_reduced_motion(true);
        // The switch stands after its label's column of 24 cells.
        let (x, y) = h.find("Keep writing").expect("the playground row is on screen");
        h.click(x + 25, y);
        assert!(h.app().pages.scroll_view.writing, "the click turned writing on\n{}", h.screen());
        // Each step runs the wait for one tick, as one pass of the terminal loop would.
        h.render().render();
        let written = last_line(&h).expect("a line is on screen");
        assert!(written > FIRST_LINES, "lines arrived and the view kept the newest in view\n{}", h.screen());
        h.click(x + 25, y);
        assert!(!h.app().pages.scroll_view.writing, "the second click turned it off");
        h.render().render();
        assert_eq!(last_line(&h), Some(h.app().pages.scroll_view.lines), "the end stays in view");
        let stopped = h.app().pages.scroll_view.lines;
        h.render().render();
        assert_eq!(h.app().pages.scroll_view.lines, stopped, "no line arrives after it is off");
    }

    #[test]
    fn ticks_write_lines_only_for_the_current_writer() {
        let mut state = State::default();
        let mut log = EventLog::new();
        let tick = Msg::Tick(state.generation);
        let _ = update(&mut state, tick, &mut log);
        assert_eq!(state.lines, FIRST_LINES, "a tick without writing adds nothing");
        // The command that waits for the next tick is not run here; tests never sleep.
        let _ = update(&mut state, Msg::Writing(true), &mut log);
        let tick = Msg::Tick(state.generation);
        let _ = update(&mut state, tick, &mut log);
        assert_eq!(state.lines, FIRST_LINES + 1);
        let old = state.generation;
        let _ = update(&mut state, Msg::Writing(false), &mut log);
        let _ = update(&mut state, Msg::Tick(old), &mut log);
        assert_eq!(state.lines, FIRST_LINES + 1, "a stopped writer ignores its ticks");
    }

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
