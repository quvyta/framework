//! Rows showing background tasks: spinner, label, step, progress and outcome.

use crate::runtime::{TaskId, TaskOutcome, Tasks};
use crate::text;
use crate::widget::{Length, NodeMut, View};
use crate::widgets::{Button, ProgressBar, Spinner, Text};

type CancelMessage<'a, Msg> = Box<dyn Fn(TaskId) -> Msg + 'a>;

/// Width reserved for a progress bar with its percentage.
const BAR_WIDTH: u16 = 22;

/// Shows a [`Tasks`] model as rows built from existing widgets.
///
/// A running task shows a [`Spinner`], its label, its latest note and, when it reports a
/// fraction, a [`ProgressBar`]. A finished task shows a status icon with a word: a success check
/// with `done`, a danger cross with the failure reason, a muted dash with `cancelled`. Columns
/// line up across rows. With no tasks it shows the empty text in faint type.
///
/// Plain by default; `on_cancel` adds a cancel button to running rows. Words come from
/// `quvyta.tasks.done`, `quvyta.tasks.cancelled`, `quvyta.tasks.cancel` and
/// `quvyta.tasks.empty`; icons `success`, `error` and `check-partial`.
pub struct TaskList<'a, Msg> {
    tasks: &'a Tasks,
    empty: Option<String>,
    on_cancel: Option<CancelMessage<'a, Msg>>,
}

impl<'a, Msg: Clone + 'static> TaskList<'a, Msg> {
    /// Rows for `tasks`.
    #[must_use]
    pub fn new(tasks: &'a Tasks) -> Self {
        Self { tasks, empty: None, on_cancel: None }
    }

    /// Text shown when there are no tasks, instead of `quvyta.tasks.empty`.
    #[must_use]
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = Some(text.into());
        self
    }

    /// Adds a cancel button to every running task; the message usually returns
    /// [`Command::cancel_task`](crate::runtime::Command::cancel_task).
    #[must_use]
    pub fn on_cancel(mut self, message: impl Fn(TaskId) -> Msg + 'a) -> Self {
        self.on_cancel = Some(Box::new(message));
        self
    }

    /// Adds the rows to `ui` as one column.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let i18n = ui.env().i18n();
        let word = |key: &str| i18n.translate(&format!("quvyta.tasks.{key}"), &[]);
        let (done, cancelled, cancel) = (word("done"), word("cancelled"), word("cancel"));
        let empty = self.empty.clone().unwrap_or_else(|| word("empty"));
        let cancel_width = text::width(&cancel).saturating_add(4);
        let icons = ui.env().icons();
        let glyphs = [
            icons.glyph("success").into_owned(),
            icons.glyph("error").into_owned(),
            icons.glyph("check-partial").into_owned(),
        ];
        ui.column(|ui| {
            if self.tasks.entries().is_empty() {
                ui.add(Text::new(empty).role("faint"));
            }
            for entry in self.tasks.entries() {
                let running = entry.outcome.is_none();
                ui.row(|ui| {
                    match &entry.outcome {
                        None => ui.add(Spinner::new()),
                        Some(TaskOutcome::Done) => ui.add(Text::new(glyphs[0].clone()).color("success").no_wrap()),
                        Some(TaskOutcome::Failed(_)) => ui.add(Text::new(glyphs[1].clone()).color("danger").no_wrap()),
                        Some(TaskOutcome::Cancelled) => ui.add(Text::new(glyphs[2].clone()).role("faint").no_wrap()),
                    }
                    .width(Length::Cells(1));
                    let label_role = if running { "body" } else { "secondary" };
                    ui.add(Text::new(entry.label.clone()).role(label_role).no_wrap()).width(Length::Fill(2));
                    let status = match &entry.outcome {
                        None => Text::new(entry.note.clone().unwrap_or_default()).role("faint"),
                        Some(TaskOutcome::Done) => Text::new(done.clone()).color("success"),
                        Some(TaskOutcome::Failed(reason)) => Text::new(reason.clone()).color("danger"),
                        Some(TaskOutcome::Cancelled) => Text::new(cancelled.clone()).role("faint"),
                    };
                    ui.add(status.no_wrap()).width(Length::Fill(3));
                    match entry.fraction.filter(|_| running) {
                        Some(fraction) => ui.add(ProgressBar::new(fraction)),
                        None => ui.spacer(),
                    }
                    .width(Length::Cells(BAR_WIDTH));
                    if let Some(message) = &self.on_cancel {
                        if running {
                            ui.add(Button::new(cancel.clone()).on_press(message(entry.id)))
                                .width(Length::Cells(cancel_width))
                                .id("cancel");
                        } else {
                            ui.spacer().width(Length::Cells(cancel_width));
                        }
                    }
                })
                .gap(2)
                .fill_width()
                .id(format!("task-{:?}", entry.id));
            }
        })
        .fill_width()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness, Task, TaskEvent};

    #[derive(Default)]
    struct Deploys {
        tasks: Tasks,
        cancellable: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Start(&'static str, bool),
        Event(TaskEvent),
        Cancel(TaskId),
        Done,
    }

    impl App for Deploys {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Start(label, fail) => Command::task(
                    Task::new(label, move |cx| {
                        cx.note("pushing layers");
                        for step in 1..=4 {
                            if !cx.sleep(Duration::from_millis(100)) {
                                return Err("stopped".into());
                            }
                            if fail && step == 2 {
                                return Err("registry timed out".into());
                            }
                            cx.progress(step as f32 / 4.0);
                        }
                        Ok(Msg::Done)
                    })
                    .on_event(Msg::Event),
                ),
                Msg::Event(event) => {
                    self.tasks.apply(&event);
                    Command::none()
                }
                Msg::Cancel(id) => Command::cancel_task(id),
                Msg::Done => Command::none(),
            }
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let list = TaskList::new(&self.tasks);
            let list = if self.cancellable { list.on_cancel(Msg::Cancel) } else { list };
            list.show(ui);
        }
    }

    fn deploys(cancellable: bool) -> Harness<Deploys> {
        Harness::new(Deploys { tasks: Tasks::new(), cancellable }, 80, 3)
    }

    #[test]
    fn shows_empty_running_and_finished_rows() {
        let mut h = deploys(false);
        assert_eq!(h.screen(), "No tasks running\n\n\n");
        h.send(Msg::Start("api", true)).advance(Duration::from_millis(100));
        let screen = h.screen();
        assert!(screen.contains("api") && screen.contains("pushing layers") && screen.contains("25%"), "{screen}");
        h.advance(Duration::from_millis(100));
        let screen = h.screen();
        assert!(screen.starts_with("✕  api"), "{screen}");
        assert!(screen.contains("registry timed out"));
        let (x, y) = h.find("registry").expect("reason shown");
        let danger = h.env().theme().color("danger");
        assert_eq!(h.fg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0)), danger);
    }

    #[test]
    fn cancel_buttons_only_when_asked_and_only_on_running_rows() {
        let mut plain = deploys(false);
        plain.send(Msg::Start("web", false));
        assert!(!plain.screen().contains("Cancel"));
        let mut h = deploys(true);
        h.send(Msg::Start("web", false)).advance(Duration::from_millis(100));
        h.click_text("Cancel");
        assert!(h.screen().contains("cancelled"), "{}", h.screen());
        assert!(!h.screen().contains("Cancel "), "{}", h.screen());
        assert_eq!(h.app().tasks.running(), 0);
    }
}
