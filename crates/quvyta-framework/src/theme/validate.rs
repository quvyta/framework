//! Readability checks for resolved themes: contrast and distinguishable status colours.

use super::{SERIES_COLORS, Theme};
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

/// Minimum contrast a series tone keeps against `surface`. Series tones are fills, not text, so
/// the bar is the one `muted` keeps rather than the 4.5 of readable type.
const MIN_SERIES_CONTRAST: f64 = 2.5;

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
    warnings.extend(validate_series(theme));
    warnings
}

/// Warnings for series tones that vanish into the surface or into each other. A chart names its
/// series in a legend, but two categories in the same tone still read as one block in a stacked
/// bar, and a tone as dark as the panel reads as an empty share.
fn validate_series(theme: &Theme) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    let surface = theme.color("surface").unwrap_or(Rgb::new(0, 0, 0));
    let tones: Vec<(String, Rgb)> =
        (0..SERIES_COLORS).map(|i| (format!("series-{}", i + 1), theme.series_color(i))).collect();
    for (name, tone) in &tones {
        let ratio = tone.contrast_ratio(surface);
        if ratio < MIN_SERIES_CONTRAST {
            warnings.push(Diagnostic::warning(
                None,
                format!(
                    "theme `{}`: `{name}` on `surface` has contrast {ratio:.2}, below {MIN_SERIES_CONTRAST}; its share of a chart will not be visible",
                    theme.id()
                ),
            ));
        }
    }
    for (i, (a, first)) in tones.iter().enumerate() {
        for (b, second) in &tones[i + 1..] {
            let distance = first.perceptual_distance(*second);
            if distance < MIN_DISTANCE {
                warnings.push(Diagnostic::warning(
                    None,
                    format!(
                        "theme `{}`: `{a}` and `{b}` look too similar (distance {distance:.3}, minimum {MIN_DISTANCE}); two categories will read as one",
                        theme.id()
                    ),
                ));
            }
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeRegistry;

    fn resolve(id: &str, text: &str) -> Vec<String> {
        let mut registry = ThemeRegistry::builtin();
        registry.add_source(id, &format!("{id}.toml"), text);
        registry.resolve(id).diagnostics.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn series_tones_that_merge_or_vanish_are_warnings() {
        let messages = resolve(
            "flat",
            "[meta]\nname = \"Flat\"\nextends = \"monochrome\"\n[colors]\nseries-2 = \"$series-1\"\nseries-5 = \"$surface\"\n",
        );
        assert!(messages.iter().any(|m| m.contains("`series-1` and `series-2` look too similar")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("`series-5` on `surface` has contrast")), "{messages:?}");
    }

    #[test]
    fn every_builtin_theme_keeps_its_series_apart() {
        let registry = ThemeRegistry::builtin();
        for (id, _) in registry.list() {
            let theme = registry.resolve(&id).theme.expect("resolves");
            assert_eq!(validate_series(&theme), Vec::new(), "theme {id}");
            let tones: Vec<Rgb> = (0..SERIES_COLORS).map(|i| theme.series_color(i)).collect();
            assert_eq!(theme.series_color(SERIES_COLORS), tones[0], "theme {id} wraps around");
            assert_eq!(theme.series_color(SERIES_COLORS * 3 + 2), tones[2], "theme {id} wraps around");
        }
    }
}
