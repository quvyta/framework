//! Gauges: how full something is, with limits that change its tone.

use super::eighths;
use crate::color::Rgb;
use crate::geometry::{Rect, Size};
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Narrowest meter drawn, in cells; below this the gauge shows only its label and value.
const MIN_METER: u16 = 4;

/// How a value stands against the gauge's thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Plain,
    Success,
    Warning,
    Danger,
}

impl Level {
    fn variant(self) -> Option<&'static str> {
        match self {
            Self::Plain => None,
            Self::Success => Some("success"),
            Self::Warning => Some("warning"),
            Self::Danger => Some("danger"),
        }
    }

    fn icon(self) -> Option<&'static str> {
        match self {
            Self::Plain => None,
            Self::Success => Some("dot"),
            Self::Warning => Some("warning"),
            Self::Danger => Some("error"),
        }
    }
}

/// A one-row meter for a value in a range: label, meter and value.
///
/// The meter fills in eighths of a cell (whole cells in ASCII mode). With thresholds the fill takes
/// the success, warning or danger tone by value, the parts of the track past each threshold are
/// tinted faintly so the limits are visible before they are reached, and the value carries a
/// marker so the state reads without colour. The value shows a percentage of the range unless a
/// text is given. When the area is too narrow for a meter, only the label and value remain.
///
/// Style keys: `gauge` and `gauge.<success|warning|danger>` (`track`, `fill`, `zone` for the
/// tint past a threshold), `gauge-label` (`fg`), `gauge-value` and `gauge-value.<level>` (`fg`,
/// `bold`).
#[derive(Debug, Clone, PartialEq)]
pub struct Gauge {
    value: f32,
    min: f32,
    max: f32,
    label: Option<String>,
    label_width: Option<u16>,
    value_text: Option<String>,
    thresholds: Option<(f32, f32)>,
}

impl Gauge {
    /// A gauge at `value` in the range 0 to 100.
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self { value, min: 0.0, max: 100.0, label: None, label_width: None, value_text: None, thresholds: None }
    }

    /// The range the value lives in, e.g. `0.0, 64.0` for gibibytes of memory.
    #[must_use]
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// A name drawn before the meter, e.g. "Memory".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Reserves `cells` for the label, so gauges stacked with different labels line up. Longer
    /// labels are cut with `…`.
    #[must_use]
    pub fn label_width(mut self, cells: u16) -> Self {
        self.label_width = Some(cells);
        self
    }

    /// Text drawn after the meter instead of the percentage, e.g. "6.2 of 8 GiB".
    #[must_use]
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.value_text = Some(text.into());
        self
    }

    /// From `warning` the gauge turns to the warning tone, from `danger` to the danger tone;
    /// below both it is in the success tone. Values are in the gauge's range.
    #[must_use]
    pub fn thresholds(mut self, warning: f32, danger: f32) -> Self {
        self.thresholds = Some((warning, danger));
        self
    }

    fn fraction(&self, value: f32) -> f32 {
        let span = self.max - self.min;
        if span > 0.0 { ((value - self.min) / span).clamp(0.0, 1.0) } else { 0.0 }
    }

    fn level(&self) -> Level {
        match self.thresholds {
            None => Level::Plain,
            Some((_, danger)) if self.value >= danger => Level::Danger,
            Some((warning, _)) if self.value >= warning => Level::Warning,
            Some(_) => Level::Success,
        }
    }

    fn shown_value(&self) -> String {
        self.value_text.clone().unwrap_or_else(|| format!("{:.0}%", self.fraction(self.value) * 100.0))
    }
}

impl<Msg: 'static> Widget<Msg> for Gauge {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        // A gauge takes the whole width it is given; narrow ones keep only the label and value.
        Size::new(available.width, 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let level = self.level();
        let variant = level.variant();
        let y = area.y;

        let value = self.shown_value();
        let marker = level.icon().map(|icon| cx.env().icons().glyph(icon).into_owned());
        let marker_width = marker.as_deref().map_or(0, |glyph| text::width(glyph).saturating_add(1));
        let value_width = (marker_width + text::width(&value)).min(area.width);
        let label = self.label.as_deref().unwrap_or_default();
        let label_budget = area.width.saturating_sub(value_width + 2).min(self.label_width.unwrap_or(u16::MAX));
        let label_shown = text::truncate(label, label_budget).into_owned();
        let label_width = match (label.is_empty(), self.label_width) {
            (true, _) => 0,
            (false, Some(_)) => label_budget + 2,
            (false, None) => text::width(&label_shown).saturating_add(2),
        };

        let mut label_style = cx.style("gauge-label", None, &[]).text();
        label_style.bg = None;
        cx.text(area.x, y, &label_shown, label_style, label_budget);

        let meter_start = area.x + i32::from(label_width);
        let meter_width = area.width.saturating_sub(label_width + value_width + 2);
        if meter_width >= MIN_METER {
            self.paint_meter(cx, Rect::new(meter_start, y, meter_width, 1), variant);
        }

        let mut value_style = cx.style("gauge-value", variant, &[]).text();
        value_style.bg = None;
        let mut x = area.right() - i32::from(value_width);
        if let Some(glyph) = marker {
            x += i32::from(cx.text(x, y, &glyph, value_style, value_width)) + 1;
        }
        let budget = crate::geometry::clamp_u16(area.right() - x);
        let value_shown = text::truncate(&value, budget).into_owned();
        cx.text(x, y, &value_shown, value_style, budget);
    }
}

impl Gauge {
    fn paint_meter(&self, cx: &mut PaintCx<'_>, meter: Rect, variant: Option<&str>) {
        let style = cx.style("gauge", variant, &[]);
        let track = style.color("track").unwrap_or_else(|| cx.color("raised"));
        let fill = style.color("fill").unwrap_or_else(|| cx.color("accent"));
        cx.clear(meter, track);
        if let Some((warning, danger)) = self.thresholds {
            // Zones past each limit are tinted with the tone the gauge will take there.
            for (limit, zone_variant) in [(warning, "warning"), (danger, "danger")] {
                let zone: Rgb = cx.style("gauge", Some(zone_variant), &[]).color("zone").unwrap_or(track);
                let start = eighths::eighths(self.fraction(limit), meter.width) / 8;
                let start = u16::try_from(start).unwrap_or(meter.width).min(meter.width);
                cx.clear(Rect::new(meter.x + i32::from(start), meter.y, meter.width - start, 1), zone);
            }
        }
        let filled = eighths::eighths(self.fraction(self.value), meter.width);
        eighths::horizontal(cx, meter, filled, fill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(Gauge);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(self.0.clone()).fill_width();
        }
    }

    #[test]
    fn label_meter_and_percentage() {
        let h = Harness::new(Demo(Gauge::new(53.0).label("CPU")), 20, 1);
        assert_eq!(h.screen(), "CPU       ▎      53%\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(5, 0), theme.color("accent"));
        assert_eq!(h.bg(10, 0), theme.color("raised"));
    }

    #[test]
    fn thresholds_choose_tone_and_marker() {
        let gauge = Gauge::new(6.9).range(0.0, 8.0).label("Memory").value_text("6.9 GiB").thresholds(6.0, 7.5);
        let h = Harness::new(Demo(gauge), 30, 1);
        assert_eq!(h.screen(), "Memory           ▌   ▲ 6.9 GiB\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(8, 0), theme.color("warning"));
        assert_eq!(h.fg(21, 0), theme.color("warning"));
        assert_ne!(h.bg(19, 0), h.bg(17, 0), "the danger zone has a tint of its own");
        let calm = Harness::new(Demo(Gauge::new(20.0).thresholds(70.0, 90.0)), 12, 1);
        assert_eq!(calm.screen(), "       ● 20%\n");
        assert_eq!(calm.bg(0, 0), theme.color("success"));
        assert_eq!(calm.fg(7, 0), theme.color("success"));
    }

    #[test]
    fn label_width_lines_meters_up() {
        let h = Harness::new(Demo(Gauge::new(50.0).label("CPU").label_width(6)), 20, 1);
        assert_eq!(h.screen(), "CPU        ▌     50%\n");
        let cut = Harness::new(Demo(Gauge::new(50.0).label("Memory pressure").label_width(6)), 20, 1);
        assert_eq!(cut.screen(), "Memor…     ▌     50%\n");
    }

    #[test]
    fn narrow_gauges_keep_label_and_value() {
        let mut h = Harness::new(Demo(Gauge::new(97.0).label("Disk").thresholds(80.0, 95.0)), 12, 1);
        assert_eq!(h.screen(), "Disk   ✕ 97%\n");
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "Disk   x 97%\n");
    }

    #[test]
    fn a_huge_label_width_is_bounded_by_the_area() {
        let h = Harness::new(Demo(Gauge::new(50.0).label("CPU").label_width(u16::MAX)), 20, 1);
        assert_eq!(h.screen(), "CPU              50%\n");
    }
}
