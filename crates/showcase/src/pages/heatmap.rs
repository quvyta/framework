//! Heatmap: a year of focus day by day, and a legend naming the categories.

use qframe::prelude::*;
use qframe::widgets::{EmptyState, Heatmap, Legend, Segmented};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "heatmap";

/// Days in a week: the rows of the grid.
const ROWS: usize = 7;

/// Weeks the playground offers.
const WEEKS: [usize; 3] = [52, 26, 12];

/// Weeks of each category grid.
const CATEGORY_WEEKS: usize = 12;

/// Width the playground gives its grid, so the page can say how many weeks are left.
const PLAYGROUND_WIDTH: u16 = 26;

/// The daily goal a fixed scale measures against, in minutes.
const GOAL: f32 = 120.0;

/// Categories of the second panel: locale key and series tone.
const CATEGORIES: [(&str, usize); 3] = [("rust", 0), ("docs", 1), ("review", 2)];

/// Minutes of focus on `day` in `series`: quiet weekends, a few empty days and a busy stretch,
/// repeatable so tests and screenshots are stable.
#[must_use]
pub fn minutes(series: u32, day: usize) -> f32 {
    let index = u32::try_from(day).unwrap_or(0);
    // A small hash gives each day its own repeatable number.
    let hash = index.wrapping_mul(2_246_822_519).wrapping_add(series.wrapping_mul(97_021)) >> 20;
    let base = f32::from(u16::try_from(hash % 150).unwrap_or(0));
    let weekend = day % ROWS >= 5;
    let quiet = hash % 7 == 0;
    if quiet {
        return 0.0;
    }
    if weekend { base * 0.3 } else { base }
}

/// The days of `series`, oldest first, for `weeks` weeks.
fn history(series: u32, weeks: usize) -> Vec<f32> {
    (0..weeks * ROWS).map(|day| minutes(series, day)).collect()
}

/// The day picked in the year grid and the playground settings.
#[derive(Debug)]
pub struct State {
    picked: Option<usize>,
    weeks: usize,
    goal: bool,
    pickable: bool,
    empty: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { picked: None, weeks: 0, goal: false, pickable: true, empty: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Pick(usize),
    Weeks(usize),
    Goal(bool),
    Pickable(bool),
    Empty(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Heatmap(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Pick(day) => {
            state.picked = Some(day);
            ("Heatmap#year", format!("day {day} = {:.0} min", minutes(0, day)))
        }
        Msg::Weeks(index) => {
            state.weeks = index;
            ("Playground", format!("weeks = {}", WEEKS[index.min(WEEKS.len() - 1)]))
        }
        Msg::Goal(on) => {
            state.goal = on;
            ("Playground", format!("goal = {on}"))
        }
        Msg::Pickable(on) => {
            state.pickable = on;
            ("Playground", format!("pick a day = {on}"))
        }
        Msg::Empty(on) => {
            state.empty = on;
            ("Playground", format!("no days yet = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("heatmap.year")).gap(0), |ui| {
        ui.add(Text::new(t!("heatmap.year-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: basic
        let year = history(0, WEEKS[0]);
        ui.add(Heatmap::new(year.iter().copied()).selected(state.picked).on_select(|day| send(Msg::Pick(day))))
            .width(Length::Fill(1))
            .height(Length::Cells(u16::try_from(ROWS).unwrap_or(7)))
            .id("year");
        // endregion
        ui.spacer().height(Length::Cells(1));
        let readout = match state.picked {
            Some(day) => t!("heatmap.picked", day = day + 1, minutes = format!("{:.0}", minutes(0, day))),
            None => t!("heatmap.pick-hint"),
        };
        ui.add(Text::new(readout).role(if state.picked.is_some() { "body" } else { "faint" }).no_wrap());
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("heatmap.categories")).gap(0), |ui| {
        ui.add(Text::new(t!("heatmap.categories-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: series
        for (key, series) in CATEGORIES {
            ui.row(|ui| {
                ui.add(Text::new(t!(&format!("heatmap.{key}"))).role("secondary").no_wrap()).width(Length::Cells(10));
                ui.add(Heatmap::<AppMsg>::new(history(series_seed(series), CATEGORY_WEEKS)).series(series))
                    .width(Length::Cells(u16::try_from(CATEGORY_WEEKS).unwrap_or(12)))
                    .height(Length::Cells(u16::try_from(ROWS).unwrap_or(7)));
            })
            .gap(2);
        }
        ui.spacer().height(Length::Cells(1));
        let names = CATEGORIES.map(|(key, _)| t!(&format!("heatmap.{key}")));
        ui.add(Legend::new(names).tones(CATEGORIES.map(|(_, series)| series))).fill_width();
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let weeks = WEEKS[state.weeks.min(WEEKS.len() - 1)];
        let values = if state.empty { Vec::new() } else { history(0, weeks) };
        let mut map: Heatmap<AppMsg> = Heatmap::new(values.iter().copied());
        if state.goal {
            map = map.max(GOAL);
        }
        if state.pickable {
            map = map.selected(state.picked).on_select(|day| send(Msg::Pick(day)));
        }
        let shown = map.columns(PLAYGROUND_WIDTH);
        if values.is_empty() {
            ui.add(EmptyState::new(t!("heatmap.nothing")).message(t!("heatmap.nothing-hint")))
                .width(Length::Cells(PLAYGROUND_WIDTH))
                .height(Length::Cells(u16::try_from(ROWS).unwrap_or(7)));
        } else {
            ui.add(map)
                .width(Length::Cells(PLAYGROUND_WIDTH))
                .height(Length::Cells(u16::try_from(ROWS).unwrap_or(7)))
                .id("configured");
        }
        // endregion
        ui.add(Text::new(t!("heatmap.shown", weeks = shown, of = weeks)).role("faint").no_wrap());
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("heatmap.weeks"), |ui| {
            let labels: Vec<String> = WEEKS.iter().map(usize::to_string).collect();
            ui.add(Segmented::new(labels).selected(state.weeks).on_select(|i| send(Msg::Weeks(i)))).id("weeks");
        });
        setting(ui, t!("heatmap.goal"), |ui| {
            ui.add(toggle(state.goal, |on| send(Msg::Goal(on)))).id("goal");
        });
        setting(ui, t!("heatmap.pickable"), |ui| {
            ui.add(toggle(state.pickable, |on| send(Msg::Pickable(on)))).id("pickable");
        });
        setting(ui, t!("heatmap.empty"), |ui| {
            ui.add(toggle(state.empty, |on| send(Msg::Empty(on)))).id("empty");
        });
    })
    .fill_width();
}

/// The sample series behind a category, so the three grids do not look alike.
fn series_seed(series: usize) -> u32 {
    u32::try_from(series).unwrap_or(0) + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{showcase_on, showcase_tall};

    #[test]
    fn a_day_is_picked_with_the_mouse_and_with_the_keyboard() {
        let mut h = showcase_on(PAGE);
        let hint = h.screen();
        assert!(hint.contains("Click a day"), "{hint}");
        let (x, _) = h.find("A YEAR OF FOCUS").expect("the year panel is on screen");
        let (_, readout) = h.find("Click a day").expect("the hint is on screen");
        // The grid sits above the readout; three rows up is inside it whatever the hint wrapped to.
        h.click(x, readout - 3);
        let picked = h.app().pages.heatmap.picked.unwrap_or_else(|| panic!("{}", h.screen()));
        assert!(h.screen().contains("Day "), "the picked day is read as text:\n{}", h.screen());
        assert!(h.is_focused("year"), "a click leaves the keyboard on the grid");
        h.press("down");
        h.press("enter");
        assert_eq!(h.app().pages.heatmap.picked, Some(picked + 1), "the keyboard walks the same cells");
        assert!(h.screen().contains(&format!("Day {}", picked + 2)), "{}", h.screen());
    }

    #[test]
    fn the_playground_narrows_the_grid_and_shows_its_empty_state() {
        let mut h = showcase_tall(crate::app::Showcase::new(), PAGE, 70);
        h.send(send(Msg::Weeks(2)));
        assert!(h.screen().contains("12 of 12 weeks"), "{}", h.screen());
        h.send(send(Msg::Empty(true)));
        assert!(h.screen().contains("No days yet"), "the empty state is designed:\n{}", h.screen());
        h.send(send(Msg::Empty(false)));
        h.send(send(Msg::Weeks(0)));
        assert!(h.screen().contains("26 of 52 weeks"), "a narrow grid keeps the newest weeks:\n{}", h.screen());
    }

    #[test]
    fn the_legend_names_every_category() {
        let h = showcase_tall(crate::app::Showcase::new(), PAGE, 70);
        let screen = h.screen();
        for name in ["Rust", "Docs", "Review"] {
            assert!(screen.contains(name), "`{name}` is named:\n{screen}");
        }
    }

    #[test]
    fn samples_are_repeatable_and_weekends_are_quieter() {
        assert_eq!(minutes(0, 40), minutes(0, 40));
        let history = history(0, 52);
        assert_eq!(history.len(), 52 * ROWS);
        assert!(history.contains(&0.0), "some days hold nothing");
        let weekdays: f32 = (0..52).map(|week| minutes(0, week * ROWS + 2)).sum();
        let weekends: f32 = (0..52).map(|week| minutes(0, week * ROWS + 6)).sum();
        assert!(weekends < weekdays, "weekends are quieter: {weekends} against {weekdays}");
    }
}
