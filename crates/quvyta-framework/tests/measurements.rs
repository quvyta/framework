//! What the framework costs, measured: one frame, scrolling, opening a large text, a big folder
//! and the terminal's coalescing.
//!
//! Every measurement here is ignored by default, like [`frame_times`](../frame_times.rs), so the
//! commit gate stays fast and so a loaded build server never fails a build over a slow clock.
//! Bounds were deliberately not used: a number that is asserted has to be loose enough to pass on
//! a busy machine, and a bound that loose catches nothing worth catching. The numbers are printed
//! instead, for a person to read and compare against the last run.
//!
//! Run them in release mode, one at a time, so the scenarios do not share the CPU:
//!
//! ```text
//! cargo test --release -p quvyta-framework --test measurements -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The terminal measurement needs the embedded terminal:
//!
//! ```text
//! cargo test --release -p quvyta-framework --features pty --test measurements -- --ignored --nocapture --test-threads=1 coalescing
//! ```
//!
//! Every timing is reported as median, 95th percentile and worst of the samples, and on Linux the
//! mean CPU time of the thread. The worst is there on purpose: a person does not notice the
//! average frame, they notice the one that arrives late.
//!
//! There is no memory measurement here. Resident pages read from `/proc/self/statm` answered the
//! order the rows ran in rather than the text they held, because the allocator hands back pages it
//! already has; a figure that moves with the order is worse than none. Measuring what a widget
//! keeps needs a counting allocator, which wants `unsafe`, so it belongs in a program of its own
//! rather than in this file.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, fs};

use qframe::prelude::*;
use qframe::widgets::{
    Badge, Bar, BarChart, BigText, CardGrid, CodeView, Column, EmptyState, FileManager, FileManagerMsg,
    FileManagerState, FileView, Gauge, Language, Markdown, ScrollView, Sparkline, Table, TableCell, TableRow,
};

/// The size every screen is measured at: a full terminal window.
const WIDTH: u16 = 160;
const HEIGHT: u16 = 48;

/// Frames thrown away before a measurement, so caches and the allocator have settled.
const WARMUP: usize = 5;

/// Samples of a cheap frame.
const SAMPLES: usize = 100;

/// Samples of an expensive frame. A large text costs so much per frame that a hundred samples of
/// it would take minutes; fifteen still show the spread.
const FEW_SAMPLES: usize = 15;

/// How long one row of a table may spend sampling. A frame that turns out to cost seconds would
/// otherwise keep the measurement running for hours, and the first sample already tells the story;
/// every row says how many samples fitted, so a row of one is read as one reading, not a median.
const BUDGET: Duration = Duration::from_secs(20);

// ---------------------------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------------------------

/// What one kind of frame cost, in milliseconds.
struct Timing {
    p50: f64,
    p95: f64,
    worst: f64,
    /// Mean CPU time of this thread per sample, where Linux reports it. Unlike wall-clock time it
    /// barely moves when other processes load the machine, so it is the number to compare between
    /// runs.
    cpu: Option<f64>,
    /// How many samples the budget allowed. One means the row is a single reading.
    taken: usize,
}

impl Timing {
    /// The timing as `p50 / p95 / worst / cpu / samples` columns.
    fn columns(&self) -> String {
        let cpu = self.cpu.map_or_else(|| "-".to_owned(), |cpu| format!("{cpu:.3}"));
        format!("{:>9.3} {:>9.3} {:>9.3} {cpu:>9} {:>4}", self.p50, self.p95, self.worst, self.taken)
    }
}

/// Nanoseconds this thread has run on a CPU, from `/proc/thread-self/schedstat` on Linux.
fn thread_cpu_ns() -> Option<u64> {
    fs::read_to_string("/proc/thread-self/schedstat").ok()?.split_whitespace().next()?.parse().ok()
}

/// Runs `sample` up to `count` times after `warmup` throwaway runs and reports what the runs cost.
///
/// Sampling stops early once [`BUDGET`] has been spent, so one very expensive frame cannot keep
/// the measurement running for hours; at least one sample is always taken.
fn measure(warmup: usize, count: usize, mut sample: impl FnMut()) -> Timing {
    for _ in 0..warmup {
        sample();
    }
    let cpu_before = thread_cpu_ns();
    let began = Instant::now();
    let mut samples: Vec<Duration> = Vec::with_capacity(count);
    for _ in 0..count {
        let started = Instant::now();
        sample();
        samples.push(started.elapsed());
        if began.elapsed() >= BUDGET {
            break;
        }
    }
    let taken = samples.len();
    let cpu = cpu_before.zip(thread_cpu_ns()).map(|(before, after)| {
        let total = Duration::from_nanos(after.saturating_sub(before));
        (total / u32::try_from(taken).unwrap_or(u32::MAX)).as_secs_f64() * 1000.0
    });
    samples.sort_unstable();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    Timing {
        p50: ms(samples[taken / 2]),
        p95: ms(samples[(taken * 95 / 100).min(taken - 1)]),
        worst: ms(samples[taken - 1]),
        cpu,
        taken,
    }
}

/// The frame timings of a cheap frame: many samples, a short warm-up.
fn frames(sample: impl FnMut()) -> Timing {
    measure(WARMUP, SAMPLES, sample)
}

/// The header above a table of timings.
fn header(first: &str) {
    println!("{first:<34}  {:>9} {:>9} {:>9} {:>9} {:>4}", "median", "p95", "worst", "cpu", "n");
}

/// The machine and the load the numbers were taken on, so a later run can be compared fairly.
fn machine() {
    let load = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let load = load.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
    println!("Machine: {} threads, load average {load}", std::thread::available_parallelism().map_or(0, Into::into));
    println!("All times are milliseconds.\n");
}

// ---------------------------------------------------------------------------------------------
// One frame: a plain screen and a dashboard
// ---------------------------------------------------------------------------------------------

/// Which screen the frame-cost application shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// What a small tool shows: a title, a few lines of text, two buttons and the key hints.
    Plain,
    /// A host dashboard built from panels, gauges, a sparkline, a bar chart and badges — the
    /// shape of the showcase's example dashboard, rebuilt here so the benchmark owns it.
    Dashboard,
}

/// The application behind the frame-cost measurement.
struct Frames {
    screen: Screen,
    tick: u32,
}

#[derive(Clone, Copy)]
enum FrameMsg {
    Refresh,
}

/// A repeatable sample series, so the dashboard's numbers move without a clock.
fn sample_at(series: u32, tick: u32) -> f32 {
    let n = f32::from(u16::try_from((series * 977 + tick * 37) % 1000).unwrap_or(0));
    20.0 + (n / 1000.0) * 75.0
}

impl App for Frames {
    type Msg = FrameMsg;

    fn update(&mut self, msg: FrameMsg) -> Command<FrameMsg> {
        match msg {
            FrameMsg::Refresh => self.tick += 1,
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, FrameMsg>) {
        match self.screen {
            Screen::Plain => {
                ui.add(Text::new("Backup").role("title"));
                ui.add(Text::new("Last run finished 12 minutes ago, 4 of 4 folders copied.")).fill_width();
                ui.add(Text::new("Next run at 03:00.").role("faint")).fill_width();
                ui.row(|ui| {
                    ui.add(Button::new("Run now").on_press(FrameMsg::Refresh)).id("run");
                    ui.add(Button::new("Settings"));
                })
                .gap(2);
                ui.spacer().fill_height();
            }
            Screen::Dashboard => self.dashboard(ui),
        }
        ui.add(KeyHints::new().hint("r", "refresh").hint("q", "quit")).fill_width();
    }
}

impl Frames {
    /// Services of the host and the sample series that drives their load.
    const SERVICES: [(&'static str, u32); 5] =
        [("api-gateway", 3), ("postgres-16", 4), ("redis-cache", 5), ("image-resizer", 6), ("worker", 7)];

    /// The load above which a service is flagged, in percent.
    const HOT: f32 = 70.0;

    fn dashboard(&self, ui: &mut View<'_, FrameMsg>) {
        let hot = Self::SERVICES.iter().filter(|(_, series)| sample_at(*series, self.tick) > Self::HOT).count();
        ui.add_with(Panel::new().gap(0), |ui| {
            ui.row(|ui| {
                ui.column(|ui| {
                    ui.add(Text::new("Region").role("faint").no_wrap());
                    ui.add(Text::new("edge-01").role("title").no_wrap());
                    ui.spacer().height(Length::Cells(1));
                    ui.row(|ui| {
                        ui.add(Badge::new("Healthy").variant("success"));
                        ui.add(Badge::new("Containers").count(12));
                        if hot > 0 {
                            ui.add(Badge::new("Busy").variant("danger").count(u32::try_from(hot).unwrap_or(0)));
                        }
                    })
                    .gap(2);
                })
                .width(Length::Fill(1));
                ui.column(|ui| {
                    ui.add(Text::new("Local time").role("faint").no_wrap());
                    let minutes = (14 * 60 + 32 + self.tick) % (24 * 60);
                    ui.add(BigText::new(format!("{:02}:{:02}", minutes / 60, minutes % 60)).variant("accent"));
                });
                ui.add(Button::new("Refresh").shortcut("r").on_press(FrameMsg::Refresh)).id("refresh");
            })
            .gap(4)
            .fill_width();
        })
        .fill_width();

        ui.row(|ui| {
            ui.add_with(Panel::new().title("Processor").gap(0), |ui| {
                let history: Vec<f32> = (0..64).map(|n| sample_at(0, self.tick + n)).collect();
                let now = history.last().copied().unwrap_or_default();
                ui.add(Sparkline::new(history).range(0.0, 100.0).highlight_extremes().baseline(80.0))
                    .fill_width()
                    .height(Length::Cells(3));
                ui.spacer().height(Length::Cells(1));
                ui.add(Text::rich([
                    Span::new(format!("{now:.0}%")).bold(),
                    Span::new(" of 8 cores, one minute average").role("secondary"),
                ]));
            })
            .width(Length::Fill(1))
            .fill_height();
            ui.add_with(Panel::new().title("Resources").gap(0), |ui| {
                let memory = 4.0 + sample_at(1, self.tick) / 100.0 * 3.8;
                ui.add(
                    Gauge::new(memory)
                        .range(0.0, 8.0)
                        .label("Memory")
                        .label_width(8)
                        .thresholds(6.0, 7.2)
                        .value_text(format!("{memory:.1} of 8 GiB")),
                )
                .fill_width();
                ui.add(Gauge::new(71.0).label("Disk").label_width(8).thresholds(80.0, 92.0)).fill_width();
                ui.add(Gauge::new(12.0).label("Swap").label_width(8).thresholds(50.0, 80.0)).fill_width();
            })
            .width(Length::Fill(1))
            .fill_height();
        })
        .gap(2)
        .fill_width();

        ui.row(|ui| {
            ui.add_with(Panel::new().title("Services").gap(0), |ui| {
                let bars = Self::SERVICES.iter().map(|(name, series)| {
                    let load = sample_at(*series, self.tick);
                    let bar = Bar::new(*name, load).value_text(format!("{load:.0}%"));
                    if load > Self::HOT { bar.variant("danger") } else { bar }
                });
                ui.add(BarChart::new(bars).max(100.0)).fill_width();
            })
            .width(Length::Fill(1))
            .fill_height();
            ui.add_with(Panel::new().title("Alerts").gap(0), |ui| {
                ui.add(EmptyState::new("No alerts").icon("success").message("Everything answered in time."))
                    .fill_width()
                    .height(Length::Cells(9));
            })
            .width(Length::Fill(1))
            .fill_height();
        })
        .gap(2)
        .fill_width();
    }
}

#[test]
#[ignore = "measurement; run with --release -- --ignored --nocapture"]
fn one_frame() {
    machine();
    println!("One frame: building the view, laying it out and painting it, at {WIDTH} x {HEIGHT} cells.");
    header("screen");
    for (screen, name) in [(Screen::Plain, "plain tool screen"), (Screen::Dashboard, "dashboard")] {
        let mut harness = Harness::new(Frames { screen, tick: 90 }, WIDTH, HEIGHT);
        harness.render();
        assert!(!harness.screen().trim().is_empty(), "{name} paints something");
        let repaint = frames(|| {
            harness.render();
        });
        println!("{:<34}  {}", format!("{name}, repaint"), repaint.columns());
        let refresh = frames(|| {
            harness.send(FrameMsg::Refresh);
        });
        println!("{:<34}  {}", format!("{name}, key press"), refresh.columns());
    }
    println!();
}

// ---------------------------------------------------------------------------------------------
// Scrolling many rows
// ---------------------------------------------------------------------------------------------

/// The row counts a scrolling widget is measured at, so growth with the number of rows shows.
const COUNTS: [usize; 3] = [1_000, 10_000, 100_000];

/// Which widget the scrolling measurement drives.
#[derive(Clone, Copy)]
enum Rows {
    List,
    Table,
    Cards,
}

/// An application showing one scrolling widget over `count` rows.
struct Scrolling {
    rows: Rows,
    count: usize,
    items: Arc<[ListItem]>,
    table: Arc<[TableRow]>,
    selected: Option<usize>,
}

impl Scrolling {
    fn new(rows: Rows, count: usize) -> Self {
        // Every name is the same width, so two counts differ only by the number of rows, never by
        // how wide a row is.
        let items: Arc<[ListItem]> = match rows {
            Rows::List => (0..count).map(|n| ListItem::new(format!("Row {n:06}"))).collect(),
            _ => Arc::from(Vec::new()),
        };
        let table: Arc<[TableRow]> = match rows {
            Rows::Table => (0..count)
                .map(|n| {
                    TableRow::new([
                        TableCell::new(format!("build-{n:06}")),
                        TableCell::new(if n % 7 == 0 { "failed" } else { "passed" }),
                        TableCell::new(format!("{:02}.{} s", n % 90, n % 10)),
                        TableCell::new("quvyta/api:2.4"),
                    ])
                })
                .collect(),
            _ => Arc::from(Vec::new()),
        };
        Self { rows, count, items, table, selected: Some(0) }
    }
}

impl App for Scrolling {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        let Msg::Select(index) = msg;
        self.selected = Some(index);
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("Rows").role("title"));
        match self.rows {
            Rows::List => {
                ui.add(List::shared(Arc::clone(&self.items)).selected(self.selected).on_select(Msg::Select))
                    .fill()
                    .id("rows");
            }
            Rows::Table => {
                let columns = [Column::new("Build"), Column::new("Status"), Column::new("Time"), Column::new("Image")];
                ui.add(Table::new(columns, Arc::clone(&self.table)).selected(self.selected).on_select(Msg::Select))
                    .fill()
                    .id("rows");
            }
            Rows::Cards => {
                let grid =
                    CardGrid::new(self.count).selected(self.selected).on_select(Msg::Select).card(|ui, index| {
                        ui.add(Text::new(format!("app-{index:06}")).role("title").no_wrap());
                        ui.add(Text::new("quvyta/api:2.4 on port 8080").role("secondary").no_wrap());
                    });
                ui.add(grid).fill().id("rows");
            }
        }
        ui.add(KeyHints::new().hint("↑↓", "move").hint("pgdn", "page")).fill_width();
    }
}

/// The one message the scrolling and folder measurements need.
#[derive(Clone)]
enum Msg {
    Select(usize),
}

#[test]
#[ignore = "measurement; run with --release -- --ignored --nocapture"]
fn scrolling_many_rows() {
    machine();
    println!("Scrolling: one step (↓) and one page (pgdn), at {WIDTH} x {HEIGHT} cells.");
    println!("The three row counts of one widget should read the same; growth here is growth with the rows.");
    header("widget and rows");
    for (rows, name) in [(Rows::List, "list"), (Rows::Table, "table"), (Rows::Cards, "card grid")] {
        for count in COUNTS {
            let mut harness = Harness::new(Scrolling::new(rows, count), WIDTH, HEIGHT);
            harness.press("tab").render();
            assert!(!harness.screen().trim().is_empty(), "{name} of {count} paints something");
            let mut down = true;
            let step = frames(|| {
                harness.press(if down { "down" } else { "up" });
                down = !down;
            });
            println!("{:<34}  {}", format!("{name} {count}, one step"), step.columns());
            let mut forward = true;
            let page = frames(|| {
                harness.press(if forward { "pgdn" } else { "pgup" });
                forward = !forward;
            });
            println!("{:<34}  {}", format!("{name} {count}, one page"), page.columns());
        }
    }
    println!();
}

// ---------------------------------------------------------------------------------------------
// Opening a large text
// ---------------------------------------------------------------------------------------------

/// Lines of the large documents.
const BIG_LINES: usize = 20_000;

/// A plausible Rust file of `lines` lines, about 50 bytes each.
fn big_rust(lines: usize) -> String {
    let mut code = String::with_capacity(lines * 52);
    for n in 0..lines {
        match n % 5 {
            0 => writeln!(code, "/// Answers the request numbered {n} of the batch.").ok(),
            1 => writeln!(code, "pub fn answer_{n}(request: &Request) -> Result<Reply, Error> {{").ok(),
            2 => writeln!(code, "    let value = request.field(\"name-{n}\")?.trim().to_owned();").ok(),
            3 => writeln!(code, "    Ok(Reply::new(value, {n}u32, Kind::Plain))").ok(),
            _ => writeln!(code, "}}").ok(),
        };
    }
    code
}

/// A plausible Markdown document of `lines` lines.
fn big_markdown(lines: usize) -> String {
    let mut text = String::with_capacity(lines * 52);
    for n in 0..lines {
        match n % 8 {
            0 => writeln!(text, "\n## Section {n}\n").ok(),
            1 | 2 => writeln!(
                text,
                "A widget draws itself inside the rectangle it is given and answers the events it knows, step {n}."
            )
            .ok(),
            3 => writeln!(text).ok(),
            4 => writeln!(text, "- Keys: arrows move, Enter activates, Esc closes; item {n}.").ok(),
            5 => writeln!(text, "- Mouse: a click activates and the wheel scrolls; item {n}.").ok(),
            6 => writeln!(text, "\n> Every state is designed: empty, loading, error, narrow, disabled.\n").ok(),
            _ => {
                writeln!(text, "Call `ui.add(widget)` to place one and **chain** `.id(\"name-{n}\")` after it.\n").ok()
            }
        };
    }
    text
}

/// `text` with a last line of its own for sample `n`, so a first frame is measured on a text no
/// earlier sample showed: `CodeView` and `Markdown` remember what they laid out by its source,
/// and a first frame answered from that memory is not a first frame.
fn unseen(text: &str, n: usize) -> String {
    format!("{text}\n// opened {n}\n")
}

/// Which large text the reading application shows.
#[derive(Clone, Copy)]
enum Reading {
    Code,
    Document,
}

/// An application showing one large text inside a scroll view, the way a viewer does.
struct Reader {
    kind: Reading,
    text: String,
}

impl App for Reader {
    type Msg = Msg;

    fn update(&mut self, _msg: Msg) -> Command<Msg> {
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new("Reading").role("title"));
        ui.add_with(ScrollView::new(), |ui| match self.kind {
            Reading::Code => {
                ui.add(CodeView::<Msg>::new(self.text.as_str(), Language::Rust)).fill_width();
            }
            Reading::Document => {
                ui.add(Markdown::new(&self.text)).fill_width();
            }
        })
        .fill()
        .selectable(true)
        .id("text");
    }
}

/// The file sizes the growth of a large text is measured over: each one twice the last, so the
/// shape of the growth can be read straight off the column. A cost that doubles with the doubling
/// grows with the size of the file; a cost that quadruples grows with its square.
const GROWTH: [usize; 5] = [1_000, 2_000, 4_000, 8_000, 16_000];

#[test]
#[ignore = "measurement; run with --release -- --ignored --nocapture"]
fn a_large_text_as_it_grows() {
    machine();
    println!("The first frame of a text, at sizes that double. A cost that quadruples with a doubling");
    println!("of the lines grows with the square of the file, not with the file.\n");
    for (kind, name) in [(Reading::Code, "CodeView, Rust"), (Reading::Document, "Markdown")] {
        header(name);
        let mut last: Option<f64> = None;
        for lines in GROWTH {
            let text = match kind {
                Reading::Code => big_rust(lines),
                Reading::Document => big_markdown(lines),
            };
            let mut opened = 0;
            let timing = measure(0, 3, || {
                opened += 1;
                let mut harness = Harness::new(Reader { kind, text: unseen(&text, opened) }, WIDTH, HEIGHT);
                harness.render();
                assert!(!harness.screen().trim().is_empty(), "the text is on screen");
            });
            let grew = last.map_or_else(|| "-".to_owned(), |last| format!("x{:.1}", timing.p50 / last));
            last = Some(timing.p50);
            println!("{:<34}  {}  {grew}", format!("{lines} lines"), timing.columns());
        }
        println!();
    }
}

#[test]
#[ignore = "measurement; run with --release -- --ignored --nocapture"]
fn opening_a_large_text() {
    machine();
    println!("A large text: the first frame, a repaint, and one scroll step, at {WIDTH} x {HEIGHT} cells.");
    println!("The first frame is what a person waits for after pressing Enter on a file.\n");
    for (kind, name) in [(Reading::Code, "CodeView, Rust"), (Reading::Document, "Markdown")] {
        let text = match kind {
            Reading::Code => big_rust(BIG_LINES),
            Reading::Document => big_markdown(BIG_LINES),
        };
        let bytes = text.len();
        let lines = text.lines().count();
        println!("{name}: {lines} lines, {:.2} MiB of source.", bytes as f64 / (1024.0 * 1024.0));
        header("what");

        // The first frame: the state takes the text and the runtime paints it once. Fresh state
        // and a text never shown before every time, so no cache of a previous run answers for it.
        let mut held = None;
        let mut opened = 0;
        let first = measure(0, 5, || {
            opened += 1;
            let mut harness = Harness::new(Reader { kind, text: unseen(&text, opened) }, WIDTH, HEIGHT);
            harness.render();
            held = Some(harness);
        });
        let mut harness = held.expect("the last harness");
        println!("{:<34}  {}", "first frame", first.columns());

        let repaint = measure(0, FEW_SAMPLES, || {
            harness.render();
        });
        println!("{:<34}  {}", "repaint, unchanged", repaint.columns());

        harness.press("tab");
        let mut down = true;
        let step = measure(0, FEW_SAMPLES, || {
            harness.press(if down { "down" } else { "up" });
            down = !down;
        });
        println!("{:<34}  {}\n", "one scroll step", step.columns());
    }
}

// ---------------------------------------------------------------------------------------------
// A folder of ten thousand entries
// ---------------------------------------------------------------------------------------------

/// A folder of this measurement's own, taken away when it ends.
struct Scratch(PathBuf);

impl Scratch {
    /// A folder holding `count` entries of one width.
    fn of(count: usize) -> Self {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = env::temp_dir().join(format!("qframe-measurements-{count}-{stamp}"));
        fs::create_dir_all(&path).expect("the folder");
        for index in 0..count {
            fs::write(path.join(format!("entry-{index:05}.txt")), "").expect("an entry");
        }
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An application showing one file manager, as qdesk's Files and qcode's panel both do.
struct Files {
    manager: FileManagerState,
}

#[derive(Clone)]
enum FileMsg {
    Files(FileManagerMsg),
}

impl App for Files {
    type Msg = FileMsg;

    fn init(&mut self) -> Command<FileMsg> {
        self.manager.load(FileMsg::Files)
    }

    fn update(&mut self, msg: FileMsg) -> Command<FileMsg> {
        let FileMsg::Files(message) = msg;
        self.manager.update(message, FileMsg::Files)
    }

    fn view(&self, ui: &mut View<'_, FileMsg>) {
        FileManager::new(&self.manager, FileMsg::Files).id("files").show(ui).fill();
    }
}

/// Opens `folder` in a file manager and paints it once, the way pressing Enter on it does.
fn open_folder(folder: &Path, view: FileView) -> Harness<Files> {
    let mut harness = Harness::new(Files { manager: FileManagerState::new(folder.to_path_buf()) }, WIDTH, HEIGHT);
    harness.set_reduced_motion(true);
    // The tree view keeps the root folded until it is shown; the flat views need the shape set.
    let _ = view;
    harness.render();
    harness
}

#[test]
#[ignore = "measurement; run with --release -- --ignored --nocapture"]
fn a_folder_of_ten_thousand_entries() {
    machine();
    println!("A folder in the file manager: reading it, painting it once, and moving inside it.");
    println!("The note promises the work does not grow with the entries; the two counts should read the same.");
    header("entries");
    for count in [200_usize, 10_000] {
        let scratch = Scratch::of(count);
        // Reading the folder on its own, which is what the background thread does before a paint.
        let read = measure(1, 5, || {
            let entries = qframe::widgets::FolderEntry::read_folder(&scratch.0).expect("the folder reads");
            assert_eq!(entries.len(), count, "every entry came in one answer");
        });
        println!("{:<34}  {}", format!("{count} entries, read from disk"), read.columns());

        let open = measure(1, 5, || {
            let harness = open_folder(&scratch.0, FileView::Tree);
            assert!(harness.screen().contains("entry-00000.txt"), "the folder is on screen");
        });
        println!("{:<34}  {}", format!("{count} entries, open and first frame"), open.columns());

        let mut harness = open_folder(&scratch.0, FileView::Tree);
        harness.press("tab");
        let mut down = true;
        let step = frames(|| {
            harness.press(if down { "down" } else { "up" });
            down = !down;
        });
        println!("{:<34}  {}", format!("{count} entries, one step"), step.columns());
        let repaint = frames(|| {
            harness.render();
        });
        println!("{:<34}  {}", format!("{count} entries, repaint"), repaint.columns());
    }
    println!();
}

// ---------------------------------------------------------------------------------------------
// The terminal's output coalescing
// ---------------------------------------------------------------------------------------------

/// How long a chatty program is watched before the measurement gives up on it.
#[cfg(feature = "pty")]
const CHATTY_BOUND: Duration = Duration::from_secs(20);

#[cfg(feature = "pty")]
#[test]
#[ignore = "measurement; run with --release --features pty -- --ignored --nocapture"]
fn terminal_output_coalescing() {
    use qframe::widgets::{TerminalChange, TerminalSession};

    machine();
    println!("The embedded terminal watching a program that writes 20 000 lines as fast as it can.");
    println!("Every reported change is a redraw an application would do; coalescing bounds how many.\n");
    println!("{:<34}  {:>9} {:>9} {:>9}", "coalescing", "changes", "seconds", "per second");
    // A fixed, harmless program: the shell writing lines with its own printf.
    let script = "i=0; while [ $i -lt 20000 ]; do printf 'line %d of the run\\n' $i; i=$((i+1)); done";
    for interval in [None, Some(Duration::from_millis(8)), Some(Duration::from_millis(16))] {
        let mut builder = TerminalSession::builder("/bin/sh").args(["-c", script]).size(80, 24);
        if let Some(interval) = interval {
            builder = builder.coalesce(interval);
        }
        let session = builder.spawn().expect("the shell starts");
        let watch = session.watch();
        let started = Instant::now();
        let mut changes = 0_u32;
        loop {
            match watch.next_change_within(CHATTY_BOUND) {
                Some(TerminalChange::Output) => changes += 1,
                Some(TerminalChange::Exited(_)) | None => break,
                Some(_) => {}
            }
            if started.elapsed() > CHATTY_BOUND {
                break;
            }
        }
        let seconds = started.elapsed().as_secs_f64();
        let name = interval.map_or_else(|| "none".to_owned(), |interval| format!("{} ms", interval.as_millis()));
        println!("{name:<34}  {changes:>9} {seconds:>9.3} {:>9.1}", f64::from(changes) / seconds);
    }
    println!();
}
