//! One-cell animations: an ordered list of frames, each with a glyph per glyph mode and an
//! optional colour, played at a theme or literal frame time.
//!
//! Every spinner style, the spinner's finish and anything an application defines are the same
//! [`CellAnimation`] data. They are written in icon set and theme files, so a theme or an
//! application replaces a built-in animation the way it replaces an icon:
//!
//! ```toml
//! [animations.spinner-arc]
//! frame = "spinner"          # a [motion] key, or a duration such as "80ms"
//! playback = "loop"          # loop | once | bounce
//! colors = "step"            # step | blend
//! rest = 1                   # the frame shown with reduced motion, counted from 1
//! frames = [
//!   { unicode = "◜", ascii = "-" },
//!   { nerd = "\uEE07", unicode = "◠", ascii = "\\", color = "mix($accent, $fg, 40%)" },
//!   { ascii = "|", color = "#38BDF8", duration = "120ms" },
//! ]
//! ```
//!
//! - **Glyphs.** `ascii` is required; a missing `unicode` falls back to `ascii` and a missing
//!   `nerd` to `unicode`. Every glyph is exactly one cell, and none may be a bracket.
//! - **Colours** are theme colour expressions: `$token`, `#RRGGBB`, `mix(a, b, N%)` and
//!   `pulse(a, b)`. `$fg` is the colour of the widget drawing the animation. A frame without a
//!   colour takes the widget's colour.
//! - **Colour modes.** `step` shows each frame in its own colour; `blend` moves the colour
//!   smoothly towards the next frame's colour while a frame is shown.
//! - **Playback.** `loop` repeats, `once` plays once and rests on the last frame, `bounce` plays
//!   forward and back.
//! - **Reduced motion** shows the `rest` frame (the first, or the last for `once`) standing still;
//!   a `pulse()` then shows its second colour.
//!
//! Draw a named animation with [`PaintCx::animation`](crate::widget::PaintCx::animation), or
//! sample a [`CellAnimation`] directly with [`CellAnimation::sample`].

mod legacy;
mod load;
mod play;
mod write;

use std::borrow::Cow;
use std::fmt;
use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use crate::color::Rgb;
use crate::icons::GlyphMode;
use crate::theme::{Expr, MOTION_KEYS, Motion, Paint, Theme, parse_duration};

pub(crate) use legacy::{LEGACY_ICONS, apply_legacy, check_legacy};
pub use load::parse_animations;
pub(crate) use load::read_animation_table;
pub use play::CellFrame;

/// The most frames one animation may have; a longer list is an error in a file.
pub const MAX_FRAMES: usize = 256;

/// Characters no glyph may be: shapes come from colour, never from brackets.
const BRACKETS: [char; 8] = ['[', ']', '(', ')', '{', '}', '<', '>'];

/// Checks that `glyph` can be the `mode` glyph of a frame: one grapheme, exactly one cell wide,
/// not a bracket, and printable ASCII in [`GlyphMode::Ascii`].
///
/// # Errors
///
/// Returns why the glyph cannot be used, in a sentence.
pub fn check_glyph(glyph: &str, mode: GlyphMode) -> Result<(), String> {
    if glyph.is_empty() {
        return Err("the glyph is empty".to_owned());
    }
    if glyph.graphemes(true).count() != 1 {
        return Err(format!("`{glyph}` is {} characters; a frame shows one", glyph.graphemes(true).count()));
    }
    let width = crate::text::width(glyph);
    if width != 1 {
        return Err(format!("`{glyph}` is {width} cells wide; a frame glyph must be exactly one cell"));
    }
    if mode == GlyphMode::Ascii && !glyph.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) {
        return Err(format!("`{glyph}` is not printable ASCII"));
    }
    if glyph.chars().any(|c| BRACKETS.contains(&c)) {
        return Err(format!("`{glyph}` is a bracket; brackets are not allowed as glyphs"));
    }
    Ok(())
}

/// One cell of a playing animation, as [`PaintCx::animation`](crate::widget::PaintCx::animation)
/// returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimatedCell {
    /// The glyph to draw, one cell wide.
    pub glyph: String,
    /// The style to draw it in: the widget's style in the frame's colour.
    pub style: crate::style::CellStyle,
    /// Whether a [`Playback::Once`] animation has played to its end, or stands still.
    pub finished: bool,
}

/// The name of a registered animation, such as `"spinner-arc"`. Widgets that play animations
/// take anything that converts into one: a string or a [`SpinnerStyle`](crate::widgets::SpinnerStyle).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnimationName(Cow<'static, str>);

impl AnimationName {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for AnimationName {
    fn from(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }
}

impl From<String> for AnimationName {
    fn from(name: String) -> Self {
        Self(Cow::Owned(name))
    }
}

impl fmt::Display for AnimationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether `name` can name an animation: lowercase letters, digits and `-`, like colour tokens.
#[must_use]
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// How long a frame is shown: a `[motion]` key of the theme, or a fixed duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameTime {
    /// A `[motion]` duration key such as `"spinner"` or `"step"`, so the theme sets the pace.
    Motion(&'static str),
    /// A fixed duration, longer than zero.
    Fixed(Duration),
}

impl Default for FrameTime {
    /// The theme's `motion.spinner`.
    fn default() -> Self {
        Self::Motion("spinner")
    }
}

impl FrameTime {
    /// Reads a motion key (`"spinner"`) or a duration (`"80ms"`, `"0.2s"`).
    ///
    /// # Errors
    ///
    /// Explains why `text` is neither a duration key of `[motion]` nor a duration longer than 0.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if let Some(key) = MOTION_KEYS.iter().find(|key| **key == text && **key != "slide") {
            return Ok(Self::Motion(key));
        }
        if text.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
            let duration = parse_duration(text)?;
            if duration.is_zero() {
                return Err(format!("`{text}` is too short; a frame lasts longer than 0ms"));
            }
            return Ok(Self::Fixed(duration));
        }
        let keys: Vec<&str> = MOTION_KEYS.iter().copied().filter(|key| *key != "slide").collect();
        Err(format!("`{text}` is not a frame time; use a motion key ({}) or a duration like \"80ms\"", keys.join(", ")))
    }

    /// The duration in `motion`, never shorter than a millisecond.
    #[must_use]
    pub fn resolve(self, motion: &Motion) -> Duration {
        let duration = match self {
            Self::Motion(key) => motion.duration(key).unwrap_or(motion.spinner),
            Self::Fixed(duration) => duration,
        };
        duration.max(Duration::from_millis(1))
    }
}

impl fmt::Display for FrameTime {
    /// The form [`FrameTime::parse`] reads back: the key, or the duration in milliseconds.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Motion(key) => f.write_str(key),
            Self::Fixed(duration) if duration.subsec_nanos() % 1_000_000 == 0 => {
                write!(f, "{}ms", duration.as_millis())
            }
            Self::Fixed(duration) => write!(f, "{}s", duration.as_secs_f64()),
        }
    }
}

/// How the frames follow one another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Playback {
    /// From the first frame to the last, again and again. The default.
    #[default]
    Loop,
    /// From the first frame to the last once, then resting on the last.
    Once,
    /// Forward to the last frame, back to the first, and again.
    Bounce,
}

impl Playback {
    /// Every playback, in the order a settings screen lists them.
    pub const ALL: [Self; 3] = [Self::Loop, Self::Once, Self::Bounce];

    /// The name used in files.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Once => "once",
            Self::Bounce => "bounce",
        }
    }

    /// Looks a playback up by name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|playback| playback.name() == name)
    }
}

/// How colours move between frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Each frame is shown in its own colour. The default.
    #[default]
    Step,
    /// While a frame is shown its colour moves smoothly towards the next frame's colour.
    Blend,
}

impl ColorMode {
    /// Every mode, in the order a settings screen lists them.
    pub const ALL: [Self; 2] = [Self::Step, Self::Blend];

    /// The name used in files.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Step => "step",
            Self::Blend => "blend",
        }
    }

    /// Looks a mode up by name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }
}

/// The colour of a frame: a theme colour expression, resolved against the theme drawing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellColor {
    text: String,
    expr: Expr,
}

impl CellColor {
    /// Reads `$token`, `#RRGGBB`, `mix(a, b, N%)` or `pulse(a, b)`; `$fg` is the widget's colour.
    ///
    /// # Errors
    ///
    /// Explains what is wrong with the expression.
    pub fn parse(text: &str) -> Result<Self, String> {
        let expr = Expr::parse(text)?;
        if let Some(message) = expr.nested_pulse() {
            return Err(message);
        }
        Ok(Self { text: text.trim().to_owned(), expr })
    }

    /// The expression as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The paint in `theme`, with `$fg` standing for `fg`.
    ///
    /// # Errors
    ///
    /// Names the first colour token `theme` does not define.
    pub fn resolve(&self, theme: &Theme, fg: Rgb) -> Result<Paint, String> {
        self.expr.resolve_by(&|name| if name == "fg" { Some(fg) } else { theme.color(name) })
    }
}

impl From<Rgb> for CellColor {
    /// A hard-coded colour, written `#RRGGBB`.
    fn from(color: Rgb) -> Self {
        Self { text: format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b), expr: Expr::Hex(color) }
    }
}

/// One frame: its glyphs, and optionally its own colour and duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationFrame {
    ascii: String,
    unicode: Option<String>,
    nerd: Option<String>,
    color: Option<CellColor>,
    duration: Option<FrameTime>,
}

impl AnimationFrame {
    /// A frame showing `ascii` in every glyph mode until [`unicode`](Self::unicode) or
    /// [`nerd`](Self::nerd) give it other glyphs. Check glyphs from users with [`check_glyph`];
    /// a glyph wider than a cell is cut to one cell when drawn.
    #[must_use]
    pub fn new(ascii: impl Into<String>) -> Self {
        Self { ascii: ascii.into(), unicode: None, nerd: None, color: None, duration: None }
    }

    /// The glyph for Unicode terminals, and for Nerd Font terminals without a `nerd` glyph.
    #[must_use]
    pub fn unicode(mut self, glyph: impl Into<String>) -> Self {
        self.unicode = Some(glyph.into());
        self
    }

    /// The glyph for terminals with a Nerd Font.
    #[must_use]
    pub fn nerd(mut self, glyph: impl Into<String>) -> Self {
        self.nerd = Some(glyph.into());
        self
    }

    /// The frame's own colour instead of the widget's.
    #[must_use]
    pub fn color(mut self, color: CellColor) -> Self {
        self.color = Some(color);
        self
    }

    /// How long this frame is shown instead of the animation's frame time.
    #[must_use]
    pub fn duration(mut self, duration: FrameTime) -> Self {
        self.duration = Some(duration);
        self
    }

    /// The glyph drawn in `mode`, following the fallback nerd → unicode → ascii.
    #[must_use]
    pub fn glyph(&self, mode: GlyphMode) -> &str {
        let unicode = || self.unicode.as_deref().unwrap_or(&self.ascii);
        match mode {
            GlyphMode::Nerd => self.nerd.as_deref().unwrap_or_else(unicode),
            GlyphMode::Unicode => unicode(),
            GlyphMode::Ascii => &self.ascii,
        }
    }

    /// The glyph written for `mode` itself, without falling back.
    #[must_use]
    pub fn own_glyph(&self, mode: GlyphMode) -> Option<&str> {
        match mode {
            GlyphMode::Nerd => self.nerd.as_deref(),
            GlyphMode::Unicode => self.unicode.as_deref(),
            GlyphMode::Ascii => Some(&self.ascii),
        }
    }

    /// The frame's own colour, if it has one.
    #[must_use]
    pub fn frame_color(&self) -> Option<&CellColor> {
        self.color.as_ref()
    }

    /// The frame's own duration, if it has one.
    #[must_use]
    pub fn frame_duration(&self) -> Option<FrameTime> {
        self.duration
    }
}

/// A one-cell animation. See the [module documentation](self) for the file format.
///
/// ```
/// use qframe::animation::{AnimationFrame, CellAnimation, CellColor, Playback};
///
/// let blink = CellAnimation::new()
///     .frame(AnimationFrame::new("*").unicode("●"))
///     .frame(AnimationFrame::new(".").unicode("·").color(CellColor::parse("$muted").expect("colour")))
///     .playback(Playback::Bounce);
/// assert_eq!(blink.frames().len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellAnimation {
    frames: Vec<AnimationFrame>,
    frame_time: FrameTime,
    playback: Playback,
    colors: ColorMode,
    rest: Option<usize>,
}

impl CellAnimation {
    /// An animation without frames, looping at the theme's `motion.spinner` with step colours.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a frame at the end.
    #[must_use]
    pub fn frame(mut self, frame: AnimationFrame) -> Self {
        self.frames.push(frame);
        self
    }

    /// How long each frame is shown, unless the frame has its own duration.
    #[must_use]
    pub fn frame_time(mut self, time: FrameTime) -> Self {
        self.frame_time = time;
        self
    }

    /// How the frames follow one another.
    #[must_use]
    pub fn playback(mut self, playback: Playback) -> Self {
        self.playback = playback;
        self
    }

    /// How colours move between frames.
    #[must_use]
    pub fn colors(mut self, colors: ColorMode) -> Self {
        self.colors = colors;
        self
    }

    /// The frame shown with reduced motion, counted from 0. Without it the first frame rests,
    /// or the last for [`Playback::Once`].
    #[must_use]
    pub fn rest(mut self, index: usize) -> Self {
        self.rest = Some(index);
        self
    }

    /// The frames in order.
    #[must_use]
    pub fn frames(&self) -> &[AnimationFrame] {
        &self.frames
    }

    /// The frame time.
    #[must_use]
    pub fn time(&self) -> FrameTime {
        self.frame_time
    }

    /// The playback.
    #[must_use]
    pub fn play_mode(&self) -> Playback {
        self.playback
    }

    /// The colour mode.
    #[must_use]
    pub fn color_mode(&self) -> ColorMode {
        self.colors
    }

    /// The rest frame as set, counted from 0.
    #[must_use]
    pub fn rest_frame(&self) -> Option<usize> {
        self.rest
    }

    /// The frame shown standing still: the rest frame, else the first, or the last for
    /// [`Playback::Once`]; always a valid index of a non-empty animation.
    #[must_use]
    pub fn rest_index(&self) -> usize {
        let last = self.frames.len().saturating_sub(1);
        self.rest.unwrap_or(if self.playback == Playback::Once { last } else { 0 }).min(last)
    }

    /// The glyph of frame `index` in `mode`; empty when there is no such frame.
    #[must_use]
    pub fn glyph(&self, index: usize, mode: GlyphMode) -> &str {
        self.frames.get(index).map_or("", |frame| frame.glyph(mode))
    }
}

#[cfg(test)]
mod tests;
