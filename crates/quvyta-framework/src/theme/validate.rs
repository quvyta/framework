//! Readability checks for resolved themes: contrast and distinguishable status colours.

use super::Theme;
use crate::color::Rgb;
use crate::diagnostics::Diagnostic;

/// `(foreground, background, minimum WCAG contrast ratio)`.
const CONTRAST: [(&str, &str, f64); 9] = [
    ("text", "canvas", 7.0),
    ("text", "surface", 7.0),
    ("dim", "surface", 4.5),
    ("muted", "surface", 2.5),
    ("success", "surface", 4.5),
    ("warning", "surface", 4.5),
    ("danger", "surface", 4.5),
    ("info", "surface", 4.5),
    ("ink", "accent", 4.5),
];

/// Colours that must look different from each other.
const DISTINCT: [&str; 5] = ["accent", "success", "warning", "danger", "info"];

/// Minimum OKLab distance between any two of [`DISTINCT`].
const MIN_DISTANCE: f64 = 0.10;

/// Warnings for pairs that are hard to read or tell apart.
pub(crate) fn validate(theme: &Theme) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    let color = |name: &str| theme.color(name).unwrap_or(Rgb::new(0, 0, 0));
    for (fg, bg, minimum) in CONTRAST {
        let ratio = color(fg).contrast_ratio(color(bg));
        if ratio < minimum {
            warnings.push(Diagnostic::warning(
                None,
                format!(
                    "theme `{}`: `{fg}` on `{bg}` has contrast {ratio:.2}, below {minimum}; text will be hard to read",
                    theme.id()
                ),
            ));
        }
    }
    for (i, a) in DISTINCT.iter().enumerate() {
        for b in &DISTINCT[i + 1..] {
            let distance = color(a).perceptual_distance(color(b));
            if distance < MIN_DISTANCE {
                warnings.push(Diagnostic::warning(
                    None,
                    format!(
                        "theme `{}`: `{a}` and `{b}` look too similar (distance {distance:.3}, minimum {MIN_DISTANCE}); users may confuse their meaning",
                        theme.id()
                    ),
                ));
            }
        }
    }
    warnings
}
