//! Work an application asks the runtime to do after an update.

use super::confirm::Confirm;
use super::task::{Task, TaskId};
use crate::icons::IconMode;
use crate::widgets::{Corner, Toast};

pub(crate) enum Action<Msg> {
    Quit,
    Focus(String),
    SetTheme(String),
    SetLocale(String),
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
    /// When no such widget is on screen yet, focus moves to it after the next frame if it
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

    /// Switches the language to locale `code`.
    #[must_use]
    pub fn set_locale(code: impl Into<String>) -> Self {
        Self::single(Action::SetLocale(code.into()))
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

    fn single(action: Action<Msg>) -> Self {
        Self { actions: vec![action] }
    }
}
