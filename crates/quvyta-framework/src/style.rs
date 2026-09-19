//! Drawing styles: what a cell looks like, and theme styles resolved for one frame.

use ratatui_core::buffer::Cell;
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

    /// Writes this style into `cell`, reducing colours to `depth`.
    #[cfg(test)]
    pub(crate) fn apply(self, cell: &mut Cell, depth: ColorDepth) {
        self.for_depth(depth).apply(cell);
    }

    /// This style with its colours reduced to `depth`, for writing into many cells.
    pub(crate) fn for_depth(self, depth: ColorDepth) -> CellPaint {
        let mut modifier = Modifier::empty();
        modifier.set(Modifier::BOLD, self.bold);
        modifier.set(Modifier::ITALIC, self.italic);
        modifier.set(Modifier::UNDERLINED, self.underline);
        modifier.set(Modifier::DIM, self.dim);
        CellPaint { fg: self.fg.map(|fg| to_color(fg, depth)), bg: self.bg.map(|bg| to_color(bg, depth)), modifier }
    }
}

/// A [`CellStyle`] with its colours already reduced to the terminal's depth. Reducing a colour
/// to 256 or 16 colours searches a palette, which is worth doing once per text rather than once
/// per cell.
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

/// Converts a colour for a terminal of `depth`.
pub(crate) fn to_color(color: Rgb, depth: ColorDepth) -> Color {
    match depth {
        ColorDepth::TrueColor => Color::Rgb(color.r, color.g, color.b),
        ColorDepth::Ansi256 => Color::Indexed(color.to_ansi256()),
        ColorDepth::Ansi16 => Color::Indexed(color.to_ansi16()),
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
    fn applies_colours_for_each_depth() {
        let mut cell = Cell::default();
        CellStyle::fg(Rgb::new(255, 0, 0))
            .on(Rgb::new(0, 0, 0))
            .with_bold(true)
            .apply(&mut cell, ColorDepth::TrueColor);
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
        assert!(cell.modifier.contains(Modifier::BOLD));
        CellStyle::fg(Rgb::new(255, 0, 0)).apply(&mut cell, ColorDepth::Ansi256);
        assert_eq!(cell.fg, Color::Indexed(196));
        assert_eq!(cell.bg, Color::Rgb(0, 0, 0));
        assert!(!cell.modifier.contains(Modifier::BOLD));
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
