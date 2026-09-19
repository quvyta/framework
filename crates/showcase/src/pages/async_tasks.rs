//! Async tasks: background work with progress, notes, cancellation and outcomes, shown with a
//! task list, and a child process whose output is streamed into a log view.

use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::{Line, Process, ProcessOutcome, Task, TaskCx, TaskEvent, TaskId, TaskOutcome, Tasks};
use qframe::widgets::{LogBuffer, LogLevel, LogLine, LogView, TaskList};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "async-tasks";

/// Steps of the image build, each with its note.
const BUILD_STEPS: [&str; 4] = ["resolving base image", "compiling release", "writing layers", "pushing to ghcr.io"];

/// Migrations run one after another; how many there are is not reported as a fraction.
const MIGRATIONS: [&str; 5] =
    ["add deploys table", "index deploys by project", "add rollout column", "backfill rollouts", "drop legacy jobs"];

/// What the streamed command prints: a word about the terminal it sees, a step per dependency,
/// a progress line overwritten with `\r`, one line on standard error and a last line.
const SCRIPT: &str = "if test -t 1; then echo 'stdout is a terminal, progress and colour stay on'; \
else echo 'stdout is a pipe, progress and colour are off'; fi; \
for name in base runtime tools; do echo \"resolving $name\"; sleep 0.08; done; \
for step in 25 50 75; do printf 'downloading %s%%\\r' \"$step\"; sleep 0.05; done; \
printf 'downloading 100%%\\n'; \
echo 'the signature of tools is from an unknown key' >&2; \
echo 'installed 3 packages'";

/// Streamed lines kept before the oldest ones fall out.
const OUTPUT_CAPACITY: usize = 2000;

/// The pseudo-terminal the streamed command is given, in cells.
const OUTPUT_SIZE: (u16, u16) = (60, 12);

/// The tasks the demo started, the streamed output and the playground.
#[derive(Debug)]
pub struct State {
    tasks: Tasks,
    cancellable: bool,
    failing: bool,
    digest: Option<String>,
    /// Lines the child process streamed, as a log.
    output: LogBuffer,
    /// The streaming task, while it runs.
    stream: Option<TaskId>,
    /// Whether the child runs on a pseudo-terminal instead of pipes.
    terminal: bool,
    /// Whether the frames a `\r` overwrites are shown as well.
    frames: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tasks: Tasks::new(),
            cancellable: true,
            failing: true,
            digest: None,
            output: LogBuffer::new(OUTPUT_CAPACITY),
            stream: None,
            terminal: false,
            frames: false,
        }
    }
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Build,
    Migrate,
    Sync,
    Event(TaskEvent),
    Built(String),
    Migrated,
    Synced,
    Cancel(TaskId),
    ClearFinished,
    Cancellable(bool),
    Failing(bool),
    Stream,
    StopStream,
    Streamed(Line),
    Overwritten(Line),
    StreamEnded(ProcessOutcome),
    Terminal(bool),
    Frames(bool),
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::AsyncTasks(message))
}

// region: build-task
/// Builds an image in the background: a note per step and a fraction as it goes. Sleeping
/// through `cx.sleep` wakes at once when the task is cancelled.
fn build_image() -> Task<AppMsg> {
    Task::new("Build deploy-api image", |cx| {
        let ticks = 6;
        for (index, step) in BUILD_STEPS.iter().enumerate() {
            cx.note(*step);
            for tick in 0..ticks {
                if !cx.sleep(Duration::from_millis(180)) {
                    return Err("stopped".into());
                }
                let done = index * ticks + tick + 1;
                cx.progress(done as f32 / (BUILD_STEPS.len() * ticks) as f32);
            }
        }
        Ok(send(Msg::Built("sha256:4f2a9c1e".into())))
    })
    .on_event(|event| send(Msg::Event(event)))
}
// endregion

fn run_migrations() -> Task<AppMsg> {
    Task::new("Run database migrations", |cx| {
        for (index, name) in MIGRATIONS.iter().enumerate() {
            cx.note(format!("{} of {}: {name}", index + 1, MIGRATIONS.len()));
            if !cx.sleep(Duration::from_millis(900)) {
                return Err("stopped".into());
            }
        }
        Ok(send(Msg::Migrated))
    })
    .on_event(|event| send(Msg::Event(event)))
}

fn sync_registry(failing: bool) -> Task<AppMsg> {
    Task::new("Sync registry mirror", move |cx| {
        cx.note("comparing manifests");
        for tick in 1..=10 {
            if !cx.sleep(Duration::from_millis(250)) {
                return Err("stopped".into());
            }
            if failing && tick == 6 {
                return Err("registry timed out after 30 s".into());
            }
            cx.progress(tick as f32 / 10.0);
        }
        Ok(send(Msg::Synced))
    })
    .on_event(|event| send(Msg::Event(event)))
}

// region: process-stream
/// Runs a shell command and turns every line it prints into a message, so the log view fills
/// while the command is still running. `cancel` reads the task's own flag, so cancelling the
/// task kills the child; on a pseudo-terminal the child sees a terminal of the size given here
/// and keeps its progress and colour. The script asks nothing, so it gets no standard input:
/// the keys stay the showcase's, and cancelling ends the `sleep` it is waiting on as well.
fn stream_output(terminal: bool, frames: bool) -> Task<AppMsg> {
    Task::new("Install packages", move |cx| {
        let process = Process::new("sh").arg("-c").arg(SCRIPT).env("LC_ALL", "C").env("LANG", "C").no_stdin();
        let process = if terminal { process.pty(OUTPUT_SIZE.0, OUTPUT_SIZE.1) } else { process };
        let outcome = run_process(process, frames, cx).map_err(|error| error.to_string())?;
        Ok(send(Msg::StreamEnded(outcome)))
    })
    .on_event(|event| send(Msg::Event(event)))
}
// endregion

// region: process-frames
/// Runs the child, cancelled with the task. With `frames` every frame a `\r` overwrites, each
/// step of the progress line here, arrives as a message of its own instead of being dropped, so a
/// parser can read the counts in it; the lines themselves stay exactly what `run` delivers.
fn run_process(process: Process, frames: bool, cx: &TaskCx<AppMsg>) -> std::io::Result<ProcessOutcome> {
    let cancel = || cx.is_cancelled();
    let mut on_line = |line| cx.send(send(Msg::Streamed(line)));
    if frames {
        process.run_with_overwritten(&cancel, &mut on_line, &mut |frame| cx.send(send(Msg::Overwritten(frame))))
    } else {
        process.run(&cancel, &mut on_line)
    }
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: start
        Msg::Build => return Command::task(build_image()),
        Msg::Event(event) => {
            if matches!(event, TaskEvent::Finished { .. }) && state.stream == Some(event.id()) {
                state.stream = None;
            }
            log_event(&event, log);
            state.tasks.apply(&event);
        }
        Msg::Cancel(id) => return Command::cancel_task(id),
        // endregion
        Msg::Migrate => return Command::task(run_migrations()),
        Msg::Sync => return Command::task(sync_registry(state.failing)),
        Msg::Built(digest) => {
            log.push(PAGE, "Task#build", format!("delivered {digest}"));
            state.digest = Some(digest);
        }
        Msg::Migrated => log.push(PAGE, "Task#migrate", "delivered"),
        Msg::Synced => log.push(PAGE, "Task#sync", "delivered"),
        Msg::ClearFinished => {
            log.push(PAGE, "Button#clear", "cleared finished tasks");
            state.tasks.clear_finished();
        }
        Msg::Cancellable(on) => {
            log.push(PAGE, "Playground", format!("cancellable = {on}"));
            state.cancellable = on;
        }
        Msg::Failing(on) => {
            log.push(PAGE, "Playground", format!("sync fails = {on}"));
            state.failing = on;
        }
        // region: process-start
        Msg::Stream => {
            let task = stream_output(state.terminal, state.frames);
            state.stream = Some(task.id());
            state.output.clear();
            return Command::task(task);
        }
        Msg::Streamed(line) => {
            let (level, text) = match line {
                Line::Out(text) => (LogLevel::Info, text),
                Line::Err(text) => (LogLevel::Error, text),
            };
            state.output.push(LogLine::new(level, text));
        }
        Msg::Overwritten(frame) => {
            let (Line::Out(text) | Line::Err(text)) = frame;
            state.output.push(LogLine::new(LogLevel::Trace, text));
        }
        Msg::StopStream => {
            if let Some(id) = state.stream {
                return Command::cancel_task(id);
            }
        }
        // endregion
        Msg::StreamEnded(outcome) => {
            let text = match outcome {
                ProcessOutcome::Finished { code: Some(code) } => format!("finished with code {code}"),
                ProcessOutcome::Finished { code: None } => "ended by a signal".to_owned(),
                ProcessOutcome::Cancelled => "cancelled".to_owned(),
            };
            log.push(PAGE, "Process#sh", text);
        }
        Msg::Terminal(on) => {
            log.push(PAGE, "Playground", format!("pseudo-terminal = {on}"));
            state.terminal = on;
        }
        Msg::Frames(on) => {
            log.push(PAGE, "Playground", format!("overwritten frames = {on}"));
            state.frames = on;
        }
    }
    Command::none()
}

/// Logs starts and outcomes; progress is visible in the list itself.
fn log_event(event: &TaskEvent, log: &mut EventLog) {
    let text = match event {
        TaskEvent::Started { label, .. } => format!("started {label:?}"),
        TaskEvent::Progress { .. } => return,
        TaskEvent::Finished { outcome: TaskOutcome::Done, .. } => "done".to_owned(),
        TaskEvent::Finished { outcome: TaskOutcome::Failed(reason), .. } => format!("failed: {reason}"),
        TaskEvent::Finished { outcome: TaskOutcome::Cancelled, .. } => "cancelled".to_owned(),
    };
    log.push(PAGE, format!("TaskEvent#{:?}", event.id()), text);
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("async-tasks.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("async-tasks.build")).variant("primary").on_press(send(Msg::Build))).id("build");
            ui.add(Button::new(t!("async-tasks.migrate")).on_press(send(Msg::Migrate))).id("migrate");
            ui.add(Button::new(t!("async-tasks.sync")).on_press(send(Msg::Sync))).id("sync");
            ui.spacer();
            let finished = state.tasks.entries().len() - state.tasks.running();
            if finished > 0 {
                ui.add(Button::new(t!("async-tasks.clear")).on_press(send(Msg::ClearFinished))).id("clear");
            }
        })
        .gap(2)
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        // region: task-list
        let list = TaskList::new(&state.tasks);
        let list = if state.cancellable { list.on_cancel(|id| send(Msg::Cancel(id))) } else { list };
        list.show(ui).id("tasks");
        // endregion
        if let Some(digest) = &state.digest {
            ui.spacer().height(Length::Cells(1));
            ui.add(Text::new(t!("async-tasks.digest", digest = digest.clone())).role("faint"));
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("async-tasks.process")).gap(0), |ui| {
        ui.add(Text::new(t!("async-tasks.process-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.add(Button::new(t!("async-tasks.stream")).variant("primary").on_press(send(Msg::Stream))).id("stream");
            if state.stream.is_some() {
                ui.add(Button::new(t!("async-tasks.stop")).on_press(send(Msg::StopStream))).id("stop");
            }
        })
        .gap(2)
        .fill_width();
        ui.spacer().height(Length::Cells(1));
        // region: process-view
        ui.add(LogView::new(&state.output).empty_text(t!("async-tasks.output-empty")))
            .width(Length::Fill(1))
            .height(Length::Cells(9))
            .id("output");
        // endregion
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("async-tasks.terminal"), |ui| {
            ui.add(toggle(state.terminal, |on| send(Msg::Terminal(on)))).id("terminal");
        });
        setting(ui, t!("async-tasks.frames"), |ui| {
            ui.add(toggle(state.frames, |on| send(Msg::Frames(on)))).id("frames");
        });
        setting(ui, t!("async-tasks.cancellable"), |ui| {
            ui.add(toggle(state.cancellable, |on| send(Msg::Cancellable(on)))).id("cancellable");
        });
        setting(ui, t!("async-tasks.failing"), |ui| {
            ui.add(toggle(state.failing, |on| send(Msg::Failing(on)))).id("failing");
        });
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn tasks_progress_fail_and_cancel_without_blocking() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("No tasks running"), "{}", h.screen());
        h.click_text("Build image");
        h.click_text("Sync registry");
        h.advance(Duration::from_millis(360));
        let screen = h.screen();
        assert!(screen.contains("resolving base image") && screen.contains("8%"), "{screen}");
        h.advance(Duration::from_millis(1200));
        assert!(h.screen().contains("registry timed out after 30 s"), "{}", h.screen());
        h.click_text("Cancel");
        assert_eq!(h.app().pages.async_tasks.tasks.running(), 0);
        assert!(h.screen().contains("cancelled"));
        h.click_text("Clear finished");
        assert!(h.app().pages.async_tasks.tasks.entries().is_empty());
    }

    #[test]
    fn a_child_process_streams_its_lines_into_the_log_view() {
        let mut h = showcase_on(PAGE);
        assert!(h.screen().contains("No command has run yet."), "{}", h.screen());
        h.click_text("Install packages");
        // The task runs a real command, so the harness waits for it rather than for a clock.
        h.advance(Duration::from_millis(0));
        let screen = h.screen();
        assert!(screen.contains("stdout is a pipe"), "{screen}");
        assert!(screen.contains("resolving tools"), "{screen}");
        assert!(screen.contains("downloading 100%"), "the overwritten lines collapse: {screen}");
        assert!(screen.contains("unknown key"), "the standard error line: {screen}");
        assert!(screen.contains("installed 3 packages"), "{screen}");
        assert_eq!(h.app().pages.async_tasks.stream, None, "the task is finished");
        let log = h.app().log.recent(PAGE, 20);
        assert!(log.iter().any(|entry| entry.message == "finished with code 0"), "{log:?}");
    }

    #[test]
    fn on_a_pseudo_terminal_the_child_sees_a_terminal() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::Terminal(true)));
        h.click_text("Install packages");
        h.advance(Duration::from_millis(0));
        assert!(h.screen().contains("stdout is a terminal"), "{}", h.screen());
    }

    #[test]
    fn overwritten_frames_show_only_when_asked_for() {
        for terminal in [false, true] {
            let mut h = showcase_on(PAGE);
            h.send(send(Msg::Terminal(terminal)));
            h.click_text("Install packages");
            h.advance(Duration::from_millis(0));
            assert!(!h.screen().contains("downloading 50%"), "dropped by default: {}", h.screen());
            h.send(send(Msg::Frames(true)));
            // The finished task's row carries the same label as the button, so the message is sent.
            h.send(send(Msg::Stream));
            h.advance(Duration::from_millis(0));
            let screen = h.screen();
            for frame in ["downloading 25%", "downloading 50%", "downloading 75%", "downloading 100%"] {
                assert!(screen.contains(frame), "{frame} (terminal {terminal}): {screen}");
            }
        }
    }

    #[test]
    fn build_delivers_its_result() {
        let mut h = showcase_on(PAGE);
        h.click_text("Build image").advance(Duration::from_secs(5));
        assert_eq!(h.app().pages.async_tasks.digest.as_deref(), Some("sha256:4f2a9c1e"));
    }
}
