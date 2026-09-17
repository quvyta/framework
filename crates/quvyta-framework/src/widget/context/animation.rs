//! Drawing named one-cell animations while painting.

use std::time::Duration;

use super::PaintCx;
use super::paint::ANIMATION_FRAME;
use crate::animation::AnimatedCell;
use crate::style::CellStyle;

impl PaintCx<'_> {
    /// The cell the animation `name` shows now, drawn in `style`: frames without a colour, and
    /// `$fg` in colour expressions, take `style.fg`. Draw it with [`PaintCx::text`] in one cell.
    ///
    /// `since` is when the animation started on the [`PaintCx::now`] clock; looping indicators
    /// pass `Some(Duration::ZERO)` so they turn in step with every other one, and `None` shows the
    /// rest frame. With reduced motion the rest frame always shows. Schedules the next frame
    /// exactly when the frame changes, and smooth frames while a colour pulses or blends. An
    /// unknown name draws `⟦name⟧`, cut to the cell, like a missing icon.
    ///
    /// ```
    /// # use std::time::Duration;
    /// # use qframe::widget::PaintCx;
    /// # use qframe::geometry::Rect;
    /// fn paint_busy_mark(cx: &mut PaintCx<'_>, area: Rect) {
    ///     let style = cx.style("spinner", None, &[]).text();
    ///     let cell = cx.animation("spinner-pulse", style, Some(Duration::ZERO));
    ///     cx.text(area.x, area.y, &cell.glyph, cell.style, 1);
    /// }
    /// ```
    pub fn animation(&mut self, name: &str, style: CellStyle, since: Option<Duration>) -> AnimatedCell {
        let env = self.env;
        let Some(animation) = env.icons().animation(name) else {
            return AnimatedCell { glyph: format!("⟦{name}⟧"), style, finished: true };
        };
        let since = since.filter(|_| !env.reduced_motion());
        let frame = animation.sample(env.theme(), style.fg, self.now, since);
        if since.is_some() {
            if frame.smooth {
                self.request_frame_in(ANIMATION_FRAME);
            } else if let Some(next) = frame.next {
                self.request_frame_in(next);
            }
        }
        let glyph = animation.glyph(frame.index, env.icons().mode()).to_owned();
        AnimatedCell { glyph, style: CellStyle { fg: frame.color.or(style.fg), ..style }, finished: frame.finished }
    }
}
