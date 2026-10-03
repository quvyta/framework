//! Icon tiles: a glyph with a name under it, the icon a desktop stands on a floor and a file
//! manager stands in a grid.
//!
//! A tile is a cell of [`IconTile::WIDTH`] columns and [`IconTile::HEIGHT`] rows: the glyph on the
//! top row, the name below it, and a free row under both so two tiles never touch. Both are
//! centred in the same column span, the one beside the column kept for the accent pillar, so they
//! stand in the same place whether the tile is selected or not.

use std::borrow::Cow;

use crate::geometry::{Rect, Size};
use crate::icons::Glyph;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// How much of the text colour a tile under the pointer lifts its ground by, as a pressable panel
/// does: a tile keeps the ground it is given, so the step is mixed in rather than named.
const HOVER_MIX: f32 = 0.08;

/// The rows of a tile a tile over a picture stands on: its glyph and its name, not the free row
/// under them.
const TILE_ROWS: u16 = 2;

/// An icon drawn as a desktop draws it: a glyph, a name under it, and how the tile stands.
///
/// The tile draws and nothing more: the surface that lays tiles out owns the pointer, as a desktop
/// floor and a file manager's grid each do, and a tile that also answered a press would take it
/// away from the surface around it. The tile is still the pointer's own, so its ground lifts a
/// little under the mouse.
///
/// A [selected](Self::selected) tile takes the theme's selected surface with the accent pillar down
/// its first column; a tile with the [cursor](Self::cursor) on it takes the pillar alone, so a
/// surface that separates the two says which is which. A [backed](Self::backed) tile stands on a
/// tile of the surface tone, for a floor with a picture under it, so its name reads whatever the
/// picture is.
///
/// Style keys: `icon-tile` (`fg`, the glyph's colour; `name`, the name's; `bg`, the surface a
/// selected tile takes; `pillar`, the accent of its first column) with states `hover`, `selected`
/// and `focus`, and a `faint` [variant](Self::faint).
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::IconTile;
///
/// struct Desktop {
///     chosen: usize,
/// }
///
/// impl App for Desktop {
///     type Msg = ();
///     fn update(&mut self, (): ()) -> Command<()> {
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, ()>) {
///         ui.row(|ui| {
///             for (index, name) in ["Notes", "Photos"].into_iter().enumerate() {
///                 ui.add(IconTile::new("file", name).selected(index == self.chosen));
///             }
///         });
///     }
/// }
///
/// let app = Harness::new(Desktop { chosen: 0 }, 22, 4);
/// // The pillar of a selected tile is its first column, so the two tiles line up as a grid does.
/// assert_eq!(app.buffer()[(0, 0)].symbol(), "▌");
/// assert_eq!(app.find("Notes"), Some((3, 1)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconTile {
    glyph: Glyph,
    name: String,
    color: Option<String>,
    faint: bool,
    selected: bool,
    cursor: bool,
    backed: bool,
}

impl IconTile {
    /// Columns one tile takes.
    pub const WIDTH: u16 = 10;

    /// Rows one tile takes: the glyph, the name under it, and a free row.
    pub const HEIGHT: u16 = 3;

    /// The column of a tile kept for the accent pillar, so the glyph and the name stand in the same
    /// place whether the tile is selected or not.
    pub const PILLAR: u16 = 1;

    /// The size of one tile.
    pub const SIZE: Size = Size::new(Self::WIDTH, Self::HEIGHT);

    /// A tile drawn with `glyph` and called `name`.
    ///
    /// The glyph is a key of the icon set, such as `"folder"`, so it follows the glyph mode; a
    /// character of the application's own is a [`Glyph::literal`], drawn as it is in every mode.
    #[must_use]
    pub fn new(glyph: impl Into<Glyph>, name: impl Into<String>) -> Self {
        Self {
            glyph: glyph.into(),
            name: name.into(),
            color: None,
            faint: false,
            selected: false,
            cursor: false,
            backed: false,
        }
    }

    /// The name as a tile shows it: whole when it fits, else cut with an ellipsis. A surface that
    /// cut a name says the whole one beside the tile, in a
    /// [`Tooltip`](crate::widgets::Tooltip).
    #[must_use]
    pub fn shown_name(name: &str) -> Cow<'_, str> {
        text::truncate(name, Self::WIDTH - Self::PILLAR)
    }

    /// Draws the glyph in theme colour `token`, such as `"accent"` or `"series-2"`, for a tile
    /// whose kind is told by its colour. The name keeps its own tone, so a grid of coloured icons
    /// still reads as one.
    #[must_use]
    pub fn color(mut self, token: impl Into<String>) -> Self {
        self.color = Some(token.into());
        self
    }

    /// Draws the tile faint, in the `faint` typography role's tone: a cut entry, or one the
    /// application has marked as on its way somewhere.
    #[must_use]
    pub fn faint(mut self, faint: bool) -> Self {
        self.faint = faint;
        self
    }

    /// Whether the tile is one of the selected ones. A selected tile takes the selected surface
    /// and the accent pillar down its first column, and its name takes the text colour.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Whether the keyboard cursor is on the tile. A cursor tile takes the accent pillar, so a
    /// surface that keeps the selection and the cursor apart can say which is which.
    #[must_use]
    pub fn cursor(mut self, cursor: bool) -> Self {
        self.cursor = cursor;
        self
    }

    /// Whether the tile stands on a tile of the theme's surface tone, for a floor with a picture
    /// under it, where the ground under its name could be any colour.
    #[must_use]
    pub fn backed(mut self, backed: bool) -> Self {
        self.backed = backed;
        self
    }

    /// The theme variant the tile is drawn in.
    fn variant(&self) -> Option<&'static str> {
        self.faint.then_some("faint")
    }
}

impl<Msg: 'static> Widget<Msg> for IconTile {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        IconTile::SIZE.min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        cx.register_hit(area);
        let mut states = Vec::new();
        if cx.is_hovered() {
            states.push(State::Hover);
        }
        if self.selected {
            states.push(State::Selected);
        }
        if cx.is_focus_visible() {
            states.push(State::Focus);
        }
        let style = cx.style("icon-tile", self.variant(), &states);
        let glyph_color = style.color("fg").unwrap_or_else(|| cx.color("text"));
        let name_color = style.color("name").unwrap_or_else(|| cx.color("dim"));
        // What lights up under the pointer: the whole cell on a bare ground, only the tile over a
        // picture, which is not the tile's to tint.
        let mut lit = area;
        if self.selected {
            if let Some(ground) = style.color("bg") {
                cx.clear(area, ground);
            }
        } else if self.backed {
            lit = Rect::new(
                area.x + i32::from(IconTile::PILLAR),
                area.y,
                area.width.saturating_sub(IconTile::PILLAR),
                area.height.min(TILE_ROWS),
            );
            cx.clear(lit, cx.color("surface"));
        }
        if cx.is_hovered() {
            cx.tint(lit, glyph_color, HOVER_MIX);
        }
        if self.selected || self.cursor {
            let pillar = style.color("pillar").unwrap_or_else(|| cx.color("accent"));
            for row in 0..area.height {
                cx.pillar(area.x, area.y + i32::from(row), pillar);
            }
        }
        let inner = Rect::new(
            area.x + i32::from(IconTile::PILLAR),
            area.y,
            area.width.saturating_sub(IconTile::PILLAR),
            area.height,
        );
        let centred = |shown: &str| inner.x + i32::from(inner.width.saturating_sub(text::width(shown)) / 2);
        let glyph = self.glyph.resolve(cx.env().icons()).into_owned();
        let color = self.color.as_deref().map_or(glyph_color, |token| cx.color(token));
        cx.text(centred(&glyph), inner.y, &glyph, CellStyle::fg(color), inner.width);
        if inner.height > 1 {
            // A tile narrower than a whole one cuts the name again, to the room it really has.
            let shown = IconTile::shown_name(&self.name);
            let name = text::truncate(&shown, inner.width);
            cx.text(centred(&name), inner.y + 1, &name, CellStyle::fg(name_color), inner.width);
        }
    }
}
