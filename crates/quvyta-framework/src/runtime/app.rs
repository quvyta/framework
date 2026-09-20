//! The application trait.

use super::clipboard::ClipboardEvent;
use super::command::Command;
use super::frame_limit::FrameLimit;
use super::termination::Termination;
use crate::geometry::Size;
use crate::widget::View;

/// An application built with quvyta-framework: data, a function that draws it and a function that
/// changes it.
///
/// An application implements this trait and runs in a [`Runtime`](super::Runtime), or in a
/// [`Harness`](super::Harness) for tests.
///
/// ```
/// use qframe::prelude::*;
///
/// struct Counter {
///     value: i32,
/// }
///
/// #[derive(Clone)]
/// enum Msg {
///     Increment,
/// }
///
/// impl App for Counter {
///     type Msg = Msg;
///
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::Increment => self.value += 1,
///         }
///         Command::none()
///     }
///
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         ui.column(|ui| {
///             ui.add(Text::new(format!("Value: {}", self.value)));
///             ui.add(Button::new("Increment").on_press(Msg::Increment));
///         });
///     }
/// }
///
/// let mut app = Harness::new(Counter { value: 0 }, 30, 4);
/// app.press("tab").press("enter");
/// assert!(app.screen().contains("Value: 1"));
/// ```
///
/// # Lifecycle
///
/// Besides `update` and `view`, four optional hooks follow the application through its life.
/// Each has a default, so an application implements only the ones it needs:
///
/// 1. [`App::resized`] hears the size of the screen: first when the application starts, then
///    whenever it changes.
/// 2. [`App::init`] runs once, right after that first size, before the first frame is built.
/// 3. [`App::before_quit`] is asked whenever the runtime is about to quit on the user's behalf.
/// 4. [`App::terminating`] hears that the system is ending the application: a `SIGTERM` or a
///    `SIGHUP`, when the SSH connection or the terminal went away. It is the one chance to save.
///
/// The hooks that only report something ([`App::resized`], [`App::before_quit`],
/// [`App::terminating`], like
/// [`App::action`] and [`App::clipboard`]) read the state and answer with a message, which then
/// goes through `update` like every other; the one that starts work ([`App::init`]) returns a
/// [`Command`] like `update` does. The [`Harness`](super::Harness) runs every hook exactly
/// where the terminal runtime does, so a test sees what a user sees.
///
/// ```
/// use qframe::prelude::*;
///
/// #[derive(Default)]
/// struct Editor {
///     size: Size,
///     unsaved: bool,
///     asking: bool,
/// }
///
/// #[derive(Clone)]
/// enum Msg {
///     Resized(Size),
///     AskBeforeQuit,
///     Quit,
/// }
///
/// impl App for Editor {
///     type Msg = Msg;
///
///     fn init(&mut self) -> Command<Msg> {
///         // The first key already reaches the list.
///         Command::focus("files")
///     }
///
///     fn resized(&self, size: Size) -> Option<Msg> {
///         Some(Msg::Resized(size))
///     }
///
///     fn before_quit(&self) -> Option<Msg> {
///         self.unsaved.then_some(Msg::AskBeforeQuit)
///     }
///
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::Resized(size) => self.size = size,
///             Msg::AskBeforeQuit => self.asking = true,
///             // Decided: this quit does not ask again.
///             Msg::Quit => return Command::quit(),
///         }
///         Command::none()
///     }
///
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         ui.add(List::new(["notes.md", "todo.md"].map(ListItem::new))).id("files");
///     }
/// }
///
/// let mut app = Harness::new(Editor { unsaved: true, ..Editor::default() }, 40, 6);
/// assert!(app.is_focused("files"));
/// assert_eq!(app.app().size, Size::new(40, 6));
/// app.resize(30, 4);
/// assert_eq!(app.app().size, Size::new(30, 4));
/// app.press("ctrl+q");
/// assert!(app.app().asking && !app.quit_requested());
/// app.send(Msg::Quit);
/// assert!(app.quit_requested());
/// ```
pub trait App: 'static {
    /// Everything that can happen in the application.
    type Msg: Send + 'static;

    /// Applies a message and returns work for the runtime to do.
    fn update(&mut self, msg: Self::Msg) -> Command<Self::Msg>;

    /// Describes the screen. Runs after every change; must not do I/O.
    fn view(&self, ui: &mut View<'_, Self::Msg>);

    /// Turns an `[app]` keymap action into a message, e.g. `"save"` into `Msg::Save`.
    fn action(&self, _name: &str) -> Option<Self::Msg> {
        None
    }

    /// Runs once when the application starts and returns its first work: the focus the first key
    /// should reach, a tick to start, a dialog to open, a file to read.
    ///
    /// It runs at the start of the first frame, after the first [`App::resized`] message and
    /// before the view of that frame is built, so the first frame already shows what it
    /// changed. A [`Command::focus`] it returns names a widget that is not on screen yet; focus
    /// reaches it as soon as that first frame is painted, before the runtime reads any input,
    /// and the frame is drawn again at once with the widget focused. The first key the user
    /// presses therefore reaches the focused widget.
    ///
    /// The runtime calls it once per run, the [`Harness`](super::Harness) once when it is
    /// created. The default does nothing.
    fn init(&mut self) -> Command<Self::Msg> {
        Command::none()
    }

    /// Hears the size of the screen, in columns and rows: when the application starts, before
    /// [`App::init`], and afterwards whenever the terminal is resized. The message it returns
    /// goes through [`App::update`], which is where work that needs the size starts, such as
    /// [`Process::pty`](super::Process::pty) with the width and height the output will have.
    ///
    /// It is the size [`View::size`] reports: the terminal size of the frame about to be drawn.
    /// The message is applied before that frame's view is built, so `update` and `view` never
    /// disagree about it. A resize that ends at the size already reported is not reported
    /// again. [`Harness::new`](super::Harness::new) reports the size it is given, and
    /// [`Harness::resize`](super::Harness::resize) the new one.
    ///
    /// The default ignores the size.
    fn resized(&self, _size: Size) -> Option<Self::Msg> {
        None
    }

    /// Asked whenever the runtime is about to quit on the user's behalf: the global `quit`
    /// action of the keymap, however it was reached (its key, the command palette, a widget
    /// that runs the action). `None` lets the runtime quit. A message keeps the application running and is
    /// delivered through [`App::update`] instead, e.g. to ask "finish and quit, keep running or
    /// cancel" first.
    ///
    /// Once the application has decided, it quits with [`Command::quit`], which is its own
    /// decision and is never asked about. While an answer is pending the user may ask to quit
    /// again, and the hook is asked again; it sees its own state and can, say, keep the
    /// question it already shows.
    ///
    /// The default lets every quit through.
    fn before_quit(&self) -> Option<Self::Msg> {
        None
    }

    /// Hears that the system is ending the application, and why: see [`Termination`] for each
    /// cause and the signal behind it. `None` quits at once. A message keeps the application
    /// running and is delivered through [`App::update`] instead, which is where it saves and
    /// then returns [`Command::quit`].
    ///
    /// The run ends in bounded time whatever the answer: after [`Termination::grace`] the runtime
    /// quits without the application, and a second `SIGTERM` or `SIGINT` quits at once. After a
    /// [`Termination::Hangup`] the terminal is usually gone, so nothing is drawn any more and a
    /// dialog would wait for nobody; save without asking. Work of [`Command::perform`] and
    /// tasks still run and deliver their messages until the run ends.
    ///
    /// The runtime tells the application once per cause: a hangup that repeats is not told
    /// again, a hangup during a pending terminate is. [`Harness::terminate`](super::Harness::terminate)
    /// simulates each cause in tests.
    ///
    /// The default answers a [`Termination::Terminate`] like a quit the user asked for, with
    /// [`App::before_quit`], and quits at once on a [`Termination::Hangup`]. So an application
    /// that implements neither hook quits cleanly on every signal, and one that asks before
    /// quitting asks on a `SIGTERM` too.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::runtime::Termination;
    ///
    /// #[derive(Default)]
    /// struct Timer {
    ///     running: bool,
    ///     saved: bool,
    /// }
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     SaveAndQuit,
    /// }
    ///
    /// impl App for Timer {
    ///     type Msg = Msg;
    ///
    ///     fn terminating(&self, _cause: Termination) -> Option<Msg> {
    ///         // Whether a person or the system ends it, a running timer is saved first.
    ///         self.running.then_some(Msg::SaveAndQuit)
    ///     }
    ///
    ///     fn update(&mut self, msg: Msg) -> Command<Msg> {
    ///         match msg {
    ///             Msg::SaveAndQuit => {
    ///                 self.saved = true;
    ///                 Command::quit()
    ///             }
    ///         }
    ///     }
    ///
    ///     fn view(&self, ui: &mut View<'_, Msg>) {
    ///         ui.add(Text::new("25:00"));
    ///     }
    /// }
    ///
    /// let mut app = Harness::new(Timer { running: true, ..Timer::default() }, 20, 3);
    /// app.terminate(Termination::Hangup);
    /// assert!(app.app().saved && app.quit_requested());
    /// ```
    fn terminating(&self, cause: Termination) -> Option<Self::Msg> {
        match cause {
            Termination::Terminate => self.before_quit(),
            Termination::Hangup => None,
        }
    }

    /// How many frames a second the runtime draws at most; see [`FrameLimit`].
    ///
    /// Asked before every frame, so an application may answer from its own state, such as a
    /// setting the user changed. The default draws 60 frames a second locally and 20 over a
    /// remote connection. Only frames the application's own work causes are merged: a frame
    /// that answers input is never held back.
    ///
    /// ```
    /// # use qframe::prelude::*;
    /// # use qframe::runtime::FrameLimit;
    /// # struct Desktop;
    /// # impl App for Desktop {
    /// #     type Msg = ();
    /// #     fn update(&mut self, (): ()) -> Command<()> {
    /// #         Command::none()
    /// #     }
    /// // A desktop of terminal windows spends a slow link on ten frames a second.
    /// fn frame_limit(&self) -> FrameLimit {
    ///     FrameLimit::per_second(60).remote(10)
    /// }
    /// #     fn view(&self, ui: &mut View<'_, ()>) {
    /// #         ui.add(Text::new("windows"));
    /// #     }
    /// # }
    /// ```
    fn frame_limit(&self) -> FrameLimit {
        FrameLimit::default()
    }

    /// Hears about the clipboard: text a widget or a mouse selection copied, and pasted text
    /// that no focused widget took. Copies the application asked for with
    /// [`Command::copy`] are not reported, so answering a copy with a copy cannot loop.
    fn clipboard(&self, _event: &ClipboardEvent) -> Option<Self::Msg> {
        None
    }
}
