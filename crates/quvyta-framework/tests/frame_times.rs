//! Frame-time benchmark over heavy, application-like screens.
//!
//! Ignored by default so the commit gate stays fast. Run it in release mode, single-threaded so
//! scenarios do not compete for the CPU:
//!
//! ```text
//! cargo test --release -p quvyta-framework --test frame_times -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Every scenario is rendered in every built-in theme at 160 × 48 cells. For each it reports the
//! median and 95th percentile wall-clock time, and on Linux the mean CPU time of the thread, of
//! two kinds of frame:
//!
//! - `repaint`: the same view painted again, the cost of a frame while something animates;
//! - `key`: a key press (↑ or ↓, alternating) handled and painted, the cost of interaction. In the
//!   page transition scenario it is a change of page instead, so every such frame blends a
//!   running transition.
//!
//! The numbers are wall-clock times of [`Harness`] calls, so they include building the view,
//! layout, painting and input routing, but not writing to a terminal. Compare CPU times between
//! runs: wall-clock times grow whenever other processes busy the machine.

use std::sync::Arc;
use std::time::{Duration, Instant};

use qframe::prelude::*;
use qframe::widgets::{
    CardGrid, Column, LogBuffer, LogLevel, LogLine, LogView, Markdown, PageTransition, ScrollView, Table, TableCell,
    TableRow,
};

const WIDTH: u16 = 160;
const HEIGHT: u16 = 48;
const WARMUP: usize = 5;
const SAMPLES: usize = 100;
const THEMES: [&str; 4] = ["monochrome", "iris", "nordic", "amber"];

/// Which heavy screen to show.
#[derive(Clone, Copy)]
enum Scenario {
    List,
    SharedList,
    Table,
    Cards,
    Markdown,
    Log,
    Nested,
    Transition,
}

impl Scenario {
    fn name(self) -> &'static str {
        match self {
            Self::List => "list 100 000 rows",
            Self::SharedList => "list 100 000 rows, shared",
            Self::Table => "table 200 000 rows",
            Self::Cards => "card grid 10 000 cards",
            Self::Markdown => "markdown, long document",
            Self::Log => "log view 10 000 lines",
            Self::Nested => "nested layout, 60 cards",
            Self::Transition => "page transition, sliding",
        }
    }
}

/// A screen like an application page: a title, the heavy widget filling the rest, key hints.
struct Screen {
    scenario: Scenario,
    rows: Arc<[TableRow]>,
    items: Arc<[ListItem]>,
    markdown: String,
    log: LogBuffer,
    selected: Option<usize>,
    /// The page shown in the page transition scenario.
    page: usize,
}

impl Screen {
    fn new(scenario: Scenario) -> Self {
        let rows: Arc<[TableRow]> = match scenario {
            Scenario::Table => (0..200_000)
                .map(|n| {
                    TableRow::new([
                        TableCell::new(format!("build-{n}")),
                        TableCell::new(if n % 7 == 0 { "failed" } else { "passed" }).icon("dot", Some("success")),
                        TableCell::new(format!("{}.{} s", n % 90, n % 10)),
                        TableCell::new("quvyta/api:2.4"),
                        TableCell::new(format!("main@{:08x}", n * 2_654_435_761_usize % 0xFFFF_FFFF)),
                    ])
                })
                .collect(),
            _ => Arc::from(Vec::new()),
        };
        let items: Arc<[ListItem]> = match scenario {
            Scenario::SharedList => (1..=100_000).map(|n| ListItem::new(format!("Row {n}"))).collect(),
            _ => Arc::from(Vec::new()),
        };
        let markdown = match scenario {
            Scenario::Markdown => every_guide(),
            _ => String::new(),
        };
        let mut log = LogBuffer::new(50_000);
        if let Scenario::Log = scenario {
            let levels = [LogLevel::Info, LogLevel::Debug, LogLevel::Warn, LogLevel::Error, LogLevel::Trace];
            for n in 0..10_000 {
                let line =
                    LogLine::new(levels[n % levels.len()], format!("deploy 2.4.{n}: health check {n} answered 200 ok"))
                        .time(format!("12:{:02}:{:02}", n / 60 % 60, n % 60));
                log.push(line);
            }
        }
        Self { scenario, rows, items, markdown, log, selected: None, page: 0 }
    }
}

/// A long Markdown document built in place: headings, paragraphs with inline code and emphasis,
/// lists, quotes and fenced code, repeated to about the size of a full set of component guides.
fn every_guide() -> String {
    let mut text = String::new();
    for section in 0..90 {
        text.push_str(&format!("## Section {section}\n\n"));
        text.push_str(
            "A widget draws itself inside the rectangle it is given, measures the room it needs and \
             answers events it understands. Call `ui.add(widget)` to place one, and chain `.id(\"name\")` \
             to keep its memory across frames. **Themes** decide every colour; *motion* follows the \
             theme's durations.\n\n",
        );
        text.push_str("- Keys: arrows move, Enter activates, Esc closes.\n");
        text.push_str("- Mouse: a click activates, the wheel scrolls, a drag reorders.\n");
        text.push_str("- Themes: styles use `$accent`, `mix($accent, $muted, 40%)` or `pulse(a, b)`.\n\n");
        text.push_str("> Every state is designed: empty, loading, error, narrow and disabled.\n\n");
        text.push_str(
            "```rust\nui.add(List::new(items).selected(state.row).on_select(Msg::Row)).id(\"rows\");\n```\n\n",
        );
    }
    text
}

#[derive(Clone)]
enum Msg {
    Select(usize),
    /// Shows the next page.
    Flip,
}

impl App for Screen {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(index) => self.selected = Some(index),
            Msg::Flip => self.page += 1,
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new(self.scenario.name()).role("title"));
        match self.scenario {
            Scenario::List => {
                // The items are built in `view` every frame, as a simple list page does.
                let items = (1..=100_000).map(|n| ListItem::new(format!("Row {n}")));
                ui.add(List::new(items).selected(self.selected).on_select(Msg::Select)).fill().id("heavy");
            }
            Scenario::SharedList => {
                // The items live in the state and the list shares them.
                let list = List::shared(Arc::clone(&self.items));
                ui.add(list.selected(self.selected).on_select(Msg::Select)).fill().id("heavy");
            }
            Scenario::Table => {
                let columns = [
                    Column::new("Build"),
                    Column::new("Status"),
                    Column::new("Time"),
                    Column::new("Image"),
                    Column::new("Commit"),
                ];
                ui.add(Table::new(columns, Arc::clone(&self.rows)).selected(self.selected).on_select(Msg::Select))
                    .fill()
                    .id("heavy");
            }
            Scenario::Cards => {
                // Only the cards on screen are built, each a column of three texts.
                let grid = CardGrid::new(10_000).selected(self.selected).on_select(Msg::Select).card(|ui, index| {
                    ui.add(Text::new(format!("app-{index}")).role("title").no_wrap());
                    ui.add(Text::new("quvyta/api:2.4 on port 8080, up 3 days").role("secondary").no_wrap());
                    ui.add(Text::new("Repo · Flatpak").role("faint").no_wrap());
                });
                ui.add(grid).fill().id("heavy");
            }
            Scenario::Markdown => {
                ui.add_with(ScrollView::new(), |ui| {
                    ui.add(Markdown::new(&self.markdown)).fill_width();
                })
                .fill()
                .selectable(true)
                .id("heavy");
            }
            Scenario::Log => {
                ui.add(LogView::new(&self.log)).fill().id("heavy");
            }
            Scenario::Nested => cards(ui),
            Scenario::Transition => {
                // An application shell: the page body inside a sliding transition. Two pages
                // take turns, so the remembered state of pages stays bounded.
                let page = format!("page-{}", self.page % 2);
                ui.add_with(PageTransition::new(page.clone()).slide(true), |ui| {
                    ui.page(page, cards);
                })
                .fill();
            }
        }
        ui.add(KeyHints::new().hint("↑↓", "move").hint("end", "last").hint("q", "quit")).fill_width();
    }
}

/// A typical page: sections of cards, each a column of rows, in a scroll view.
fn cards(ui: &mut View<'_, Msg>) {
    ui.add_with(ScrollView::new(), |ui| {
        for section in 0..6 {
            ui.add(Text::new(format!("Section {section}")).role("title"));
            ui.row(|ui| {
                for card in 0..10 {
                    ui.column(|ui| {
                        ui.add(Text::new(format!("container-{section}-{card}")));
                        ui.row(|ui| {
                            ui.add(Text::new("running").role("faint"));
                            ui.add(Button::new("Restart"));
                        })
                        .gap(1);
                        ui.add(Text::new("quvyta/api:2.4 on port 8080, up 3 days")).fill_width();
                    })
                    .padding(Padding::symmetric(0, 1))
                    .width(Length::Cells(30));
                }
            })
            .gap(1);
        }
    })
    .fill()
    .id("heavy");
}

/// What one kind of frame cost.
struct Timing {
    /// Median wall-clock time, in milliseconds.
    p50: f64,
    /// 95th percentile wall-clock time, in milliseconds.
    p95: f64,
    /// Mean CPU time of this thread per frame, in milliseconds, where Linux reports it. Unlike
    /// wall-clock time it barely moves when other processes load the machine, which makes it the
    /// number to compare between runs.
    cpu: Option<f64>,
}

/// Nanoseconds this thread has run on a CPU, from `/proc/thread-self/schedstat` on Linux.
fn thread_cpu_ns() -> Option<u64> {
    std::fs::read_to_string("/proc/thread-self/schedstat").ok()?.split_whitespace().next()?.parse().ok()
}

fn time(mut frame: impl FnMut()) -> Timing {
    for _ in 0..WARMUP {
        frame();
    }
    let cpu_before = thread_cpu_ns();
    let mut samples: Vec<Duration> = (0..SAMPLES)
        .map(|_| {
            let started = Instant::now();
            frame();
            started.elapsed()
        })
        .collect();
    let cpu = cpu_before.zip(thread_cpu_ns()).map(|(before, after)| {
        let per_frame = Duration::from_nanos(after.saturating_sub(before)) / u32::try_from(SAMPLES).unwrap_or(u32::MAX);
        per_frame.as_secs_f64() * 1000.0
    });
    samples.sort();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    Timing { p50: ms(samples[samples.len() / 2]), p95: ms(samples[samples.len() * 95 / 100]), cpu }
}

/// A timing as `p50 / p95 / cpu` columns.
fn columns(timing: &Timing) -> String {
    let cpu = timing.cpu.map_or_else(|| "-".to_owned(), |cpu| format!("{cpu:.3}"));
    format!("{:>8.3} {:>8.3} {cpu:>8}", timing.p50, timing.p95)
}

#[test]
#[ignore = "benchmark; run with --release -- --ignored --nocapture"]
fn frame_times() {
    println!("Milliseconds per frame: wall-clock median and 95th percentile, mean CPU time of the thread.");
    println!(
        "{:<26} {:<10}  {:>8} {:>8} {:>8}  {:>8} {:>8} {:>8}",
        "scenario", "theme", "repaint", "p95", "cpu", "key", "p95", "cpu"
    );
    let scenarios = [
        Scenario::List,
        Scenario::SharedList,
        Scenario::Table,
        Scenario::Cards,
        Scenario::Markdown,
        Scenario::Log,
        Scenario::Nested,
        Scenario::Transition,
    ];
    for scenario in scenarios {
        let mut harness = Harness::new(Screen::new(scenario), WIDTH, HEIGHT);
        harness.press("tab").press("end");
        assert!(!harness.screen().trim().is_empty(), "the scenario paints something");
        for theme in THEMES {
            harness.set_theme(theme);
            let repaint = time(|| {
                harness.render();
            });
            let mut up = true;
            let key = time(|| {
                if let Scenario::Transition = scenario {
                    harness.send(Msg::Flip);
                } else {
                    harness.press(if up { "up" } else { "down" });
                    up = !up;
                }
            });
            // Let the last transition end, so the next theme's repaints measure an idle page.
            harness.advance(Duration::from_secs(5));
            println!("{:<26} {:<10}  {}  {}", scenario.name(), theme, columns(&repaint), columns(&key));
        }
    }
}

/// A wallpaper: one picture covering the whole screen.
#[cfg(feature = "image")]
struct Wallpaper(qframe::widgets::ImageData);

#[cfg(feature = "image")]
impl App for Wallpaper {
    type Msg = ();

    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, ()>) {
        use qframe::widgets::{Fit, Image};
        ui.add(Image::new(&self.0).fit(Fit::Cover)).fill().id("wallpaper");
    }
}

/// A picture filling a 200 × 60 screen, the way a desktop shows its wallpaper, measured two ways:
/// `repaint` draws the same picture again from the cells kept in memory, `resize` changes the
/// screen by a column every frame, so every frame works the cells out anew.
///
/// ```text
/// cargo test --release -p quvyta-framework --features image --test frame_times image -- --ignored --nocapture
/// ```
#[cfg(feature = "image")]
#[test]
#[ignore = "benchmark; run with --release --features image -- --ignored --nocapture"]
fn image_frame_times() {
    use qframe::widgets::ImageData;

    // A 16:9 picture a little larger than the screen's 200 × 120 pixels, as decoded for it.
    let (width, height) = (320u32, 180u32);
    let mut rgb = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let shade = |value: u32| u8::try_from(value % 256).unwrap_or(0);
            rgb.extend([shade(x * 255 / width), shade(y * 255 / height), shade((x * y) % 256)]);
        }
    }
    let picture = ImageData::from_rgb(width, height, &rgb).expect("every pixel is there");
    let mut harness = Harness::new(Wallpaper(picture), 200, 60);
    assert!(harness.screen().contains('▀'), "the picture is drawn");
    let repaint = time(|| {
        harness.render();
    });
    let mut wide = true;
    let resize = time(|| {
        wide = !wide;
        harness.resize(if wide { 200 } else { 199 }, 60);
    });
    println!("Milliseconds per frame: wall-clock median and 95th percentile, mean CPU time of the thread.");
    println!("{:<30} {:>8} {:>8} {:>8}", "wallpaper 200 x 60, cover", "p50", "p95", "cpu");
    println!("{:<30} {}", "repaint (cells kept)", columns(&repaint));
    println!("{:<30} {}", "resize (cells worked out)", columns(&resize));
}
