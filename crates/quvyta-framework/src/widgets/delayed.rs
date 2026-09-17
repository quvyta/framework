//! Deciding when a loading indicator shows, so quick work never blinks one.

use std::time::Duration;

/// How long work runs before its indicator shows.
///
/// Around 300 ms is where people start to notice waiting; anything quicker feels instant, and an
/// indicator shown for it is only a blink of motion that reads as a glitch. This is the delay
/// Material's content loading guidance, Apple's activity indicator guidance and React Suspense
/// transitions all settle on.
pub(crate) const SHOW_AFTER: Duration = Duration::from_millis(300);

/// How long an indicator stays once it shows, even when the work ends sooner.
///
/// An indicator that appears and vanishes a few frames later flickers just like one shown for
/// instant work; half a second is long enough to be read as a deliberate state.
pub(crate) const SHOW_AT_LEAST: Duration = Duration::from_millis(500);

/// Where an indicator is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    /// Nothing is loading and nothing shows.
    #[default]
    Idle,
    /// Work started at this time; the indicator waits for [`SHOW_AFTER`].
    Waiting(Duration),
    /// The indicator has shown since this time.
    Shown(Duration),
}

/// Whether a loading indicator shows right now, given when work started and ended.
///
/// Feed it whether the work is still busy on every frame with the frame's time
/// ([`PaintCx::now`](crate::widget::PaintCx::now)); it answers whether to draw the indicator and
/// when the answer changes next, so the widget can ask for a frame then. Work shorter than
/// [`SHOW_AFTER`] never shows it; once shown it stays [`SHOW_AT_LEAST`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DelayedIndicator {
    phase: Phase,
}

impl DelayedIndicator {
    /// Moves on to `now` with the work `busy` or not, and returns whether the indicator shows.
    pub(crate) fn update(&mut self, busy: bool, now: Duration) -> bool {
        self.phase = match (self.phase, busy) {
            (Phase::Idle, false) => Phase::Idle,
            (Phase::Idle, true) => Self::waiting(now, now),
            (Phase::Waiting(since), true) => Self::waiting(since, now),
            // Finished before anyone noticed: nothing was shown, nothing is.
            (Phase::Waiting(_), false) => Phase::Idle,
            (Phase::Shown(since), true) => Phase::Shown(since),
            (Phase::Shown(since), false) if now < since + SHOW_AT_LEAST => Phase::Shown(since),
            (Phase::Shown(_), false) => Phase::Idle,
        };
        self.is_shown()
    }

    fn waiting(since: Duration, now: Duration) -> Phase {
        // The minimum counts from the frame that first draws the indicator: a frame that came
        // late must not shorten how long it is on screen.
        if now >= since + SHOW_AFTER { Phase::Shown(now) } else { Phase::Waiting(since) }
    }

    /// Whether the indicator shows, as of the last [`DelayedIndicator::update`].
    pub(crate) fn is_shown(self) -> bool {
        matches!(self.phase, Phase::Shown(_))
    }

    /// Whether nothing is waiting or showing, so the state can be forgotten.
    pub(crate) fn is_idle(self) -> bool {
        self.phase == Phase::Idle
    }

    /// Hides the indicator at once, e.g. when the work failed and its error takes over.
    pub(crate) fn cancel(&mut self) {
        self.phase = Phase::Idle;
    }

    /// How long after `now` the answer can change while the work stays as it was last updated:
    /// when a waiting indicator is due, or when a shown one may hide.
    pub(crate) fn next_change(self, busy: bool, now: Duration) -> Option<Duration> {
        match self.phase {
            Phase::Waiting(since) => Some((since + SHOW_AFTER).saturating_sub(now)),
            Phase::Shown(since) if !busy => Some((since + SHOW_AT_LEAST).saturating_sub(now)),
            Phase::Idle | Phase::Shown(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn quick_work_never_shows() {
        let mut indicator = DelayedIndicator::default();
        assert!(!indicator.update(true, ms(1000)));
        assert_eq!(indicator.next_change(true, ms(1000)), Some(ms(300)));
        assert!(!indicator.update(true, ms(1299)));
        assert!(!indicator.update(false, ms(1299)));
        assert!(indicator.is_idle());
        assert_eq!(indicator.next_change(false, ms(1300)), None);
    }

    #[test]
    fn slow_work_shows_after_the_delay_and_stays_the_minimum() {
        let mut indicator = DelayedIndicator::default();
        indicator.update(true, ms(0));
        assert!(indicator.update(true, ms(300)));
        // The work ends at 350 ms; the indicator stays until 800 ms.
        assert!(indicator.update(false, ms(350)));
        assert_eq!(indicator.next_change(false, ms(350)), Some(ms(450)));
        assert!(indicator.update(false, ms(799)));
        assert!(!indicator.update(false, ms(800)));
        assert!(indicator.is_idle());
    }

    #[test]
    fn long_work_shows_for_as_long_as_it_runs() {
        let mut indicator = DelayedIndicator::default();
        indicator.update(true, ms(0));
        assert!(indicator.update(true, ms(300)));
        assert!(indicator.update(true, ms(2000)));
        assert_eq!(indicator.next_change(true, ms(2000)), None);
        assert!(!indicator.update(false, ms(2000)));
    }

    #[test]
    fn new_work_while_lingering_keeps_it_shown_and_cancel_hides_at_once() {
        let mut indicator = DelayedIndicator::default();
        indicator.update(true, ms(0));
        indicator.update(true, ms(300));
        indicator.update(false, ms(400));
        assert!(indicator.update(true, ms(900)));
        indicator.cancel();
        assert!(!indicator.is_shown());
        assert!(!indicator.update(false, ms(901)));
    }
}
