//! Background tasks with progress, cancellation and an outcome, and the model that tracks them
//! for display.
//!
//! A [`Task`] runs on its own thread and talks to the application only through messages, so
//! drawing never waits for it. The application keeps a [`Tasks`] model up to date from
//! [`TaskEvent`]s and shows it, for example with [`TaskList`](crate::widgets::TaskList).

use std::collections::{HashMap, HashSet};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SendError, Sender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::command::MapFn;

/// Identifies one task for its whole life. Ids are unique within the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// How a task ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    /// The work returned `Ok`; its message was delivered just before this outcome.
    Done,
    /// The work returned `Err` with this reason, or panicked.
    Failed(String),
    /// [`Command::cancel_task`](crate::runtime::Command::cancel_task) asked it to stop; its
    /// result was dropped.
    Cancelled,
}

/// Why [`TaskCx::recv_timeout`] returned without an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecvWait {
    /// The time ran out before an item came.
    Timeout,
    /// The task was asked to stop.
    Cancelled,
    /// Every sender is gone and nothing is left to receive.
    Closed,
}

/// What happened to a task, delivered through [`Task::on_event`].
#[derive(Debug, Clone, PartialEq)]
pub enum TaskEvent {
    /// The task was started with this label.
    Started {
        /// The task.
        id: TaskId,
        /// Its label.
        label: String,
    },
    /// The task reported progress. `None` fields keep their previous value.
    Progress {
        /// The task.
        id: TaskId,
        /// Completed share from 0 to 1.
        fraction: Option<f32>,
        /// A short note about the current step.
        note: Option<String>,
    },
    /// The task ended.
    Finished {
        /// The task.
        id: TaskId,
        /// How.
        outcome: TaskOutcome,
    },
}

impl TaskEvent {
    /// The task the event is about.
    #[must_use]
    pub fn id(&self) -> TaskId {
        match self {
            Self::Started { id, .. } | Self::Progress { id, .. } | Self::Finished { id, .. } => *id,
        }
    }
}

type Work<Msg> = Box<dyn FnOnce(&TaskCx<Msg>) -> Result<Msg, String> + Send>;
type EventMessage<Msg> = Arc<dyn Fn(TaskEvent) -> Msg + Send + Sync>;
/// Hands a message of running work to the event loop.
type Deliver<Msg> = Arc<dyn Fn(Msg) + Send + Sync>;
/// Hands a task event, already turned into a message, to the event loop.
type Report = Arc<dyn Fn(TaskEvent) + Send + Sync>;

/// Work to run in the background with progress, cancellation and an outcome.
///
/// ```
/// use std::time::Duration;
/// use qframe::runtime::{Command, Task, TaskEvent};
///
/// enum Msg {
///     Task(TaskEvent),
///     Built(String),
/// }
///
/// let task = Task::new("Build image", |cx| {
///     for step in 0..4 {
///         if !cx.sleep(Duration::from_millis(300)) {
///             return Err("stopped".into());
///         }
///         cx.progress((step + 1) as f32 / 4.0);
///     }
///     Ok(Msg::Built("sha256:4f2a".into()))
/// })
/// .on_event(Msg::Task);
/// let id = task.id(); // keep it to cancel the task later
/// let command: Command<Msg> = Command::task(task);
/// # let _ = (id, command);
/// ```
pub struct Task<Msg> {
    id: TaskId,
    label: String,
    work: Work<Msg>,
    on_event: Option<EventMessage<Msg>>,
}

impl<Msg: Send + 'static> Task<Msg> {
    /// A task labelled `label` running `work`. `Ok` delivers its message; `Err` fails the task
    /// with a reason.
    #[must_use]
    pub fn new(
        label: impl Into<String>,
        work: impl FnOnce(&TaskCx<Msg>) -> Result<Msg, String> + Send + 'static,
    ) -> Self {
        Self { id: TaskId::next(), label: label.into(), work: Box::new(work), on_event: None }
    }

    /// Turns start, progress and the outcome into messages, e.g. to update a [`Tasks`] model.
    #[must_use]
    pub fn on_event(mut self, message: impl Fn(TaskEvent) -> Msg + Send + Sync + 'static) -> Self {
        self.on_event = Some(Arc::new(message));
        self
    }

    /// The task's id, known before it starts.
    #[must_use]
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// The task's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The same task delivering `map(message)` for every message it would deliver: its result,
    /// what the work sends while it runs and its events.
    pub(crate) fn map<B: Send + 'static>(self, map: MapFn<Msg, B>) -> Task<B> {
        let Self { id, label, work, on_event } = self;
        let on_event = on_event.map(|message| {
            let map = Arc::clone(&map);
            Arc::new(move |event| map(message(event))) as EventMessage<B>
        });
        let work: Work<B> = Box::new(move |cx: &TaskCx<B>| {
            let deliver = Arc::clone(&cx.deliver);
            let inner_map = Arc::clone(&map);
            let inner = TaskCx {
                id: cx.id,
                clock: Arc::clone(&cx.clock),
                deliver: Arc::new(move |message| deliver(inner_map(message))),
                report: cx.report.clone(),
            };
            work(&inner).map(|message| map(message))
        });
        Task { id, label, work, on_event }
    }
}

/// A message from a background thread to the event loop.
pub(crate) enum Delivery<Msg> {
    /// Apply this message.
    Message(Msg),
    /// A piece of background work ended.
    Ended,
}

/// Where background work hands the loop what it has: a channel whose every send also wakes the
/// loop, so a result is applied the moment it arrives. The loop then never has to look for one,
/// and an application with long work running sleeps as soundly as an idle one.
pub(crate) struct Outlet<Msg>(Sender<Delivery<Msg>>);

impl<Msg> Clone for Outlet<Msg> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Msg> Outlet<Msg> {
    /// Hands `delivery` to the loop and wakes it.
    ///
    /// # Errors
    ///
    /// The delivery back, once the loop has gone.
    pub(crate) fn send(&self, delivery: Delivery<Msg>) -> Result<(), SendError<Delivery<Msg>>> {
        let sent = self.0.send(delivery);
        super::signals::wake();
        sent
    }
}

/// A channel for background work: the outlet the work sends on, and the end the loop reads.
pub(crate) fn channel<Msg>() -> (Outlet<Msg>, Receiver<Delivery<Msg>>) {
    let (sender, receiver) = mpsc::channel();
    (Outlet(sender), receiver)
}

/// Time as tasks see it: the real clock, or the test harness's fake clock that only moves when
/// the test advances it.
pub(crate) struct TaskClock {
    fake: bool,
    state: Mutex<ClockState>,
    changed: Condvar,
}

#[derive(Default)]
struct ClockState {
    now: Duration,
    /// Tasks that are working rather than sleeping.
    busy: usize,
    cancelled: HashSet<TaskId>,
    /// Fake clock only: tasks asleep and when they wake. Whoever wakes a sleeper (the clock
    /// moving or a cancel) counts it as busy right away, so settling never misses it.
    sleeping: HashMap<TaskId, Duration>,
    /// Fake clock only: where each task is in time. A task woken by a big clock jump carries on
    /// from the moment its sleep ended, so a loop of sleeps keeps its rhythm.
    task_time: HashMap<TaskId, Duration>,
}

/// How long a task waiting for a channel item listens before it looks again whether it was
/// cancelled. A channel of the standard library cannot be woken by anything but its own senders,
/// so a cancel reaches a waiting task within this.
const RECV_SLICE: Duration = Duration::from_millis(50);

/// How long the harness waits for tasks to reach a sleep before it gives up.
const SETTLE_LIMIT: Duration = Duration::from_secs(10);

impl TaskClock {
    pub(crate) fn new(fake: bool) -> Arc<Self> {
        Arc::new(Self { fake, state: Mutex::new(ClockState::default()), changed: Condvar::new() })
    }

    fn lock(&self) -> MutexGuard<'_, ClockState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Asks task `id` to stop; its sleeps return at once.
    pub(crate) fn cancel(&self, id: TaskId) {
        let mut state = self.lock();
        state.cancelled.insert(id);
        if state.sleeping.remove(&id).is_some() {
            state.busy += 1;
            let now = state.now;
            state.task_time.insert(id, now);
        }
        drop(state);
        self.changed.notify_all();
    }

    /// Moves the fake clock to `now` and waits until every task is asleep or finished.
    ///
    /// # Panics
    ///
    /// Panics when a task keeps working for longer than ten seconds, which in a test means it
    /// never sleeps or blocks forever.
    pub(crate) fn settle(&self, now: Duration) {
        let mut state = self.lock();
        state.now = state.now.max(now);
        let current = state.now;
        let due: Vec<(TaskId, Duration)> =
            state.sleeping.iter().filter(|(_, until)| **until <= current).map(|(id, until)| (*id, *until)).collect();
        for (id, until) in due {
            state.sleeping.remove(&id);
            state.task_time.insert(id, until);
            state.busy += 1;
        }
        self.changed.notify_all();
        let started = Instant::now();
        while state.busy > 0 {
            let waited = started.elapsed();
            assert!(waited < SETTLE_LIMIT, "a background task kept working for {SETTLE_LIMIT:?} without sleeping");
            state = self.changed.wait_timeout(state, SETTLE_LIMIT - waited).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    fn is_cancelled(&self, id: TaskId) -> bool {
        self.lock().cancelled.contains(&id)
    }

    fn begin(&self, id: TaskId) {
        let mut state = self.lock();
        state.busy += 1;
        let now = state.now;
        state.task_time.insert(id, now);
    }

    fn end(&self, id: TaskId) {
        let mut state = self.lock();
        state.busy = state.busy.saturating_sub(1);
        state.cancelled.remove(&id);
        state.task_time.remove(&id);
        drop(state);
        self.changed.notify_all();
    }

    /// Sleeps task `id` for `duration`; returns `false` when it was cancelled.
    fn sleep(&self, id: TaskId, duration: Duration) -> bool {
        let mut state = self.lock();
        if self.fake {
            let until = state.task_time.get(&id).copied().unwrap_or(state.now) + duration;
            if until <= state.now {
                state.task_time.insert(id, until);
            } else if !state.cancelled.contains(&id) {
                state.sleeping.insert(id, until);
                state.busy = state.busy.saturating_sub(1);
                self.changed.notify_all();
                while state.sleeping.contains_key(&id) {
                    state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
                }
            }
        } else {
            let deadline = Instant::now() + duration;
            while !state.cancelled.contains(&id) {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                state = self.changed.wait_timeout(state, left).unwrap_or_else(PoisonError::into_inner).0;
            }
        }
        !state.cancelled.contains(&id)
    }
}

impl TaskClock {
    /// Waits for task `id`'s next item from `rx`, for at most `timeout` when one is given.
    ///
    /// On the fake clock a waiting task rests like a sleeper, so the harness does not wait for
    /// it: `timeout` runs on the fake clock, and a cancel or the clock passing the deadline wakes
    /// it and counts it as busy, as they do a sleeper. An item wakes it too, counting it as busy
    /// itself unless the clock or a cancel already did.
    fn receive<T>(&self, id: TaskId, rx: &Receiver<T>, timeout: Option<Duration>) -> Result<T, RecvWait> {
        if self.is_cancelled(id) {
            return Err(RecvWait::Cancelled);
        }
        match rx.try_recv() {
            Ok(item) => return Ok(item),
            Err(TryRecvError::Disconnected) => return Err(RecvWait::Closed),
            Err(TryRecvError::Empty) => {}
        }
        if timeout.is_some_and(|timeout| timeout.is_zero()) {
            return Err(RecvWait::Timeout);
        }
        if self.fake { self.receive_fake(id, rx, timeout) } else { self.receive_real(id, rx, timeout) }
    }

    fn receive_real<T>(&self, id: TaskId, rx: &Receiver<T>, timeout: Option<Duration>) -> Result<T, RecvWait> {
        let deadline = timeout.and_then(|timeout| Instant::now().checked_add(timeout));
        loop {
            if self.is_cancelled(id) {
                return Err(RecvWait::Cancelled);
            }
            let left = deadline.map_or(RECV_SLICE, |deadline| deadline.saturating_duration_since(Instant::now()));
            if left.is_zero() {
                return Err(RecvWait::Timeout);
            }
            match rx.recv_timeout(left.min(RECV_SLICE)) {
                Ok(item) => return Ok(item),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Err(RecvWait::Closed),
            }
        }
    }

    fn receive_fake<T>(&self, id: TaskId, rx: &Receiver<T>, timeout: Option<Duration>) -> Result<T, RecvWait> {
        let mut state = self.lock();
        // A cancel that came since the first look found no sleeper to wake, so nothing would wake
        // this one.
        if state.cancelled.contains(&id) {
            return Err(RecvWait::Cancelled);
        }
        // A wait without a deadline rests until an item, a cancel or a closed channel.
        let until = match timeout {
            Some(timeout) => state.task_time.get(&id).copied().unwrap_or(state.now).saturating_add(timeout),
            None => Duration::MAX,
        };
        // A task behind the clock has already waited the time out, as a sleep would have.
        if until <= state.now {
            state.task_time.insert(id, until);
            return Err(RecvWait::Timeout);
        }
        state.sleeping.insert(id, until);
        state.busy = state.busy.saturating_sub(1);
        drop(state);
        self.changed.notify_all();
        loop {
            let heard = rx.recv_timeout(RECV_SLICE);
            let mut state = self.lock();
            let woken = !state.sleeping.contains_key(&id);
            match heard {
                Err(RecvTimeoutError::Timeout) if !woken => continue,
                // Nobody woke the task, so it counts itself busy again, from the present moment.
                Ok(_) | Err(RecvTimeoutError::Disconnected) if !woken => {
                    state.sleeping.remove(&id);
                    state.busy += 1;
                    let now = state.now;
                    state.task_time.insert(id, now);
                }
                _ => {}
            }
            return match heard {
                _ if state.cancelled.contains(&id) => Err(RecvWait::Cancelled),
                Ok(item) => Ok(item),
                Err(RecvTimeoutError::Disconnected) => Err(RecvWait::Closed),
                Err(RecvTimeoutError::Timeout) => Err(RecvWait::Timeout),
            };
        }
    }
}

/// What running work can do: report progress, send messages, notice cancellation and sleep.
pub struct TaskCx<Msg> {
    id: TaskId,
    clock: Arc<TaskClock>,
    deliver: Deliver<Msg>,
    /// Events go out as messages of the application's type, which a task built for another
    /// message type ([`Command::map`](crate::runtime::Command::map)) does not know.
    report: Option<Report>,
}

impl<Msg: Send + 'static> TaskCx<Msg> {
    /// The running task.
    #[must_use]
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Reports the completed share, from 0 to 1.
    pub fn progress(&self, fraction: f32) {
        self.event(TaskEvent::Progress { id: self.id, fraction: Some(fraction.clamp(0.0, 1.0)), note: None });
    }

    /// Describes the current step, e.g. "pushing layers".
    pub fn note(&self, note: impl Into<String>) {
        self.event(TaskEvent::Progress { id: self.id, fraction: None, note: Some(note.into()) });
    }

    /// Delivers `message` to the application while the work goes on, e.g. a log line.
    pub fn send(&self, message: Msg) {
        (self.deliver)(message);
    }

    /// Whether the application asked this task to stop. Long work should check it and return.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.clock.is_cancelled(self.id)
    }

    /// Waits `duration`, waking early when the task is cancelled. Returns `false` when it was
    /// cancelled. In tests the harness's fake clock decides when the sleep ends.
    #[must_use]
    pub fn sleep(&self, duration: Duration) -> bool {
        self.clock.sleep(self.id, duration)
    }

    /// Waits for the next item from `rx`, the receiving end of a channel another thread feeds,
    /// such as a connection handing over what it read. Returns `None` when the task is cancelled
    /// (which wins over an item already waiting) or when every sender is gone and nothing is
    /// left. A cancel reaches a waiting task within 50 ms: a standard channel wakes only for its
    /// senders, so the wait looks at the cancel in slices.
    ///
    /// In a [`Harness`](crate::runtime::Harness) a task waiting here rests like one asleep, so no
    /// step of the harness waits for it. An item sent from the test wakes it on its own thread;
    /// the harness applies what the task then sends at a later step, such as
    /// [`advance`](crate::runtime::Harness::advance).
    #[must_use]
    pub fn recv<T>(&self, rx: &Receiver<T>) -> Option<T> {
        self.clock.receive(self.id, rx, None).ok()
    }

    /// Like [`TaskCx::recv`], giving up after `timeout`, and telling why no item came. In a
    /// [`Harness`](crate::runtime::Harness) the timeout runs on the fake clock, like
    /// [`TaskCx::sleep`].
    ///
    /// # Errors
    ///
    /// [`RecvWait::Timeout`] when `timeout` passed without an item, [`RecvWait::Cancelled`]
    /// when the task was cancelled and [`RecvWait::Closed`] when every sender is gone and
    /// nothing is left to receive.
    pub fn recv_timeout<T>(&self, rx: &Receiver<T>, timeout: Duration) -> Result<T, RecvWait> {
        self.clock.receive(self.id, rx, Some(timeout))
    }

    fn event(&self, event: TaskEvent) {
        if let Some(report) = &self.report {
            report(event);
        }
    }
}

/// Starts a thread named `name` running `run`. A failed start drops `run`.
pub(crate) type Spawner = fn(String, Box<dyn FnOnce() + Send>) -> io::Result<()>;

/// The [`Spawner`] of the runtime: a real thread.
pub(crate) fn spawn_thread(name: String, run: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    std::thread::Builder::new().name(name).spawn(run).map(drop)
}

/// The reason of a task whose thread could not start.
const NO_THREAD: &str = "could not start a thread";

/// Starts `task` on a thread. The `Started` message is returned for the caller to apply at once.
/// When no thread can start, the task fails at once and still ends.
pub(crate) fn spawn<Msg: Send + 'static>(
    task: Task<Msg>,
    clock: &Arc<TaskClock>,
    sender: &Outlet<Msg>,
    spawner: Spawner,
) -> Option<Msg> {
    let Task { id, label, work, on_event } = task;
    let started = on_event.as_ref().map(|message| message(TaskEvent::Started { id, label: label.clone() }));
    let failed = on_event.clone();
    let outlet = sender.clone();
    let deliver: Deliver<Msg> = Arc::new(move |message| {
        // A message from a task that outlived the loop has nowhere to be handled. The task is
        // still ended below, so nothing waits for it either way.
        let _ = outlet.send(Delivery::Message(message));
    });
    let report = on_event.map(|message| {
        let deliver = Arc::clone(&deliver);
        Arc::new(move |event| deliver(message(event))) as Report
    });
    let cx = TaskCx { id, clock: Arc::clone(clock), deliver, report };
    let ended = sender.clone();
    clock.begin(id);
    let run = Box::new(move || {
        let result = catch_unwind(AssertUnwindSafe(|| work(&cx)))
            .unwrap_or_else(|_| Err(format!("the task `{label}` panicked")));
        let outcome = match result {
            _ if cx.is_cancelled() => TaskOutcome::Cancelled,
            Ok(message) => {
                cx.send(message);
                TaskOutcome::Done
            }
            Err(reason) => TaskOutcome::Failed(reason),
        };
        // The event's message is the application's code (and a conversion of `Command::map`);
        // if it panics there is nothing left to tell, but the task must still end, or the
        // runtime would wait for it forever.
        let _ = catch_unwind(AssertUnwindSafe(|| cx.event(TaskEvent::Finished { id, outcome })));
        let _ = ended.send(Delivery::Ended);
        cx.clock.end(id);
    });
    if spawner(format!("quvyta-task-{}", id.0), run).is_err() {
        // The work went with the closure; report the failure so the task never stays running.
        clock.end(id);
        if let Some(message) = failed {
            let outcome = TaskOutcome::Failed(NO_THREAD.to_owned());
            let _ = sender.send(Delivery::Message(message(TaskEvent::Finished { id, outcome })));
        }
        // Both sends fail only once the loop has gone, and a loop that has gone is not counting
        // tasks any more. What must not happen is the task ending without this, which would
        // leave the loop waiting for a task that never ran.
        let _ = sender.send(Delivery::Ended);
    }
    started
}

/// One task as the application shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskEntry {
    /// The task.
    pub id: TaskId,
    /// Its label.
    pub label: String,
    /// Completed share from 0 to 1, when the task reports one.
    pub fraction: Option<f32>,
    /// The latest note.
    pub note: Option<String>,
    /// How it ended; `None` while running.
    pub outcome: Option<TaskOutcome>,
}

/// Tasks an application shows, kept up to date with [`Tasks::apply`]. Newest last.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tasks {
    entries: Vec<TaskEntry>,
}

impl Tasks {
    /// No tasks.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `event`.
    pub fn apply(&mut self, event: &TaskEvent) {
        match event {
            TaskEvent::Started { id, label } => {
                self.entries.retain(|entry| entry.id != *id);
                self.entries.push(TaskEntry {
                    id: *id,
                    label: label.clone(),
                    fraction: None,
                    note: None,
                    outcome: None,
                });
            }
            TaskEvent::Progress { id, fraction, note } => {
                if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == *id) {
                    entry.fraction = fraction.or(entry.fraction);
                    if note.is_some() {
                        entry.note.clone_from(note);
                    }
                }
            }
            TaskEvent::Finished { id, outcome } => {
                if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == *id) {
                    entry.outcome = Some(outcome.clone());
                }
            }
        }
    }

    /// Every task, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[TaskEntry] {
        &self.entries
    }

    /// The task `id`.
    #[must_use]
    pub fn get(&self, id: TaskId) -> Option<&TaskEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// How many tasks are still running.
    #[must_use]
    pub fn running(&self) -> usize {
        self.entries.iter().filter(|entry| entry.outcome.is_none()).count()
    }

    /// Forgets finished tasks.
    pub fn clear_finished(&mut self) {
        self.entries.retain(|entry| entry.outcome.is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::engine::{Engine, TaskMode};
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::Text;

    #[derive(Default)]
    struct Pipeline {
        tasks: Tasks,
        built: Option<String>,
        lines: Vec<String>,
        build: Option<TaskId>,
    }

    enum Msg {
        Build,
        Cancel,
        Fail,
        Panic,
        Task(TaskEvent),
        Built(String),
        Line(String),
    }

    impl App for Pipeline {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Build => {
                    let task = Task::new("Build image", |cx| {
                        cx.note("resolving layers");
                        for step in 0..4 {
                            if !cx.sleep(Duration::from_millis(100)) {
                                return Err("stopped".into());
                            }
                            cx.progress((step + 1) as f32 / 4.0);
                            cx.send(Msg::Line(format!("layer {step}")));
                        }
                        Ok(Msg::Built("sha256:4f2a".into()))
                    })
                    .on_event(Msg::Task);
                    self.build = Some(task.id());
                    return Command::task(task);
                }
                Msg::Cancel => return self.build.map_or_else(Command::none, Command::cancel_task),
                Msg::Fail => {
                    return Command::task(
                        Task::new("Sync registry", |cx| {
                            let _ = cx.sleep(Duration::from_millis(50));
                            Err("registry timed out".into())
                        })
                        .on_event(Msg::Task),
                    );
                }
                Msg::Panic => {
                    return Command::task(
                        Task::new("Broken", |_| -> Result<Msg, String> { panic!("boom") }).on_event(Msg::Task),
                    );
                }
                Msg::Task(event) => self.tasks.apply(&event),
                Msg::Built(digest) => self.built = Some(digest),
                Msg::Line(line) => self.lines.push(line),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add(Text::new(format!("running {}", self.tasks.running())));
        }
    }

    #[test]
    fn progress_follows_the_fake_clock_and_completes() {
        let mut h = Harness::new(Pipeline::default(), 20, 1);
        h.send(Msg::Build);
        assert_eq!(h.screen(), "running 1\n");
        let entry = h.app().tasks.entries()[0].clone();
        assert_eq!(entry.label, "Build image");
        assert_eq!(entry.note.as_deref(), Some("resolving layers"));
        assert_eq!(entry.fraction, None);
        h.advance(Duration::from_millis(100));
        assert_eq!(h.app().tasks.entries()[0].fraction, Some(0.25));
        assert_eq!(h.app().lines, ["layer 0"]);
        h.advance(Duration::from_millis(250));
        assert_eq!(h.app().tasks.entries()[0].fraction, Some(0.75));
        h.advance(Duration::from_millis(100));
        assert_eq!(h.app().built.as_deref(), Some("sha256:4f2a"));
        assert_eq!(h.app().tasks.entries()[0].outcome, Some(TaskOutcome::Done));
        assert_eq!(h.screen(), "running 0\n");
    }

    #[test]
    fn cancelling_wakes_the_sleep_and_drops_the_result() {
        let mut h = Harness::new(Pipeline::default(), 20, 1);
        h.send(Msg::Build).advance(Duration::from_millis(150)).send(Msg::Cancel);
        let entry = &h.app().tasks.entries()[0];
        assert_eq!(entry.outcome, Some(TaskOutcome::Cancelled));
        assert_eq!(entry.fraction, Some(0.25));
        assert!(h.app().built.is_none());
    }

    fn no_thread(_: String, _: Box<dyn FnOnce() + Send>) -> io::Result<()> {
        Err(io::Error::other("no threads left"))
    }

    #[test]
    fn a_task_whose_thread_cannot_start_fails_and_ends() {
        let mut engine = Engine::new(Pipeline::default(), crate::env::Env::builtin(), TaskMode::Threads);
        engine.spawner = no_thread;
        engine.update(Msg::Build);
        assert_eq!(engine.poll_tasks(), 2, "Finished, then Ended");
        let entry = &engine.app.tasks.entries()[0];
        assert_eq!(entry.outcome, Some(TaskOutcome::Failed("could not start a thread".into())));
        assert_eq!((engine.app.tasks.running(), engine.pending_tasks), (0, 0));
        assert!(engine.app.built.is_none());
    }

    /// Starts, on `Some(())`, a task whose event message panics once the task finishes.
    struct Fragile;

    impl App for Fragile {
        type Msg = Option<()>;
        fn update(&mut self, start: Option<()>) -> Command<Option<()>> {
            if start.is_none() {
                return Command::none();
            }
            Command::task(Task::new("Fragile", |_| Ok(None)).on_event(|event| match event {
                TaskEvent::Finished { .. } => panic!("the message of the outcome failed"),
                _ => None,
            }))
        }
        fn view(&self, ui: &mut View<'_, Option<()>>) {
            ui.add(Text::new("fragile"));
        }
    }

    #[test]
    fn a_task_whose_last_event_message_panics_still_ends() {
        let mut engine = Engine::new(Fragile, crate::env::Env::builtin(), TaskMode::Threads);
        engine.update(Some(()));
        let started = Instant::now();
        while engine.pending_tasks > 0 {
            assert!(started.elapsed() < Duration::from_secs(10), "the runtime waits for the task forever");
            engine.poll_tasks();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn failures_and_panics_become_outcomes() {
        let mut h = Harness::new(Pipeline::default(), 20, 1);
        h.send(Msg::Fail).send(Msg::Panic);
        assert_eq!(h.app().tasks.running(), 1, "the failing task still sleeps");
        assert_eq!(h.app().tasks.entries()[1].outcome, Some(TaskOutcome::Failed("the task `Broken` panicked".into())));
        h.advance(Duration::from_millis(50));
        assert_eq!(h.app().tasks.entries()[0].outcome, Some(TaskOutcome::Failed("registry timed out".into())));
        let mut tasks = h.app().tasks.clone();
        tasks.clear_finished();
        assert!(tasks.entries().is_empty());
    }

    /// Listens to a channel in a task and hands every line it hears to the application.
    #[derive(Default)]
    struct Listener {
        rx: Option<std::sync::mpsc::Receiver<String>>,
        tasks: Tasks,
        heard: Vec<String>,
        ended: Option<String>,
        listening: Option<TaskId>,
    }

    enum Heard {
        /// Listens until the channel closes or the task is cancelled.
        Listen,
        /// Waits for one line at most this long and tells why none came.
        Wait(Duration),
        Cancel,
        Task(TaskEvent),
        Line(String),
        Ended(String),
    }

    impl App for Listener {
        type Msg = Heard;
        fn update(&mut self, msg: Heard) -> Command<Heard> {
            match msg {
                Heard::Listen => {
                    let rx = self.rx.take().expect("one channel per test");
                    let task = Task::new("Listen", move |cx| {
                        while let Some(line) = cx.recv(&rx) {
                            cx.send(Heard::Line(line));
                        }
                        Ok(Heard::Ended("closed".into()))
                    })
                    .on_event(Heard::Task);
                    self.listening = Some(task.id());
                    return Command::task(task);
                }
                Heard::Wait(timeout) => {
                    let rx = self.rx.take().expect("one channel per test");
                    let task = Task::new("Wait", move |cx| {
                        let why = match cx.recv_timeout(&rx, timeout) {
                            Ok(line) => line,
                            Err(why) => format!("{why:?}"),
                        };
                        Ok(Heard::Ended(why))
                    })
                    .on_event(Heard::Task);
                    self.listening = Some(task.id());
                    return Command::task(task);
                }
                Heard::Cancel => return self.listening.map_or_else(Command::none, Command::cancel_task),
                Heard::Task(event) => self.tasks.apply(&event),
                Heard::Line(line) => self.heard.push(line),
                Heard::Ended(why) => self.ended = Some(why),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Heard>) {
            ui.add(Text::new(format!("heard {}", self.heard.len())));
        }
    }

    /// A listener on real threads with the sending end of its channel.
    fn listening_engine() -> (Engine<Listener>, std::sync::mpsc::Sender<String>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let app = Listener { rx: Some(rx), ..Listener::default() };
        (Engine::new(app, crate::env::Env::builtin(), TaskMode::Threads), tx)
    }

    /// Polls `engine` until `done` holds, failing after a generous bound.
    fn poll_until(engine: &mut Engine<Listener>, what: &str, done: impl Fn(&Listener) -> bool) {
        let started = Instant::now();
        while !done(&engine.app) {
            assert!(started.elapsed() < Duration::from_secs(20), "{what} never happened");
            engine.poll_tasks();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_task_receives_what_another_thread_sends_and_ends_when_the_channel_closes() {
        let (mut engine, tx) = listening_engine();
        engine.update(Heard::Listen);
        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            tx.send("GET /index.html 200".into()).expect("the task listens");
            tx.send("GET /style.css 304".into()).expect("the task listens");
        });
        poll_until(&mut engine, "the lines", |app| app.heard.len() == 2);
        assert_eq!(engine.app.heard, ["GET /index.html 200", "GET /style.css 304"]);
        sender.join().expect("the sender ends");
        poll_until(&mut engine, "the end", |app| app.ended.is_some());
        assert_eq!(engine.app.ended.as_deref(), Some("closed"), "the dropped sender ended the wait");
        assert_eq!(engine.app.tasks.entries()[0].outcome, Some(TaskOutcome::Done));
    }

    #[test]
    fn cancelling_ends_a_wait_for_the_channel_promptly() {
        let (mut engine, tx) = listening_engine();
        engine.update(Heard::Listen);
        std::thread::sleep(Duration::from_millis(100));
        let asked = Instant::now();
        engine.update(Heard::Cancel);
        poll_until(&mut engine, "the cancel", |app| app.tasks.running() == 0);
        assert!(asked.elapsed() < Duration::from_secs(5), "the cancel took {:?}", asked.elapsed());
        assert_eq!(engine.app.tasks.entries()[0].outcome, Some(TaskOutcome::Cancelled));
        assert!(tx.send("late".into()).is_err(), "the task and its receiver are gone");
    }

    #[test]
    fn a_wait_with_a_timeout_says_why_no_item_came() {
        let (mut engine, tx) = listening_engine();
        engine.update(Heard::Wait(Duration::from_millis(40)));
        poll_until(&mut engine, "the timeout", |app| app.ended.is_some());
        assert_eq!(engine.app.ended.as_deref(), Some("Timeout"));
        drop(tx);

        let (mut engine, tx) = listening_engine();
        drop(tx);
        engine.update(Heard::Wait(Duration::from_secs(30)));
        poll_until(&mut engine, "the closed channel", |app| app.ended.is_some());
        assert_eq!(engine.app.ended.as_deref(), Some("Closed"));
    }

    #[test]
    fn in_the_harness_a_waiting_task_rests_and_still_hears_and_cancels() {
        let (tx, rx) = std::sync::mpsc::channel();
        let started = Instant::now();
        let mut h = Harness::new(Listener { rx: Some(rx), ..Listener::default() }, 20, 1);
        h.send(Heard::Listen);
        assert_eq!(h.app().tasks.running(), 1, "the harness did not wait for the listening task");
        tx.send("GET /health 200".into()).expect("the task listens");
        while h.app().heard.is_empty() {
            assert!(started.elapsed() < Duration::from_secs(20), "the line never arrived");
            std::thread::sleep(Duration::from_millis(5));
            h.advance(Duration::ZERO);
        }
        assert_eq!(h.screen(), "heard 1\n");
        h.send(Heard::Cancel);
        assert_eq!(h.app().tasks.entries()[0].outcome, Some(TaskOutcome::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(20), "the harness hung on the wait");
    }

    #[test]
    fn in_the_harness_a_timeout_follows_the_fake_clock() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let mut h = Harness::new(Listener { rx: Some(rx), ..Listener::default() }, 20, 1);
        h.send(Heard::Wait(Duration::from_secs(60)));
        h.advance(Duration::from_secs(59));
        assert_eq!(h.app().ended, None, "a minute has not passed on the fake clock");
        h.advance(Duration::from_secs(1));
        assert_eq!(h.app().ended.as_deref(), Some("Timeout"));
        drop(tx);
    }
}
