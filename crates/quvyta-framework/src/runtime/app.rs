//! The application trait.

use super::clipboard::ClipboardEvent;
use super::command::Command;
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

    /// Hears about the clipboard: text a widget or a mouse selection copied, and pasted text
    /// that no focused widget took. Copies the application asked for with
    /// [`Command::copy`] are not reported, so answering a copy with a copy cannot loop.
    fn clipboard(&self, _event: &ClipboardEvent) -> Option<Self::Msg> {
        None
    }
}
