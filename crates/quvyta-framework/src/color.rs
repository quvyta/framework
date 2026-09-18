//! Colours: parsing, blending, contrast and perceptual distance, and reduction to the
//! 256- and 16-colour palettes for terminals without 24-bit colour.

use std::fmt;

/// A 24-bit sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Creates a colour from its channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#RRGGBB` or `#RGB`, in either letter case.
    #[must_use]
    pub fn parse_hex(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |s: &str| u8::from_str_radix(s, 16).ok();
        match hex.len() {
            6 => Some(Self::new(channel(&hex[0..2])?, channel(&hex[2..4])?, channel(&hex[4..6])?)),
            3 => {
                let short = |i: usize| channel(&hex[i..=i]).map(|v| v * 17);
                Some(Self::new(short(0)?, short(1)?, short(2)?))
            }
            _ => None,
        }
    }

    /// Blends towards `other`: `t = 0` gives `self`, `t = 1` gives `other`.
    /// `t` is clamped to `0..=1`.
    #[must_use]
    pub fn mix(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let blend = |a: u8, b: u8| {
            let value = f32::from(a) + (f32::from(b) - f32::from(a)) * t;
            // `value` stays within 0..=255 because `t` is clamped.
            value.round().clamp(0.0, 255.0) as u8
        };
        Self::new(blend(self.r, other.r), blend(self.g, other.g), blend(self.b, other.b))
    }

    /// WCAG relative luminance, from 0 (black) to 1 (white).
    #[must_use]
    pub fn relative_luminance(self) -> f64 {
        let [r, g, b] = self.linear();
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// WCAG contrast ratio between two colours, from 1 to 21.
    #[must_use]
    pub fn contrast_ratio(self, other: Self) -> f64 {
        let (a, b) = (self.relative_luminance(), other.relative_luminance());
        let (light, dark) = if a >= b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }

    /// The colour in the OKLab perceptual space as `[L, a, b]`.
    #[must_use]
    pub fn oklab(self) -> [f64; 3] {
        let [r, g, b] = self.linear();
        let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
        let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
        let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
        let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
        [
            0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
            1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
            0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
        ]
    }

    /// Euclidean distance in OKLab. Around 0.02 is barely visible; 0.10 reads as a clearly
    /// different colour.
    #[must_use]
    pub fn perceptual_distance(self, other: Self) -> f64 {
        let [l1, a1, b1] = self.oklab();
        let [l2, a2, b2] = other.oklab();
        ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
    }

    /// Nearest entry of the xterm 256-colour palette (indices 16..=255).
    #[must_use]
    pub fn to_ansi256(self) -> u8 {
        const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
        let nearest_level = |v: u8| (0u8..6).min_by_key(|&i| v.abs_diff(LEVELS[usize::from(i)])).unwrap_or(0);
        let (ri, gi, bi) = (nearest_level(self.r), nearest_level(self.g), nearest_level(self.b));
        let cube = Self::new(LEVELS[usize::from(ri)], LEVELS[usize::from(gi)], LEVELS[usize::from(bi)]);
        let cube_index = 16 + 36 * ri + 6 * gi + bi;

        let average = (u16::from(self.r) + u16::from(self.g) + u16::from(self.b)) / 3;
        // Grey ramp values are 8, 18, ..., 238.
        let step = (average.saturating_sub(3) / 10).min(23) as u8;
        let grey_value = 8 + 10 * step;
        let grey = Self::new(grey_value, grey_value, grey_value);
        let grey_index = 232 + step;

        if self.squared_distance(grey) < self.squared_distance(cube) { grey_index } else { cube_index }
    }

    /// Nearest of the 16 standard terminal colours (xterm defaults).
    #[must_use]
    pub fn to_ansi16(self) -> u8 {
        const PALETTE: [Rgb; 16] = [
            Rgb::new(0, 0, 0),
            Rgb::new(205, 0, 0),
            Rgb::new(0, 205, 0),
            Rgb::new(205, 205, 0),
            Rgb::new(0, 0, 238),
            Rgb::new(205, 0, 205),
            Rgb::new(0, 205, 205),
            Rgb::new(229, 229, 229),
            Rgb::new(127, 127, 127),
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(255, 255, 0),
            Rgb::new(92, 92, 255),
            Rgb::new(255, 0, 255),
            Rgb::new(0, 255, 255),
            Rgb::new(255, 255, 255),
        ];
        (0u8..16).min_by_key(|&i| self.squared_distance(PALETTE[usize::from(i)])).unwrap_or(0)
    }

    fn linear(self) -> [f64; 3] {
        let channel = |v: u8| {
            let c = f64::from(v) / 255.0;
            if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        [channel(self.r), channel(self.g), channel(self.b)]
    }

    fn squared_distance(self, other: Self) -> u32 {
        let d = |a: u8, b: u8| u32::from(a.abs_diff(b)).pow(2);
        d(self.r, other.r) + d(self.g, other.g) + d(self.b, other.b)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// How many colours the terminal can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 24-bit colour.
    TrueColor,
    /// The xterm 256-colour palette.
    Ansi256,
    /// The 16 standard colours.
    Ansi16,
}

impl ColorDepth {
    /// Detects the colour depth from environment variables.
    ///
    /// `env` looks a variable up; pass `|name| std::env::var(name).ok()` in applications and a
    /// fixed map in tests.
    #[must_use]
    pub fn detect(env: impl Fn(&str) -> Option<String>) -> Self {
        let lower = |name: &str| env(name).map(|v| v.to_lowercase());
        if let Some(value) = lower("COLORTERM")
            && (value.contains("truecolor") || value.contains("24bit"))
        {
            return Self::TrueColor;
        }
        if env("WT_SESSION").is_some() {
            return Self::TrueColor;
        }
        if let Some(program) = lower("TERM_PROGRAM")
            && ["iterm", "wezterm", "vscode", "ghostty"].iter().any(|p| program.contains(p))
        {
            return Self::TrueColor;
        }
        match lower("TERM") {
            Some(term) if term.contains("direct") => Self::TrueColor,
            Some(term) if term.contains("256color") => Self::Ansi256,
            Some(term) if term == "dumb" || term == "linux" || term.is_empty() => Self::Ansi16,
            _ => Self::Ansi256,
        }
    }

    /// Whether a terminal of this depth shows `a` and `b` as two different colours.
    pub(crate) fn tells_apart(self, a: Rgb, b: Rgb) -> bool {
        match self {
            Self::TrueColor => a != b,
            Self::Ansi256 => a.to_ansi256() != b.to_ansi256(),
            Self::Ansi16 => a.to_ansi16() != b.to_ansi16(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn parses_long_and_short_hex() {
        assert_eq!(Rgb::parse_hex("#0B1118"), Some(Rgb::new(11, 17, 24)));
        assert_eq!(Rgb::parse_hex("#fff"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::parse_hex("0B1118"), None);
        assert_eq!(Rgb::parse_hex("#38BDZ8"), None);
        assert_eq!(Rgb::parse_hex("#12345"), None);
        assert_eq!(Rgb::new(11, 17, 24).to_string(), "#0b1118");
    }

    #[test]
    fn mix_blends_linearly_and_clamps() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        assert_eq!(black.mix(white, 0.0), black);
        assert_eq!(black.mix(white, 1.0), white);
        assert_eq!(black.mix(white, 0.5), Rgb::new(128, 128, 128));
        assert_eq!(black.mix(white, 7.0), white);
    }

    #[test]
    fn contrast_matches_wcag_extremes() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        assert!((black.contrast_ratio(white) - 21.0).abs() < 1e-9);
        assert!((white.contrast_ratio(white) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn oklab_of_white_is_unit_lightness() {
        let [l, a, b] = Rgb::new(255, 255, 255).oklab();
        assert!((l - 1.0).abs() < 1e-3 && a.abs() < 1e-3 && b.abs() < 1e-3);
        let red = Rgb::new(255, 0, 0);
        assert!(red.perceptual_distance(red) < 1e-12);
        assert!(red.perceptual_distance(Rgb::new(0, 0, 255)) > 0.3);
    }

    #[test]
    fn reduces_to_256_palette() {
        assert_eq!(Rgb::new(255, 0, 0).to_ansi256(), 196);
        assert_eq!(Rgb::new(128, 128, 128).to_ansi256(), 244);
        assert_eq!(Rgb::new(0, 0, 0).to_ansi256(), 16);
    }

    #[test]
    fn reduces_to_16_palette() {
        assert_eq!(Rgb::new(10, 10, 12).to_ansi16(), 0);
        assert_eq!(Rgb::new(250, 250, 250).to_ansi16(), 15);
        assert_eq!(Rgb::new(240, 20, 20).to_ansi16(), 9);
    }

    #[test]
    fn detects_color_depth() {
        assert_eq!(ColorDepth::detect(env(&[("COLORTERM", "truecolor")])), ColorDepth::TrueColor);
        assert_eq!(ColorDepth::detect(env(&[("TERM", "xterm-256color")])), ColorDepth::Ansi256);
        assert_eq!(ColorDepth::detect(env(&[("TERM", "linux")])), ColorDepth::Ansi16);
        assert_eq!(ColorDepth::detect(env(&[("TERM", "xterm-direct")])), ColorDepth::TrueColor);
        assert_eq!(ColorDepth::detect(env(&[])), ColorDepth::Ansi256);
    }
}
