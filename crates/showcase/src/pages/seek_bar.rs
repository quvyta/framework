//! Seek bar: a progress bar a person can click and drag to a position.

use qframe::prelude::*;
use qframe::widgets::SeekBar;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "seek-bar";

/// Seconds in the song the demo plays.
const LENGTH: u32 = 3 * 60 + 45;

/// Where the demo song is, and whether the bar names the position under the pointer.
#[derive(Debug)]
pub struct State {
    position: f32,
    labelled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { position: 0.42, labelled: true }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Seek(f32),
    Labelled(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::SeekBar(message))
}

// region: clock
/// Where the demo song is, written the way a player writes it.
fn clock(fraction: f32) -> String {
    let seconds = (fraction * LENGTH as f32).round() as u32;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Seek(fraction) => {
            state.position = fraction;
            log.push(PAGE, "SeekBar#track", format!("seek {}", clock(fraction)));
        }
        Msg::Labelled(on) => {
            state.labelled = on;
            log.push(PAGE, "Playground", format!("hover_label = {on}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("seek-bar.playback")).gap(0), |ui| {
        ui.add(Text::new(t!("seek-bar.playback-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: seek
        let mut bar = SeekBar::new(state.position).on_seek(|fraction| send(Msg::Seek(fraction)));
        if state.labelled {
            bar = bar.hover_label(clock);
        }
        ui.add(bar).width(Length::Fill(1)).id("track");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("seek-bar.hover-label"), |ui| {
            ui.add(toggle(state.labelled, |on| send(Msg::Labelled(on)))).id("labelled");
        });
        ui.add(Text::new(t!("seek-bar.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::runtime::Harness;

    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_on;

    /// Clicks the switch of the playground row labelled `label`; the switch stands after the
    /// label's column of 24 cells.
    fn click_setting(h: &mut Harness<Showcase>, label: &str) {
        let (x, y) = h.find(label).unwrap_or_else(|| panic!("the {label} row:\n{}", h.screen()));
        h.click(x + 25, y);
    }

    /// The percentage the demo's bar writes after itself, which finds the row it is drawn on.
    fn track(h: &Harness<Showcase>) -> (i32, i32) {
        h.find("42%").unwrap_or_else(|| panic!("the bar writes where the song is:\n{}", h.screen()))
    }

    /// The row above the bar, where the label stands while the pointer is on it.
    fn label_line(h: &Harness<Showcase>, row: i32) -> String {
        let above = usize::try_from(row.saturating_sub(1)).unwrap_or(0);
        h.screen().lines().nth(above).unwrap_or_default().to_owned()
    }

    /// The time the label on that row names, when it names one: the only token there written as a
    /// player's clock.
    fn label(h: &Harness<Showcase>, row: i32) -> Option<String> {
        let digits = |word: &str| !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit());
        label_line(h, row)
            .split_whitespace()
            .find(|word| {
                word.split_once(':')
                    .is_some_and(|(minutes, seconds)| digits(minutes) && seconds.len() == 2 && digits(seconds))
            })
            .map(str::to_owned)
    }

    #[test]
    fn a_press_moves_the_song_and_the_label_names_the_cell_it_landed_on() {
        let mut h = showcase_on(PAGE);
        let (x, y) = track(&h);
        h.click(x - 30, y).advance(std::time::Duration::from_millis(200));
        let moved = h.app().pages.seek_bar.position;
        assert_ne!(moved, 0.42, "the press moved the song: {}", h.screen());
        let named = clock(moved);
        assert_eq!(label(&h, y).as_deref(), Some(named.as_str()), "the label names that cell:\n{}", h.screen());
        h.hover(x - 30, y + 3);
        assert_eq!(label(&h, y), None, "the label leaves with the pointer:\n{}", h.screen());
    }

    #[test]
    fn the_label_switch_turns_the_label_off_and_on() {
        let mut h = showcase_on(PAGE);
        let (x, y) = track(&h);
        assert_eq!(label(&h, y), None, "no label before the pointer arrives:\n{}", h.screen());
        h.hover(x - 10, y).advance(std::time::Duration::from_millis(200));
        let here = label(&h, y).unwrap_or_else(|| panic!("the label names that cell:\n{}", h.screen()));
        h.hover(x - 40, y).advance(std::time::Duration::from_millis(200));
        assert_ne!(label(&h, y).as_deref(), Some(here.as_str()), "another cell names another time");
        click_setting(&mut h, "Label above the pointer");
        h.hover(x - 40, y).advance(std::time::Duration::from_millis(200));
        assert_eq!(label(&h, y), None, "the switch turned it off:\n{}", h.screen());
        click_setting(&mut h, "Label above the pointer");
        h.hover(x - 40, y).advance(std::time::Duration::from_millis(200));
        assert!(label(&h, y).is_some(), "and on again:\n{}", h.screen());
    }
}
