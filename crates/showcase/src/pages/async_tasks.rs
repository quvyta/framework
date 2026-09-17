//! Async tasks: background work with progress, notes, cancellation and outcomes, shown with a
//! task list.

use std::time::Duration;

use qframe::prelude::*;
use qframe::runtime::{Task, TaskEvent, TaskId, TaskOutcome, Tasks};
use qframe::widgets::TaskList;

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "async-tasks";

/// Steps of the image build, each with its note.
const BUILD_STEPS: [&str; 4] = ["resolving base image", "compiling release", "writing layers", "pushing to ghcr.io"];

/// Migrations run one after another; how many there are is not reported as a fraction.
const MIGRATIONS: [&str; 5] =
    ["add deploys table", "index deploys by project", "add rollout column", "backfill rollouts", "drop legacy jobs"];

/// The tasks the demo started and the playground.
#[derive(Debug)]
pub struct State {
    tasks: Tasks,
    cancellable: bool,
    failing: bool,
    digest: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self { tasks: Tasks::new(), cancellable: true, failing: true, digest: None }
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

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: start
        Msg::Build => return Command::task(build_image()),
        Msg::Event(event) => {
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

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
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
    fn build_delivers_its_result() {
        let mut h = showcase_on(PAGE);
        h.click_text("Build image").advance(Duration::from_secs(5));
        assert_eq!(h.app().pages.async_tasks.digest.as_deref(), Some("sha256:4f2a9c1e"));
    }
}
