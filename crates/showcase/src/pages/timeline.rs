//! Timeline: a day of focus on a time axis, a night across midnight, a session that runs on from
//! one day into the next, and the axes a chart can stand on.

use qframe::date::{TimeOfDay, Weekday};
use qframe::prelude::*;
use qframe::widgets::{Axis, Legend, Segmented, TimeBlock, Timeline};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "timeline";

/// Categories: locale key and the series tone each keeps everywhere.
const CATEGORIES: [(&str, usize); 3] = [("rust", 0), ("docs", 1), ("review", 2)];

/// An hour and a minute.
type Clock = (u8, u8);

/// The sample day: category, start and end. The last review is five minutes long, less than a
/// cell of a whole-day strip.
const DAY: [(usize, Clock, Clock); 5] = [
    (0, (9, 0), (11, 10)),
    (2, (11, 30), (12, 15)),
    (1, (13, 30), (15, 0)),
    (0, (15, 15), (17, 40)),
    (2, (21, 0), (21, 5)),
];

/// Widths the playground and the axes offer, in cells.
const WIDTHS: [u16; 3] = [72, 40, 16];

fn at((hour, minute): Clock) -> TimeOfDay {
    TimeOfDay::new(hour, minute, 0)
}

/// The name of category `index` in the active language.
fn category(index: usize) -> String {
    t!(&format!("timeline.{}", CATEGORIES[index].0))
}

/// The sample day as blocks, each in its category's tone.
fn day() -> Vec<TimeBlock> {
    DAY.iter()
        .map(|(kind, start, end)| {
            // region: blocks
            TimeBlock::new(category(*kind), at(*start), at(*end)).tone(CATEGORIES[*kind].1)
            // endregion
        })
        .collect()
}

/// Yesterday evening: docs, then a Rust session that runs on past midnight, so this day's strip
/// shows it open where midnight cuts it.
fn yesterday() -> Vec<TimeBlock> {
    vec![
        TimeBlock::new(category(1), at((19, 0)), at((20, 30))).tone(1),
        // region: open-end
        TimeBlock::new(category(0), at((22, 40)), at((1, 50))).tone(0).open_end(),
        // endregion
    ]
}

/// Today: the rest of that session, faint and open where it came from, a review, and a Rust
/// session whose counter is still running.
fn today() -> Vec<TimeBlock> {
    vec![
        // region: carried-over
        TimeBlock::new(category(0), at((0, 0)), at((1, 50))).tone(0).faint().open_start(),
        // endregion
        TimeBlock::new(category(2), at((10, 0)), at((11, 15))).tone(2),
        // region: running
        TimeBlock::new(category(0), at((13, 30)), at((16, 20))).tone(0).open_end(),
        // endregion
    ]
}

/// The block picked in the day, its visible range and the playground settings.
#[derive(Debug)]
pub struct State {
    picked: Option<usize>,
    yesterday_picked: Option<usize>,
    today_picked: Option<usize>,
    range: Option<(TimeOfDay, TimeOfDay)>,
    width: usize,
    axis_width: usize,
    axis: bool,
    readout: bool,
    pickable: bool,
    zoom: bool,
    disabled: bool,
    empty: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            picked: None,
            yesterday_picked: None,
            today_picked: None,
            range: None,
            width: 0,
            axis_width: 0,
            axis: true,
            readout: true,
            pickable: true,
            zoom: true,
            disabled: false,
            empty: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Pick(usize),
    PickYesterday(usize),
    PickToday(usize),
    Zoom(TimeOfDay, TimeOfDay),
    Width(usize),
    AxisWidth(usize),
    Axis(bool),
    Readout(bool),
    Pickable(bool),
    Zoomable(bool),
    Disabled(bool),
    Empty(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Timeline(message))
}

/// `time` as the log writes it.
fn hhmm(time: TimeOfDay) -> String {
    format!("{:02}:{:02}", time.hour, time.minute)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let (source, text) = match message {
        Msg::Pick(index) => {
            state.picked = Some(index);
            ("Timeline#day", format!("selected {index}"))
        }
        Msg::PickYesterday(index) => {
            state.yesterday_picked = Some(index);
            ("Timeline#yesterday", format!("selected {index}"))
        }
        Msg::PickToday(index) => {
            state.today_picked = Some(index);
            ("Timeline#today", format!("selected {index}"))
        }
        Msg::Zoom(from, to) => {
            state.range = if from == to { None } else { Some((from, to)) };
            ("Timeline#day", format!("range {}–{}", hhmm(from), hhmm(to)))
        }
        Msg::Width(index) => {
            state.width = index;
            ("Playground", format!("width = {}", WIDTHS[index.min(WIDTHS.len() - 1)]))
        }
        Msg::AxisWidth(index) => {
            state.axis_width = index;
            ("Axis", format!("width = {}", WIDTHS[index.min(WIDTHS.len() - 1)]))
        }
        Msg::Axis(on) => {
            state.axis = on;
            ("Playground", format!("axis = {on}"))
        }
        Msg::Readout(on) => {
            state.readout = on;
            ("Playground", format!("readout = {on}"))
        }
        Msg::Pickable(on) => {
            state.pickable = on;
            ("Playground", format!("select = {on}"))
        }
        Msg::Zoomable(on) => {
            state.zoom = on;
            ("Playground", format!("zoom = {on}"))
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            ("Playground", format!("disabled = {on}"))
        }
        Msg::Empty(on) => {
            state.empty = on;
            ("Playground", format!("empty day = {on}"))
        }
    };
    log.push(PAGE, source, text);
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("timeline.day")).gap(0), |ui| {
        ui.add(Text::new(t!("timeline.day-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: basic
        let mut timeline = Timeline::new(day())
            .axis()
            .readout()
            .selected(state.picked)
            .on_select(|index| send(Msg::Pick(index)))
            .on_zoom(|from, to| send(Msg::Zoom(from, to)));
        if let Some((from, to)) = state.range {
            timeline = timeline.range(from, to);
        }
        ui.add(timeline).width(Length::Cells(72)).id("day");
        ui.spacer().height(Length::Cells(1));
        let names = CATEGORIES.iter().map(|(key, _)| t!(&format!("timeline.{key}")));
        ui.add(Legend::new(names).tones(CATEGORIES.map(|(_, tone)| tone))).width(Length::Cells(72));
        // endregion
        let shown = match state.range {
            Some((from, to)) => t!("timeline.range", from = hhmm(from), to = hhmm(to)),
            None => t!("timeline.whole-day"),
        };
        ui.add(Text::new(shown).role("faint").no_wrap());
        ui.add(Text::new(t!("timeline.keys")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("timeline.night")).gap(0), |ui| {
        ui.add(Text::new(t!("timeline.night-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: night
        let night = [
            TimeBlock::new(t!("timeline.reading"), at((19, 30)), at((21, 0))).tone(1),
            TimeBlock::new(t!("timeline.sleep"), at((23, 10)), at((7, 5))).tone(3),
            TimeBlock::new(t!("timeline.alarm"), at((6, 30)), at((6, 45))).tone(4),
        ];
        let timeline = Timeline::<AppMsg>::new(night).day_starts_at(at((18, 0))).range(at((18, 0)), at((9, 0))).axis();
        ui.add(timeline).width(Length::Cells(72));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("timeline.open")).gap(0), |ui| {
        ui.add(Text::new(t!("timeline.open-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("timeline.yesterday")).role("faint"));
        let strip = Timeline::new(yesterday())
            .readout()
            .selected(state.yesterday_picked)
            .on_select(|index| send(Msg::PickYesterday(index)));
        ui.add(strip).width(Length::Cells(72)).id("yesterday");
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(t!("timeline.today")).role("faint"));
        let strip = Timeline::new(today())
            .axis()
            .readout()
            .selected(state.today_picked)
            .on_select(|index| send(Msg::PickToday(index)));
        ui.add(strip).width(Length::Cells(72)).id("today");
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("timeline.axes")).gap(0), |ui| {
        ui.add(Text::new(t!("timeline.axes-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let width = Length::Cells(WIDTHS[state.axis_width.min(WIDTHS.len() - 1)]);
        // region: axis
        ui.add(Axis::weekdays(Weekday::Monday, 7)).width(width);
        ui.add(Axis::months(1, 12)).width(width);
        ui.add(Axis::hours(TimeOfDay::default(), TimeOfDay::default())).width(width);
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("timeline.width"), |ui| {
            let labels = WIDTHS.map(|w| w.to_string());
            ui.add(Segmented::new(labels).selected(state.axis_width).on_select(|i| send(Msg::AxisWidth(i))))
                .id("axis-width");
        });
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let blocks = if state.empty { Vec::new() } else { day() };
        let mut timeline = Timeline::new(blocks).disabled(state.disabled);
        if state.axis {
            timeline = timeline.axis();
        }
        if state.readout {
            timeline = timeline.readout();
        }
        if state.pickable {
            timeline = timeline.selected(state.picked).on_select(|index| send(Msg::Pick(index)));
        }
        if state.zoom {
            timeline = timeline.on_zoom(|from, to| send(Msg::Zoom(from, to)));
        }
        if let Some((from, to)) = state.range {
            timeline = timeline.range(from, to);
        }
        ui.add(timeline).width(Length::Cells(WIDTHS[state.width.min(WIDTHS.len() - 1)])).id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("timeline.width"), |ui| {
            let labels = WIDTHS.map(|w| w.to_string());
            ui.add(Segmented::new(labels).selected(state.width).on_select(|i| send(Msg::Width(i)))).id("width");
        });
        setting(ui, t!("timeline.axis"), |ui| {
            ui.add(toggle(state.axis, |on| send(Msg::Axis(on)))).id("axis");
        });
        setting(ui, t!("timeline.readout"), |ui| {
            ui.add(toggle(state.readout, |on| send(Msg::Readout(on)))).id("readout");
        });
        setting(ui, t!("timeline.pickable"), |ui| {
            ui.add(toggle(state.pickable, |on| send(Msg::Pickable(on)))).id("pickable");
        });
        setting(ui, t!("timeline.zoom"), |ui| {
            ui.add(toggle(state.zoom, |on| send(Msg::Zoomable(on)))).id("zoom");
        });
        setting(ui, t!("timeline.empty"), |ui| {
            ui.add(toggle(state.empty, |on| send(Msg::Empty(on)))).id("empty");
        });
        setting(ui, t!("timeline.disabled"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::showcase_tall;

    fn page() -> qframe::runtime::Harness<Showcase> {
        showcase_tall(Showcase::new(), PAGE, 80)
    }

    #[test]
    fn the_day_is_read_with_the_pointer_and_the_keyboard() {
        let mut h = page();
        let (x, y) = h.find("Rust").expect("the first block names itself");
        h.hover(x, y);
        assert!(h.screen().contains("Rust  09:00–11:10  2 h 10 min"), "hovering reads the block:\n{}", h.screen());
        h.click(x, y);
        assert_eq!(h.app().pages.timeline.picked, Some(0));
        assert!(h.is_focused("day"), "a click leaves the keyboard on the day");
        h.press("right");
        assert_eq!(h.app().pages.timeline.picked, Some(1), "the arrow keys walk the blocks in time order");
        h.hover(0, 0);
        assert!(h.screen().contains("Review  11:30–12:15  45 min"), "{}", h.screen());
    }

    #[test]
    fn the_day_zooms_and_says_what_it_shows() {
        let mut h = page();
        assert!(h.screen().contains("Showing the whole day"), "{}", h.screen());
        let (x, y) = h.find("Rust").expect("drawn");
        h.click(x, y);
        h.press("+");
        assert!(h.app().pages.timeline.range.is_some(), "plus asks for a closer range");
        assert!(h.screen().contains("Showing 05:02–17:02"), "{}", h.screen());
        h.press("0");
        assert_eq!(h.app().pages.timeline.range, None, "0 is the whole day again");
    }

    #[test]
    fn the_night_keeps_sleep_whole_and_the_alarm_in_its_own_lane() {
        let h = page();
        let screen = h.screen();
        assert!(screen.contains("Sleep"), "the night names its blocks:\n{screen}");
        let (_, sleep_row) = h.find("Sleep").expect("drawn");
        let lanes = screen.lines().nth(usize::try_from(sleep_row + 2).unwrap_or(0)).unwrap_or_default();
        assert!(lanes.contains("00:00"), "the hours run on past midnight under the two lanes:\n{screen}");
    }

    #[test]
    fn the_playground_designs_the_empty_and_the_disabled_day() {
        let mut h = page();
        h.send(send(Msg::Empty(true)));
        assert!(h.screen().contains("Nothing on this day"), "{}", h.screen());
        h.send(send(Msg::Empty(false)));
        h.send(send(Msg::Disabled(true)));
        h.send(send(Msg::Width(2)));
        assert!(h.app().pages.timeline.disabled);
        let log = h.app().log.recent(PAGE, 1).first().map(|entry| entry.message.clone());
        assert_eq!(log.as_deref(), Some("width = 16"));
    }

    #[test]
    fn a_session_across_midnight_reads_open_on_both_days() {
        let mut h = page();
        h.send(send(Msg::PickYesterday(1)));
        assert!(
            h.screen().contains("Rust  22:40–01:50  continues next day  3 h 10 min"),
            "yesterday's strip ends the session open:\n{}",
            h.screen()
        );
        h.send(send(Msg::PickToday(0)));
        assert!(h.screen().contains("Rust  00:00–01:50  from the previous day  1 h 50 min"), "{}", h.screen());
        h.send(send(Msg::PickToday(2)));
        assert!(h.screen().contains("Rust  13:30–16:20  running  2 h 50 min"), "{}", h.screen());
        h.set_locale("tr");
        assert!(h.screen().contains("Rust  13:30–16:20  sürüyor  2 sa 50 dk"), "{}", h.screen());
    }

    #[test]
    fn the_axes_thin_their_labels_when_narrow() {
        let mut h = page();
        assert!(h.screen().contains("Wednesday"), "wide axes write whole names:\n{}", h.screen());
        h.send(send(Msg::AxisWidth(2)));
        let screen = h.screen();
        assert!(!screen.contains("Wednesday") && screen.contains("Mo"), "narrow axes shorten:\n{screen}");
    }
}
