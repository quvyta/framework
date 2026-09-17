//! The one-column scrollbar drawn by scrolling widgets.

use crate::geometry::Rect;
use crate::style::CellStyle;
use crate::widget::PaintCx;

/// Position of a scrolled view: how many rows exist, how many are visible and the first
/// visible row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollMetrics {
    /// Rows of content.
    pub total: usize,
    /// Rows that fit.
    pub visible: usize,
    /// First visible row.
    pub offset: usize,
}

impl ScrollMetrics {
    /// Whether the content is taller than the view.
    #[must_use]
    pub fn overflows(self) -> bool {
        self.total > self.visible
    }

    /// The largest valid offset.
    #[must_use]
    pub fn max_offset(self) -> usize {
        self.total.saturating_sub(self.visible)
    }

    /// Thumb start row and length inside a track of `track` rows.
    #[must_use]
    pub fn thumb(self, track: u16) -> (u16, u16) {
        let track_len = usize::from(track);
        if track_len == 0 || self.total == 0 {
            return (0, 0);
        }
        let length = (track_len * self.visible / self.total).clamp(1, track_len);
        let travel = track_len - length;
        let start = (self.offset * travel).checked_div(self.max_offset()).unwrap_or(0);
        (u16::try_from(start).unwrap_or(0), u16::try_from(length).unwrap_or(1))
    }

    /// The offset that puts the thumb under track row `row` (for clicks and drags).
    #[must_use]
    pub fn offset_at(self, row: u16, track: u16) -> usize {
        let (_, length) = self.thumb(track);
        let travel = usize::from(track.saturating_sub(length));
        let row = usize::from(row.saturating_sub(length / 2)).min(travel);
        (row * self.max_offset()).checked_div(travel).unwrap_or(0)
    }
}

/// How a scrollbar is drawn. The theme picks one with `[style.scrollbar] style = "…"`;
/// scrolling widgets can pin one with their `scrollbar` option.
///
/// Every style is a single column that appears only when content overflows; none of them draws
/// a frame. In ASCII mode glyph styles fall back to coloured cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarStyle {
    /// A solid thumb on a solid track, drawn only with background colours: no glyph ever enters a
    /// text selection, and it looks the same in every glyph mode. The default.
    #[default]
    Block,
    /// A thin track `▕` with a half-block thumb `▐`.
    Half,
    /// No track; only a thin thumb `▕`.
    Thin,
    /// A dotted track `·` with a filled thumb `•`.
    Dots,
}

impl ScrollbarStyle {
    /// Every style, in the order the theme documentation lists them: the default first.
    pub const ALL: [Self; 4] = [Self::Block, Self::Half, Self::Thin, Self::Dots];

    /// The theme word for this style.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Half => "half",
            Self::Thin => "thin",
            Self::Dots => "dots",
        }
    }

    /// The style named `name` in a theme.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.name() == name)
    }

    /// Icon keys of the track (if drawn as a glyph) and the thumb; `None` means a coloured cell.
    fn glyphs(self) -> (Option<&'static str>, Option<&'static str>) {
        match self {
            Self::Block => (None, None),
            Self::Half => (Some("scroll-track"), Some("scroll-thumb")),
            Self::Thin => (None, Some("scroll-thin")),
            Self::Dots => (Some("scroll-dot"), Some("scroll-dot-thumb")),
        }
    }

    /// Whether the track is painted at all.
    fn has_track(self) -> bool {
        self != Self::Thin
    }
}

/// Paints a scrollbar into the one-column `rect` using the `scrollbar` style: a faint track
/// and a thumb that brightens while `active` (hovered or dragged). `pinned` overrides the
/// theme's `style` word. Colours come from `scrollbar.<style>` so a theme can tune each style.
pub(crate) fn paint(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    metrics: ScrollMetrics,
    active: bool,
    pinned: Option<ScrollbarStyle>,
) {
    if !metrics.overflows() || rect.is_empty() {
        return;
    }
    // Whatever style draws it, the column is decoration: a clean copy leaves it out.
    cx.decoration(rect);
    let states = if active { vec![crate::theme::State::Hover] } else { Vec::new() };
    let kind = pinned.unwrap_or_else(|| {
        let word = cx.env().theme().style("scrollbar", None, &[]).word("style");
        word.and_then(ScrollbarStyle::from_name).unwrap_or_default()
    });
    let style = cx.style("scrollbar", Some(kind.name()), &states);
    let track = style.color("track").unwrap_or_else(|| cx.color("raised"));
    let thumb = style.color("thumb").unwrap_or_else(|| cx.color("muted"));
    let (start, length) = metrics.thumb(rect.height);
    let (track_key, thumb_key) = kind.glyphs();
    let glyph = |key: Option<&str>| key.map(|key| cx.env().icons().glyph(key).into_owned()).unwrap_or_default();
    let (track_glyph, thumb_glyph) = (glyph(track_key), glyph(thumb_key));
    for row in 0..rect.height {
        let on_thumb = row >= start && row < start + length;
        if !on_thumb && !kind.has_track() {
            continue;
        }
        let (glyph, color) = if on_thumb { (&thumb_glyph, thumb) } else { (&track_glyph, track) };
        let y = rect.y + i32::from(row);
        // A blank glyph (the block style, or ASCII mode) shows the colour as the cell background.
        if glyph.trim().is_empty() {
            cx.clear(Rect::new(rect.x, y, 1, 1), color);
        } else {
            cx.text(rect.x, y, glyph, CellStyle::fg(color), 1);
        }
    }
}

/// Column `x` of a test screen read as a default block scrollbar, top to bottom: `#` for a cell
/// in a thumb colour (resting or hovered), `-` for a cell in the track colour and a space for any
/// other cell. The block style draws no glyphs, so tests of scrolling widgets look at colours.
#[cfg(test)]
pub(crate) fn column<A: crate::runtime::App>(h: &crate::runtime::Harness<A>, x: u16) -> String {
    use crate::theme::State;
    let theme = h.env().theme();
    let color =
        |states: &[State], key: &str| theme.style("scrollbar", Some("block"), states).paint(key).map(|p| p.at(0.0));
    let (track, thumb, lit) = (color(&[], "track"), color(&[], "thumb"), color(&[State::Hover], "thumb"));
    let rows = u16::try_from(h.screen().lines().count()).unwrap_or(0);
    (0..rows)
        .map(|y| match h.bg(x, y) {
            bg if bg.is_some() && (bg == thumb || bg == lit) => '#',
            bg if bg.is_some() && bg == track => '-',
            _ => ' ',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_tracks_offset() {
        let metrics = ScrollMetrics { total: 100, visible: 10, offset: 0 };
        assert_eq!(metrics.thumb(10), (0, 1));
        assert_eq!(ScrollMetrics { offset: 90, ..metrics }.thumb(10), (9, 1));
        assert_eq!(ScrollMetrics { offset: 45, ..metrics }.thumb(10), (4, 1));
        assert_eq!(ScrollMetrics { total: 20, visible: 10, offset: 5 }.thumb(10), (2, 5));
        assert!(!ScrollMetrics { total: 5, visible: 10, offset: 0 }.overflows());
    }

    #[test]
    fn style_names_round_trip_and_match_the_theme_words() {
        let words = crate::theme::WORD_PROPS.iter().find(|(widget, key, _)| *widget == "scrollbar" && *key == "style");
        let words = words.map(|(_, _, words)| *words).unwrap_or_default();
        let names: Vec<&str> = ScrollbarStyle::ALL.iter().map(|style| style.name()).collect();
        assert_eq!(names, words);
        for style in ScrollbarStyle::ALL {
            assert_eq!(ScrollbarStyle::from_name(style.name()), Some(style));
        }
        assert_eq!(ScrollbarStyle::from_name("wavy"), None);
        assert_eq!(names, ["block", "half", "thin", "dots"], "the default comes first");
        assert_eq!(ScrollbarStyle::default(), ScrollbarStyle::Block);
    }

    #[test]
    fn the_retired_cell_word_is_a_diagnostic_and_the_default_block_is_used() {
        let mut registry = crate::theme::ThemeRegistry::builtin();
        let text = "[meta]\nname = \"Old\"\nextends = \"monochrome\"\n\n[style.scrollbar]\nstyle = \"cell\"\n";
        assert!(registry.add_source("old", "old.toml", text), "the rest of the file still loads");
        let problem = registry.diagnostics().iter().find(|d| d.message.contains("`style` must be one of"));
        let problem = problem.expect("the word `cell` is reported");
        assert!(problem.message.ends_with("block, half, thin, dots"), "{}", problem.message);
        assert_eq!(problem.location.as_ref().map(|l| l.line), Some(6));
        let theme = registry.resolve("old").theme.expect("resolves");
        assert_eq!(theme.style("scrollbar", None, &[]).word("style"), Some("block"), "inherits the default");
    }

    /// A list taller than its view, for drawing the scrollbar in each style.
    struct Deploys(Option<ScrollbarStyle>);

    impl crate::runtime::App for Deploys {
        type Msg = ();
        fn update(&mut self, _: ()) -> crate::runtime::Command<()> {
            crate::runtime::Command::none()
        }
        fn view(&self, ui: &mut crate::widget::View<'_, ()>) {
            let items = (0..12).map(|i| crate::widgets::ListItem::new(format!("deploy {i}")));
            let list = crate::widgets::List::new(items);
            ui.add(match self.0 {
                Some(style) => list.scrollbar(style),
                None => list,
            })
            .fill();
        }
    }

    /// A log line with a glyph scrollbar beside it, in a selectable region.
    struct Logged;

    impl crate::widget::Widget<()> for Logged {
        fn measure(
            &self,
            _cx: &mut crate::widget::MeasureCx<'_>,
            available: crate::geometry::Size,
        ) -> crate::geometry::Size {
            available
        }
        fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
            cx.text(area.x, area.y, "pulled image", CellStyle::default(), 12);
            let metrics = ScrollMetrics { total: 8, visible: usize::from(area.height), offset: 0 };
            paint(cx, Rect::new(area.right() - 1, area.y, 1, area.height), metrics, false, Some(ScrollbarStyle::Half));
        }
    }

    struct LoggedApp;

    impl crate::runtime::App for LoggedApp {
        type Msg = ();
        fn update(&mut self, _: ()) -> crate::runtime::Command<()> {
            crate::runtime::Command::none()
        }
        fn view(&self, ui: &mut crate::widget::View<'_, ()>) {
            ui.add(Logged).selectable(true).fill();
        }
    }

    #[test]
    fn a_clean_copy_leaves_every_scrollbar_style_out() {
        let mut h = crate::runtime::Harness::new(LoggedApp, 16, 2);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        assert!(h.screen().starts_with("pulled image   ▐"), "{}", h.screen());
        h.drag((0, 0), (15, 0)).press("ctrl+c");
        assert_eq!(h.clipboard(), Some("pulled image"), "no scrollbar glyph in a clean copy");
    }

    #[test]
    fn block_is_the_default_and_draws_only_colours_in_every_glyph_mode() {
        use crate::icons::GlyphMode;
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            let mut theme_default = crate::runtime::Harness::new(Deploys(None), 16, 4);
            let mut pinned = crate::runtime::Harness::new(Deploys(Some(ScrollbarStyle::Block)), 16, 4);
            theme_default.set_glyph_mode(mode);
            pinned.set_glyph_mode(mode);
            assert_eq!(column(&theme_default, 15), "#---", "{mode:?}");
            assert_eq!(theme_default.screen(), pinned.screen());
            assert!(theme_default.screen().lines().all(|line| !line.ends_with(['█', '▐', '▕', '•', '·'])));
            let theme = theme_default.env().theme();
            assert_eq!(theme_default.bg(15, 0), theme.color("muted"));
            assert_eq!(theme_default.bg(15, 3), theme.color("raised"));
        }
        let mut h = crate::runtime::Harness::new(Deploys(None), 16, 4);
        h.hover(15, 2);
        assert_eq!(h.bg(15, 0), h.env().theme().color("dim"), "the thumb brightens under the pointer");
    }

    #[test]
    fn offset_from_track_row() {
        let metrics = ScrollMetrics { total: 100, visible: 10, offset: 0 };
        assert_eq!(metrics.offset_at(0, 10), 0);
        assert_eq!(metrics.offset_at(9, 10), 90);
        assert_eq!(metrics.offset_at(20, 10), 90);
    }
}
