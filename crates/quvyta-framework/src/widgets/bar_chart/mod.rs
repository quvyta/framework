//! Bar charts: values compared side by side, in one series or in several.
//!
//! The model an application builds is here, together with the palette, the scale and the input
//! handling; `layout` works out where the categories and their bars sit and `paint` draws them.

mod layout;
mod paint;
#[cfg(test)]
mod tests;

use crate::color::Rgb;
use crate::event::{Event, KeyEvent, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::theme::{State, Theme};
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::IndexMessage;

/// Widest a vertical bar gets, in cells.
const MAX_BAR_WIDTH: u16 = 6;

/// Horizontal charts drop their labels below this width.
const LABEL_MIN_WIDTH: u16 = 24;

/// Tones a disabled chart walks through before it repeats.
const SERIES_TONES: usize = 4;

/// One bar of a [`BarChart`].
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    label: String,
    value: f32,
    value_text: Option<String>,
    variant: Option<String>,
}

impl Bar {
    /// A bar called `label` at `value`.
    #[must_use]
    pub fn new(label: impl Into<String>, value: f32) -> Self {
        Self { label: label.into(), value: value.max(0.0), value_text: None, variant: None }
    }

    /// A category of a chart whose values come from its series, so the bar carries only a label.
    fn category(label: impl Into<String>) -> Self {
        Self::new(label, 0.0)
    }

    /// Text shown for the value instead of the number, e.g. "1.2 GiB".
    #[must_use]
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.value_text = Some(text.into());
        self
    }

    /// Theme variant of this bar, e.g. `"danger"` for a service over its limit. A bar with a
    /// variant carries a marker before its value, so its meaning reads without colour.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }
}

/// One series of a [`BarChart`]: a name and one value per category.
///
/// A series takes its tone from the theme's series palette, never from a colour the application
/// picks, so charts in different applications of the family read the same. By default the n-th
/// series takes the n-th tone; [`tone`](Self::tone) pins a series to one tone of the palette, so
/// a category keeps its colour from one chart to the next however many categories each shows.
/// Values shorter than the categories count as zero and values past the last category are left
/// out.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    name: String,
    values: Vec<f32>,
    tone: Option<usize>,
}

impl Series {
    /// A series called `name` with one value per category, in the order of the categories.
    #[must_use]
    pub fn new(name: impl Into<String>, values: impl IntoIterator<Item = f32>) -> Self {
        Self { name: name.into(), values: values.into_iter().map(|value| value.max(0.0)).collect(), tone: None }
    }

    /// Takes the theme's `index`-th series tone
    /// ([`Theme::series_color`](crate::theme::Theme::series_color)) instead of the tone of the
    /// series' position. Give a category the same index everywhere — in every chart and in the
    /// [`Legend`](super::Legend) beside it, through [`Legend::tones`](super::Legend::tones) — and
    /// it keeps its colour whichever other categories are shown.
    #[must_use]
    pub fn tone(mut self, index: usize) -> Self {
        self.tone = Some(index);
        self
    }

    /// The value of category `category`, zero when the series is shorter than that.
    fn value(&self, category: usize) -> f32 {
        self.values.get(category).copied().unwrap_or(0.0)
    }
}

/// Bars measured in eighths of a cell, with labels and values.
///
/// Horizontal by default: one row per bar, labels on the left, values on the right. Vertical
/// charts stand bars side by side with the value above and the label below each. Bars scale to
/// the largest value unless a maximum is given. On narrow areas horizontal charts drop their
/// labels first; vertical bars get thinner and labels are cut with `…`. ASCII mode fills whole
/// cells.
///
/// A chart of [`Series`] shows several values per category: side by side by default, or as
/// segments of one bar with [`stacked`](Self::stacked). Series take their tones from the theme,
/// walking a ramp from the accent towards the faint end so neighbouring shares read apart
/// without a second accent colour; a horizontal segment writes its series name inside itself
/// when the name fits, so a stack is not read by colour alone.
///
/// The chart answers the pointer and the keyboard once it is given
/// [`on_select`](Self::on_select): the hovered and the selected category rise on a raised ground,
/// a horizontal chart marks the selected row with the accent pillar in its own lead cell, and the
/// arrow keys along the bars' axis (↑/↓ or k/j horizontally, ←/→ or h/l vertically) with Home and
/// End move the selection. Nothing moves or resizes when a category is hovered or selected: a
/// chart is not a list, so it never slides.
///
/// Style keys: `bar-chart` and `bar-chart.<variant>` (`fill`), `bar-chart-bar` with `hover`,
/// `selected` and `focus` (`bg`, `pillar`), `bar-chart-label` (`fg`), `bar-chart-value` and
/// `bar-chart-value.<variant>` (`fg`, `bold`). Series take the theme's `series-<n>` colour
/// tokens when it has them.
pub struct BarChart<Msg> {
    bars: Vec<Bar>,
    series: Vec<Series>,
    stacked: bool,
    vertical: bool,
    max: Option<f32>,
    gap: u16,
    unit: Option<String>,
    selected: Option<usize>,
    disabled: bool,
    on_select: Option<IndexMessage<Msg>>,
}

impl<Msg> BarChart<Msg> {
    /// A horizontal chart of `bars`.
    #[must_use]
    pub fn new(bars: impl IntoIterator<Item = Bar>) -> Self {
        Self {
            bars: bars.into_iter().collect(),
            series: Vec::new(),
            stacked: false,
            vertical: false,
            max: None,
            gap: 1,
            unit: None,
            selected: None,
            disabled: false,
            on_select: None,
        }
    }

    /// A chart of the categories `labels` with one value per category in every series of
    /// `series`.
    ///
    /// The series of a category stand next to each other; [`stacked`](Self::stacked) puts them in
    /// one bar instead. A chart of a single series looks exactly like a chart of plain bars.
    #[must_use]
    pub fn series(
        labels: impl IntoIterator<Item = impl Into<String>>,
        series: impl IntoIterator<Item = Series>,
    ) -> Self {
        let mut chart = Self::new(labels.into_iter().map(Bar::category));
        chart.series = series.into_iter().collect();
        chart
    }

    /// Draws the series as segments of one bar per category instead of bars next to each other.
    #[must_use]
    pub fn stacked(mut self) -> Self {
        self.stacked = true;
        self
    }

    /// Stands the bars up side by side.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// The value of a full bar, e.g. `100.0` for percentages; the largest value by default. A
    /// stacked chart scales to the largest category total instead.
    #[must_use]
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Empty rows (horizontal) or columns (vertical) between bars; 1 by default, so neighbouring
    /// bars never merge into one block. Vertical charts keep at least one column. The bars of one
    /// category stand right next to each other, so a group reads as one shape.
    #[must_use]
    pub fn gap(mut self, cells: u16) -> Self {
        self.gap = cells;
        self
    }

    /// Unit written after every value the chart itself formats, e.g. `"h"` for hours. A bar with
    /// its own [`value_text`](Bar::value_text) keeps that text as it is.
    #[must_use]
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// The selected category, which rises on a raised ground.
    #[must_use]
    pub fn selected(mut self, category: Option<usize>) -> Self {
        self.selected = category;
        self
    }

    /// Greys the chart out: it cannot be focused, hovered or selected.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for moving the selection to a category, which turns the chart's pointer and
    /// keyboard handling on.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// How many categories the chart shows.
    fn categories(&self) -> usize {
        self.bars.len()
    }

    /// How many bars one category shows: one per series, or one when the series are stacked or
    /// the chart has plain bars.
    fn bars_per_category(&self) -> u16 {
        if self.stacked || self.series.len() < 2 {
            1
        } else {
            clamp_u16(i32::try_from(self.series.len()).unwrap_or(i32::MAX))
        }
    }

    /// The value of one series in one category, or the bar's own value in a chart of plain bars.
    fn value(&self, category: usize, series: usize) -> f32 {
        match self.series.get(series) {
            Some(series) => series.value(category),
            None => self.bars.get(category).map_or(0.0, |bar| bar.value),
        }
    }

    /// Every value of a category, one per series.
    fn values(&self, category: usize) -> Vec<f32> {
        if self.series.is_empty() {
            vec![self.value(category, 0)]
        } else {
            (0..self.series.len()).map(|series| self.value(category, series)).collect()
        }
    }

    /// The sum of a category's series.
    fn total(&self, category: usize) -> f32 {
        self.values(category).iter().sum()
    }

    /// The value a full bar stands for: the given maximum, else the largest value a bar can
    /// reach, which for stacked series is the largest category total.
    fn scale(&self) -> f32 {
        let largest = if self.stacked && !self.series.is_empty() {
            (0..self.categories()).map(|category| self.total(category)).fold(0.0, f32::max)
        } else {
            (0..self.categories()).flat_map(|category| self.values(category)).fold(0.0, f32::max)
        };
        let max = self.max.unwrap_or(largest);
        if max > 0.0 { max } else { 1.0 }
    }

    fn vertical_gap(&self) -> u16 {
        self.gap.max(1)
    }

    fn count(&self) -> u16 {
        clamp_u16(i32::try_from(self.categories()).unwrap_or(i32::MAX))
    }

    /// Whether the chart answers the pointer and the keyboard.
    fn interactive(&self) -> bool {
        self.on_select.is_some() && !self.disabled && self.categories() > 0
    }

    /// How far a bar of `value` reaches across `cells`, in eighths of a cell. A value that is
    /// there at all reaches at least one eighth, so a share next to a much larger one is seen
    /// rather than read as nothing.
    fn reached(&self, value: f32, cells: u16) -> u32 {
        let eighths = super::eighths::eighths(value / self.scale(), cells);
        if eighths == 0 && value > 0.0 { 1 } else { eighths }
    }

    /// `value` as text, with the chart's unit when it has one.
    fn format(&self, value: f32) -> String {
        let number = if value.fract() == 0.0 { format!("{value:.0}") } else { format!("{value:.1}") };
        match &self.unit {
            Some(unit) => format!("{number} {unit}"),
            None => number,
        }
    }

    /// The value text of a plain bar: its own text or the formatted number, after the marker of a
    /// bar that carries a variant.
    fn bar_value(&self, cx: &PaintCx<'_>, bar: &Bar) -> String {
        let value = bar.value_text.clone().unwrap_or_else(|| self.format(bar.value));
        match &bar.variant {
            Some(_) => format!("{} {}", cx.env().icons().glyph("dot"), value),
            None => value,
        }
    }

    /// The tone of one series, or of a plain bar when `series` is `None`.
    fn fill(&self, cx: &mut PaintCx<'_>, bar: &Bar, series: Option<usize>) -> Rgb {
        if let Some(index) = series.filter(|_| !self.series.is_empty()) {
            return series_fill(cx.env().theme(), self.tone_index(index), self.disabled);
        }
        if self.disabled {
            return series_fill(cx.env().theme(), 0, true);
        }
        cx.style("bar-chart", bar.variant.as_deref(), &[]).color("fill").unwrap_or_else(|| cx.color("accent"))
    }

    /// The palette index of the series at `position`: the tone it was pinned to, else its
    /// position.
    fn tone_index(&self, position: usize) -> usize {
        self.series.get(position).and_then(|series| series.tone).unwrap_or(position)
    }

    /// The style of a bar's value text, faint while the chart is disabled.
    fn value_style(&self, cx: &mut PaintCx<'_>, bar: &Bar) -> CellStyle {
        if self.disabled {
            return CellStyle::fg(cx.color("muted"));
        }
        let mut style = cx.style("bar-chart-value", bar.variant.as_deref(), &[]).text();
        style.bg = None;
        style
    }

    /// The style of the labels, faint while the chart is disabled.
    fn label_style(&self, cx: &mut PaintCx<'_>) -> CellStyle {
        if self.disabled {
            return CellStyle::fg(cx.color("muted"));
        }
        let mut style = cx.style("bar-chart-label", None, &[]).text();
        style.bg = None;
        style
    }

    /// The states of one category: hovered under the pointer, selected, and focused with it. A
    /// disabled chart is in no state at all: it answers neither the pointer nor the keyboard.
    fn states(&self, hovered: bool, category: usize, focused: bool) -> Vec<State> {
        let mut states = Vec::new();
        if self.disabled {
            return states;
        }
        if hovered {
            states.push(State::Hover);
        }
        if self.selected == Some(category) {
            states.push(State::Selected);
            if focused {
                states.push(State::Focus);
            }
        }
        states
    }

    /// Raises the ground of a touched category and, in a horizontal chart, stands the pillar in
    /// its lead cell. Nothing moves: the ground is drawn behind the same cells the resting
    /// category uses.
    fn paint_ground(&self, cx: &mut PaintCx<'_>, rect: Rect, states: &[State]) {
        if states.is_empty() || rect.is_empty() {
            return;
        }
        let style = cx.style("bar-chart-bar", None, states);
        let selected = states.contains(&State::Selected);
        let ground = style.text().bg.unwrap_or_else(|| cx.color(if selected { "active" } else { "raised" }));
        cx.fill(rect, ground);
        if self.vertical {
            return;
        }
        let pillar = style.color("pillar").unwrap_or_else(|| {
            let accent = cx.color("accent");
            if selected { accent } else { accent.mix(cx.color("active"), 0.45) }
        });
        cx.pillar(rect.x, rect.y, pillar);
    }

    fn select(&self, cx: &mut EventCx<'_, Msg>, category: usize) {
        if let Some(message) = &self.on_select
            && self.selected != Some(category)
        {
            cx.emit(message(category));
        }
    }

    /// Whether the chart keeps room for the pillar of a marked category. A chart given a
    /// selection keeps it even without `on_select`, so the lead cells are there before anything
    /// is hovered and nothing moves when something is.
    fn marks_selection(&self) -> bool {
        !self.disabled && (self.on_select.is_some() || self.selected.is_some())
    }

    /// The category a plain key moves to, along the axis the bars run along.
    fn key_target(&self, key: &KeyEvent) -> Option<usize> {
        let last = self.categories().checked_sub(1)?;
        let (back, forward) = if self.vertical {
            ([Key::Left, Key::Char('h')], [Key::Right, Key::Char('l')])
        } else {
            ([Key::Up, Key::Char('k')], [Key::Down, Key::Char('j')])
        };
        if back.iter().any(|k| key.is_plain(*k)) {
            return Some(self.selected.map_or(last, |current| current.saturating_sub(1)));
        }
        if forward.iter().any(|k| key.is_plain(*k)) {
            return Some(self.selected.map_or(0, |current| (current + 1).min(last)));
        }
        if key.is_plain(Key::Home) {
            return Some(0);
        }
        if key.is_plain(Key::End) {
            return Some(last);
        }
        None
    }
}

/// A theme colour token, black when the theme has no such token, as everywhere else.
fn token(theme: &Theme, name: &str) -> Rgb {
    theme.color(name).unwrap_or(Rgb::new(0, 0, 0))
}

/// The tone of series `index`.
///
/// The theme decides first, through its `series-<n>` colour tokens. Without them the tone walks
/// a ramp from the accent towards the faint end of the theme, which keeps a chart inside the
/// theme's one accent; the ramp repeats after [`SERIES_TONES`] series, so a legend carries the
/// meaning of a chart with more series than that. A disabled chart walks a quiet ramp instead.
fn series_fill(theme: &Theme, index: usize, disabled: bool) -> Rgb {
    // The same call a `Legend` makes for its n-th name, so a series keeps one tone across the
    // charts and legends of a page, wrapping after the theme's last series colour.
    if !disabled {
        return theme.series_color(index);
    }
    // A disabled chart keeps its series apart without colour: a ramp from muted to dim, whose
    // last step reaches the far end so the tones stay apart in a 256-colour terminal.
    let step = (index % SERIES_TONES) as f32 / (SERIES_TONES - 1) as f32;
    token(theme, "muted").mix(token(theme, "dim"), step)
}

impl<Msg: 'static> Widget<Msg> for BarChart<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let count = self.count();
        if count == 0 {
            return Size::default();
        }
        let size = if self.vertical {
            Size::new(available.width, 8)
        } else {
            let bars = count.saturating_mul(self.bars_per_category());
            Size::new(available.width, bars.saturating_add(self.gap.saturating_mul(count - 1)))
        };
        size.min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() || self.bars.is_empty() {
            return;
        }
        if self.interactive() {
            cx.register_hit(area);
        }
        if self.vertical {
            self.paint_vertical(cx, area);
        } else {
            self.paint_horizontal(cx, area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.interactive() {
            return false;
        }
        let area = cx.area();
        match event {
            Event::Key(key) => {
                let Some(target) = self.key_target(key) else { return false };
                self.select(cx, target);
                true
            }
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                let Some(category) = self.category_at(area, mouse.x, mouse.y) else { return false };
                self.select(cx, category);
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.interactive()
    }
}
