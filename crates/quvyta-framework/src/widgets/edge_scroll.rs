//! Scrolling a view by itself while a dragged item rests against one of its ends.
//!
//! A tab view that hides tabs cannot take a dragged tab further than what is on screen, so when
//! the pointer is held on an end (the scroll arrow of a strip, the first or last block of a rail,
//! or anywhere past the view) the view scrolls one item after a short wait and then keeps stepping
//! while the pointer stays there, the way file managers and editors scroll a list under a drag.
//! Moving the pointer off the end stops at once; the view's own end stops it too.
//!
//! The timing is the same for every view. Terminals send nothing while the pointer is held still,
//! so the steps come from [`EventCx::repeat_pointer`]: one wakeup at each step, never a busy loop.

use std::time::Duration;

use crate::widget::EventCx;

/// How long a dragged item rests on an end before the first step. Desktop toolkits wait about this
/// long before a drag scrolls or springs a folder open (400 ms is the common value): long enough
/// that passing over an end on the way to a drop never scrolls, short enough to feel like an
/// answer rather than a pause. A second or more, as first suggested, reads as nothing happening.
pub(crate) const DELAY: Duration = Duration::from_millis(400);

/// The time between steps while the item stays on the end: about seven items a second, slow
/// enough to read each item that comes into view and stop on it.
pub(crate) const REPEAT: Duration = Duration::from_millis(150);

/// How much sooner each step comes for every cell the pointer is past the view's edge, so pulling
/// further out scrolls faster.
const FASTER: Duration = Duration::from_millis(30);

/// The shortest time between steps, however far past the edge the pointer is.
pub(crate) const FASTEST: Duration = Duration::from_millis(60);

/// An end of a scrolling view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    /// The start: left of a strip, top of a rail.
    Back,
    /// The end: right of a strip, bottom of a rail.
    Forward,
}

/// Where a dragged item is held against an end of a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Zone {
    /// The end it is held against.
    pub(crate) edge: Edge,
    /// Cells the pointer is past the view's edge; zero within the view.
    pub(crate) beyond: u16,
}

/// The time between steps for a pointer `beyond` cells past the view's edge.
pub(crate) fn interval(beyond: u16) -> Duration {
    REPEAT.saturating_sub(FASTER * u32::from(beyond)).max(FASTEST)
}

/// The scrolling of one drag, kept by the view for as long as the drag lasts.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EdgeScroll {
    /// The end the pointer is on and when its next step is due.
    armed: Option<(Edge, Duration)>,
}

impl EdgeScroll {
    /// When the next step is due while the pointer rests on an end, so a view waiting on the
    /// pointer for another reason can wake it no later than the scrolling needs.
    pub(crate) fn due(&self) -> Option<Duration> {
        self.armed.map(|(_, due)| due)
    }

    /// Follows a drag event with the pointer in `zone` (none when it is off both ends). Arriving on
    /// an end starts the wait; a due step calls `step`, which scrolls one item towards the end and
    /// returns what it did, or none when the view is already at that end, which ends the stepping
    /// until the pointer leaves and comes back. Returns what the step returned.
    pub(crate) fn drive<Msg, R>(
        &mut self,
        cx: &mut EventCx<'_, Msg>,
        zone: Option<Zone>,
        step: impl FnOnce(&mut EventCx<'_, Msg>, Edge) -> Option<R>,
    ) -> Option<R> {
        let now = cx.now();
        let Some(zone) = zone else {
            if self.armed.take().is_some() {
                cx.stop_pointer_repeat();
            }
            return None;
        };
        match self.armed {
            Some((edge, due)) if edge == zone.edge => {
                if now < due {
                    return None;
                }
                let stepped = step(cx, edge);
                if stepped.is_some() {
                    let every = interval(zone.beyond);
                    self.armed = Some((edge, now + every));
                    cx.repeat_pointer(every);
                } else {
                    cx.stop_pointer_repeat();
                }
                stepped
            }
            _ => {
                self.armed = Some((zone.edge, now + DELAY));
                cx.repeat_pointer(DELAY);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_come_sooner_the_further_past_the_edge_down_to_a_floor() {
        assert_eq!(interval(0), REPEAT);
        assert_eq!(interval(1), Duration::from_millis(120));
        assert_eq!(interval(3), FASTEST);
        assert_eq!(interval(u16::MAX), FASTEST);
    }
}
