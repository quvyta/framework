//! The number model shared by sliders and number inputs: a range, a step, snapping and the
//! default way a value is written.

/// Most decimals a step may carry; finer steps are written with this many.
const MAX_DECIMALS: usize = 6;

/// Builds a label from a value.
pub(crate) type Formatter = Box<dyn Fn(f64) -> String>;

/// A closed range of numbers moved in steps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Steps {
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) step: f64,
}

impl Steps {
    /// A range from `min` to `max` in steps of `step`. A reversed range is put in order, a bound
    /// that is not a number leaves that side open, and a step that is not positive becomes 1.
    pub(crate) fn new(min: f64, max: f64, step: f64) -> Self {
        // `f64::clamp` panics on a NaN bound, and every value passes through it.
        let min = if min.is_nan() { f64::NEG_INFINITY } else { min };
        let max = if max.is_nan() { f64::INFINITY } else { max };
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        let step = if step > 0.0 && step.is_finite() { step } else { 1.0 };
        Self { min, max, step }
    }

    /// Decimals needed to write every step exactly.
    pub(crate) fn decimals(self) -> usize {
        let mut scaled = self.step;
        for decimals in 0..MAX_DECIMALS {
            if (scaled - scaled.round()).abs() < 1e-9 {
                return decimals;
            }
            scaled *= 10.0;
        }
        MAX_DECIMALS
    }

    /// `value` inside the range, without snapping.
    pub(crate) fn clamp(self, value: f64) -> f64 {
        self.round(value.clamp(self.min, self.max))
    }

    /// The step position nearest to `value`, inside the range.
    pub(crate) fn snap(self, value: f64) -> f64 {
        let count = ((value - self.min) / self.step).round();
        self.clamp(self.min + count * self.step)
    }

    /// `value` moved by `count` steps (negative moves down), snapped and inside the range.
    pub(crate) fn nudge(self, value: f64, count: f64) -> f64 {
        self.snap(value + count * self.step)
    }

    /// Steps in a large move: a tenth of the range, at least one step.
    pub(crate) fn large(self) -> f64 {
        ((self.max - self.min) / 10.0 / self.step).round().max(1.0)
    }

    /// Where `value` lies in the range, from 0 to 1.
    pub(crate) fn fraction(self, value: f64) -> f64 {
        if self.max <= self.min { 0.0 } else { ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0) }
    }

    /// The value at `fraction` of the range, snapped.
    pub(crate) fn at(self, fraction: f64) -> f64 {
        self.snap(self.min + fraction.clamp(0.0, 1.0) * (self.max - self.min))
    }

    /// `value` written with the decimals of the step.
    pub(crate) fn write(self, value: f64) -> String {
        let text = format!("{value:.*}", self.decimals());
        // `-0` reads as a mistake.
        if text.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') { text.replace('-', "") } else { text }
    }

    /// Removes floating point noise below the step's decimals.
    fn round(self, value: f64) -> f64 {
        let scale = 10f64.powi(i32::try_from(self.decimals()).unwrap_or(0));
        (value * scale).round() / scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_clamps_and_writes() {
        let steps = Steps::new(0.0, 1.0, 0.05);
        assert_eq!(steps.decimals(), 2);
        assert_eq!(steps.snap(0.337), 0.35);
        assert_eq!(steps.nudge(0.95, 3.0), 1.0);
        assert_eq!(steps.write(0.1 + 0.2), "0.30");
        let whole = Steps::new(100.0, -20.0, 0.0);
        assert_eq!((whole.min, whole.max, whole.step), (-20.0, 100.0, 1.0));
        assert_eq!(whole.large(), 12.0);
        assert_eq!(whole.at(0.5), 40.0);
        assert_eq!(whole.fraction(-20.0), 0.0);
        assert_eq!(whole.write(-0.0), "0");
    }

    #[test]
    fn a_bound_that_is_not_a_number_never_panics() {
        let steps = Steps::new(f64::NAN, 10.0, 1.0);
        assert_eq!((steps.min, steps.clamp(4.0), steps.clamp(12.0)), (f64::NEG_INFINITY, 4.0, 10.0));
        let steps = Steps::new(0.0, f64::NAN, 1.0);
        assert_eq!(steps.clamp(-3.0), 0.0);
    }
}
