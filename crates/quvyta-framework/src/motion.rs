//! Motion: easing, values that move over time, and cell-stepped progress.
//!
//! Terminal motion happens in whole cells, so movement is expressed as progress from 0 to 1
//! that widgets turn into cell positions with [`steps`], while colours blend continuously with
//! the same progress. Durations come from the theme's `[motion]` table; when motion is reduced
//! every animation lands on its end state at once.

use std::time::Duration;

/// How progress accelerates over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// Starts slowly.
    EaseIn,
    /// Ends slowly; the natural choice for things arriving.
    #[default]
    EaseOut,
    /// Starts and ends slowly.
    EaseInOut,
}

impl Easing {
    /// Every easing.
    pub const ALL: [Self; 4] = [Self::Linear, Self::EaseIn, Self::EaseOut, Self::EaseInOut];

    /// A short name, e.g. for settings screens and locale keys.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::EaseIn => "ease-in",
            Self::EaseOut => "ease-out",
            Self::EaseInOut => "ease-in-out",
        }
    }

    /// Eased progress for linear progress `t` in `0..=1`.
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
        }
    }
}

/// A number moving from one value to another over a duration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    from: f32,
    to: f32,
    start: Duration,
    duration: Duration,
    easing: Easing,
}

impl Tween {
    /// A value resting at `value`.
    #[must_use]
    pub fn settled(value: f32) -> Self {
        Self { from: value, to: value, start: Duration::ZERO, duration: Duration::ZERO, easing: Easing::Linear }
    }

    /// The value at time `now`.
    #[must_use]
    pub fn value(&self, now: Duration) -> f32 {
        if self.duration.is_zero() || now >= self.end() {
            return self.to;
        }
        let elapsed = now.saturating_sub(self.start).as_secs_f32() / self.duration.as_secs_f32();
        self.from + (self.to - self.from) * self.easing.apply(elapsed)
    }

    /// The value the tween is heading to.
    #[must_use]
    pub fn target(&self) -> f32 {
        self.to
    }

    /// Whether the value is still moving at `now`.
    #[must_use]
    pub fn is_running(&self, now: Duration) -> bool {
        now < self.end() && self.from != self.to
    }

    /// When the movement ends; saturates rather than overflowing the clock.
    fn end(&self) -> Duration {
        self.start.saturating_add(self.duration)
    }

    /// Moves towards `to` from wherever the value is at `now`.
    pub fn retarget(&mut self, to: f32, now: Duration, duration: Duration, easing: Easing) {
        let current = self.value(now);
        *self = Self { from: current, to, start: now, duration, easing };
    }
}

/// Turns progress in `0..=1` into one of `count + 1` cell positions, `0..=count`.
#[must_use]
pub fn steps(progress: f32, count: u16) -> u16 {
    let position = (progress.clamp(0.0, 1.0) * f32::from(count)).round();
    // `position` lies in 0..=count, so it fits in u16.
    position as u16
}

/// Animated values of one widget, by name.
#[derive(Debug, Default)]
pub(crate) struct Tweens {
    values: Vec<(&'static str, Tween)>,
}

impl Tweens {
    /// The current value of `name`, retargeted to `target` when it changed. A value seen for
    /// the first time starts at its target, so nothing animates on first paint.
    pub(crate) fn drive(
        &mut self,
        name: &'static str,
        target: f32,
        now: Duration,
        duration: Duration,
        easing: Easing,
    ) -> Tween {
        match self.values.iter_mut().find(|(n, _)| *n == name) {
            Some((_, tween)) => {
                if (tween.target() - target).abs() > f32::EPSILON {
                    tween.retarget(target, now, duration, easing);
                }
                *tween
            }
            None => {
                let tween = Tween::settled(target);
                self.values.push((name, tween));
                tween
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_start_at_zero_and_end_at_one() {
        for easing in [Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut] {
            assert!(easing.apply(0.0).abs() < 1e-6);
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-6);
        }
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
    }

    #[test]
    fn tween_moves_and_retargets_from_current_value() {
        let mut tween = Tween::settled(0.0);
        tween.retarget(10.0, Duration::ZERO, Duration::from_millis(100), Easing::Linear);
        assert!((tween.value(Duration::from_millis(50)) - 5.0).abs() < 1e-4);
        assert!(tween.is_running(Duration::from_millis(50)));
        tween.retarget(0.0, Duration::from_millis(50), Duration::from_millis(100), Easing::Linear);
        assert!((tween.value(Duration::from_millis(50)) - 5.0).abs() < 1e-4);
        assert_eq!(tween.value(Duration::from_millis(200)), 0.0);
        assert!(!tween.is_running(Duration::from_millis(200)));
    }

    #[test]
    fn endless_tween_does_not_overflow_the_clock() {
        let mut tween = Tween::settled(0.0);
        tween.retarget(1.0, Duration::from_secs(5), Duration::MAX, Easing::Linear);
        assert!(tween.is_running(Duration::from_secs(6)));
        assert!(tween.value(Duration::from_secs(6)) < 1e-6);
    }

    #[test]
    fn steps_round_to_cells() {
        assert_eq!(steps(0.0, 3), 0);
        assert_eq!(steps(0.49, 3), 1);
        assert_eq!(steps(1.0, 3), 3);
        assert_eq!(steps(7.0, 3), 3);
    }

    #[test]
    fn first_sight_does_not_animate() {
        let mut tweens = Tweens::default();
        let first = tweens.drive("x", 1.0, Duration::ZERO, Duration::from_millis(100), Easing::Linear);
        assert!(!first.is_running(Duration::ZERO));
        let moving = tweens.drive("x", 0.0, Duration::from_millis(10), Duration::from_millis(100), Easing::Linear);
        assert!(moving.is_running(Duration::from_millis(20)));
    }
}
