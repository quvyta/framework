//! Date and time: the local day and clock next to UTC, the two monotonic clocks that tell work
//! from sleep, how long no input reached the terminal, and ISO dates read from text.

use std::time::Duration;

use qframe::date::{Date, DateTime, local_offset};
use qframe::prelude::*;
use qframe::uptime::Uptime;
use qframe::widgets::{Segmented, TextInput};

use super::{PageMsg, setting};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "date-time";

/// Offsets the playground writes one instant in: the machine's own, UTC, Istanbul and Los
/// Angeles in summer.
const OFFSETS: [i16; 4] = [0, 0, 180, -420];

/// The silences, in seconds, the playground can ask to be told about.
const SILENCES: [u32; 3] = [5, 30, 60];

/// The clock readings the page compares, and the text typed into the date field.
#[derive(Debug)]
pub struct State {
    /// The reading the measurement starts from.
    started: Uptime,
    text: String,
    offset: usize,
    /// Which of [`SILENCES`] the idleness watch waits for.
    silence: usize,
    /// Whether the watch was told the silence began and has not been told it ended.
    away: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { started: Uptime::now(), text: "2026-09-17".to_owned(), offset: 0, silence: 0, away: false }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Restart,
    Text(String),
    Offset(usize),
    Silence(usize),
    Idle(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::DateTime(message))
}

/// An offset from UTC as `+03:00`, the way a date and time are written down.
fn written_offset(minutes: i16) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let minutes = minutes.unsigned_abs();
    format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

/// A duration as `hh:mm:ss`, with hours running past a day.
fn written_duration(seconds: u64) -> String {
    format!("{:02}:{:02}:{:02}", seconds / 3_600, seconds / 60 % 60, seconds % 60)
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Restart => {
            state.started = Uptime::now();
            log.push(PAGE, "Uptime::now", "the measurement starts again");
        }
        Msg::Text(text) => {
            let result = match Date::parse(&text) {
                Ok(date) => format!("{date} is a {:?}", date.weekday()),
                Err(error) => error,
            };
            state.text = text;
            log.push(PAGE, "Date::parse", result);
        }
        Msg::Offset(index) => {
            state.offset = index.min(OFFSETS.len() - 1);
            let minutes = if state.offset == 0 { qframe::date::local_offset_minutes() } else { OFFSETS[state.offset] };
            log.push(PAGE, "DateTime::from_unix", format!("written at {}", written_offset(minutes)));
        }
        Msg::Silence(index) => {
            state.silence = index.min(SILENCES.len() - 1);
            // A new length is a new watch: it has not told anything yet.
            state.away = false;
            log.push(PAGE, "View::on_idle", format!("waits for {} s of silence", SILENCES[state.silence]));
        }
        Msg::Idle(away) => {
            state.away = away;
            let told = if away {
                format!("{} s without input", SILENCES[state.silence])
            } else {
                "input came back".to_owned()
            };
            log.push(PAGE, "View::on_idle", told);
        }
    }
    Command::none()
}

/// A row of the demo: a faint label and a value that stays in its column.
fn row(ui: &mut View<'_, AppMsg>, label: String, value: String) {
    ui.row(|ui| {
        ui.add(Text::new(label).role("faint").no_wrap()).width(Length::Cells(22));
        ui.add(Text::new(value).role("body").no_wrap());
    })
    .fill_width();
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    // region: date-time-local
    let now = DateTime::now_local();
    let utc = DateTime::from_unix(now.to_unix(), 0);
    let offset = match local_offset() {
        Some(minutes) => written_offset(minutes),
        None => t!("date-time.offset-unknown"),
    };
    // endregion

    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        row(ui, t!("date-time.local"), format!("{}  {}  {offset}", now.date, now.time));
        row(ui, t!("date-time.utc"), format!("{}  {}", utc.date, utc.time));
        ui.spacer().height(Length::Cells(1));

        // region: date-time-clock
        let reading = Uptime::now();
        let worked = reading.awake.saturating_sub(state.started.awake).as_secs();
        let asleep = reading.suspended_since(&state.started).as_secs();
        // endregion
        row(ui, t!("date-time.awake"), written_duration(reading.awake.as_secs()));
        row(ui, t!("date-time.since"), written_duration(worked));
        row(ui, t!("date-time.asleep"), written_duration(asleep));
        let icons = ui.env().icons();
        let (glyph, text, color) = if Uptime::detects_suspend() {
            (icons.glyph("success").into_owned(), t!("date-time.detected"), "success")
        } else {
            (icons.glyph("warning").into_owned(), t!("date-time.undetected"), "warning")
        };
        ui.row(|ui| {
            ui.add(Text::new(glyph).color(color).no_wrap());
            ui.add(Text::new(text).role("secondary")).fill_width();
        })
        .gap(1);
        ui.spacer().height(Length::Cells(1));
        ui.add(Button::new(t!("date-time.restart")).on_press(send(Msg::Restart))).id("restart");
        ui.spacer().height(Length::Cells(1));

        setting(ui, t!("date-time.typed"), |ui| {
            ui.add(TextInput::new(&state.text).placeholder("2026-09-17").on_change(|text| send(Msg::Text(text))))
                .width(Length::Cells(18))
                .id("typed");
        });
        // region: date-time-iso
        let (mark, message, color) = match Date::parse(&state.text) {
            Ok(date) => ("success", t!("date-time.read", date = date.to_string(), n = date.to_days()), "success"),
            Err(error) => ("warning", error, "warning"),
        };
        // endregion
        ui.row(|ui| {
            ui.add(Text::new(ui.env().icons().glyph(mark).into_owned()).color(color).no_wrap());
            ui.add(Text::new(message).role("secondary")).fill_width();
        })
        .gap(1);
    })
    .fill_width();

    idleness(state, ui);

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        let labels = [t!("date-time.here"), "UTC".to_owned(), written_offset(180), written_offset(-420)];
        setting(ui, t!("date-time.offset"), |ui| {
            ui.add(Segmented::new(labels).selected(state.offset).on_select(|index| send(Msg::Offset(index))))
                .id("offset");
        });
        // region: date-time-offset
        let minutes = if state.offset == 0 { now.offset_minutes } else { OFFSETS[state.offset] };
        let there = DateTime::from_unix(now.to_unix(), minutes);
        // endregion
        row(ui, t!("date-time.same-instant"), format!("{}  {}  {}", there.date, there.time, written_offset(minutes)));
        row(ui, t!("date-time.unix"), there.to_unix().to_string());
        ui.add(Text::new(t!("date-time.hint")).role("faint"));
        ui.spacer().height(Length::Cells(1));
        let labels = SILENCES.map(|seconds| t!("date-time.seconds", n = seconds));
        setting(ui, t!("date-time.silence"), |ui| {
            ui.add(Segmented::new(labels).selected(state.silence).on_select(|index| send(Msg::Silence(index))))
                .id("silence");
        });
    })
    .fill_width();
}

/// How long no input reached this terminal, and a watch told when a silence begins and ends.
fn idleness(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("date-time.idleness")).gap(0), |ui| {
        ui.add(Text::new(t!("date-time.idleness-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: date-time-idle
        // Reading it redraws the page as the seconds pass; the watch wakes it once, at the moment.
        let idle = ui.idle_for();
        let after = Duration::from_secs(u64::from(SILENCES[state.silence]));
        ui.on_idle(after, |away| send(Msg::Idle(away)));
        // endregion
        row(ui, t!("date-time.idle"), written_duration(idle.as_secs()));
        let seconds = SILENCES[state.silence];
        let (glyph, text, color) = if state.away {
            ("dot-outline", t!("date-time.away", n = seconds), "muted")
        } else {
            ("dot", t!("date-time.here-now", n = seconds), "success")
        };
        let glyph = ui.env().icons().glyph(glyph).into_owned();
        ui.row(|ui| {
            ui.add(Text::new(glyph).color(color).no_wrap());
            ui.add(Text::new(text).role("secondary")).fill_width();
        })
        .gap(1);
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn the_local_day_and_utc_are_shown_side_by_side() {
        let h = showcase_on(PAGE);
        let screen = h.screen();
        let today = Date::today_local();
        assert!(screen.contains(&today.to_string()), "{screen}");
        assert!(screen.contains(&Date::today_utc().to_string()), "{screen}");
        let offset = local_offset().map_or_else(|| "not known".to_owned(), written_offset);
        assert!(screen.contains(&offset), "the offset `{offset}` is shown:\n{screen}");
    }

    #[test]
    fn the_clocks_report_whether_they_can_tell_sleep_apart() {
        let h = showcase_on(PAGE);
        let screen = h.screen();
        let expected = if Uptime::detects_suspend() { "tells sleep from work" } else { "cannot tell sleep from work" };
        assert!(screen.contains(expected), "{screen}");
        // Nothing slept while the test ran, so the sleep row is zero and the awake clock is not.
        assert!(screen.contains("00:00:00"), "{screen}");
        let awake = Uptime::now().awake.as_secs();
        assert!(screen.contains(&written_duration(awake)) || awake > 0, "{screen}");
    }

    #[test]
    fn typing_a_date_reads_it_or_says_what_is_wrong() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Text("2026-02-30".to_owned())));
        let screen = h.screen();
        assert!(screen.contains("not a day this calendar has"), "{screen}");
        assert!(h.app().log.recent(PAGE, 1).first().is_some_and(|entry| entry.source == "Date::parse"));

        h.send(send(Msg::Text("2024-02-29".to_owned())));
        assert!(h.screen().contains("2024-02-29 is 19782 days after the epoch"), "{}", h.screen());
        let logged = h.app().log.recent(PAGE, 1).first().map(|entry| entry.message.clone());
        assert_eq!(logged.as_deref(), Some("2024-02-29 is a Thursday"));

        h.send(send(Msg::Text("2026-9-17".to_owned())));
        assert!(h.screen().contains("2 digits"), "{}", h.screen());
    }

    #[test]
    fn the_playground_writes_one_instant_in_another_offset() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Offset(1)));
        let utc = h.screen();
        assert!(utc.contains("+00:00"), "{utc}");
        h.send(send(Msg::Offset(2)));
        let istanbul = h.screen();
        assert!(istanbul.contains("+03:00"), "{istanbul}");
        h.send(send(Msg::Offset(3)));
        assert!(h.screen().contains("-07:00"), "{}", h.screen());
        // The same instant, whatever it is written in.
        let instant = |screen: &str| {
            let row = screen.lines().find(|line| line.contains("Seconds since epoch")).expect("the instant row");
            row.split_whitespace().last().and_then(|word| word.parse::<i64>().ok()).expect("the second itself")
        };
        // The page reads the live clock, and the two screens are two frames drawn a moment apart,
        // so a second may change between them; the offset must not move the instant any further.
        assert!((instant(&istanbul) - instant(&utc)).abs() <= 1, "{} then {}", instant(&utc), instant(&istanbul));
        h.send(send(Msg::Offset(9)));
        assert_eq!(h.app().pages.date_time.offset, OFFSETS.len() - 1, "an offset that is not there is clamped");
    }

    /// The row the idleness demo writes the silence in.
    fn silence(h: &qframe::runtime::Harness<crate::app::Showcase>) -> String {
        let screen = h.screen();
        let row = screen.lines().find(|line| line.contains("Since the last input"));
        row.expect("the idleness row").split_whitespace().last().unwrap_or_default().to_owned()
    }

    fn last_log(h: &qframe::runtime::Harness<crate::app::Showcase>) -> Option<(String, String)> {
        h.app().log.recent(PAGE, 1).first().map(|entry| (entry.source.clone(), entry.message.clone()))
    }

    #[test]
    fn idleness_counts_the_silence_and_the_watch_is_told_when_it_begins_and_ends() {
        let mut h = crate::tests::showcase_tall(crate::app::Showcase::new(), PAGE, 90);
        h.hover(1, 1);
        assert_eq!(silence(&h), "00:00:00", "{}", h.screen());
        h.advance(std::time::Duration::from_secs(3));
        assert_eq!(silence(&h), "00:00:03");
        assert!(!h.app().pages.date_time.away);
        h.resize(crate::tests::SIZE.0, 91);
        assert_eq!(silence(&h), "00:00:03", "a resize is not the user");

        h.advance(std::time::Duration::from_secs(2));
        assert!(h.app().pages.date_time.away, "told at 5 s");
        assert!(h.screen().contains("Away for 5 s or more"), "{}", h.screen());
        assert_eq!(last_log(&h), Some(("View::on_idle".to_owned(), "5 s without input".to_owned())));

        h.hover(2, 1);
        assert!(!h.app().pages.date_time.away);
        assert_eq!(silence(&h), "00:00:00");
        assert_eq!(last_log(&h), Some(("View::on_idle".to_owned(), "input came back".to_owned())));

        h.send(send(Msg::Silence(1)));
        assert!(h.screen().contains("After 30 s without input"), "{}", h.screen());
        h.advance(std::time::Duration::from_secs(29));
        assert!(!h.app().pages.date_time.away, "the longer watch waits");
        h.advance(std::time::Duration::from_secs(1));
        assert!(h.app().pages.date_time.away);
    }

    #[test]
    fn restarting_the_measurement_starts_the_count_again() {
        let mut h = showcase_on(PAGE);
        let before = h.app().pages.date_time.started;
        h.click_text("Start the measurement again");
        assert!(h.app().pages.date_time.started.awake >= before.awake);
        assert_eq!(
            h.app().log.recent(PAGE, 1).first().map(|entry| entry.source.clone()).as_deref(),
            Some("Uptime::now")
        );
    }
}
