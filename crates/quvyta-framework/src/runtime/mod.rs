//! Running applications: the [`App`] trait, [`Command`]s, the terminal [`Runtime`] and the
//! test [`Harness`].

mod app;
mod clipboard;
mod command;
mod confirm;
mod debug;
mod detached;
#[cfg(test)]
mod detached_rules;
mod engine;
#[cfg(test)]
mod focus_action_rules;
#[cfg(unix)]
mod foreground;
mod handoff;
mod harness;
#[cfg(test)]
mod layer_rules;
#[cfg(test)]
mod lifecycle_rules;
mod live_child;
#[cfg(test)]
mod map_rules;
mod process;
mod selection;
mod selection_menu;
#[cfg(test)]
mod selection_rules;
#[cfg(all(test, target_os = "linux"))]
mod signal_rules;
#[cfg(unix)]
mod signals;
#[cfg(not(unix))]
#[path = "signals_other.rs"]
mod signals;
mod task;
mod terminal;
mod terminal_clipboard;
mod termination;
#[cfg(test)]
mod termination_rules;
#[cfg(test)]
mod toast_rules;

pub use app::App;
pub use clipboard::ClipboardEvent;
pub use command::Command;
pub use confirm::Confirm;
pub use detached::{DetachedHandoff, DetachedOutcome};
pub use handoff::{Handoff, HandoffOutcome, HandoffRequest};
pub use harness::{Harness, html_page};
pub use live_child::{ChildLine, LiveChild, TestChild};
pub use process::{Line, Process, ProcessOutcome};
pub use task::{Task, TaskCx, TaskEntry, TaskEvent, TaskId, TaskOutcome, Tasks};
pub use terminal::Runtime;
pub use termination::Termination;

pub(crate) use selection::CopyKind;
