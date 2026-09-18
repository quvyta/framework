//! What a view reads and asks about input idleness while it is built; the runtime's
//! `runtime::engine::idle` keeps the time and wakes the application.

use std::cell::{Cell, RefCell};
use std::time::Duration;

/// Idleness as one frame's view sees it, shared by every builder of that view.
pub(crate) struct IdleScope<Msg> {
    /// How long no input has arrived, at the time of the frame.
    pub(crate) silent: Duration,
    /// Whether the view read `silent`, so the runtime draws again when it changes on screen.
    pub(crate) read: Cell<bool>,
    /// The watches the view declared, in the order it declared them.
    pub(crate) watches: RefCell<Vec<IdleWatch<Msg>>>,
}

impl<Msg> IdleScope<Msg> {
    pub(crate) fn new(silent: Duration) -> Self {
        Self { silent, read: Cell::new(false), watches: RefCell::new(Vec::new()) }
    }
}

/// One `View::on_idle`: how long the silence must last and the message that reports it.
pub(crate) struct IdleWatch<Msg> {
    pub(crate) after: Duration,
    pub(crate) message: Box<dyn Fn(bool) -> Msg>,
}
