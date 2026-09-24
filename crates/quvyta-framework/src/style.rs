//! Drawing styles: what a cell looks like, and theme styles resolved for one frame.

use std::collections::HashMap;

use ratatui_core::buffer::{Buffer, Cell};
use ratatui_core::style::{Color, Modifier};

use crate::color::{ColorDepth, Rgb};
use crate::geometry::Padding;
use crate::theme::{Paint, PropValue, StyleProps};

/// Colours and attributes of drawn text. `None` colours keep what is already underneath.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellStyle {
    /// Text colour.
    pub fg: Option<Rgb>,
    /// Background colour.
    pub bg: Option<Rgb>,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underlined.
    pub underline: bool,
    /// Faint.
    pub dim: bool,
}

impl CellStyle {
    /// A style with only a text colour.
    #[must_use]
    pub fn fg(color: Rgb) -> Self {
        Self { fg: Some(color), ..Self::default() }
    }

    /// Replaces the background colour.
    #[must_use]
    pub fn on(mut self, color: Rgb) -> Self {
        self.bg = Some(color);
        self
    }

    /// Turns bold on or off.
    #[must_use]
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Writes this style into `cell`.
    #[cfg(test)]
    pub(crate) fn apply(self, cell: &mut Cell) {
        self.paint().apply(cell);
    }

    /// This style as cell colours and modifiers, for writing into many cells.
    pub(crate) fn paint(self) -> CellPaint {
        let mut modifier = Modifier::empty();
        modifier.set(Modifier::BOLD, self.bold);
        modifier.set(Modifier::ITALIC, self.italic);
        modifier.set(Modifier::UNDERLINED, self.underline);
        modifier.set(Modifier::DIM, self.dim);
        CellPaint { fg: self.fg.map(to_color), bg: self.bg.map(to_color), modifier }
    }
}

/// A [`CellStyle`] as the colours and modifiers a cell carries, worked out once per text rather
/// than once per cell.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CellPaint {
    fg: Option<Color>,
    bg: Option<Color>,
    modifier: Modifier,
}

impl CellPaint {
    /// Writes this style into `cell`; `None` colours keep what is there.
    pub(crate) fn apply(self, cell: &mut Cell) {
        if let Some(fg) = self.fg {
            cell.fg = fg;
        }
        if let Some(bg) = self.bg {
            cell.bg = bg;
        }
        cell.modifier = self.modifier;
    }
}

/// Converts a colour for painting a frame.
///
/// Every frame is painted in full colour; one below true colour is reduced to its palette once
/// it is complete, by [`reduce`]. Blending (a dimmed screen behind a dialog, a lifted menu, a page
/// sliding in) needs the full colours to work on, and which palette entry a colour takes can
/// depend on the theme's ground and, for text, on the colour behind it.
pub(crate) fn to_color(color: Rgb) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Reduces a frame painted in full colour to the palette of `depth`, on a screen whose ground is
/// `ground`; a true-colour frame is left as it is. Cells already in palette colours are left as
/// they are.
///
/// In sixteen colours backgrounds take [`Rgb::to_ansi16_on`] and text [`Rgb::to_ansi16_text`]
/// against the background of its own cell; in 256 colours backgrounds take [`Rgb::to_ansi256`]
/// and text [`Rgb::to_ansi256_text`]. A half block (`▀`, `▄`) in 256 colours is a fill rather than
/// text, two pixels of a picture or of big letters, so both its halves take [`Rgb::to_ansi256`]:
/// pushing the upper half away from the lower one to keep it readable would streak a smooth
/// picture wherever two neighbouring pixels are close.
pub(crate) fn reduce(buf: &mut Buffer, depth: ColorDepth, ground: Rgb) {
    match depth {
        ColorDepth::TrueColor => {}
        ColorDepth::Ansi256 => reduce_with(buf, Rgb::to_ansi256, Rgb::to_ansi256_text, true),
        ColorDepth::Ansi16 => {
            let text = |text: Rgb, bg| text.to_ansi16_text(bg, ground);
            reduce_with(buf, |tone| tone.to_ansi16_on(ground), text, false);
        }
    }
}

/// Whether `symbol` is a half block, which splits its cell into two areas of colour.
fn is_half_block(symbol: &str) -> bool {
    matches!(symbol, "▀" | "▄")
}

/// Reduces every full-colour cell of `buf`: backgrounds and glyphless text by `tone`, the text of
/// a glyph by `text` against its cell's background. With `half_blocks_fill`, a half block counts
/// as glyphless.
fn reduce_with(buf: &mut Buffer, tone: impl Fn(Rgb) -> u8, text: impl Fn(Rgb, Rgb) -> u8, half_blocks_fill: bool) {
    // A frame holds few distinct colours and many cells; each reduction searches the palette once.
    // Neighbouring cells mostly share their colours, so the last cell's answer is tried first.
    let mut tones: HashMap<Rgb, u8> = HashMap::new();
    let mut texts: HashMap<(Rgb, Rgb), u8> = HashMap::new();
    let mut last: Option<(CellColours, (Color, Color))> = None;
    for cell in &mut buf.content {
        let (bg, fg) = (rgb(cell.bg), rgb(cell.fg));
        if bg.is_none() && fg.is_none() {
            continue;
        }
        let symbol = cell.symbol();
        let glyph =
            fg.is_some() && bg.is_some() && !symbol.trim().is_empty() && !(half_blocks_fill && is_half_block(symbol));
        let key = (cell.fg, cell.bg, glyph);
        if let Some((seen, (fg, bg))) = last
            && seen == key
        {
            (cell.fg, cell.bg) = (fg, bg);
            continue;
        }
        if let Some(fg) = fg {
            let index = match bg {
                Some(bg) if glyph => *texts.entry((fg, bg)).or_insert_with(|| text(fg, bg)),
                _ => *tones.entry(fg).or_insert_with(|| tone(fg)),
            };
            cell.fg = Color::Indexed(index);
        }
        if let Some(bg) = bg {
            cell.bg = Color::Indexed(*tones.entry(bg).or_insert_with(|| tone(bg)));
        }
        last = Some((key, (cell.fg, cell.bg)));
    }
}

/// A cell's text and background colours as painted, and whether it holds a glyph.
type CellColours = (Color, Color, bool);

/// The colour of a cell painted in full colour.
fn rgb(color: Color) -> Option<Rgb> {
    match color {
        Color::Rgb(r, g, b) => Some(Rgb::new(r, g, b)),
        _ => None,
    }
}

/// A theme style resolved for the current frame: pulses are evaluated at one phase.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetStyle {
    props: StyleProps,
    phase: f32,
}

impl WidgetStyle {
    pub(crate) fn new(props: StyleProps, phase: f32) -> Self {
        Self { props, phase }
    }

    /// This style without `key`, e.g. a selected row that shares the selection tone but leaves the
    /// pillar to the row that has the cursor.
    #[must_use]
    pub(crate) fn without(mut self, key: &str) -> Self {
        self.props.remove(key);
        self
    }

    /// The colour stored under `key` (`fg`, `bg`, `pillar`, `track`, ...).
    #[must_use]
    pub fn color(&self, key: &str) -> Option<Rgb> {
        self.props.paint(key).map(|paint: Paint| paint.at(self.phase))
    }

    /// A flag such as `bold`; `false` when unset.
    #[must_use]
    pub fn flag(&self, key: &str) -> bool {
        self.props.flag(key)
    }

    /// A cell count such as `gap`.
    #[must_use]
    pub fn cells(&self, key: &str) -> Option<u16> {
        self.props.cells(key)
    }

    /// A word such as a scrollbar `style`.
    #[must_use]
    pub fn word(&self, key: &str) -> Option<&'static str> {
        self.props.word(key)
    }

    /// Padding from `padding = [vertical, horizontal]` or `padding = n`; zero when unset.
    #[must_use]
    pub fn padding(&self) -> Padding {
        match self.props.get("padding") {
            Some(PropValue::Pair(v, h)) => Padding::symmetric(v, h),
            Some(PropValue::Cells(n)) => Padding::all(n),
            _ => Padding::default(),
        }
    }

    /// The text style: `fg`, `bg`, `bold`, `italic`, `underline`, `dim`.
    #[must_use]
    pub fn text(&self) -> CellStyle {
        CellStyle {
            fg: self.color("fg"),
            bg: self.color("bg"),
            bold: self.flag("bold"),
            italic: self.flag("italic"),
            underline: self.flag("underline"),
            dim: self.flag("dim"),
        }
    }

    /// Whether drawing this style needs animation frames.
    #[must_use]
    pub fn is_animated(&self) -> bool {
        self.props.is_animated()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeRegistry;

    #[test]
    fn applies_colours_and_modifiers() {
        let mut cell = Cell::default();
        CellStyle::fg(Rgb::new(255, 0, 0)).on(Rgb::new(0, 0, 0)).with_bold(true).apply(&mut cell);
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
        assert!(cell.modifier.contains(Modifier::BOLD));
        CellStyle::fg(Rgb::new(0, 0, 255)).apply(&mut cell);
        assert_eq!(cell.fg, Color::Rgb(0, 0, 255));
        assert_eq!(cell.bg, Color::Rgb(0, 0, 0), "a style without a background keeps the one there");
        assert!(!cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn a_frame_is_reduced_to_its_palette_once_painted() {
        let ground = Rgb::new(12, 12, 14);
        let painted = || {
            let mut buf = Buffer::empty(ratatui_core::layout::Rect::new(0, 0, 3, 1));
            CellStyle::fg(Rgb::new(255, 0, 0)).on(Rgb::new(0, 0, 0)).apply(&mut buf.content[0]);
            buf.content[0].set_symbol("a");
            buf.content[1].set_bg(Color::Indexed(4));
            buf.content[2].set_bg(Color::Rgb(128, 128, 128));
            buf
        };
        let mut buf = painted();
        reduce(&mut buf, ColorDepth::TrueColor, ground);
        assert_eq!(buf, painted(), "true colour is sent as painted");
        reduce(&mut buf, ColorDepth::Ansi256, ground);
        assert_eq!((buf.content[0].fg, buf.content[0].bg), (Color::Indexed(196), Color::Indexed(16)));
        assert_eq!(buf.content[1].bg, Color::Indexed(4), "a palette colour is left as it is");
        assert_eq!(buf.content[2].bg, Color::Indexed(244));
        let mut buf = painted();
        reduce(&mut buf, ColorDepth::Ansi16, ground);
        assert_eq!((buf.content[0].fg, buf.content[0].bg), (Color::Indexed(9), Color::Indexed(0)));
        assert_eq!(buf.content[2].bg, Color::Indexed(8));
    }

    #[test]
    fn a_half_block_in_256_colours_is_two_fills_not_text() {
        let (top, bottom) = (Rgb::new(120, 120, 120), Rgb::new(124, 124, 124));
        let painted = |symbol: &str| {
            let mut buf = Buffer::empty(ratatui_core::layout::Rect::new(0, 0, 1, 1));
            CellStyle::fg(top).on(bottom).apply(&mut buf.content[0]);
            buf.content[0].set_symbol(symbol);
            buf
        };
        let mut buf = painted("▀");
        reduce(&mut buf, ColorDepth::Ansi256, Rgb::new(12, 12, 14));
        assert_eq!(buf.content[0].fg, Color::Indexed(top.to_ansi256()), "the upper half is its nearest entry");
        assert_eq!(buf.content[0].bg, Color::Indexed(bottom.to_ansi256()));
        let mut buf = painted("a");
        reduce(&mut buf, ColorDepth::Ansi256, Rgb::new(12, 12, 14));
        assert_ne!(buf.content[0].fg, Color::Indexed(top.to_ansi256()), "a letter is still kept readable");
    }

    #[test]
    fn resolves_theme_properties() {
        let (theme, _) = ThemeRegistry::builtin().resolve_or_default("monochrome");
        let style = WidgetStyle::new(theme.style("button", Some("primary"), &[]), 0.0);
        // Primary rests on a tint of the accent, never the full fill, so hover and press can rise.
        assert_ne!(style.text().bg, theme.color("accent"));
        assert_eq!(style.text().fg, theme.color("accent"));
        assert!(style.text().bold);
        assert_eq!(style.padding(), Padding::symmetric(0, 2));
    }
}
