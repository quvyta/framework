//! Playing an animation: which frame is shown at a moment, and in which colour.

use std::time::Duration;

use super::{CellAnimation, ColorMode, Playback};
use crate::color::Rgb;
use crate::theme::{Paint, Theme};

/// What an animation shows at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellFrame {
    /// The frame shown, counted from 0.
    pub index: usize,
    /// Its colour, or `None` when the frame has no colour and the widget gave none.
    pub color: Option<Rgb>,
    /// Whether a [`Playback::Once`] animation has played to its end.
    pub finished: bool,
    /// How long until the frame changes; `None` when it stands still.
    pub next: Option<Duration>,
    /// Whether the colour moves between frame changes (a pulse or a blend), so the cell needs
    /// smooth redraws.
    pub smooth: bool,
}

/// Where a playing animation is: the frame shown, the frame after it and how far into it.
struct Position {
    index: usize,
    following: usize,
    /// How far into the frame, 0 to 1.
    progress: f32,
    remaining: Duration,
    finished: bool,
}

impl CellAnimation {
    /// The frame and colour at `now`.
    ///
    /// `since` is when the animation started (a time on the same clock as `now`, such as
    /// [`PaintCx::now`](crate::widget::PaintCx::now)); `None` shows the rest frame standing still,
    /// which is what reduced motion asks for. Looping spinners pass `Duration::ZERO`, so every
    /// spinner on screen turns in step. Frames without a colour, and `$fg` in colour
    /// expressions, take `fg`; a colour naming a token the theme lacks is treated the same way.
    /// Pulses breathe over the theme's `motion.pulse-period` on the `now` clock.
    #[must_use]
    pub fn sample(&self, theme: &Theme, fg: Option<Rgb>, now: Duration, since: Option<Duration>) -> CellFrame {
        if self.frames.is_empty() {
            return CellFrame { index: 0, color: fg, finished: true, next: None, smooth: false };
        }
        let Some(since) = since else {
            let index = self.rest_index();
            let color = self.paint(index, theme, fg).map(|paint| match paint {
                // Standing still, a pulse shows its second colour: the one it breathes towards.
                Paint::Pulse(_, strong) => strong,
                Paint::Solid(color) => color,
            });
            return CellFrame { index, color, finished: true, next: None, smooth: false };
        };
        let position = self.position(theme, now.saturating_sub(since));
        let phase = pulse_phase(theme, now);
        let here = self.paint(position.index, theme, fg);
        let mut smooth = here.is_some_and(Paint::is_animated);
        let mut color = here.map(|paint| paint.at(phase));
        if self.colors == ColorMode::Blend && position.following != position.index {
            let there = self.paint(position.following, theme, fg);
            smooth |= there.is_some_and(Paint::is_animated);
            if let (Some(from), Some(to)) = (color, there.map(|paint| paint.at(phase))) {
                smooth |= from != to;
                color = Some(from.mix(to, position.progress));
            }
        }
        let next = (!position.finished).then_some(position.remaining);
        CellFrame { index: position.index, color, finished: position.finished, next, smooth }
    }

    /// The paint of frame `index`: its own colour, else `fg`.
    fn paint(&self, index: usize, theme: &Theme, fg: Option<Rgb>) -> Option<Paint> {
        let own = self.frames.get(index).and_then(|frame| frame.color.as_ref());
        match own {
            Some(color) => {
                let widget = fg.or_else(|| theme.color("text")).unwrap_or(Rgb::new(0, 0, 0));
                color.resolve(theme, widget).ok().or(fg.map(Paint::Solid))
            }
            None => fg.map(Paint::Solid),
        }
    }

    /// How long frame `index` is shown, in whole milliseconds and at least one, as the stepped
    /// motion of the rest of the framework counts it.
    fn frame_millis(&self, index: usize, theme: &Theme) -> u128 {
        let time = self.frames.get(index).and_then(|frame| frame.duration).unwrap_or(self.frame_time);
        time.resolve(&theme.motion()).as_millis().max(1)
    }

    /// The order frames are shown in one pass.
    fn sequence(&self) -> impl Iterator<Item = usize> + Clone + '_ {
        let count = self.frames.len();
        let back = if self.playback == Playback::Bounce { count.saturating_sub(1) } else { 0 };
        (0..count).chain((1..back).rev())
    }

    fn position(&self, theme: &Theme, elapsed: Duration) -> Position {
        let sequence: Vec<(usize, u128)> =
            self.sequence().map(|index| (index, self.frame_millis(index, theme))).collect();
        let total: u128 = sequence.iter().map(|(_, millis)| millis).sum();
        let elapsed = elapsed.as_millis();
        let last = self.frames.len() - 1;
        if self.playback == Playback::Once && elapsed >= total {
            return Position { index: last, following: last, progress: 0.0, remaining: Duration::ZERO, finished: true };
        }
        let mut into = elapsed % total.max(1);
        for (step, (index, millis)) in sequence.iter().enumerate() {
            if into < *millis {
                let following = match sequence.get(step + 1) {
                    Some((next, _)) => *next,
                    None if self.playback == Playback::Once => *index,
                    None => sequence[0].0,
                };
                let remaining = Duration::from_millis(u64::try_from(millis - into).unwrap_or(u64::MAX));
                let progress = into as f32 / *millis as f32;
                return Position { index: *index, following, progress, remaining, finished: false };
            }
            into -= millis;
        }
        // `into` is below the total, so the loop always returns; stay on the last frame otherwise.
        Position { index: last, following: last, progress: 0.0, remaining: Duration::ZERO, finished: true }
    }
}

/// Where the theme's pulse is at `now`, `0.0..1.0`, counted in whole milliseconds like
/// [`PaintCx::cycle`](crate::widget::PaintCx::cycle).
fn pulse_phase(theme: &Theme, now: Duration) -> f32 {
    let period = theme.motion().pulse_period.as_millis().max(1);
    (now.as_millis() % period) as f32 / period as f32
}
