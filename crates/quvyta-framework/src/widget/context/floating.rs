//! Floating surfaces (menus, popovers, tooltips, toasts, dialogs) standing apart from whatever
//! lies around them.
//!
//! Floating layers paint on the theme's `overlay` tone, which sits close to the panel tones
//! (`surface`, `raised`), so a menu opened over a panel would melt into it. The ground around a
//! surface is only known once the view beneath it is painted, so the surface looks at the cells
//! around it before it paints, and moves its own backgrounds one small step towards a theme
//! colour when it came out too close to them. No colour is written by hand: the step is a blend
//! towards the theme's `text` (lighter on a dark theme, darker on a light one) or `canvas`.

use ratatui_core::style::Color;

use super::PaintCx;
use crate::color::{Lift, Rgb, lift_apart};
use crate::geometry::Rect;
use crate::style::to_color;

/// A ground counts when it covers at least this share of the ring around a surface, so a stray
/// cell of another colour (a pillar, a scrollbar, a letter) does not steer the lift.
const GROUND_SHARE: usize = 4;

/// Colours seen around a floating surface and how many cells showed each.
#[derive(Debug, Default)]
struct Tally(Vec<(Rgb, usize)>);

impl Tally {
    fn count(&mut self, color: Rgb) {
        match self.0.iter_mut().find(|(seen, _)| *seen == color) {
            Some((_, count)) => *count += 1,
            None => self.0.push((color, 1)),
        }
    }

    fn total(&self) -> usize {
        self.0.iter().map(|(_, count)| count).sum()
    }

    /// The colours that cover at least a quarter of the cells counted.
    fn grounds(&self) -> Vec<Rgb> {
        let total = self.total();
        self.0.iter().filter(|(_, count)| count * GROUND_SHARE >= total).map(|(color, _)| *color).collect()
    }

    /// The colour most cells showed.
    fn dominant(&self) -> Option<Rgb> {
        self.0.iter().max_by_key(|(_, count)| *count).map(|(color, _)| *color)
    }
}

/// The grounds a floating surface has to stand apart from, sampled before it paints.
#[derive(Debug, Clone, Default)]
pub(crate) struct Grounds(Vec<Rgb>);

impl PaintCx<'_> {
    /// Paints a floating surface such as a menu, popover or tooltip over `rect` with `paint`,
    /// and keeps it apart from what lies around it.
    ///
    /// Before `paint` runs, the ring of cells just outside `rect` is read; every background
    /// covering at least a quarter of it is a ground. After `paint`, when the background most
    /// cells of `rect` show sits closer to a ground than a barely visible step (0.05 in OKLab),
    /// every background inside `rect` is blended by the same small amount towards the theme's
    /// `text` or `canvas` colour, whichever clears every ground sooner, at most 30%. Text
    /// colours are kept, and the surface's own ladder (a highlighted row, a checked row) moves
    /// with it, so it stays as distinct as before. A surface over the screen ground keeps its
    /// tone; one opened over a panel of nearly the same tone steps lighter on a dark theme and
    /// darker on a light one. Nothing moves in 256 and 16 colours, where the screen holds only
    /// palette entries.
    ///
    /// Overlays paint after the view (see [`PaintCx::request_overlay`]), so call this from
    /// [`Widget::paint_overlay`](crate::widget::Widget::paint_overlay), where the cells around
    /// `rect` already show what the surface floats over.
    pub fn floating(&mut self, rect: Rect, paint: impl FnOnce(&mut Self)) {
        let grounds = self.grounds_around(rect);
        paint(self);
        self.stand_apart(rect, &grounds, None);
    }

    /// The grounds around `rect`: the backgrounds that cover at least a quarter of the ring of
    /// on-screen cells just outside it. Empty when nothing of the ring is on screen, as around
    /// a surface that fills the screen: there is nothing to stand apart from.
    pub(crate) fn grounds_around(&self, rect: Rect) -> Grounds {
        if rect.is_empty() {
            return Grounds::default();
        }
        let mut tally = Tally::default();
        let (left, right, top, bottom) = (rect.x - 1, rect.right(), rect.y - 1, rect.bottom());
        for x in left..=right {
            self.tally_cell(&mut tally, x, top);
            self.tally_cell(&mut tally, x, bottom);
        }
        for y in rect.y..rect.bottom() {
            self.tally_cell(&mut tally, left, y);
            self.tally_cell(&mut tally, right, y);
        }
        Grounds(tally.grounds())
    }

    /// Lifts the surface painted over `rect` apart from `grounds` when it came out too close to
    /// one of them. `background` is the surface's own tone when the caller knows it; otherwise
    /// the background most cells of `rect` show is taken.
    pub(crate) fn stand_apart(&mut self, rect: Rect, grounds: &Grounds, background: Option<Rgb>) {
        if let Some(lift) = self.lift_for(rect, grounds, background) {
            self.lift(rect, lift);
        }
    }

    /// How the surface over `rect` would be lifted apart from `grounds`, if at all.
    pub(crate) fn lift_for(&self, rect: Rect, grounds: &Grounds, background: Option<Rgb>) -> Option<Lift> {
        if grounds.0.is_empty() {
            return None;
        }
        let background = background.or_else(|| self.dominant_background(rect))?;
        lift_apart(background, &grounds.0, &[self.color("text"), self.color("canvas")])
    }

    /// Blends every true-colour background inside `rect` by `lift`, keeping text colours. Cells
    /// in palette colours are left alone.
    pub(crate) fn lift(&mut self, rect: Rect, lift: Lift) {
        let depth = self.env.depth();
        self.each_cell(rect, |cell| {
            if let Color::Rgb(r, g, b) = cell.bg {
                cell.bg = to_color(lift.apply(Rgb::new(r, g, b)), depth);
            }
        });
    }

    /// The true-colour background most visible cells of `rect` show.
    fn dominant_background(&self, rect: Rect) -> Option<Rgb> {
        let area = rect.intersect(self.clip);
        let mut tally = Tally::default();
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                self.tally_cell(&mut tally, x, y);
            }
        }
        tally.dominant()
    }

    /// Counts the background of the screen cell at `(x, y)`, when it is on screen and in true
    /// colour.
    fn tally_cell(&self, tally: &mut Tally, x: i32, y: i32) {
        let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
            return;
        };
        if let Some(Color::Rgb(r, g, b)) = self.buf.cell((x, y)).map(|cell| cell.bg) {
            tally.count(Rgb::new(r, g, b));
        }
    }
}
