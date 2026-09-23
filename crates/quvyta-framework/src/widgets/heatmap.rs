//! Heatmaps: a grid of days, each one a tone for how much happened that day.

use crate::color::{ColorDepth, Rgb};
use crate::event::Event;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::press::{self, Press};

/// Rows of a heatmap unless the caller asks for another number: a week of days.
const DEFAULT_ROWS: u16 = 7;

/// How many tones a value can take besides the empty one.
///
/// Four steps is what a reader can name — quiet, some, busy, busiest — and what a low-colour
/// terminal can still show something of. A continuous ramp would look precise and read as noise.
const LEVELS: u32 = 4;

/// How far each level mixes from the empty tone towards the full one. The first step starts well
/// clear of the empty tone, so a single quiet day is never mistaken for nothing at all.
const MIX: [f32; LEVELS as usize] = [0.30, 0.53, 0.76, 1.0];

/// Smallest colour difference a reader notices, from [`Rgb::perceptual_distance`].
const VISIBLE: f64 = 0.03;

/// Builds a message from the index of a chosen value.
type SelectMessage<Msg> = Box<dyn Fn(usize) -> Msg>;

/// What the runtime remembers about a heatmap between frames.
#[derive(Debug, Default)]
struct HeatmapMemory {
    /// Where the keyboard left the cursor, as an index into the values.
    cursor: Option<usize>,
}

/// A grid of days as tones: one cell per value, a column per week, the tone carrying how much
/// that day holds.
///
/// Values run oldest first and fill the grid column by column, so a column is a week and a row
/// is a weekday; the newest column is on the right. `starts_at` leaves the first cells of the
/// first column blank, for a year that does not begin on the first weekday: a blank cell is a
/// day outside the range and shows nothing at all, while a day inside it that holds nothing
/// takes the empty tone. The two never look the same.
///
/// The tone is a step, not a gradient: a value at or below zero takes the empty tone, and any
/// value above it takes one of four steps towards the full tone, the topmost step reserved for
/// the largest value (or for `max`, when the scale is fixed). With `series` the full tone is one
/// of the theme's series tones instead of the accent, so several heatmaps beside each other read
/// as different categories — name them with a [`Legend`](super::Legend).
///
/// Nothing is drawn with characters, so the grid looks the same in every glyph mode. Where the
/// terminal cannot tell two steps apart (a 16-colour terminal), the steps that would collapse
/// are dropped and the remaining tones are spread over the four levels: fewer steps, but never
/// two different levels in the same tone.
///
/// A heatmap is a picture and stays passive until `on_select` is given. With it, the cell under
/// the pointer and the cell the keyboard cursor is on light up, a click or Enter reports that
/// cell's index, and the caller writes its value as text — a single cell's number cannot be read
/// out of a tone, and the runtime does not tell widgets about pointer movement, so lighting a
/// cell is what hovering can do and reading the number needs the one press that the keyboard
/// makes with Enter.
///
/// Narrow areas keep the newest columns and drop the oldest whole columns, so the grid never
/// shows a half week; [`Heatmap::columns`] says how many are left, for a caller that wants to
/// write "the last 12 weeks". An area shorter than the grid keeps the rows that fit from the
/// top. An area with no room, or a heatmap with no values, draws nothing and measures nothing,
/// which leaves the caller room for an empty state.
///
/// Style keys: `heatmap` (`empty` for a day that holds nothing, `fill` for the full tone,
/// `cursor` for the tone the lit cell mixes towards) and `heatmap:focus` (`cursor` while the
/// keyboard moved the cursor last).
pub struct Heatmap<Msg> {
    values: Vec<f32>,
    rows: u16,
    max: Option<f32>,
    starts_at: u16,
    series: Option<usize>,
    selected: Option<usize>,
    on_select: Option<SelectMessage<Msg>>,
}

impl<Msg: 'static> Heatmap<Msg> {
    /// A heatmap of `values`, oldest first, in a grid seven rows tall.
    #[must_use]
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        Self {
            values: values.into_iter().collect(),
            rows: DEFAULT_ROWS,
            max: None,
            starts_at: 0,
            series: None,
            selected: None,
            on_select: None,
        }
    }

    /// Rows of the grid; seven by default, one per weekday. At least one.
    #[must_use]
    pub fn rows(mut self, rows: u16) -> Self {
        self.rows = rows.max(1);
        self
    }

    /// The value the topmost step stands for, e.g. a daily goal; the largest value by default.
    #[must_use]
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Blank cells before the first value, for a range that does not start on the first row of a
    /// column. Kept within one column.
    #[must_use]
    pub fn starts_at(mut self, row: u16) -> Self {
        self.starts_at = row % self.rows;
        self
    }

    /// Builds the tones from the theme's `index`-th series tone instead of the accent, for one
    /// category among several.
    #[must_use]
    pub fn series(mut self, index: usize) -> Self {
        self.series = Some(index);
        self
    }

    /// The value the cursor rests on, which the caller keeps as it hears `on_select`.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Message for a cell chosen with a click or with Enter, carrying its index into the values.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// How many columns the grid has in `width` cells: every week that fits, newest kept.
    #[must_use]
    pub fn columns(&self, width: u16) -> u16 {
        self.total_columns().min(width)
    }

    /// Cells the grid holds, blank leading cells included.
    fn cell_count(&self) -> usize {
        usize::from(self.starts_at).saturating_add(self.values.len())
    }

    /// Columns the whole grid needs.
    fn total_columns(&self) -> u16 {
        let rows = usize::from(self.rows);
        let columns = self.cell_count().div_ceil(rows);
        clamp_u16(i32::try_from(columns).unwrap_or(i32::MAX))
    }

    /// The first column shown in `width` cells; earlier columns are dropped.
    fn first_column(&self, width: u16) -> u16 {
        self.total_columns().saturating_sub(self.columns(width))
    }

    /// The value the topmost step stands for; never zero, so a grid of zeroes stays empty
    /// instead of lighting up.
    fn scale(&self) -> f32 {
        let largest = self.values.iter().copied().fold(0.0, f32::max);
        let max = self.max.unwrap_or(largest);
        if max > 0.0 { max } else { 1.0 }
    }

    /// The step `value` takes: zero for nothing, else 1 to [`LEVELS`], the top step only for a
    /// value at or above the scale.
    fn level(&self, value: f32) -> u32 {
        if value <= 0.0 {
            return 0;
        }
        // The share is small and positive, so the product fits f32 and the ceiling fits u32.
        let share = (value / self.scale()).clamp(0.0, 1.0);
        ((share * LEVELS as f32).ceil() as u32).clamp(1, LEVELS)
    }

    /// Where the value at `index` sits on screen, or `None` when its column or row is cut.
    fn cell_rect(&self, area: Rect, index: usize) -> Option<Rect> {
        let place = usize::from(self.starts_at).checked_add(index)?;
        let rows = usize::from(self.rows);
        let column = u16::try_from(place / rows).ok()?;
        let row = u16::try_from(place % rows).ok()?;
        let first = self.first_column(area.width);
        if column < first || row >= area.height {
            return None;
        }
        Some(Rect::new(area.x + i32::from(column - first), area.y + i32::from(row), 1, 1))
    }

    /// The value the cell at `x`, `y` holds, or `None` outside the grid or on a blank cell.
    fn value_at(&self, area: Rect, x: i32, y: i32) -> Option<usize> {
        if !area.contains(x, y) {
            return None;
        }
        let column = u16::try_from(x - area.x).ok()?.checked_add(self.first_column(area.width))?;
        let row = u16::try_from(y - area.y).ok()?;
        let place = usize::from(column).checked_mul(usize::from(self.rows))?.checked_add(usize::from(row))?;
        let index = place.checked_sub(usize::from(self.starts_at))?;
        (index < self.values.len()).then_some(index)
    }

    /// The empty tone and the full one.
    fn tones(&self, cx: &mut PaintCx<'_>) -> (Rgb, Rgb) {
        let style = cx.style("heatmap", None, &[]);
        let empty = style.color("empty").unwrap_or_else(|| cx.color("raised"));
        let full = style.color("fill").unwrap_or_else(|| match self.series {
            Some(index) => cx.env().theme().series_color(index),
            None => cx.color("accent"),
        });
        (empty, full)
    }

    /// The cell the keyboard cursor is on, and the value the pointer is over.
    fn lit(&self, cx: &mut PaintCx<'_>, area: Rect) -> (Option<usize>, Option<usize>) {
        if self.on_select.is_none() {
            return (None, None);
        }
        let pointed = cx.pointer_within().and_then(|(x, y)| self.value_at(area, x, y));
        let focused = cx.is_focus_visible();
        let cursor = cx.memory::<HeatmapMemory>().cursor.or(self.selected).filter(|_| focused);
        (cursor, pointed)
    }

    /// Moves the keyboard cursor `step` cells along the values, clamped to their ends, and says
    /// that the key was used.
    fn move_cursor(&self, cx: &mut EventCx<'_, Msg>, step: i32) -> bool {
        let last = self.values.len().saturating_sub(1);
        let from = cx.memory::<HeatmapMemory>().cursor.or(self.selected).unwrap_or(last);
        let target = i32::try_from(from).unwrap_or(0).saturating_add(step);
        let target = usize::try_from(target.max(0)).unwrap_or(0).min(last);
        cx.memory::<HeatmapMemory>().cursor = Some(target);
        true
    }

    /// Reports the cell the cursor is on, which a press or Enter chooses.
    fn choose(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        cx.memory::<HeatmapMemory>().cursor = Some(index);
        cx.flash();
        if let Some(message) = &self.on_select {
            cx.emit(message(index));
        }
    }
}

/// The tones from the quietest step to the full one that `depth` can tell apart on a screen whose
/// ground is `ground`.
///
/// A step the terminal would show in the tone of the step below it is left out, so no two levels
/// share a tone; the levels are then spread over the tones that are left. A terminal that shows
/// the whole ramp as one colour still gets the full tone, because a day that holds something must
/// never look like a day that holds nothing.
fn ramp(empty: Rgb, full: Rgb, depth: ColorDepth, ground: Rgb) -> Vec<Rgb> {
    let mut tones = Vec::with_capacity(LEVELS as usize);
    let mut previous = empty;
    for mix in MIX {
        let tone = empty.mix(full, mix);
        if depth.tells_apart(tone, previous, ground) {
            tones.push(tone);
            previous = tone;
        }
    }
    if tones.is_empty() {
        tones.push(full);
    }
    tones
}

/// `tone` stepped away from itself so the cell under the cursor stands out: `amount` of the way
/// towards `towards`, or, when that changes nothing because the tone is already there (a theme
/// whose accent is its text colour, drawn on the busiest day), the same step back towards
/// `empty`. Either way the cell moves by a step a reader can see, and the value it carries is
/// still the tone it is a step away from.
fn lift(tone: Rgb, towards: Rgb, empty: Rgb, amount: f32) -> Rgb {
    let lifted = tone.mix(towards, amount);
    if lifted.perceptual_distance(tone) < VISIBLE { tone.mix(empty, amount) } else { lifted }
}

/// The tone of `level` (1 to [`LEVELS`]) in `ramp`: the quietest level takes the first tone, the
/// top level the last, and the levels between are spread over what is left.
fn tone_of(ramp: &[Rgb], level: u32) -> Option<Rgb> {
    let steps = u32::try_from(ramp.len()).unwrap_or(1).saturating_sub(1);
    let last = LEVELS - 1;
    // Rounded so the levels in the middle do not all fall to the quieter tone.
    let index = usize::try_from((level.saturating_sub(1) * steps + last / 2) / last).unwrap_or(0);
    ramp.get(index).copied()
}

impl<Msg: 'static> Widget<Msg> for Heatmap<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.values.is_empty() {
            return Size::default();
        }
        Size::new(self.total_columns(), self.rows).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() || self.values.is_empty() {
            return;
        }
        let (empty, full) = self.tones(cx);
        let steps = ramp(empty, full, cx.env().depth(), cx.color("canvas"));
        let (cursor, pointed) = self.lit(cx, area);
        let style = cx.style("heatmap", None, &[]);
        let pointer_lift = style.color("cursor").unwrap_or_else(|| cx.color("text"));
        let keyboard_lift = cx.style("heatmap", None, &[State::Focus]).color("cursor").unwrap_or(pointer_lift);
        let tone = |level: u32| match level {
            0 => empty,
            level => tone_of(&steps, level).unwrap_or(empty),
        };
        for (index, value) in self.values.iter().copied().enumerate() {
            let Some(rect) = self.cell_rect(area, index) else {
                continue;
            };
            let plain = tone(self.level(value));
            let color = if pointed == Some(index) {
                lift(plain, pointer_lift, empty, 0.35)
            } else if pointed.is_none() && cursor == Some(index) {
                lift(plain, keyboard_lift, empty, 0.5)
            } else {
                plain
            };
            cx.fill(rect, color);
        }
        if self.on_select.is_some() {
            cx.register_hit(area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.on_select.is_none() || self.values.is_empty() {
            return false;
        }
        let area = cx.area();
        let rows = i32::from(self.rows);
        if let Event::Key(key) = event {
            if key.is_plain(Key::Left) {
                return self.move_cursor(cx, -rows);
            } else if key.is_plain(Key::Right) {
                return self.move_cursor(cx, rows);
            } else if key.is_plain(Key::Up) {
                return self.move_cursor(cx, -1);
            } else if key.is_plain(Key::Down) {
                return self.move_cursor(cx, 1);
            } else if key.is_plain(Key::Home) {
                return self.move_cursor(cx, i32::MIN);
            } else if key.is_plain(Key::End) {
                return self.move_cursor(cx, i32::MAX);
            }
        }
        match press::read(cx, event) {
            Press::Ignored => false,
            Press::Used => true,
            Press::Key => {
                let last = self.values.len() - 1;
                let index = cx.memory::<HeatmapMemory>().cursor.or(self.selected).unwrap_or(last);
                self.choose(cx, index.min(last));
                true
            }
            Press::Click(x, y) => {
                if let Some(index) = self.value_at(area, x, y) {
                    self.choose(cx, index);
                }
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.on_select.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    #[derive(Default)]
    struct Demo {
        values: Vec<f32>,
        rows: u16,
        height: Option<u16>,
        starts_at: u16,
        series: Option<usize>,
        interactive: bool,
        chosen: Option<usize>,
    }

    impl Demo {
        fn new(values: impl IntoIterator<Item = f32>) -> Self {
            Self { values: values.into_iter().collect(), rows: 7, ..Self::default() }
        }
    }

    impl App for Demo {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.chosen = Some(index);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            let mut map: Heatmap<usize> =
                Heatmap::new(self.values.iter().copied()).rows(self.rows).starts_at(self.starts_at);
            if let Some(index) = self.series {
                map = map.series(index);
            }
            if self.interactive {
                map = map.selected(self.chosen).on_select(|index| index);
            }
            let height = self.height.map_or(Length::Fill(1), Length::Cells);
            ui.add(map).width(Length::Fill(1)).height(height).id("map");
        }
    }

    /// The tones the built-in theme gives a heatmap: the empty one and the four steps.
    fn tones(h: &Harness<Demo>) -> (Rgb, [Rgb; 4]) {
        let theme = h.env().theme();
        let empty = theme.color("raised").expect("token");
        let full = theme.color("accent").expect("token");
        (empty, [0, 1, 2, 3].map(|i| empty.mix(full, MIX[i])))
    }

    /// The colour of a cell the heatmap did not draw: the screen's own ground.
    fn blank(h: &Harness<Demo>) -> Option<Rgb> {
        h.env().theme().color("canvas")
    }

    #[test]
    fn a_week_of_days_fills_a_column_and_the_newest_column_is_last() {
        let h = Harness::new(Demo::new([0.0, 1.0, 2.0, 3.0, 4.0, 0.0, 0.0, 4.0]), 4, 7);
        let (empty, steps) = tones(&h);
        assert_eq!(h.screen(), "\n\n\n\n\n\n\n", "a heatmap is made of colour, not characters");
        assert_eq!(h.bg(0, 0), Some(empty), "a day with nothing takes the empty tone");
        assert_eq!(h.bg(0, 1), Some(steps[0]), "the quietest day takes the first step");
        assert_eq!(h.bg(0, 4), Some(steps[3]), "the busiest day takes the top step");
        assert_eq!(h.bg(1, 0), Some(steps[3]), "the eighth value starts the next column");
        assert_eq!(h.bg(2, 0), blank(&h), "nothing is drawn past the last column");
    }

    #[test]
    fn days_outside_the_range_stay_blank_while_empty_days_take_a_tone() {
        let mut demo = Demo::new([5.0, 5.0]);
        demo.starts_at = 3;
        let h = Harness::new(demo, 2, 7);
        let (empty, steps) = tones(&h);
        for row in 0..3 {
            assert_eq!(h.bg(0, row), blank(&h), "row {row} is before the first value");
        }
        assert_eq!(h.bg(0, 3), Some(steps[3]));
        assert_eq!(h.bg(0, 4), Some(steps[3]));
        assert_eq!(h.bg(0, 5), blank(&h), "nothing follows the last value");
        assert_ne!(blank(&h), Some(empty), "a day outside the range is not an empty day");
    }

    #[test]
    fn nothing_and_all_zeroes_are_different_pictures() {
        let nothing = Harness::new(Demo::new([]), 4, 7);
        assert!((0..7).all(|row| nothing.bg(0, row) == blank(&nothing)), "no values draw nothing at all");
        let zeroes = Harness::new(Demo::new([0.0; 7]), 4, 7);
        let (empty, _) = tones(&zeroes);
        assert!((0..7).all(|row| zeroes.bg(0, row) == Some(empty)), "a quiet week is a column of empty tone");
    }

    #[test]
    fn a_single_value_takes_the_top_step_and_a_fixed_scale_holds_it_down() {
        let h = Harness::new(Demo::new([3.0]), 2, 7);
        let (_, steps) = tones(&h);
        assert_eq!(h.bg(0, 0), Some(steps[3]), "the only value is the largest one");

        struct Fixed;
        impl App for Fixed {
            type Msg = ();
            fn update(&mut self, _: ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                let map: Heatmap<()> = Heatmap::new([3.0]).max(12.0);
                ui.add(map).width(Length::Fill(1)).height(Length::Fill(1));
            }
        }
        let fixed = Harness::new(Fixed, 2, 7);
        let theme = fixed.env().theme();
        let empty = theme.color("raised").expect("token");
        let step = empty.mix(theme.color("accent").expect("token"), MIX[0]);
        assert_eq!(fixed.bg(0, 0), Some(step), "a quarter of the goal is the first step");
    }

    #[test]
    fn a_narrow_area_keeps_the_newest_weeks_and_says_how_many() {
        let values: Vec<f32> = (0u16..70).map(|i| f32::from(i % 5)).collect();
        let map: Heatmap<()> = Heatmap::new(values.iter().copied());
        assert_eq!(map.columns(80), 10, "ten weeks fit in a wide area");
        assert_eq!(map.columns(4), 4, "a narrow area shows four weeks");
        assert_eq!(map.columns(0), 0);

        let h = Harness::new(Demo::new(values), 3, 7);
        let (_, steps) = tones(&h);
        // Three columns fit, so the grid starts at the eighth week: value 63, which is 3 of 4.
        assert_eq!(h.bg(2, 0), Some(steps[2]), "the rightmost column is the newest week");
        // The leftmost column drawn is the eighth week, whose first value is 49: 4 of 4.
        assert_eq!(h.bg(0, 0), Some(steps[3]), "the oldest weeks are dropped, not squeezed");
    }

    #[test]
    fn a_short_area_keeps_the_rows_that_fit() {
        let mut demo = Demo::new([4.0; 14]);
        demo.height = Some(3);
        let h = Harness::new(demo, 2, 5);
        let (_, steps) = tones(&h);
        for column in 0..2 {
            for row in 0..3 {
                assert_eq!(h.bg(column, row), Some(steps[3]), "{column},{row}");
            }
        }
        assert_eq!(h.bg(0, 3), blank(&h), "rows past the area are not drawn");
    }

    #[test]
    fn tiny_areas_draw_what_they_can_without_panicking() {
        for (width, height) in [(1, 1), (2, 1), (1, 3), (3, 2)] {
            let h = Harness::new(Demo::new([1.0, 2.0, 3.0, 4.0, 5.0]), width, height);
            assert_eq!(h.screen().lines().count(), usize::from(height), "{width}×{height}");
        }
    }

    #[test]
    fn the_grid_is_the_same_in_every_glyph_mode() {
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            let mut h = Harness::new(Demo::new([0.0, 2.0, 4.0]), 2, 7);
            h.set_glyph_mode(mode);
            let (empty, steps) = tones(&h);
            assert_eq!(h.screen(), "\n\n\n\n\n\n\n", "{mode:?} draws no characters");
            assert_eq!(h.bg(0, 0), Some(empty), "{mode:?}");
            assert_eq!(h.bg(0, 2), Some(steps[3]), "{mode:?}");
        }
    }

    #[test]
    fn every_theme_tells_the_steps_and_the_empty_tone_apart() {
        for theme in ["monochrome", "nordic", "amber", "iris"] {
            let mut h = Harness::new(Demo::new([1.0, 2.0, 3.0, 4.0]), 2, 7);
            h.set_theme(theme);
            let drawn: Vec<Rgb> = (0..4).filter_map(|row| h.bg(0, row)).collect();
            assert_eq!(drawn.len(), 4, "{theme}");
            let empty = h.env().theme().color("raised").expect("token");
            for pair in drawn.windows(2) {
                assert!(
                    pair[1].perceptual_distance(pair[0]) >= VISIBLE,
                    "{theme}: {:?} and {:?} are one tone",
                    pair[0],
                    pair[1]
                );
            }
            assert!(drawn[0].perceptual_distance(empty) >= VISIBLE, "{theme}: a quiet day shows over an empty one");
        }
    }

    #[test]
    fn the_ramp_keeps_only_the_tones_a_terminal_can_tell_apart() {
        let h = Harness::new(Demo::new([1.0]), 2, 7);
        let (empty, _) = tones(&h);
        let full = h.env().theme().color("accent").expect("token");
        let ground = h.env().theme().color("canvas").expect("token");

        let true_color = ramp(empty, full, ColorDepth::TrueColor, ground);
        assert_eq!(true_color.len(), 4, "true colour shows every step");
        assert_eq!(tone_of(&true_color, 1), Some(true_color[0]));
        assert_eq!(tone_of(&true_color, 4), Some(true_color[3]), "the top level takes the full tone");

        for depth in [ColorDepth::Ansi256, ColorDepth::Ansi16] {
            let steps = ramp(empty, full, depth, ground);
            assert!(!steps.is_empty(), "{depth:?} still shows that something is there");
            let shown: Vec<u32> = steps.iter().map(|tone| depth.shown(*tone, ground)).collect();
            let mut distinct = shown.clone();
            distinct.sort_unstable();
            distinct.dedup();
            assert_eq!(distinct.len(), shown.len(), "{depth:?} shows no two steps in one tone: {shown:?}");
            let levels: Vec<Rgb> = (1..=LEVELS).filter_map(|level| tone_of(&steps, level)).collect();
            assert_eq!(levels.len(), LEVELS as usize, "{depth:?} gives every level a tone");
            assert_eq!(levels.last(), steps.last(), "{depth:?} keeps the busiest day the brightest");
            for pair in levels.windows(2) {
                assert!(
                    pair[0].relative_luminance() <= pair[1].relative_luminance(),
                    "{depth:?} never turns a busier day quieter: {pair:?}"
                );
            }
        }

        // A terminal that shows the whole ramp in one tone still draws the days.
        let flat = ramp(empty, empty, ColorDepth::Ansi16, ground);
        assert_eq!(flat, vec![empty]);
        assert_eq!(tone_of(&flat, 4), Some(empty));
    }

    #[test]
    fn a_heatmap_without_a_message_is_a_picture() {
        let mut h = Harness::new(Demo::new([1.0, 2.0]), 4, 7);
        h.press("tab");
        assert!(!h.is_focused("map"), "a picture takes no focus");
        h.click(0, 0);
        assert_eq!(h.app().chosen, None, "a click on a picture chooses nothing");
    }

    #[test]
    fn the_pointer_and_the_keyboard_both_reach_a_cell() {
        let mut demo = Demo::new((0u16..21).map(|i| f32::from(i % 5 + 1)));
        demo.interactive = true;
        let mut h = Harness::new(demo, 4, 7);
        let plain = h.bg(0, 3);

        h.hover(0, 2);
        let lit = h.bg(0, 2).expect("the hovered cell is drawn");
        assert_ne!(Some(lit), h.bg(0, 3), "the cell under the pointer lights up");
        assert_eq!(h.bg(0, 3), plain, "its neighbours keep their tone");

        h.click(0, 2);
        assert_eq!(h.app().chosen, Some(2), "a click reports the cell");

        h.press("tab");
        h.press("down");
        h.press("enter");
        assert_eq!(h.app().chosen, Some(3), "the keyboard moves one day and Enter reports it");
        h.press("right");
        h.press("enter");
        assert_eq!(h.app().chosen, Some(10), "a column is a week");
        h.press("left");
        h.press("up");
        h.press("enter");
        assert_eq!(h.app().chosen, Some(2), "back a week and up a day");
        h.press("home");
        h.press("enter");
        assert_eq!(h.app().chosen, Some(0));
        h.press("end");
        h.press("enter");
        assert_eq!(h.app().chosen, Some(20), "End goes to the newest day");
    }

    #[test]
    fn the_keyboard_cursor_lights_a_cell_without_choosing_it() {
        let mut demo = Demo::new([1.0; 7]);
        demo.interactive = true;
        let mut h = Harness::new(demo, 2, 7);
        h.press("tab");
        h.press("home");
        let lit = h.bg(0, 0).expect("drawn");
        let quiet = h.bg(0, 1).expect("drawn");
        assert_ne!(lit, quiet, "the cursor cell steps away from its tone");
        assert_eq!(h.app().chosen, None, "moving the cursor chooses nothing");
    }

    #[test]
    fn a_lit_cell_steps_away_from_its_tone_in_every_theme() {
        for theme in ["monochrome", "nordic", "amber", "iris"] {
            let mut demo = Demo::new([4.0; 7]);
            demo.interactive = true;
            let mut h = Harness::new(demo, 2, 7);
            h.set_theme(theme);
            h.hover(0, 0);
            let lit = h.bg(0, 0).expect("drawn");
            let plain = h.bg(0, 1).expect("drawn");
            assert!(
                lit.perceptual_distance(plain) >= VISIBLE,
                "{theme}: the busiest day shows the pointer ({lit:?} against {plain:?})"
            );
        }
    }

    #[test]
    fn levels_step_with_the_share_of_the_scale() {
        let map: Heatmap<()> = Heatmap::new([0.0, 1.0, 25.0, 50.0, 75.0, 100.0]).max(100.0);
        assert_eq!(map.level(0.0), 0);
        assert_eq!(map.level(-4.0), 0, "a negative value is nothing");
        assert_eq!(map.level(1.0), 1, "any day with something reaches the first step");
        assert_eq!(map.level(25.0), 1);
        assert_eq!(map.level(26.0), 2);
        assert_eq!(map.level(75.0), 3);
        assert_eq!(map.level(76.0), 4);
        assert_eq!(map.level(100.0), 4);
        assert_eq!(map.level(400.0), 4, "values above the scale stay at the top step");
        let zeroes: Heatmap<()> = Heatmap::new([0.0, 0.0]);
        assert_eq!(zeroes.level(0.0), 0, "a grid of zeroes never lights up");
    }

    #[test]
    fn a_series_heatmap_takes_its_tone_from_the_theme() {
        let mut demo = Demo::new([4.0]);
        demo.series = Some(2);
        let h = Harness::new(demo, 2, 7);
        let theme = h.env().theme();
        let empty = theme.color("raised").expect("token");
        assert_eq!(h.bg(0, 0), Some(empty.mix(theme.series_color(2), MIX[3])));
    }
}
