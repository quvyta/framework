//! Bar chart: memory per service, hours per day stacked, two weeks grouped, and a limit that
//! colours a bar.

use qframe::prelude::*;
use qframe::widgets::{Bar, BarChart, Legend, Segmented, Series};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "bar-chart";

/// Services with their memory in mebibytes.
const SERVICES: [(&str, f32); 5] = [
    ("api-gateway", 412.0),
    ("postgres-16", 1_536.0),
    ("redis-cache", 268.0),
    ("image-resizer", 1_890.0),
    ("worker", 734.0),
];

/// Memory limit per service; bars above it take the danger tone.
const LIMIT: f32 = 1_800.0;

/// The working days the two series charts are built over.
const DAYS: [&str; 5] = ["mon", "tue", "wed", "thu", "fri"];

/// Hours given to each kind of work, day by day.
const FOCUS: [(&str, [f32; 5]); 3] =
    [("rust", [2.5, 3.0, 1.5, 4.0, 2.0]), ("docs", [1.0, 0.5, 2.0, 1.0, 1.5]), ("review", [0.5, 1.5, 1.0, 0.5, 2.5])];

/// The palette tone each kind of work keeps on every chart, whichever kinds a week shows.
const TONES: [(&str, usize); 3] = [("rust", 0), ("docs", 1), ("review", 2)];

/// Hours of the week before, when no Rust was written: only docs and review.
const LAST_WEEK: [(&str, [f32; 5]); 2] = [("docs", [2.0, 1.5, 3.0, 1.0, 2.5]), ("review", [1.0, 2.0, 0.5, 2.5, 1.0])];

/// Deploys of this week and of the week before, day by day.
const WEEKS: [(&str, [f32; 5]); 2] =
    [("this-week", [4.0, 7.0, 5.0, 9.0, 3.0]), ("last-week", [2.0, 5.0, 8.0, 4.0, 6.0])];

/// Playground settings.
#[derive(Debug, Default)]
pub struct State {
    vertical: bool,
    limit: bool,
    disabled: bool,
    layout: usize,
    width: usize,
    /// The day picked in the stacked chart.
    day: Option<usize>,
    /// The category picked in the playground chart.
    picked: Option<usize>,
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    Vertical(bool),
    Limit(bool),
    Disabled(bool),
    Layout(usize),
    Width(usize),
    Day(usize),
    Pick(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::BarChart(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Vertical(on) => {
            state.vertical = on;
            log.push(PAGE, "Playground", format!("vertical = {on}"));
        }
        Msg::Limit(on) => {
            state.limit = on;
            log.push(PAGE, "Playground", format!("limit = {on}"));
        }
        Msg::Disabled(on) => {
            state.disabled = on;
            log.push(PAGE, "Playground", format!("disabled = {on}"));
        }
        Msg::Layout(index) => {
            state.layout = index;
            log.push(PAGE, "Playground", format!("layout = {}", LAYOUTS[index]));
        }
        Msg::Width(index) => {
            state.width = index;
            log.push(PAGE, "Playground", format!("width = {}", WIDTHS[index]));
        }
        Msg::Day(day) => {
            state.day = Some(day);
            log.push(PAGE, "BarChart#focus", format!("selected {}", DAYS[day]));
        }
        Msg::Pick(category) => {
            state.picked = Some(category);
            log.push(PAGE, "BarChart#configured", format!("selected {category}"));
        }
    }
    Command::none()
}

/// Widths the playground offers, in cells.
const WIDTHS: [u16; 3] = [72, 40, 20];

/// Layouts the playground offers: plain bars, stacked series, grouped series.
const LAYOUTS: [&str; 3] = ["bars", "stacked", "grouped"];

/// The services as bars, the ones over the limit in the danger tone when `limit` is on.
fn bars(limit: bool) -> Vec<Bar> {
    SERVICES
        .iter()
        .map(|(name, mebibytes)| {
            // region: bars
            let bar = Bar::new(*name, *mebibytes).value_text(format!("{mebibytes:.0} MiB"));
            if limit && *mebibytes > LIMIT { bar.variant("danger") } else { bar }
            // endregion
        })
        .collect()
}

/// The working days in the active language.
fn days() -> Vec<String> {
    DAYS.iter().map(|day| t!(&format!("bar-chart.{day}"))).collect()
}

/// `table` as series named in the active language.
fn series<const N: usize>(table: &[(&str, [f32; 5]); N]) -> Vec<Series> {
    table.iter().map(|(key, values)| Series::new(t!(&format!("bar-chart.{key}")), *values)).collect()
}

/// The hours of one day, named kind by kind, so a stack reads as text as well as tone. The hours
/// are written the way the chart writes them: a whole number without its decimal.
fn shares(day: usize) -> String {
    let hour = t!("bar-chart.hour");
    let parts: Vec<String> = FOCUS
        .iter()
        .map(|(key, hours)| {
            let value = hours[day];
            let value = if value.fract() == 0.0 { format!("{value:.0}") } else { format!("{value:.1}") };
            format!("{} {value} {hour}", t!(&format!("bar-chart.{key}")))
        })
        .collect();
    parts.join("  ")
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("bar-chart.memory")).gap(0), |ui| {
        ui.add(Text::new(t!("bar-chart.memory-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: horizontal-bars
        ui.add(BarChart::new(bars(true))).width(Length::Cells(72));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("bar-chart.week")).gap(0), |ui| {
        ui.add(Text::new(t!("bar-chart.week-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let days = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
        let deploys = [4.0, 7.0, 5.0, 9.0, 3.0, 0.0, 1.0];
        // region: vertical-bars
        let week = days.iter().zip(deploys).map(|(day, count)| Bar::new(t!(&format!("bar-chart.{day}")), count));
        ui.add(BarChart::new(week).vertical()).width(Length::Cells(56)).height(Length::Cells(9));
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("bar-chart.focus")).gap(0), |ui| {
        ui.add(Text::new(t!("bar-chart.focus-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: stacked-series
        let chart = BarChart::series(days(), series(&FOCUS))
            .stacked()
            .unit(t!("bar-chart.hour"))
            .selected(state.day)
            .on_select(|day| send(Msg::Day(day)));
        ui.add(chart).width(Length::Cells(72)).height(Length::Cells(9)).id("focus");
        // endregion
        let picked = match state.day {
            Some(day) => t!("bar-chart.focus-day", day = days()[day].as_str(), shares = shares(day).as_str()),
            None => t!("bar-chart.focus-none"),
        };
        ui.add(Text::new(picked).role("faint"));
        ui.add(Text::new(t!("bar-chart.keys")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("bar-chart.tones")).gap(0), |ui| {
        ui.add(Text::new(t!("bar-chart.tones-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: fixed-tones
        let tone = |key: &str| TONES.iter().find(|(name, _)| *name == key).map_or(0, |(_, tone)| *tone);
        for (title, table) in [(t!("bar-chart.this-week"), &FOCUS[..]), (t!("bar-chart.last-week"), &LAST_WEEK[..])] {
            let pinned: Vec<Series> = table
                .iter()
                .map(|(key, hours)| Series::new(t!(&format!("bar-chart.{key}")), *hours).tone(tone(key)))
                .collect();
            let names = table.iter().map(|(key, _)| t!(&format!("bar-chart.{key}")));
            ui.add(Text::new(title).role("faint"));
            ui.add(BarChart::series(days(), pinned).stacked().unit(t!("bar-chart.hour")).gap(0))
                .width(Length::Cells(72));
            ui.add(Legend::new(names).tones(table.iter().map(|(key, _)| tone(key)))).width(Length::Cells(72));
            ui.spacer().height(Length::Cells(1));
        }
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("bar-chart.weeks")).gap(0), |ui| {
        ui.add(Text::new(t!("bar-chart.weeks-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: grouped-series
        let chart = BarChart::series(days(), series(&WEEKS)).vertical();
        ui.add(chart).width(Length::Cells(72)).height(Length::Cells(9));
        // endregion
        ui.add(Text::new(t!("bar-chart.weeks-legend")).role("faint"));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        // region: configured
        let mut chart = if state.layout == 0 {
            BarChart::new(bars(state.limit))
        } else {
            let chart = BarChart::series(days(), series(&FOCUS)).unit(t!("bar-chart.hour"));
            if state.layout == 1 { chart.stacked() } else { chart }
        };
        if state.vertical {
            chart = chart.vertical();
        }
        chart = chart.disabled(state.disabled).selected(state.picked).on_select(|category| send(Msg::Pick(category)));
        let height = if state.vertical { Length::Cells(9) } else { Length::Auto };
        ui.add(chart).width(Length::Cells(WIDTHS[state.width])).height(height).id("configured");
        // endregion
        ui.spacer().height(Length::Cells(1));
        setting(ui, t!("bar-chart.width"), |ui| {
            let labels = WIDTHS.map(|w| w.to_string());
            ui.add(Segmented::new(labels).selected(state.width).on_select(|i| send(Msg::Width(i)))).id("width");
        });
        setting(ui, t!("bar-chart.layout"), |ui| {
            let labels = LAYOUTS.map(|name| t!(&format!("bar-chart.{name}")));
            ui.add(Segmented::new(labels).selected(state.layout).on_select(|i| send(Msg::Layout(i)))).id("layout");
        });
        setting(ui, t!("bar-chart.vertical"), |ui| {
            ui.add(toggle(state.vertical, |on| send(Msg::Vertical(on)))).id("vertical");
        });
        setting(ui, t!("bar-chart.limit"), |ui| {
            ui.add(toggle(state.limit, |on| send(Msg::Limit(on)))).id("limit");
        });
        setting(ui, t!("bar-chart.disabled"), |ui| {
            ui.add(toggle(state.disabled, |on| send(Msg::Disabled(on)))).id("disabled");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::{click_text_below, showcase_on, showcase_tall};

    /// The page shows six panels, so its tests need a terminal taller than the default.
    fn page() -> qframe::runtime::Harness<Showcase> {
        showcase_tall(Showcase::new(), PAGE, 110)
    }

    #[test]
    fn services_over_the_limit_carry_a_marker() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("● 1890 MiB"), "{}", h.screen());
        h.send(send(Msg::Vertical(true)));
        h.send(send(Msg::Width(2)));
        h.send(send(Msg::Limit(true)));
        assert!(h.app().pages.bar_chart.vertical);
        assert!(h.screen().contains("Thu"), "{}", h.screen());
    }

    #[test]
    fn the_stacked_chart_names_its_shares_and_answers_the_keyboard() {
        let mut h = page();
        let screen = h.screen();
        assert!(screen.contains("Rust") && screen.contains("Review"), "the segments name themselves:\n{screen}");
        assert!(screen.contains("No day picked yet"), "{screen}");
        // The days name the rows of the stacked chart, below its own title.
        let title = h.find("HOURS PER DAY").expect("the stacked panel has a title").1;
        click_text_below(&mut h, "Thu", title);
        assert_eq!(h.app().pages.bar_chart.day, Some(3));
        let screen = h.screen();
        assert!(screen.contains("Thu: Rust 4 h  Docs 1 h  Review 0.5 h"), "{screen}");
        h.press("up");
        assert_eq!(h.app().pages.bar_chart.day, Some(2), "the arrow keys walk the days");
        let last = h.app().log.recent(PAGE, 1).first().map(|entry| entry.message.clone());
        assert_eq!(last.as_deref(), Some("selected wed"));
    }

    #[test]
    fn a_kind_of_work_keeps_its_tone_when_another_is_missing() {
        let h = showcase_tall(Showcase::new(), PAGE, 120);
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        let title = lines.iter().position(|line| line.contains("THE SAME COLOUR EVERY WEEK")).expect("on screen");
        // The first day's bar sits on the row under each caption; the docs segment names itself.
        let docs_cell = |caption: &str| {
            let y = lines.iter().skip(title).position(|line| line.trim_end().ends_with(caption)).expect("drawn")
                + title
                + 1;
            let x = lines[y].find("Docs").unwrap_or_else(|| panic!("docs is named on row {y}:\n{screen}"));
            h.bg(u16::try_from(lines[y][..x].chars().count()).unwrap_or(0), u16::try_from(y).unwrap_or(0))
        };
        let docs = Some(h.env().theme().series_color(1));
        assert_eq!(docs_cell("This week"), docs, "this week docs takes tone 1, after rust:\n{screen}");
        assert_eq!(docs_cell("Last week"), docs, "last week, with no rust, it keeps tone 1:\n{screen}");
    }

    #[test]
    fn the_playground_switches_between_bars_stacks_and_groups() {
        let mut h = page();
        h.send(send(Msg::Layout(1)));
        assert!(h.screen().contains("5.5 h"), "a stack shows the day's total:\n{}", h.screen());
        h.send(send(Msg::Layout(2)));
        let screen = h.screen();
        assert!(screen.contains("Rust 2.5 h") && screen.contains("Docs 1 h"), "a group names every series:\n{screen}");
        h.send(send(Msg::Disabled(true)));
        h.send(send(Msg::Layout(0)));
        assert!(h.screen().contains("1890 MiB"), "{}", h.screen());
    }
}
