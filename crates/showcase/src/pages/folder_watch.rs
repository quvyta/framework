//! Folder watch: the changes in a folder as the operating system reports them, gathered into
//! batches, without polling.

use std::path::{Path, PathBuf};
use std::time::Duration;

use qframe::prelude::*;
use qframe::storage::{FolderChange, FolderChangeKind, FolderChanges, FolderWatch};

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "folder-watch";

/// How many files one press of the burst button makes and removes again.
const BURST: usize = 200;

/// How many of the latest batches the page keeps on screen.
const KEPT: usize = 4;

/// How many changes of one batch are spelled out; the rest are counted.
const SPELLED: usize = 3;

/// The demo folder, its watch while it runs, and the batches that arrived.
#[derive(Debug, Default)]
pub struct State {
    /// The watch while the demo runs; dropping it wakes the waiting thread, which then stops.
    watch: Option<FolderWatch>,
    /// Counts started watches, so the answer of a stopped one is recognised and ignored.
    run: u64,
    /// The folder of this run the demo writes into; made on the first start.
    folder: Option<PathBuf>,
    /// The latest batches, oldest first.
    batches: Vec<Vec<FolderChange>>,
    /// Every batch that arrived since the first start.
    received: usize,
    /// Numbers the files the buttons make, so no name is used twice.
    made: usize,
    /// Why the watch could not start, or why a button's file operation failed.
    problem: Option<String>,
}

impl Drop for State {
    /// Takes the demo folder away with the page, so a run leaves nothing in the temporary folder.
    fn drop(&mut self) {
        self.watch = None;
        if let Some(folder) = &self.folder {
            let _ = std::fs::remove_dir_all(folder);
        }
    }
}

/// Tells the demo folders of two pages in one process apart, which the tests need.
static DEMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A folder of this run for the demo's files, never one of the user's own.
fn demo_dir() -> PathBuf {
    let ticket = DEMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("quvyta-showcase-watch-{}-{ticket}", std::process::id()))
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Start,
    Stop,
    /// A batch from the watch of run `u64`; empty once that watch is dropped.
    Changed(u64, Vec<FolderChange>),
    /// A wait of the watch of run `u64` ended with nothing changed.
    Quiet(u64),
    Create,
    Rename,
    Remove,
    Burst,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::FolderWatch(message))
}

// region: folder-watch-wait
/// How long one wait lasts before it says nothing changed and is asked again.
const PATIENCE: Duration = Duration::from_millis(250);

/// Waits for the next batch on a background thread and hands it to `update`. Each wait has a
/// bound, so the same code also runs in a screen test, which does the work in place: a wait with
/// no end would hold the test for good.
fn wait(changes: FolderChanges, run: u64) -> Command<AppMsg> {
    Command::perform(move || match changes.next_within(PATIENCE) {
        Some(batch) => send(Msg::Changed(run, batch)),
        None => send(Msg::Quiet(run)),
    })
}
// endregion

/// The folder's entries, sorted, for the list on screen and for the buttons to pick from.
fn entries(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .map(|read| read.flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

/// Starts watching the demo folder, making it first.
fn start(state: &mut State) -> Result<Command<AppMsg>, String> {
    let folder = state.folder.get_or_insert_with(demo_dir).clone();
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    // region: folder-watch-start
    let mut watch = FolderWatch::new().map_err(|error| error.to_string())?;
    watch.watch(&folder).map_err(|error| error.to_string())?;
    let changes = watch.changes();
    // endregion
    state.run += 1;
    state.watch = Some(watch);
    Ok(wait(changes, state.run))
}

/// Runs one file operation of a button and remembers what went wrong, if anything did.
fn file_operation(
    state: &mut State,
    log: &mut EventLog,
    what: &str,
    work: impl FnOnce(&Path, usize) -> std::io::Result<()>,
) {
    let Some(folder) = state.folder.clone() else {
        return;
    };
    state.made += 1;
    match work(&folder, state.made) {
        Ok(()) => {
            log.push(PAGE, "std::fs", what.to_owned());
            state.problem = None;
        }
        Err(error) => {
            log.push(PAGE, "std::fs", format!("{what}: {error}"));
            state.problem = Some(error.to_string());
        }
    }
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Start => match start(state) {
            Ok(command) => {
                log.push(PAGE, "FolderWatch::watch", "watching the demo folder");
                state.problem = None;
                command
            }
            Err(error) => {
                log.push(PAGE, "FolderWatch::new", error.clone());
                state.problem = Some(error);
                Command::none()
            }
        },
        Msg::Stop => {
            log.push(PAGE, "FolderWatch", "dropped the watch");
            state.watch = None;
            Command::none()
        }
        // region: folder-watch-changed
        Msg::Changed(run, batch) if run == state.run && !batch.is_empty() => {
            log.push(PAGE, "FolderChanges::next", format!("{} changes in one batch", batch.len()));
            state.received += 1;
            state.batches.push(batch);
            if state.batches.len() > KEPT {
                state.batches.remove(0);
            }
            // Wait again for as long as the watch lives; a dropped watch answers empty.
            match &state.watch {
                Some(watch) => wait(watch.changes(), run),
                None => Command::none(),
            }
        }
        // endregion
        // A stopped watch's last answer, or an earlier run's.
        Msg::Changed(..) => Command::none(),
        // Nothing changed within the bound: the watch that still lives waits again.
        Msg::Quiet(run) => match &state.watch {
            Some(watch) if run == state.run => wait(watch.changes(), run),
            _ => Command::none(),
        },
        Msg::Create => {
            file_operation(state, log, "created a file", |folder, n| {
                std::fs::write(folder.join(format!("note-{n}.txt")), "")
            });
            Command::none()
        }
        Msg::Rename => {
            file_operation(state, log, "renamed a file", |folder, n| match entries(folder).first() {
                Some(name) => std::fs::rename(folder.join(name), folder.join(format!("draft-{n}.txt"))),
                None => Ok(()),
            });
            Command::none()
        }
        Msg::Remove => {
            file_operation(state, log, "removed a file", |folder, _| match entries(folder).first() {
                Some(name) => std::fs::remove_file(folder.join(name)),
                None => Ok(()),
            });
            Command::none()
        }
        Msg::Burst => {
            let what = format!("made and removed {BURST} files");
            file_operation(state, log, &what, |folder, n| {
                let paths: Vec<PathBuf> = (0..BURST).map(|i| folder.join(format!("burst-{n}-{i}.txt"))).collect();
                for path in &paths {
                    std::fs::write(path, "")?;
                }
                paths.iter().try_for_each(std::fs::remove_file)
            });
            Command::none()
        }
    }
}

/// The marker, its colour and the sentence for one change.
fn describe(ui: &View<'_, AppMsg>, change: &FolderChange) -> (String, &'static str, String) {
    let icons = ui.env().icons();
    let name = change.name.as_ref().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    match &change.kind {
        FolderChangeKind::Created => {
            (icons.glyph("add").into_owned(), "success", t!("folder-watch.created", name = name))
        }
        FolderChangeKind::Removed => {
            (icons.glyph("close").into_owned(), "danger", t!("folder-watch.removed", name = name))
        }
        FolderChangeKind::Renamed { from } => (
            icons.glyph("arrow-right").into_owned(),
            "accent",
            t!("folder-watch.renamed", from = from.to_string_lossy().into_owned(), name = name),
        ),
        FolderChangeKind::Modified => {
            (icons.glyph("dot").into_owned(), "muted", t!("folder-watch.modified", name = name))
        }
        FolderChangeKind::Gone => (icons.glyph("warning").into_owned(), "warning", t!("folder-watch.gone")),
        FolderChangeKind::Overflow => (icons.glyph("warning").into_owned(), "warning", t!("folder-watch.overflow")),
    }
}

/// One line with a marker in its own column.
fn marked(ui: &mut View<'_, AppMsg>, marker: String, color: &'static str, text: String, role: &'static str) {
    ui.row(|ui| {
        ui.add(Text::new(marker).color(color).no_wrap());
        ui.add(Text::new(text).role(role)).fill_width();
    })
    .gap(1);
}

/// The live demo.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("folder-watch.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        let watching = state.watch.is_some();
        let names = state.folder.as_deref().map(entries).unwrap_or_default();
        ui.row(|ui| {
            if watching {
                ui.add(Button::new(t!("folder-watch.stop")).on_press(send(Msg::Stop))).id("stop");
            } else {
                ui.add(Button::new(t!("folder-watch.start")).variant("primary").on_press(send(Msg::Start))).id("start");
            }
            ui.add(Button::new(t!("folder-watch.create")).disabled(!watching).on_press(send(Msg::Create))).id("create");
            let none = names.is_empty();
            ui.add(Button::new(t!("folder-watch.rename")).disabled(!watching || none).on_press(send(Msg::Rename)))
                .id("rename");
            ui.add(Button::new(t!("folder-watch.remove")).disabled(!watching || none).on_press(send(Msg::Remove)))
                .id("remove");
            ui.add(Button::new(t!("folder-watch.burst", n = BURST)).disabled(!watching).on_press(send(Msg::Burst)))
                .id("burst");
        })
        .gap(2);
        ui.spacer().height(Length::Cells(1));
        let icons = ui.env().icons();
        let (info, success, error) =
            (icons.glyph("info").into_owned(), icons.glyph("success").into_owned(), icons.glyph("error").into_owned());
        match (&state.problem, watching) {
            (Some(problem), _) => marked(ui, error, "danger", problem.clone(), "body"),
            (None, true) => marked(ui, success, "success", t!("folder-watch.watching", n = state.received), "body"),
            (None, false) if state.received > 0 => {
                marked(ui, info, "muted", t!("folder-watch.stopped", n = state.received), "secondary");
            }
            (None, false) => marked(ui, info, "muted", t!("folder-watch.idle"), "secondary"),
        }
        if state.folder.is_some() {
            ui.spacer().height(Length::Cells(1));
            let shown: Vec<&str> = names.iter().take(6).map(String::as_str).collect();
            let listing = match names.len() {
                0 => t!("folder-watch.empty-folder"),
                n if n > shown.len() => {
                    format!("{}  {}", shown.join("  "), t!("folder-watch.more", n = n - shown.len()))
                }
                _ => shown.join("  "),
            };
            ui.add(
                Text::rich([
                    Span::new(t!("folder-watch.in-folder")).role("faint"),
                    Span::new(format!("  {listing}")).role("body"),
                ])
                .no_wrap(),
            );
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("folder-watch.batches")).gap(0), |ui| {
        if state.batches.is_empty() {
            ui.add(Text::new(t!("folder-watch.no-batches")).role("faint"));
        }
        let first = state.received - state.batches.len();
        for (index, batch) in state.batches.iter().enumerate().rev() {
            ui.add(Text::new(t!("folder-watch.batch", number = first + index + 1, n = batch.len())).role("faint"));
            // region: folder-watch-show
            for change in batch.iter().take(SPELLED) {
                let (marker, color, text) = describe(ui, change);
                marked(ui, marker, color, text, "body");
            }
            // endregion
            if batch.len() > SPELLED {
                ui.add(Text::new(t!("folder-watch.and-more", n = batch.len() - SPELLED)).role("faint"));
            }
        }
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Showcase;
    use crate::tests::{showcase_on, showcase_tall};

    #[test]
    fn a_click_on_start_leaves_the_page_answering_and_a_change_arrives() {
        // A screen test runs the waiting work in place: a wait with no end would hold it for good.
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut h = showcase_tall(Showcase::new(), PAGE, 60);
            h.click_text("Start watching");
            h.click_text("Create a file");
            for _ in 0..50 {
                h.advance(std::time::Duration::from_millis(100));
                if h.screen().contains("1 batch arrived") {
                    break;
                }
            }
            let _ = done.send(h.screen());
        });
        let screen = finished.recv_timeout(std::time::Duration::from_secs(60)).expect("the page kept answering");
        assert!(screen.contains("1 batch arrived"), "the file made arrived as a batch:\n{screen}");
    }

    #[test]
    fn the_page_waits_to_be_started() {
        let h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("Start watching"), "{screen}");
        assert!(screen.contains("Nothing is watched yet"), "{screen}");
        assert!(screen.contains("No batches yet"), "{screen}");
    }

    /// Takes the next batch the way the waiting thread would, and hands it to `update`.
    fn arrive(state: &mut State, log: &mut EventLog) {
        let changes = state.watch.as_ref().expect("watching").changes();
        let batch = changes.next();
        // The command it returns would wait again; the test does the waiting itself.
        let _ = update(state, Msg::Changed(state.run, batch), log);
    }

    #[test]
    fn the_buttons_changes_arrive_as_batches_and_are_shown() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let (mut state, mut log) = (State::default(), EventLog::new());
        // The command waits on the next batch; the test takes the batches itself instead.
        let _ = update(&mut state, Msg::Start, &mut log);
        assert!(state.problem.is_none(), "{:?}", state.problem);

        let _ = update(&mut state, Msg::Create, &mut log);
        arrive(&mut state, &mut log);
        let _ = update(&mut state, Msg::Rename, &mut log);
        arrive(&mut state, &mut log);
        let kinds: Vec<&FolderChangeKind> = state.batches.iter().map(|batch| &batch[0].kind).collect();
        assert_eq!(kinds, [&FolderChangeKind::Created, &FolderChangeKind::Renamed { from: "note-1.txt".into() }]);
        let _ = update(&mut state, Msg::Burst, &mut log);
        arrive(&mut state, &mut log);
        while state.batches.iter().flatten().filter(|c| c.kind == FolderChangeKind::Removed).count() < BURST {
            arrive(&mut state, &mut log);
        }
        assert!(state.received <= 6, "{} batches for three presses", state.received);

        let mut showcase = Showcase::new();
        showcase.pages.folder_watch = state;
        let h = showcase_tall(showcase, PAGE, 80);
        let screen = h.screen();
        assert!(screen.contains("note-1.txt became draft-2.txt"), "{screen}");
        assert!(screen.contains("Stop watching"), "{screen}");
        assert!(screen.contains("draft-2.txt"), "the folder lists what is in it:\n{screen}");
        assert!(screen.contains("more in this batch"), "a burst is counted, not spelled out:\n{screen}");
    }

    #[test]
    fn a_stopped_watch_answers_nothing_and_its_late_batches_are_ignored() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let (mut state, mut log) = (State::default(), EventLog::new());
        let _ = update(&mut state, Msg::Start, &mut log);
        let changes = state.watch.as_ref().expect("watching").changes();
        let run = state.run;
        let _ = update(&mut state, Msg::Stop, &mut log);
        assert!(changes.next().is_empty(), "the dropped watch wakes its waiter with nothing");
        let late = vec![FolderChange { folder: PathBuf::from("/x"), name: None, kind: FolderChangeKind::Overflow }];
        let _ = update(&mut state, Msg::Changed(run - 1, late), &mut log);
        assert!(state.batches.is_empty(), "an earlier run's batch is not shown");
        let _ = update(&mut state, Msg::Create, &mut log);
        let folder = state.folder.clone().expect("the demo folder");
        assert_eq!(entries(&folder), vec!["note-1.txt".to_owned()], "the buttons still work on the folder");
    }
}
