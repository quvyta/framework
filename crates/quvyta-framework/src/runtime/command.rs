//! Work an application asks the runtime to do after an update.

use std::sync::Arc;

use super::confirm::Confirm;
use super::detached::DetachedHandoff;
use super::handoff::Handoff;
use super::open::Open;
use super::task::{Task, TaskId};
use crate::icons::IconMode;
use crate::widgets::{Corner, Toast};

pub(crate) enum Action<Msg> {
    Quit,
    Focus(String),
    SetTheme(String),
    SetLocale(String),
    SetRegion(Option<String>),
    SetIconMode(IconMode),
    SetReducedMotion(bool),
    SetPillar(crate::icons::PillarStyle),
    SetSlide(bool),
    Copy(String),
    Confirm(Confirm<Msg>),
    ReadClipboard(Box<dyn FnOnce(Option<String>) -> Msg>),
    Perform(Box<dyn FnOnce() -> Msg + Send>),
    Toast(Toast<Msg>),
    DismissToast(String),
    ToastCorner(Corner),
    Task(Task<Msg>),
    CancelTask(TaskId),
    Handoff(Handoff<Msg>),
    HandoffDetached(DetachedHandoff<Msg>),
    Open(Open<Msg>),
    #[cfg(feature = "updates")]
    CheckForUpdate(super::update_check::UpdateCheck<Msg>),
}

/// A message conversion shared by every action of a mapped command; the work of tasks and
/// performs calls it on their own threads.
pub(crate) type MapFn<A, B> = Arc<dyn Fn(A) -> B + Send + Sync>;

impl<A: Send + 'static> Action<A> {
    /// The same action delivering `map(message)` wherever it would deliver `message`.
    fn map<B: Send + 'static>(self, map: &MapFn<A, B>) -> Action<B> {
        match self {
            Self::Quit => Action::Quit,
            Self::Focus(name) => Action::Focus(name),
            Self::SetTheme(id) => Action::SetTheme(id),
            Self::SetLocale(code) => Action::SetLocale(code),
            Self::SetRegion(region) => Action::SetRegion(region),
            Self::SetIconMode(mode) => Action::SetIconMode(mode),
            Self::SetReducedMotion(reduced) => Action::SetReducedMotion(reduced),
            Self::SetPillar(style) => Action::SetPillar(style),
            Self::SetSlide(slide) => Action::SetSlide(slide),
            Self::Copy(text) => Action::Copy(text),
            Self::Confirm(confirm) => Action::Confirm(confirm.map(|message| map(message))),
            Self::ReadClipboard(message) => {
                let map = Arc::clone(map);
                Action::ReadClipboard(Box::new(move |text| map(message(text))))
            }
            Self::Perform(work) => {
                let map = Arc::clone(map);
                Action::Perform(Box::new(move || map(work())))
            }
            Self::Toast(toast) => Action::Toast(toast.map(Arc::clone(map))),
            Self::DismissToast(key) => Action::DismissToast(key),
            Self::ToastCorner(corner) => Action::ToastCorner(corner),
            Self::Task(task) => Action::Task(task.map(Arc::clone(map))),
            Self::CancelTask(id) => Action::CancelTask(id),
            Self::Handoff(handoff) => {
                let map = Arc::clone(map);
                Action::Handoff(handoff.map(move |message| map(message)))
            }
            Self::HandoffDetached(handoff) => Action::HandoffDetached(handoff.map(Arc::clone(map))),
            Self::Open(open) => {
                let map = Arc::clone(map);
                Action::Open(open.map(move |message| map(message)))
            }
            #[cfg(feature = "updates")]
            Self::CheckForUpdate(check) => {
                let map = Arc::clone(map);
                Action::CheckForUpdate(check.map(move |message| map(message)))
            }
        }
    }
}

/// Work for the runtime, returned from [`App::update`](crate::runtime::App::update).
pub struct Command<Msg> {
    pub(crate) actions: Vec<Action<Msg>>,
}

impl<Msg: Send + 'static> Command<Msg> {
    /// Nothing to do.
    #[must_use]
    pub fn none() -> Self {
        Self { actions: Vec::new() }
    }

    /// Several commands, run in order.
    #[must_use]
    pub fn batch(commands: impl IntoIterator<Item = Self>) -> Self {
        Self { actions: commands.into_iter().flat_map(|command| command.actions).collect() }
    }

    /// Leaves the application after this update.
    #[must_use]
    pub fn quit() -> Self {
        Self::single(Action::Quit)
    }

    /// Moves keyboard focus to the widget named `name` with [`NodeMut::id`](crate::widget::NodeMut::id).
    /// A name on a widget that takes no focus itself, such as a column, focuses the first widget
    /// inside it that does. When no such widget is on screen yet, focus moves to it after the next frame if it
    /// appears there, so an update can show a widget and focus it at once.
    #[must_use]
    pub fn focus(name: impl Into<String>) -> Self {
        Self::single(Action::Focus(name.into()))
    }

    /// Switches to theme `id`. An unusable theme falls back to the default and is reported in
    /// the environment's diagnostics.
    #[must_use]
    pub fn set_theme(id: impl Into<String>) -> Self {
        Self::single(Action::SetTheme(id.into()))
    }

    /// Switches the language to the locale that serves `code`: a locale code such as `tr`, or a
    /// language tag such as `en-GB`, which also sets the region; see
    /// [`I18n::select`](crate::i18n::I18n::select). An unknown language changes nothing and is
    /// reported in the environment's diagnostics.
    #[must_use]
    pub fn set_locale(code: impl Into<String>) -> Self {
        Self::single(Action::SetLocale(code.into()))
    }

    /// Sets the region whose conventions apply, such as `GB`, or with `None` leaves them to the
    /// language again; see [`I18n::set_region`](crate::i18n::I18n::set_region). A code that is not
    /// a region changes nothing and is reported in the environment's diagnostics.
    #[must_use]
    pub fn set_region(region: Option<&str>) -> Self {
        Self::single(Action::SetRegion(region.map(str::to_owned)))
    }

    /// Switches between Nerd Font, Unicode, ASCII or detected glyphs.
    #[must_use]
    pub fn set_icon_mode(mode: IconMode) -> Self {
        Self::single(Action::SetIconMode(mode))
    }

    /// Turns reduced motion on or off: layers appear at once and nothing breathes or spins.
    /// Has no effect while the `QUVYTA_REDUCED_MOTION` environment variable decides.
    #[must_use]
    pub fn set_reduced_motion(reduced: bool) -> Self {
        Self::single(Action::SetReducedMotion(reduced))
    }

    /// Draws every pillar in `style` over the theme's choice.
    #[must_use]
    pub fn set_pillar(style: crate::icons::PillarStyle) -> Self {
        Self::single(Action::SetPillar(style))
    }

    /// Turns the one-cell slide of hovered and selected entries in list structures (lists, menus,
    /// trees, tables, tab rails, setting rows, dropdown options and tabs) on or off over the
    /// theme's `motion.slide`. Buttons and other controls never slide.
    #[must_use]
    pub fn set_slide(slide: bool) -> Self {
        Self::single(Action::SetSlide(slide))
    }

    /// Copies `text` to the system clipboard (OSC 52, which also works over SSH) and to the
    /// application's in-process clipboard, which keeps pasting inside the application working on
    /// terminals without OSC 52.
    #[must_use]
    pub fn copy(text: impl Into<String>) -> Self {
        Self::single(Action::Copy(text.into()))
    }

    /// Reads the clipboard and delivers its text, or `None` when there is none. Like the `paste`
    /// key and Paste menu entries it tries, in order: the system clipboard through its tool
    /// (`wl-paste`, `xclip` or `xsel`, `pbpaste`; run without a shell, briefly, off the drawing
    /// thread), the terminal's clipboard through an OSC 52 query (many terminals do not answer,
    /// so the wait is short), and the text copied last inside this application (by a widget, a
    /// selection or [`Command::copy`]). The message arrives in a later update once the text is
    /// known; drawing never waits for it. Text pasted with the terminal's own paste arrives as
    /// [`Event::Paste`](crate::event::Event::Paste) instead.
    #[must_use]
    pub fn read_clipboard(message: impl FnOnce(Option<String>) -> Msg + 'static) -> Self {
        Self::single(Action::ReadClipboard(Box::new(message)))
    }

    /// Runs `work` on a background thread and delivers its message when done. Drawing never
    /// waits for it.
    #[must_use]
    pub fn perform(work: impl FnOnce() -> Msg + Send + 'static) -> Self {
        Self::single(Action::Perform(Box::new(work)))
    }

    /// Asks the user a question in a dialog the runtime shows over the application, and
    /// delivers the message of their answer: the confirm message, or the cancel message (if
    /// any) for Cancel, Esc and the close mark. Cancel, the safe answer, has focus when the dialog opens.
    /// Several requests stack; the newest is answered first.
    ///
    /// ```
    /// use qframe::prelude::*;
    ///
    /// enum Msg {
    ///     AskRemove,
    ///     Remove,
    /// }
    ///
    /// fn update(msg: Msg) -> Command<Msg> {
    ///     match msg {
    ///         Msg::AskRemove => Command::confirm(
    ///             Confirm::new("Remove container?", Msg::Remove).message("Its volumes are deleted too.").danger(),
    ///         ),
    ///         Msg::Remove => Command::none(),
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn confirm(confirm: Confirm<Msg>) -> Self {
        Self::single(Action::Confirm(confirm))
    }

    /// Shows `toast` in the toast corner, above everything else. It slides in, stays for its
    /// duration (paused while the pointer is on it) and slides out; a click on its close mark dismisses it.
    ///
    /// It never covers an open dialog or other modal layer: it keeps to the rows between its
    /// corner and the dialog, and when there is no room there it waits, its time stopped,
    /// until there is, such as when the dialog closes.
    #[must_use]
    pub fn toast(toast: Toast<Msg>) -> Self {
        Self::single(Action::Toast(toast))
    }

    /// Removes the toast shown with [`Toast::key`] `key`.
    #[must_use]
    pub fn dismiss_toast(key: impl Into<String>) -> Self {
        Self::single(Action::DismissToast(key.into()))
    }

    /// Stacks toasts in `corner` from now on; bottom right by default.
    #[must_use]
    pub fn toast_corner(corner: Corner) -> Self {
        Self::single(Action::ToastCorner(corner))
    }

    /// Starts `task` on a background thread. Its `Started` event is applied before this update
    /// returns; progress, messages and the outcome arrive as the work goes on.
    #[must_use]
    pub fn task(task: Task<Msg>) -> Self {
        Self::single(Action::Task(task))
    }

    /// Asks task `id` to stop: its sleeps wake at once, [`TaskCx::is_cancelled`](crate::runtime::TaskCx::is_cancelled)
    /// turns true and it ends as [`TaskOutcome::Cancelled`](crate::runtime::TaskOutcome::Cancelled).
    /// Asking a finished task does nothing.
    #[must_use]
    pub fn cancel_task(id: TaskId) -> Self {
        Self::single(Action::CancelTask(id))
    }

    /// Hands the terminal to another program and waits for it: the application leaves raw mode
    /// and the alternate screen, the program runs attached to the real terminal, and afterwards
    /// the screen is taken back and drawn again in full. Use it for programs that talk to the
    /// user themselves, such as `sudo` asking for a password, an editor or a pager. The message
    /// of [`Handoff::new`] arrives once the application has the screen back. Several handoffs run
    /// one after another.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::runtime::{Handoff, HandoffOutcome};
    ///
    /// enum Msg {
    ///     Edit,
    ///     Edited(HandoffOutcome),
    /// }
    ///
    /// fn update(msg: Msg) -> Command<Msg> {
    ///     match msg {
    ///         Msg::Edit => Command::handoff(Handoff::new("vi", Msg::Edited).arg("notes.md")),
    ///         Msg::Edited(_) => Command::none(),
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn handoff(handoff: Handoff<Msg>) -> Self {
        Self::single(Action::Handoff(handoff))
    }

    /// Hands the terminal to a program until it writes its first line, then takes the screen
    /// back and leaves the program running in the background, its standard input and output
    /// piped to the application. Use it for a program that asks the user something on the
    /// terminal and then serves the application, such as a privileged helper started through
    /// `pkexec`. See [`DetachedHandoff`] for the whole course; it queues with
    /// [`Command::handoff`], one after another.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::runtime::{ChildLine, DetachedHandoff, DetachedOutcome};
    ///
    /// enum Msg {
    ///     Start,
    ///     Started(DetachedOutcome),
    ///     Said(ChildLine),
    /// }
    ///
    /// fn update(msg: Msg) -> Command<Msg> {
    ///     match msg {
    ///         Msg::Start => Command::handoff_detached(
    ///             DetachedHandoff::new("sh", Msg::Started).args(["-c", "echo ready; cat"]).on_line(Msg::Said),
    ///         ),
    ///         Msg::Started(_) | Msg::Said(_) => Command::none(),
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn handoff_detached(handoff: DetachedHandoff<Msg>) -> Self {
        Self::single(Action::HandoffDetached(handoff))
    }

    /// Opens an address, a file or a folder on the person's own desktop, without leaving the
    /// screen: no step aside, no blink, nothing drawn again.
    ///
    /// This is the short way of saying `Command::open_with(Open::new(target))`, for an
    /// application that has nothing to say about the opening. [`Command::open_with`] takes the
    /// same opening with a message, a program of its own, arguments, a directory or environment.
    ///
    /// ```
    /// use qframe::prelude::*;
    ///
    /// enum Msg {
    ///     ReadTheGuide,
    /// }
    ///
    /// fn update(msg: Msg) -> Command<Msg> {
    ///     match msg {
    ///         Msg::ReadTheGuide => Command::open("https://quvyta.com/guide"),
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn open(target: impl Into<std::ffi::OsString>) -> Self {
        Self::single(Action::Open(Open::new(target)))
    }

    /// Carries out `open`: a program started quietly beside the application, with the screen left
    /// exactly as it is. See [`Open`] for the whole of it.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::runtime::{Open, OpenOutcome};
    ///
    /// enum Msg {
    ///     Opened(OpenOutcome),
    /// }
    ///
    /// fn update(_msg: Msg) -> Command<Msg> {
    ///     Command::open_with(Open::new("/home/me/notes.pdf").answer(Msg::Opened))
    /// }
    /// ```
    #[must_use]
    pub fn open_with(open: Open<Msg>) -> Self {
        Self::single(Action::Open(open))
    }

    /// Asks the package registry, on a thread of its own, whether a newer version of the
    /// application is out, and sends the check's message only when one is. At most once a day,
    /// never while the ecosystem's update notice is off, and silent without a network; see
    /// [`UpdateCheck`](super::UpdateCheck). Needs the `updates` feature.
    #[cfg(feature = "updates")]
    #[must_use]
    pub fn check_for_update(check: super::update_check::UpdateCheck<Msg>) -> Self {
        Self::single(Action::CheckForUpdate(check))
    }

    /// The same work delivering `map(message)` wherever it would deliver `message`, so a screen
    /// with messages of its own can return its commands from the application's `update`:
    ///
    /// ```
    /// use qframe::prelude::*;
    ///
    /// mod search {
    ///     use qframe::prelude::*;
    ///
    ///     pub enum Msg {
    ///         Run,
    ///         Found(usize),
    ///     }
    ///
    ///     pub fn update(msg: Msg) -> Command<Msg> {
    ///         match msg {
    ///             Msg::Run => Command::perform(|| Msg::Found(3)),
    ///             Msg::Found(_) => Command::none(),
    ///         }
    ///     }
    /// }
    ///
    /// enum Msg {
    ///     Search(search::Msg),
    /// }
    ///
    /// fn update(msg: Msg) -> Command<Msg> {
    ///     match msg {
    ///         Msg::Search(msg) => search::update(msg).map(Msg::Search),
    ///     }
    /// }
    /// ```
    ///
    /// Every kind of work is carried over: a message the work of [`Command::perform`] or a
    /// [`Task`] produces later on its own thread (its result, what it sends while it runs, its
    /// events), the answers of [`Command::confirm`], the action and presses of a toast, the
    /// clipboard text of [`Command::read_clipboard`], the message after a [`Command::handoff`],
    /// and the messages of a [`Command::handoff_detached`] and of the child it leaves running.
    /// Work without messages (focus, theme, copy, cancelling a task) is unchanged.
    ///
    /// `map` runs on the threads of that background work, and one command can hold several of
    /// them, so it is shared rather than copied: it must be `Send` and `Sync`, and it is never
    /// required to be `Clone`. An enum variant such as `Msg::Search` or a closure over
    /// `Send + Sync` values qualifies.
    #[must_use]
    pub fn map<B: Send + 'static>(self, map: impl Fn(Msg) -> B + Send + Sync + 'static) -> Command<B> {
        let map: MapFn<Msg, B> = Arc::new(map);
        Command { actions: self.actions.into_iter().map(|action| action.map(&map)).collect() }
    }

    fn single(action: Action<Msg>) -> Self {
        Self { actions: vec![action] }
    }
}
