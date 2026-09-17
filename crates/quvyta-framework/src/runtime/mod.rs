//! Running applications: the [`App`] trait, [`Command`]s, the terminal [`Runtime`] and the
//! test [`Harness`].

mod app;
mod clipboard;
mod command;
mod confirm;
mod debug;
mod engine;
mod harness;
#[cfg(test)]
mod layer_rules;
mod selection;
mod selection_menu;
#[cfg(test)]
mod selection_rules;
mod task;
mod terminal;
mod terminal_clipboard;

pub use app::App;
pub use clipboard::ClipboardEvent;
pub use command::Command;
pub use confirm::Confirm;
pub use harness::{Harness, html_page};
pub use task::{Task, TaskCx, TaskEntry, TaskEvent, TaskId, TaskOutcome, Tasks};
pub use terminal::Runtime;

pub(crate) use selection::CopyKind;
