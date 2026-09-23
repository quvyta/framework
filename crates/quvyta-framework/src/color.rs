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

    /// The entry of the xterm 256-colour palette this colour takes as text on `bg`.
    ///
    /// Mostly the nearest entry, as [`Rgb::to_ansi256`] gives it. Faint text, such as a page
    /// dimmed behind a dialog, can land on an entry that no longer reads on its background's —
    /// under 1.6:1 in the WCAG ratio, where a glyph starts to disappear; it then takes the entry
    /// closest to it in OKLab that still keeps 1.6:1, so faint text stays faint rather than
    /// vanishing. Text drawn in the very colour of its background is left as it is, since that
    /// is a fill rather than something to read.
    #[must_use]
    pub fn to_ansi256_text(self, bg: Self) -> u8 {
        let fg = self.to_ansi256();
        let behind = Self::from_ansi256(bg.to_ansi256());
        if self == bg || Self::from_ansi256(fg).contrast_ratio(behind) >= READABLE {
            return fg;
        }
        (16..=255u8)
            .filter(|&index| Self::from_ansi256(index).contrast_ratio(behind) >= READABLE)
            .min_by(|&a, &b| {
                let distance = |index| self.perceptual_distance(Self::from_ansi256(index));
                distance(a).total_cmp(&distance(b))
            })
            .unwrap_or(fg)
    }

    /// Nearest of the 16 standard terminal colours (xterm defaults), by distance alone.
    ///
    /// This is the plain reduction: on a dark screen every surface tone lands on black here.
    /// What a sixteen-colour frame actually shows is [`Rgb::to_ansi16_on`] for backgrounds and
    /// [`Rgb::to_ansi16_text`] for text, which keep lifted surfaces and faint text apart from the
    /// ground.
    #[must_use]
    pub fn to_ansi16(self) -> u8 {
        (0u8..16).min_by_key(|&i| self.squared_distance(ANSI16[usize::from(i)])).unwrap_or(0)
    }

    /// The colour of entry `index` of the 16 standard terminal colours, as xterm shows them by
    /// default; an index past 15 gives the last entry, white. A terminal may be themed
    /// differently, so these are the colours a reduction can count on, not the ones a person
    /// necessarily sees.
    #[must_use]
    pub fn from_ansi16(index: u8) -> Self {
        ANSI16[usize::from(index.min(15))]
    }

    /// The colour of entry `index` of the xterm 256-colour palette as xterm shows it by default:
    /// the 16 standard colours as [`Rgb::from_ansi16`] gives them, then the 6×6×6 colour cube,
    /// then the 24 greys from 8 to 238.
    #[must_use]
    pub fn from_ansi256(index: u8) -> Self {
        const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
        match index {
            0..=15 => Self::from_ansi16(index),
            16..=231 => {
                let cube = index - 16;
                let level = |i: u8| LEVELS[usize::from(i)];
                Self::new(level(cube / 36), level(cube / 6 % 6), level(cube % 6))
            }
            _ => {
                let grey = 8 + 10 * (index - 232);
                Self::new(grey, grey, grey)
            }
        }
    }

    /// The entry of the 16 standard colours this colour takes as a background on a screen whose
    /// ground is `ground` (the theme's `canvas`).
    ///
    /// Mostly the nearest entry, as [`Rgb::to_ansi16`] gives it. The exception is a tone the eye
    /// tells from the ground in full colour — at least 0.05 away in OKLab, the same distance
    /// under which a floating surface melts into the ground — that would still land on the
    /// ground's own entry: it moves one step along the grey ladder black, bright black, white,
    /// bright white, away from the ground. On a dark theme the ground stays black and raised
    /// surfaces, tab strips and dialogs become bright black; on a light theme they step down
    /// from bright white to white. Panels a shade off the ground stay on it, as in full colour
    /// they barely differ.
    #[must_use]
    pub fn to_ansi16_on(self, ground: Self) -> u8 {
        let nearest = self.to_ansi16();
        let base = ground.to_ansi16();
        if nearest != base || self.perceptual_distance(ground) < APART {
            return nearest;
        }
        let Some(rung) = GREYS.iter().position(|&grey| grey == base) else {
            return nearest;
        };
        let step = if self.oklab()[0] > ground.oklab()[0] { rung.checked_add(1) } else { rung.checked_sub(1) };
        step.and_then(|rung| GREYS.get(rung)).copied().unwrap_or(nearest)
    }

    /// The entry of the 16 standard colours this colour takes as text on `bg`, on a screen whose
    /// ground is `ground`.
    ///
    /// Both colours are reduced as [`Rgb::to_ansi16_on`] does. When the text would then no longer
    /// read on its background — under 1.6:1 in the WCAG ratio, which is where a glyph starts to
    /// disappear — it takes the quietest grey that still keeps 3:1 against it: faint text stays
    /// faint rather than vanishing. On bright black that is white, on black bright black. Text
    /// drawn in the very colour of its background is left as it is, since that is a fill rather
    /// than something to read.
    #[must_use]
    pub fn to_ansi16_text(self, bg: Self, ground: Self) -> u8 {
        let fg = self.to_ansi16_on(ground);
        let behind = bg.to_ansi16_on(ground);
        if self == bg || Self::from_ansi16(fg).contrast_ratio(Self::from_ansi16(behind)) >= READABLE {
            return fg;
        }
        let behind = Self::from_ansi16(behind);
        GREYS
            .iter()
            .copied()
            .map(|grey| (grey, Self::from_ansi16(grey).contrast_ratio(behind)))
            .filter(|(_, ratio)| *ratio >= QUIET_READABLE)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map_or(fg, |(grey, _)| grey)
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

/// The 16 standard terminal colours as xterm shows them by default.
const ANSI16: [Rgb; 16] = [
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

/// The greys of the 16 standard colours from dark to light: black, bright black, white, bright
/// white.
const GREYS: [u8; 4] = [0, 8, 7, 15];

/// Contrast under which reduced text starts to disappear into its background.
const READABLE: f64 = 1.6;

/// Contrast the grey that replaces unreadable text keeps against its background.
const QUIET_READABLE: f64 = 3.0;

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// OKLab distance under which a floating surface melts into the ground around it. Every built-in
/// theme's `overlay` sits just past it from `canvas`, so menus over the screen keep their tone,
/// and well within it from `surface` and `raised`, so menus over panels are lifted.
pub(crate) const APART: f64 = 0.05;

/// The furthest a floating surface is moved towards a theme colour, so a lift never turns the
/// surface into a different colour.
pub(crate) const LIFT_CAP: f32 = 0.3;

/// The steps a lift is searched in.
const LIFT_STEP: f32 = 0.01;

/// How a floating surface's backgrounds are moved so it stands apart from the ground around it:
/// every background is blended towards `towards` by `amount`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Lift {
    /// The theme colour the backgrounds move towards.
    pub(crate) towards: Rgb,
    /// How far they move, 0 to [`LIFT_CAP`].
    pub(crate) amount: f32,
}

impl Lift {
    /// `color` lifted.
    pub(crate) fn apply(self, color: Rgb) -> Rgb {
        color.mix(self.towards, self.amount)
    }
}

/// The smallest lift that takes `surface` at least [`APART`] from every colour in `grounds`,
/// trying each of `towards` (theme colours, in order of preference) up to [`LIFT_CAP`]. The
/// direction that needs less wins; an earlier one wins a tie. `None` when the surface already
/// stands apart, or when no lift within the cap moves it any further from the nearest ground;
/// when none reaches [`APART`], the lift that gets furthest does.
pub(crate) fn lift_apart(surface: Rgb, grounds: &[Rgb], towards: &[Rgb]) -> Option<Lift> {
    let grounds: Vec<[f64; 3]> = grounds.iter().map(|ground| ground.oklab()).collect();
    let clearance = |color: Rgb| {
        let [l, a, b] = color.oklab();
        grounds
            .iter()
            .map(|[gl, ga, gb]| ((l - gl).powi(2) + (a - ga).powi(2) + (b - gb).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min)
    };
    let resting = clearance(surface);
    if resting >= APART {
        return None;
    }
    // (lift, clearance): the first lift that clears, else the one that gets furthest.
    let mut cleared: Option<Lift> = None;
    let mut furthest: Option<(Lift, f64)> = None;
    let steps = (LIFT_CAP / LIFT_STEP).round() as u16;
    for &target in towards {
        for step in 1..=steps {
            let amount = f32::from(step) * LIFT_STEP;
            if cleared.is_some_and(|lift| lift.amount <= amount) {
                break;
            }
            let lift = Lift { towards: target, amount };
            let reach = clearance(lift.apply(surface));
            if reach >= APART {
                cleared = Some(lift);
                break;
            }
            if furthest.is_none_or(|(_, best)| reach > best) {
                furthest = Some((lift, reach));
            }
        }
    }
    cleared.or_else(|| furthest.filter(|(_, reach)| *reach > resting).map(|(lift, _)| lift))
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

    /// How a terminal of this depth shows `color` as a background on a screen whose ground is
    /// `ground`: two colours with the same answer are one colour on screen.
    pub(crate) fn shown(self, color: Rgb, ground: Rgb) -> u32 {
        match self {
            Self::TrueColor => u32::from(color.r) << 16 | u32::from(color.g) << 8 | u32::from(color.b),
            Self::Ansi256 => u32::from(color.to_ansi256()),
            Self::Ansi16 => u32::from(color.to_ansi16_on(ground)),
        }
    }

    /// Whether a terminal of this depth shows `a` and `b` as two different colours on a screen
    /// whose ground is `ground`.
    pub(crate) fn tells_apart(self, a: Rgb, b: Rgb, ground: Rgb) -> bool {
        self.shown(a, ground) != self.shown(b, ground)
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

    /// The monochrome theme's canvas, surface, overlay, raised and active tones.
    const LADDER: [Rgb; 5] =
        [Rgb::new(12, 12, 14), Rgb::new(19, 19, 23), Rgb::new(24, 24, 29), Rgb::new(29, 29, 35), Rgb::new(40, 40, 47)];

    #[test]
    fn a_lifted_surface_takes_bright_black_on_a_dark_ground() {
        let [canvas, surface, overlay, raised, active] = LADDER;
        for tone in LADDER {
            assert_eq!(tone.to_ansi16(), 0, "the plain reduction puts {tone} on black");
        }
        assert_eq!(canvas.to_ansi16_on(canvas), 0, "the ground stays black");
        assert_eq!(surface.to_ansi16_on(canvas), 0, "a panel a shade off the ground stays on it");
        for tone in [overlay, raised, active] {
            assert_eq!(tone.to_ansi16_on(canvas), 8, "{tone} is lifted to bright black");
        }
        // A dialog's dimmed page: the ground under it stays black, its faint text does not.
        let (dimmed_text, dimmed_ground) = (Rgb::new(49, 49, 55), Rgb::new(15, 15, 18));
        assert_eq!(dimmed_ground.to_ansi16_on(canvas), 0);
        assert_eq!(dimmed_text.to_ansi16_text(dimmed_ground, canvas), 8);
        // Colours that already land apart from the ground keep their nearest entry.
        assert_eq!(Rgb::new(240, 20, 20).to_ansi16_on(canvas), 9);
        assert_eq!(Rgb::new(245, 245, 247).to_ansi16_on(canvas), 15);
    }

    #[test]
    fn a_lifted_surface_steps_down_on_a_light_ground() {
        let canvas = Rgb::new(250, 250, 250);
        assert_eq!(canvas.to_ansi16_on(canvas), 15);
        assert_eq!(Rgb::new(244, 244, 245).to_ansi16_on(canvas), 15, "a shade off the ground stays on it");
        assert_eq!(Rgb::new(238, 238, 240).to_ansi16_on(canvas), 7, "a raised tone steps down to white");
        let text = Rgb::new(24, 24, 27);
        assert_eq!(text.to_ansi16_text(Rgb::new(238, 238, 240), canvas), 0, "dark text keeps its black");
    }

    #[test]
    fn text_that_would_vanish_takes_the_quietest_readable_grey() {
        let [canvas, _, _, raised, _] = LADDER;
        let muted = Rgb::new(95, 95, 105);
        assert_eq!(muted.to_ansi16_on(canvas), 8, "muted text alone is bright black");
        assert_eq!(muted.to_ansi16_text(canvas, canvas), 8, "and reads so on the ground");
        assert_eq!(muted.to_ansi16_text(raised, canvas), 7, "on bright black it steps up to white");
        let accent = Rgb::new(129, 140, 248);
        assert_eq!(accent.to_ansi16_on(canvas), 12);
        assert_eq!(accent.to_ansi16_text(raised, canvas), 7, "a blue that melts into bright black turns white");
        assert_eq!(raised.to_ansi16_text(raised, canvas), 8, "a fill in its own colour is left alone");
        for bg in 0..16 {
            let behind = Rgb::from_ansi16(bg);
            let text = Rgb::new(behind.r ^ 1, behind.g, behind.b);
            let shown = text.to_ansi16_text(behind, behind);
            let ratio = Rgb::from_ansi16(shown).contrast_ratio(Rgb::from_ansi16(behind.to_ansi16_on(behind)));
            assert!(ratio >= READABLE, "text on entry {bg} keeps {ratio:.2}:1");
        }
    }

    #[test]
    fn the_sixteen_colours_round_trip() {
        for index in 0..16 {
            assert_eq!(Rgb::from_ansi16(index).to_ansi16(), index);
        }
        assert_eq!(Rgb::from_ansi16(200), Rgb::new(255, 255, 255));
    }

    #[test]
    fn the_256_colours_round_trip() {
        for index in 16..=255 {
            assert_eq!(Rgb::from_ansi256(index).to_ansi256(), index);
        }
        assert_eq!(Rgb::from_ansi256(9), Rgb::from_ansi16(9));
        assert_eq!(Rgb::from_ansi256(196), Rgb::new(255, 0, 0));
        assert_eq!(Rgb::from_ansi256(232), Rgb::new(8, 8, 8));
        assert_eq!(Rgb::from_ansi256(255), Rgb::new(238, 238, 238));
    }

    #[test]
    fn faint_text_in_256_colours_keeps_a_readable_entry_near_its_own() {
        let ground = Rgb::new(15, 15, 18);
        // Nearest, this faint grey lands on an entry under 1.6:1 against its ground's.
        let faint = Rgb::new(46, 46, 52);
        let behind = Rgb::from_ansi256(ground.to_ansi256());
        assert!(Rgb::from_ansi256(faint.to_ansi256()).contrast_ratio(behind) < READABLE);
        let shown = Rgb::from_ansi256(faint.to_ansi256_text(ground));
        let ratio = shown.contrast_ratio(behind);
        assert!(ratio >= READABLE, "{shown} keeps {ratio:.2}:1");
        assert!(ratio < 2.0, "and stays faint: {shown} at {ratio:.2}:1");
        // Text that reads keeps its nearest entry; a fill in its own colour is left alone.
        let text = Rgb::new(245, 245, 247);
        assert_eq!(text.to_ansi256_text(ground), text.to_ansi256());
        assert_eq!(ground.to_ansi256_text(ground), ground.to_ansi256());
    }

    const DARK_TEXT: Rgb = Rgb::new(245, 245, 247);
    const DARK_CANVAS: Rgb = Rgb::new(12, 12, 14);

    #[test]
    fn a_surface_on_its_own_tone_is_lifted_apart() {
        let ground = Rgb::new(29, 29, 35);
        let lift = lift_apart(ground, &[ground], &[DARK_TEXT, DARK_CANVAS]).expect("the same tone is lifted");
        let lifted = lift.apply(ground);
        assert!(lifted.perceptual_distance(ground) >= APART);
        assert!(lifted.relative_luminance() > ground.relative_luminance(), "a dark theme lifts lighter");
        assert!(lift.amount <= LIFT_CAP);
    }

    #[test]
    fn a_close_tone_is_lifted_by_the_smallest_step_that_clears() {
        let (surface, ground) = (Rgb::new(24, 24, 29), Rgb::new(19, 19, 23));
        let lift = lift_apart(surface, &[ground], &[DARK_TEXT, DARK_CANVAS]).expect("a close tone is lifted");
        assert!(lift.apply(surface).perceptual_distance(ground) >= APART);
        let smaller = Lift { amount: lift.amount - LIFT_STEP, ..lift };
        assert!(smaller.apply(surface).perceptual_distance(ground) < APART, "no smaller step clears");
    }

    #[test]
    fn a_far_tone_is_left_alone() {
        let (surface, ground) = (Rgb::new(24, 24, 29), Rgb::new(12, 12, 14));
        assert!(surface.perceptual_distance(ground) >= APART);
        assert_eq!(lift_apart(surface, &[ground], &[DARK_TEXT, DARK_CANVAS]), None);
        assert_eq!(lift_apart(surface, &[], &[DARK_TEXT, DARK_CANVAS]), None, "nothing around, nothing to do");
    }

    #[test]
    fn a_light_theme_lifts_darker() {
        let (text, canvas) = (Rgb::new(24, 24, 27), Rgb::new(250, 250, 250));
        let ground = Rgb::new(238, 238, 240);
        let lift = lift_apart(ground, &[ground], &[text, canvas]).expect("lifted");
        let lifted = lift.apply(ground);
        assert!(lifted.relative_luminance() < ground.relative_luminance());
        assert!(lifted.perceptual_distance(ground) >= APART);
    }

    #[test]
    fn grey_stays_grey() {
        let (ground, text, canvas) = (Rgb::new(29, 29, 29), Rgb::new(245, 245, 245), Rgb::new(12, 12, 12));
        let lifted = lift_apart(ground, &[ground], &[text, canvas]).expect("lifted").apply(ground);
        assert!(lifted.r == lifted.g && lifted.g == lifted.b, "{lifted} is not a grey");
    }

    #[test]
    fn every_ground_is_cleared_and_the_nearer_direction_wins() {
        // A darker and a lighter ground on either side: lifting away from both needs a step
        // past the lighter one.
        let surface = Rgb::new(120, 120, 120);
        let grounds = [Rgb::new(116, 116, 116), Rgb::new(130, 130, 130)];
        let lift = lift_apart(surface, &grounds, &[DARK_TEXT, Rgb::new(0, 0, 0)]).expect("lifted");
        let lifted = lift.apply(surface);
        assert!(grounds.iter().all(|ground| lifted.perceptual_distance(*ground) >= APART));
        // Towards black is shorter here: the lighter ground sits in the way of the text.
        assert_eq!(lift.towards, Rgb::new(0, 0, 0));
    }

    #[test]
    fn a_lift_never_passes_the_cap() {
        // Towards a colour barely different from the ground, nothing clears; the furthest step
        // within the cap is taken.
        let ground = Rgb::new(100, 100, 100);
        let lift = lift_apart(ground, &[ground], &[Rgb::new(112, 112, 112)]).expect("the furthest lift");
        assert!((lift.amount - LIFT_CAP).abs() < 1e-6);
        assert!(lift.apply(ground).perceptual_distance(ground) < APART);
        assert_eq!(lift_apart(ground, &[ground], &[ground]), None, "a lift that gets nowhere is none");
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
