//! Believable metric samples for the chart demos: repeatable, so tests and screenshots are stable.

/// The load of a busy host at tick `tick`, from 0 to 100: a slow daily wave, a faster ripple
/// and a little noise, offset per `series` so several series do not move together.
#[must_use]
pub fn load(series: u32, tick: u32) -> f32 {
    let t = tick as f32;
    let phase = series as f32 * 1.7;
    let wave = (t / 9.0 + phase).sin() * 22.0;
    let ripple = (t / 2.3 + phase * 2.0).sin() * 9.0;
    // A small hash gives each tick its own repeatable noise.
    let hash = tick.wrapping_mul(2_654_435_761).wrapping_add(series.wrapping_mul(40_503)) >> 24;
    let noise = hash as f32 / 255.0 * 8.0 - 4.0;
    (48.0 + wave + ripple + noise).clamp(2.0, 98.0)
}

/// The last `count` samples of `series` up to tick `tick`, oldest first.
#[must_use]
pub fn history(series: u32, tick: u32, count: u32) -> Vec<f32> {
    (tick.saturating_sub(count)..tick).map(|t| load(series, t)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_repeatable_and_in_range() {
        assert_eq!(history(0, 40, 30), history(0, 40, 30));
        assert!(history(2, 90, 60).iter().all(|v| (2.0..=98.0).contains(v)));
        assert_ne!(load(0, 10), load(1, 10));
    }
}
