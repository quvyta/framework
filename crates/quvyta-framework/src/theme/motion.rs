//! Motion settings: how fast things breathe, flash, blink and step.

use std::time::Duration;

/// Timing values from the theme's `[motion]` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Motion {
    /// One full breath of `pulse()` paints.
    pub pulse_period: Duration,
    /// How long a pressed control flashes.
    pub flash: Duration,
    /// Half period of the text cursor blink.
    pub cursor_blink: Duration,
    /// Time between frames of cell-stepped animations such as the switch knob.
    pub step: Duration,
    /// Whether selected rows slide their leading text one cell to the right.
    pub slide: bool,
    /// How long layers such as dropdowns and dialogs take to appear.
    pub enter: Duration,
    /// Time each spinner frame is shown.
    pub spinner: Duration,
    /// One pass of light across shimmering text.
    pub shimmer: Duration,
    /// How long the pointer rests on something before its tooltip appears. Optional in theme
    /// files; 450ms when missing.
    pub hover_delay: Duration,
    /// How long a page change takes. Optional in theme files; twice `enter` when missing.
    pub page: Duration,
}

impl Motion {
    /// The duration stored under a `[motion]` key such as `"step"` or `"spinner"`; `None` for
    /// `slide`, which is not a duration, and for unknown keys.
    #[must_use]
    pub fn duration(&self, key: &str) -> Option<Duration> {
        Some(match key {
            "pulse-period" => self.pulse_period,
            "flash" => self.flash,
            "cursor-blink" => self.cursor_blink,
            "step" => self.step,
            "enter" => self.enter,
            "spinner" => self.spinner,
            "shimmer" => self.shimmer,
            "hover-delay" => self.hover_delay,
            "page" => self.page,
            _ => return None,
        })
    }
}

/// The keys accepted in `[motion]`.
pub(crate) const MOTION_KEYS: [&str; 10] =
    ["pulse-period", "flash", "cursor-blink", "step", "slide", "enter", "spinner", "shimmer", "page", "hover-delay"];

/// The longest motion duration a theme may set. Widgets multiply durations and add them to the
/// clock, so an unbounded value from a file could overflow; no animation needs more than this.
const MAX_DURATION: Duration = Duration::from_secs(3600);

/// Parses `"1400ms"`, `"1.4s"` or `"0ms"`, up to one hour.
pub(crate) fn parse_duration(text: &str) -> Result<Duration, String> {
    let invalid = || format!("`{text}` is not a duration; write it like \"90ms\" or \"1.4s\"");
    let (number, scale) = if let Some(ms) = text.strip_suffix("ms") {
        (ms, 0.001)
    } else if let Some(s) = text.strip_suffix('s') {
        (s, 1.0)
    } else {
        return Err(invalid());
    };
    let value: f64 = number.trim().parse().map_err(|_| invalid())?;
    if !value.is_finite() || value < 0.0 {
        return Err(invalid());
    }
    match Duration::try_from_secs_f64(value * scale) {
        Ok(duration) if duration <= MAX_DURATION => Ok(duration),
        _ => Err(format!("`{text}` is too long; motion durations are at most one hour")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_milliseconds_and_seconds() {
        assert_eq!(parse_duration("1400ms"), Ok(Duration::from_millis(1400)));
        assert_eq!(parse_duration("1.4s"), Ok(Duration::from_millis(1400)));
        assert_eq!(parse_duration("0ms"), Ok(Duration::ZERO));
    }

    #[test]
    fn rejects_other_forms() {
        assert!(parse_duration("1400").is_err());
        assert!(parse_duration("-5ms").is_err());
        assert!(parse_duration("fast").is_err());
    }

    #[test]
    fn rejects_durations_longer_than_an_hour_without_panicking() {
        assert_eq!(parse_duration("3600s"), Ok(MAX_DURATION));
        assert!(parse_duration("3601s").is_err_and(|message| message.contains("at most one hour")));
        // Beyond what `Duration` can hold, which must be an error rather than a panic.
        assert!(parse_duration("1e30s").is_err());
    }
}
