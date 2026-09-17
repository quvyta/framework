//! Single-cell activity indicators.

use std::time::Duration;

use crate::animation::AnimationName;
use crate::color::Rgb;
use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// How a [`Spinner`] moves. Every style is a built-in [cell animation](crate::animation) with
/// Nerd Font, Unicode and ASCII frames, which themes and applications can replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerStyle {
    /// An arc sweeping round (animation `spinner-arc`). The default.
    #[default]
    Arc,
    /// Braille dots turning in place (animation `spinner-dots`).
    Dots,
    /// A single dot orbiting a cell (animation `spinner-orbit`).
    Orbit,
    /// A dot growing and shrinking (animation `spinner-pop`).
    Pop,
    /// A dot breathing between faint and the spinner's colour (animation `spinner-pulse`).
    Pulse,
    /// A filled quarter of the cell turning clockwise (animation `spinner-quarters`). The
    /// quadrant blocks are drawn by terminals themselves, so they fill exactly one cell in any font.
    Quarters,
    /// A pie filling slice by slice, then starting again (animation `spinner-slices`). The Nerd
    /// Font frames are `nf-md-circle_slice_1..8` (Nerd Font v3).
    Slices,
}

impl SpinnerStyle {
    /// Every style, in alphabetical order.
    pub const ALL: [Self; 7] =
        [Self::Arc, Self::Dots, Self::Orbit, Self::Pop, Self::Pulse, Self::Quarters, Self::Slices];

    /// A short name, e.g. for settings screens and locale keys.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Arc => "arc",
            Self::Dots => "dots",
            Self::Orbit => "orbit",
            Self::Pop => "pop",
            Self::Pulse => "pulse",
            Self::Quarters => "quarters",
            Self::Slices => "slices",
        }
    }

    /// The name of the built-in animation this style plays, such as `"spinner-arc"`.
    #[must_use]
    pub fn animation(self) -> &'static str {
        match self {
            Self::Arc => "spinner-arc",
            Self::Dots => "spinner-dots",
            Self::Orbit => "spinner-orbit",
            Self::Pop => "spinner-pop",
            Self::Pulse => "spinner-pulse",
            Self::Quarters => "spinner-quarters",
            Self::Slices => "spinner-slices",
        }
    }
}

impl From<SpinnerStyle> for AnimationName {
    fn from(style: SpinnerStyle) -> Self {
        Self::from(style.animation())
    }
}

/// The animation a spinner plays once when its work is done.
const DONE_ANIMATION: &str = "spinner-done";

/// A one-cell indicator that something is working, with an optional label.
///
/// It plays a [cell animation](crate::animation): a [`SpinnerStyle`] or any animation by name.
/// With reduced motion the animation's rest frame stands still. Style keys: `spinner` (`fg`, with
/// variants for tones such as `spinner.success`) and `spinner-label`. The spinner's colour is the
/// `$fg` of its animation.
///
/// With [`Spinner::done`] the spinner stops turning and plays the animation `spinner-done` once:
/// the built-in one grows a tick, one `motion.step` per frame, blending from the colour on screen
/// into `$success`, and rests on the last frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spinner {
    animation: AnimationName,
    label: Option<String>,
    variant: Option<String>,
    done: bool,
}

impl Default for Spinner {
    fn default() -> Self {
        Self { animation: SpinnerStyle::default().into(), label: None, variant: None, done: false }
    }
}

impl Spinner {
    /// An arc spinner, the default style.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Chooses how it moves.
    #[must_use]
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.animation = style.into();
        self
    }

    /// Plays the animation `name` from the icon set or theme instead of a style, e.g. one an
    /// application defines in its theme. An unknown name draws `⟦`, like a missing icon.
    #[must_use]
    pub fn animation(mut self, name: impl Into<AnimationName>) -> Self {
        self.animation = name.into();
        self
    }

    /// Text after the spinner, e.g. "Pulling image".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Theme variant, e.g. `"success"` or `"warning"`.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Marks the work as finished. When this turns on, the spinner stops turning, plays the
    /// animation `spinner-done` once, starting from the colour on screen, and rests on its last
    /// frame. Turning it off spins again. A spinner that is already done when first drawn, or
    /// drawn with reduced motion, shows the last frame at once. The cell and the label stay where
    /// they are. Off by default.
    #[must_use]
    pub fn done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    /// The style the animation draws in: the `spinner` style, its colour falling back to the accent.
    fn cell_style(&self, cx: &mut PaintCx<'_>) -> CellStyle {
        let style = cx.style("spinner", self.variant.as_deref(), &[]).text();
        CellStyle { fg: Some(style.fg.unwrap_or_else(|| cx.color("accent"))), ..style }
    }

    /// Draws the finish: the frame due now, starting from the colour the spinner had on screen.
    fn paint_done(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let style = self.cell_style(cx);
        let started = match *cx.memory::<Finish>() {
            Finish::Unseen | Finish::Resting => None,
            Finish::Since { at, from } => Some((at, from)),
            Finish::Spinning => {
                // Where the turning animation is now, so a pulse finishes from the colour on screen.
                let now = cx.now();
                let turning = cx.animation(self.animation.as_str(), style, Some(Duration::ZERO));
                Some((now, turning.style.fg))
            }
        };
        let since = started.filter(|_| !cx.reduced_motion());
        let from = CellStyle { fg: since.and_then(|(_, from)| from).or(style.fg), ..style };
        let cell = cx.animation(DONE_ANIMATION, from, since.map(|(at, _)| at));
        *cx.memory::<Finish>() = match since {
            Some((at, from)) if !cell.finished => Finish::Since { at, from },
            _ => Finish::Resting,
        };
        cx.text(area.x, area.y, &cell.glyph, cell.style, 1);
    }

    /// Draws the frame of the turning spinner due now.
    fn paint_turning(&self, cx: &mut PaintCx<'_>, area: Rect) {
        *cx.memory::<Finish>() = Finish::Spinning;
        let style = self.cell_style(cx);
        let cell = cx.animation(self.animation.as_str(), style, Some(Duration::ZERO));
        cx.text(area.x, area.y, &cell.glyph, cell.style, 1);
    }
}

/// Where a spinner is in its finish, kept between frames.
#[derive(Debug, Clone, Copy, Default)]
enum Finish {
    /// Not drawn before: a spinner that starts done rests on its last frame at once.
    #[default]
    Unseen,
    /// Turning.
    Spinning,
    /// Finished at `at`, when the spinner showed the colour `from`.
    Since { at: Duration, from: Option<Rgb> },
    /// Showing the last frame.
    Resting,
}

impl<Msg: 'static> Widget<Msg> for Spinner {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let label = self.label.as_deref().map_or(0, |label| text::width(label).saturating_add(2));
        Size::new(label.saturating_add(1), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if self.done {
            self.paint_done(cx, area);
        } else {
            self.paint_turning(cx, area);
        }
        if let Some(label) = &self.label {
            let label_style = cx.style("spinner-label", self.variant.as_deref(), &[]).text();
            let budget = area.width.saturating_sub(2);
            let shown = text::truncate(label, budget).into_owned();
            cx.text(area.x + 2, area.y, &shown, label_style, budget);
        }
    }
}

#[cfg(test)]
mod tests;
