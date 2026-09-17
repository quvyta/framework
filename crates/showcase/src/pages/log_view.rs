//! Log view: a deploy streaming its log, level filters, search and a burst of many lines.

use std::time::Duration;

use qframe::prelude::*;
use qframe::widgets::{LogBuffer, LogLevel, LogLine, LogView, Select, TextInput};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "log-view";

/// Lines kept before the oldest ones fall out.
const CAPACITY: usize = 50_000;

/// Lines added by the burst button.
const BURST: usize = 10_000;

/// Time between streamed lines.
const INTERVAL: Duration = Duration::from_millis(350);

/// What a deploy says, in order; the stream repeats it.
const SCRIPT: [(LogLevel, &str); 16] = [
    (LogLevel::Info, "deploy 2.4.1 started by ci"),
    (LogLevel::Debug, "resolving image quvyta/api:2.4.1"),
    (LogLevel::Info, "pulling layer 3 of 7, 48.2 MiB"),
    (LogLevel::Trace, "registry answered 200 in 84 ms"),
    (LogLevel::Info, "image ready, digest sha256:9f2c41e0"),
    (LogLevel::Info, "stopping quvyta-api-1"),
    (LogLevel::Warn, "quvyta-api-1 took 6.2 s to stop, above the 5 s budget"),
    (LogLevel::Info, "starting quvyta-api-1 on port 8080"),
    (LogLevel::Debug, "health check 1: connection refused"),
    (LogLevel::Debug, "health check 2: 503 service unavailable"),
    (LogLevel::Info, "health check 3: 200 ok"),
    (LogLevel::Error, "migration 0042_add_tokens failed: column token already exists"),
    (LogLevel::Warn, "rolling back migration 0042"),
    (LogLevel::Info, "migration 0042 skipped, schema already current"),
    (LogLevel::Info, "routing traffic to quvyta-api-1"),
    (LogLevel::Info, "deploy 2.4.1 finished in 41 s"),
];

/// Level filter choices; `None` shows everything.
const LEVELS: [Option<LogLevel>; 5] =
    [None, Some(LogLevel::Debug), Some(LogLevel::Info), Some(LogLevel::Warn), Some(LogLevel::Error)];

/// The buffer, the stream and the filters.
#[derive(Debug)]
pub struct State {
    buffer: LogBuffer,
    written: usize,
    streaming: bool,
    /// Bumped whenever streaming stops, so a tick from an older stream ends it.
    generation: u64,
    level: usize,
    search: String,
}

impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            buffer: LogBuffer::new(CAPACITY),
            written: 0,
            streaming: false,
            generation: 0,
            level: 0,
            search: String::new(),
        };
        for _ in 0..SCRIPT.len() {
            state.write_line();
        }
        state
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Tick(u64),
    Streaming(bool),
    Burst,
    Clear,
    Level(usize),
    Search(String),
    Copied(usize),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::LogView(message))
}

impl State {
    // region: log-push
    fn write_line(&mut self) {
        let (level, text) = SCRIPT[self.written % SCRIPT.len()];
        let millis = 14 * 3_600_000 + 2 * 60_000 + self.written * 137;
        let time = format!(
            "{:02}:{:02}:{:02}.{:03}",
            millis / 3_600_000 % 24,
            millis / 60_000 % 60,
            millis / 1000 % 60,
            millis % 1000
        );
        self.buffer.push(LogLine::new(level, text).time(time));
        self.written += 1;
    }
    // endregion
}

// region: log-stream
/// Waits one interval on a background thread, then asks for the next line.
fn next_tick(generation: u64) -> Command<AppMsg> {
    Command::perform(move || {
        std::thread::sleep(INTERVAL);
        send(Msg::Tick(generation))
    })
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Tick(generation) if state.streaming && generation == state.generation => {
            state.write_line();
            return next_tick(generation);
        }
        Msg::Tick(_) => {}
        Msg::Streaming(on) => {
            state.streaming = on;
            state.generation += 1;
            log.push(PAGE, "Playground", format!("streaming = {on}"));
            if on {
                return next_tick(state.generation);
            }
        }
        // endregion
        Msg::Burst => {
            for _ in 0..BURST {
                state.write_line();
            }
            log.push(PAGE, "Button#burst", format!("{} lines kept", state.buffer.len()));
        }
        Msg::Clear => {
            state.buffer.clear();
            log.push(PAGE, "Button#clear", "cleared");
        }
        Msg::Level(index) => {
            state.level = index;
            log.push(PAGE, "Playground", format!("level = {index}"));
        }
        Msg::Search(text) => state.search = text,
        Msg::Copied(lines) => {
            let plural = if lines == 1 { "" } else { "s" };
            log.push(PAGE, "LogView#deploy", format!("copied {lines} line{plural}"));
        }
    }
    Command::none()
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        // region: log-view
        let mut view = LogView::new(&state.buffer)
            .search(state.search.clone())
            .empty_text(t!("log-view.empty"))
            .on_copy(|lines| send(Msg::Copied(lines)));
        if let Some(level) = LEVELS[state.level] {
            view = view.min_level(level);
        }
        ui.add(view).width(Length::Fill(1)).height(Length::Cells(12)).id("deploy");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("log-view.streaming"), |ui| {
            ui.add(toggle(state.streaming, |on| send(Msg::Streaming(on)))).id("streaming");
        });
        setting(ui, t!("log-view.level"), |ui| {
            let names = LEVELS.map(|level| t!(&format!("log-view.level-{}", level.map_or("all", LogLevel::name))));
            ui.add(Select::new(names).selected(Some(state.level)).on_select(|i| send(Msg::Level(i))))
                .width(Length::Cells(22))
                .id("level");
        });
        setting(ui, t!("log-view.search"), |ui| {
            ui.add(
                TextInput::new(&state.search)
                    .placeholder(t!("log-view.search-placeholder"))
                    .on_change(|text| send(Msg::Search(text))),
            )
            .width(Length::Cells(30))
            .id("search");
        });
        setting(ui, t!("log-view.lines"), |ui| {
            ui.row(|ui| {
                ui.add(Button::new(t!("log-view.burst")).on_press(send(Msg::Burst))).id("burst");
                ui.add(Button::new(t!("log-view.clear")).on_press(send(Msg::Clear))).id("clear");
                ui.add(
                    Text::new(t!("log-view.kept", n = state.buffer.len(), capacity = state.buffer.capacity()))
                        .role("faint")
                        .no_wrap(),
                );
            })
            .gap(2);
        });
        ui.add(Text::new(t!("log-view.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn ticks_write_lines_only_for_the_current_stream() {
        let mut state = State::default();
        let mut log = EventLog::new();
        let tick = Msg::Tick(state.generation);
        let _ = update(&mut state, tick, &mut log);
        assert_eq!(state.written, SCRIPT.len(), "a tick without streaming writes nothing");
        // The command that waits for the next tick is not run here; tests never sleep.
        let _ = update(&mut state, Msg::Streaming(true), &mut log);
        let tick = Msg::Tick(state.generation);
        let _ = update(&mut state, tick, &mut log);
        assert_eq!(state.written, SCRIPT.len() + 1);
        let old = state.generation;
        let _ = update(&mut state, Msg::Streaming(false), &mut log);
        let _ = update(&mut state, Msg::Tick(old), &mut log);
        assert_eq!(state.written, SCRIPT.len() + 1, "a stopped stream ignores its ticks");
    }

    #[test]
    fn filters_searches_and_bursts() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("deploy 2.4.1 finished in 41 s"), "{}", h.screen());
        h.send(send(Msg::Level(4)));
        assert!(!h.screen().contains("health check"), "{}", h.screen());
        assert!(h.screen().contains("column token already exists"));
        h.send(send(Msg::Level(0)));
        h.send(send(Msg::Search("health".to_owned())));
        assert!(h.screen().contains("health check 3"));
        h.send(send(Msg::Search(String::new())));
        h.send(send(Msg::Burst));
        assert_eq!(h.app().pages.log_view.buffer.len(), BURST + SCRIPT.len());
        h.send(send(Msg::Clear));
        assert!(h.screen().contains("Waiting for the deploy"), "{}", h.screen());
    }

    #[test]
    fn a_right_click_copies_a_line_clean_or_raw_and_logs_it() {
        use qframe::event::{MouseButton, MouseKind};

        let mut h = showcase_on(PAGE);
        h.set_reduced_motion(true);
        let (x, y) = h.find("deploy 2.4.1 finished in 41 s").expect("log on screen");
        h.mouse(MouseKind::Down(MouseButton::Right), x, y).mouse(MouseKind::Up(MouseButton::Right), x, y);
        crate::tests::click_text_below(&mut h, "Raw copy", 0);
        let raw = h.clipboard().unwrap_or_default().to_owned();
        assert!(raw.contains("  info   deploy 2.4.1 finished in 41 s"), "columns as shown: {raw:?}");
        let log = h.app().log.recent(PAGE, 10);
        assert!(log.iter().any(|entry| entry.message == "copied 1 line"), "{log:?}");
    }
}
