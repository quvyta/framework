//! How many clicks open a row, and telling a double click from two clicks.
//!
//! A terminal reports presses and releases, never clicks, so a widget counts them itself: a
//! second press on the same row soon after the first is a double click. A press that turns into
//! a drag, or one with Ctrl or Shift held, starts nothing, so a drag followed by a click is never
//! read as a double click.

use std::time::Duration;

use crate::runtime::MULTI_PRESS;

/// How many clicks open a row of a [`Tree`](super::Tree), a [`Table`](super::Table) or a
/// [`CardGrid`](super::CardGrid).
///
/// A list of choices opens what is clicked at once; a file explorer selects with one click and
/// opens with two, so a click can start a drag or a selection without opening anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// One click selects a row and opens it.
    Single,
    /// One click only selects a row; a second press on the same row within
    /// [`Click::INTERVAL`] opens it. Enter still opens the selected row.
    Double,
}

impl Click {
    /// Two presses on the same row closer together than this are a double click: 400 ms, the
    /// interval desktops start with.
    pub const INTERVAL: Duration = MULTI_PRESS;
}

/// Whether a press at `now` follows one at `last` closely enough to make a double click.
pub(crate) fn is_double(last: Duration, now: Duration) -> bool {
    now.saturating_sub(last) < Click::INTERVAL
}

/// The last press on a row, kept in a widget's memory to tell a double click.
#[derive(Debug)]
pub(crate) struct LastPress<K> {
    last: Option<(K, Duration)>,
}

impl<K> Default for LastPress<K> {
    fn default() -> Self {
        Self { last: None }
    }
}

impl<K: PartialEq> LastPress<K> {
    /// Counts a press on `row` at `now`. True when it makes a double click with the press before
    /// it, which is then used up, so a third press starts over rather than opening again.
    pub(crate) fn press(&mut self, row: K, now: Duration) -> bool {
        let double = self.last.as_ref().is_some_and(|(last, at)| *last == row && is_double(*at, now));
        self.last = if double { None } else { Some((row, now)) };
        double
    }

    /// Forgets the last press: it became a drag or a modified click, which start no double click.
    pub(crate) fn forget(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn two_presses_on_one_row_within_the_interval_are_a_double_click_and_a_third_starts_over() {
        let mut presses = LastPress::default();
        assert!(!presses.press(3, ms(1_000)));
        assert!(presses.press(3, ms(1_399)), "399 ms later is a double click");
        assert!(!presses.press(3, ms(1_450)), "a third press starts a new count");
        assert!(presses.press(3, ms(1_500)));
    }

    #[test]
    fn a_slow_second_press_another_row_or_a_drag_between_are_not_a_double_click() {
        let mut presses = LastPress::default();
        assert!(!presses.press(3, ms(0)));
        assert!(!presses.press(3, ms(400)), "the interval itself is too slow");
        assert!(!presses.press(4, ms(500)), "another row");
        presses.forget();
        assert!(!presses.press(4, ms(600)), "the press before became a drag");
    }
}
