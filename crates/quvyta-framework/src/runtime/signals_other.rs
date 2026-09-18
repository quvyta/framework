//! The terminal loop's signals where there are none to catch: outside Unix the runtime installs
//! no handlers, a run hears no [`Termination`], and the loop waits on the keyboard alone.

use std::io;
use std::time::Duration;

use crossterm::event as ct;

use super::termination::Termination;

/// What the loop takes from the signals: never anything here.
#[derive(Debug, Default)]
pub(crate) struct Heard {
    pub(crate) causes: Vec<Termination>,
    pub(crate) resized: bool,
}

/// Why a wait ended.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Woken {
    pub(crate) keyboard: bool,
    pub(crate) hung_up: bool,
}

/// The terminal loop's side of the signals.
pub(crate) struct Signals;

impl Signals {
    pub(crate) fn catch() -> io::Result<Self> {
        Ok(Self)
    }

    pub(crate) fn take(&self) -> Heard {
        Heard::default()
    }

    pub(crate) fn pending(&self) -> bool {
        false
    }

    pub(crate) fn hung_up(&self) {}

    pub(crate) fn terminal_gone(&self) -> bool {
        false
    }

    pub(crate) fn hung_up_now(&self) -> bool {
        false
    }

    pub(crate) fn handoff(&self, _running: bool) {}

    pub(crate) fn wait(&self, timeout: Duration, keyboard: bool) -> io::Result<Woken> {
        if !keyboard {
            std::thread::sleep(timeout);
            return Ok(Woken::default());
        }
        Ok(Woken { keyboard: ct::poll(timeout)?, hung_up: false })
    }
}

/// Wakes the loop: there is no socket to wake it through here, so background work is applied
/// when the loop next wakes by itself, within half a second.
pub(crate) fn wake() {}
