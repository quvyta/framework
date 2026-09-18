//! Themes: colour tokens, motion timing, typography roles and CSS-like style rules.
//!
//! ```toml
//! [meta]
//! name = "Nordic"
//! extends = "monochrome"
//!
//! [colors]
//! accent = "#38BDF8"
//!
//! [style."button.primary"]
//! bg = "$accent"
//! fg = "$ink"
//!
//! [style."button:focus"]
//! bg = "pulse($accent, $accent-2)"
//! ```
//!
//! A theme only writes what differs from the theme it extends. Style rules match
//! `widget.variant:state` selectors; more specific rules win, and at equal specificity the
//! later rule wins, with rules of the extended theme counted first.

mod cache;
mod motion;
mod paint;
mod registry;
mod selector;
mod source;
mod style;
mod validate;

use std::collections::BTreeMap;
use std::sync::Arc;

pub use motion::Motion;
pub(crate) use motion::{MOTION_KEYS, parse_duration};
pub(crate) use paint::Expr;
pub use paint::Paint;
pub use registry::{Resolved, ThemeRegistry};
pub use selector::{Selector, State};
pub use style::{PropValue, StyleProps, WORD_PROPS};

use crate::animation::CellAnimation;
use crate::color::Rgb;
use crate::icons::IconGlyphs;
use cache::StyleCache;

/// Colour tokens every resolved theme defines.
pub const REQUIRED_COLORS: [&str; 20] = [
    "canvas", "surface", "raised", "active", "overlay", "accent", "accent-2", "text", "dim", "muted", "ink", "success",
    "warning", "danger", "info", "series-1", "series-2", "series-3", "series-4", "series-5",
];

/// How many categorical series tones a theme carries: `series-1` to `series-5`.
///
/// The set is deliberately small. Five tones a reader can tell apart are worth more than a dozen
/// they cannot, and a chart with more than five series is usually a table wearing a chart's
/// clothes.
pub const SERIES_COLORS: usize = 5;

/// A fully resolved theme, ready to style widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    id: String,
    name: String,
    colors: BTreeMap<String, Rgb>,
    motion: Motion,
    typography: BTreeMap<String, StyleProps>,
    rules: Vec<(Selector, StyleProps)>,
    icon_set: String,
    icons: BTreeMap<String, IconGlyphs>,
    animations: BTreeMap<String, Arc<CellAnimation>>,
    /// Styles already layered, so a widget painted every frame does not match every rule again.
    cache: StyleCache,
}

impl Theme {
    /// The theme id, which is its file stem.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The display name from `[meta] name`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// A colour token such as `"accent"`.
    #[must_use]
    pub fn color(&self, token: &str) -> Option<Rgb> {
        self.colors.get(token).copied()
    }

    /// Resolves a colour written the way theme files write it (`"$danger"`, `"#38BDF8"`,
    /// `"mix($accent, $danger, 50%)"`) against this theme's tokens.
    ///
    /// # Errors
    ///
    /// A message explaining why `expression` is not a single colour: it does not parse, names an
    /// unknown token, or uses `pulse()`, which breathes and so has no single colour.
    pub fn solid(&self, expression: &str) -> Result<Rgb, String> {
        paint::Expr::parse(expression)?.solid(&self.colors)
    }

    /// The tone of the `index`-th series of a chart, counted from zero.
    ///
    /// A theme carries [`SERIES_COLORS`] series tones, so the tones wrap around: series five takes
    /// the tone of series zero. Wrapping is why a chart must name its series with a
    /// [`Legend`](crate::widgets::Legend) instead of leaving the meaning in the colour, and why a
    /// chart that needs more than five kinds is better off grouping the small ones together.
    #[must_use]
    pub fn series_color(&self, index: usize) -> Rgb {
        let token = format!("series-{}", index % SERIES_COLORS + 1);
        self.color(&token).unwrap_or_else(|| self.colors.get("accent").copied().unwrap_or(Rgb::new(0, 0, 0)))
    }

    /// Every colour token, sorted by name.
    pub fn colors(&self) -> impl Iterator<Item = (&str, Rgb)> {
        self.colors.iter().map(|(name, color)| (name.as_str(), *color))
    }

    /// Motion timing.
    #[must_use]
    pub fn motion(&self) -> Motion {
        self.motion
    }

    /// A typography role such as `"title"`.
    #[must_use]
    pub fn typography(&self, role: &str) -> Option<&StyleProps> {
        self.typography.get(role)
    }

    /// The icon set this theme uses.
    #[must_use]
    pub fn icon_set(&self) -> &str {
        &self.icon_set
    }

    /// Icons this theme overrides on top of its icon set.
    #[must_use]
    pub fn icon_overrides(&self) -> &BTreeMap<String, IconGlyphs> {
        &self.icons
    }

    /// Animations this theme defines or replaces on top of its icon set's.
    #[must_use]
    pub fn animation_overrides(&self) -> &BTreeMap<String, Arc<CellAnimation>> {
        &self.animations
    }

    /// The style of `widget` drawn with `variant` in `states`: every matching rule layered
    /// from least to most specific.
    ///
    /// The result is remembered for the lifetime of the theme, so asking again (as widgets do
    /// every frame) is a lookup, and the returned properties share their storage.
    #[must_use]
    pub fn style(&self, widget: &str, variant: Option<&str>, states: &[State]) -> StyleProps {
        self.cache.get_or_insert(widget, variant, states, || self.layer_rules(widget, variant, states))
    }

    /// Layers every rule matching `widget`, `variant` and `states`, least specific first.
    fn layer_rules(&self, widget: &str, variant: Option<&str>, states: &[State]) -> StyleProps {
        let mut matching: Vec<(usize, &(Selector, StyleProps))> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, (selector, _))| selector.matches(widget, variant, states))
            .collect();
        matching.sort_by_key(|(order, (selector, _))| (selector.specificity(), *order));
        let mut props = StyleProps::default();
        for (_, (_, rule)) in matching {
            props.overlay(rule);
        }
        props
    }
}
