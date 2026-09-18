//! Drawing a timeline: the track, the blocks in their lanes with their open edges, and the axis.
//! The readout row is written by `readout`.

use crate::color::{ColorDepth, Rgb};
use crate::geometry::Rect;
use crate::style::CellStyle;
use crate::text;
use crate::theme::{State, Theme};
use crate::widget::{PaintCx, Widget};

use super::super::axis::Axis;
use super::layout::Placed;
use super::{TimeBlock, Timeline};

/// How far a hovered block steps towards its hover tone.
const HOVER: f32 = 0.25;

/// How far the selected block steps towards its selected tone.
const SELECTED: f32 = 0.4;

/// How far the selected block steps while the keyboard is on the timeline: past the selected
/// step, so the ladder rest < hover < selected < focus never turns round.
const FOCUS: f32 = 0.55;

/// How far the first cell of a block steps back towards the track where it touches a block of
/// the same tone, so the two read as two.
const SEAM: f32 = 0.35;

/// How far a faint block stands from its tone towards the track: halfway, so it is still plainly
/// its category's colour and plainly not the gap.
const FAINT: f32 = 0.5;

/// Smallest colour difference a reader notices, from [`Rgb::perceptual_distance`].
const VISIBLE: f64 = 0.03;

/// The tones a timeline paints with, read from the theme once per frame.
struct Tones {
    track: Rgb,
    fill: Rgb,
    muted: Rgb,
    text: Rgb,
    hover: Rgb,
    selected: Rgb,
    depth: ColorDepth,
}

impl Tones {
    /// `tone` moved `amount` of the way towards `towards`, or, where the terminal would not show
    /// that step, further along or back towards the track, until it shows as a different colour.
    fn step(&self, tone: Rgb, towards: Rgb, amount: f32) -> Rgb {
        let candidates = [tone.mix(towards, amount), tone.mix(self.track, amount), towards, self.track, self.text];
        candidates
            .into_iter()
            .find(|candidate| self.apart(*candidate, tone))
            .unwrap_or_else(|| tone.mix(towards, amount))
    }

    /// Like [`step`](Self::step), but never onto a tone the terminal would show as the track: a
    /// faint block already stands near it, and a step that landed on it would hide the block.
    fn step_clear_of_track(&self, tone: Rgb, towards: Rgb, amount: f32) -> Rgb {
        let candidates = [tone.mix(towards, amount), towards, self.text, tone.mix(self.track, amount)];
        candidates
            .into_iter()
            .find(|candidate| self.apart(*candidate, tone) && self.apart(*candidate, self.track))
            .unwrap_or_else(|| self.step(tone, towards, amount))
    }

    /// `tone` moved about `amount` of the way towards the track, as a colour this terminal shows
    /// apart from both `tone` and the track. The nearest amount that gives one is taken, so on a
    /// terminal with few colours the step lands on whatever lies between the two; `None` when
    /// nothing does.
    fn towards_track(&self, tone: Rgb, amount: f32) -> Option<Rgb> {
        let mut amounts: Vec<f32> = (1..20u8).map(|i| f32::from(i) / 20.0).collect();
        amounts.sort_by(|a, b| (a - amount).abs().total_cmp(&(b - amount).abs()));
        std::iter::once(amount)
            .chain(amounts)
            .map(|amount| tone.mix(self.track, amount))
            .find(|candidate| self.apart(*candidate, tone) && self.apart(*candidate, self.track))
    }

    /// Whether `a` and `b` read as two tones on this terminal.
    fn apart(&self, a: Rgb, b: Rgb) -> bool {
        self.depth.tells_apart(a, b) && (self.depth != ColorDepth::TrueColor || a.perceptual_distance(b) >= VISIBLE)
    }
}

impl<Msg: 'static> Timeline<Msg> {
    /// Draws every part of the timeline into `area`.
    pub(super) fn paint_all(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let rows = self.rows(area.height);
        let tones = self.tones(cx);
        let strip = Rect::new(area.x, area.y, area.width, rows.strip);
        cx.fill(strip, tones.track);

        let focus = self.selectable() && cx.is_focus_visible();
        let selected_towards = if focus {
            cx.style("timeline", None, &[State::Focus]).color("selected").unwrap_or(tones.selected)
        } else {
            tones.selected
        };
        let hovered = if self.selectable() { cx.pointer().and_then(|(x, y)| self.block_at(area, x, y)) } else { None };

        let theme = cx.env().theme();
        let drawn: Vec<(Placed, i32, u16, Rgb)> = self
            .paint_order()
            .into_iter()
            .filter_map(|p| {
                let (first, cells) = self.columns(&p, area.width)?;
                Some((p, first, cells, self.plain(theme, &tones, &self.blocks[p.index])))
            })
            .collect();
        for (p, first, cells, plain) in &drawn {
            let block = &self.blocks[p.index];
            let row = Self::row_of(p.lane, rows.strip);
            let step = |tone: Rgb, towards: Rgb, amount: f32| {
                if block.faint {
                    tones.step_clear_of_track(tone, towards, amount)
                } else {
                    tones.step(tone, towards, amount)
                }
            };
            let color = if self.selected == Some(p.index) && !self.disabled {
                step(*plain, selected_towards, if focus { FOCUS } else { SELECTED })
            } else if hovered == Some(p.index) {
                step(*plain, tones.hover, HOVER)
            } else {
                *plain
            };
            let y = area.y + i32::from(row);
            let x = area.x + first;
            cx.fill(Rect::new(x, y, *cells, 1), color);
            let (lead, trail) = self.fades(p, *cells);
            let steps = lead.max(trail);
            for cell in 0..steps {
                // The outermost cell is the quietest: with two cells, two thirds and one third of
                // the way to the track.
                let amount = f32::from(steps - cell) / f32::from(steps + 1);
                let Some(faded) = tones.towards_track(color, amount) else { continue };
                if cell < lead {
                    cx.fill(Rect::new(x + i32::from(cell), y, 1, 1), faded);
                }
                if cell < trail {
                    cx.fill(Rect::new(x + i32::from(*cells - 1 - cell), y, 1, 1), faded);
                }
            }
            let touches = drawn.iter().any(|(q, q_first, q_cells, q_plain)| {
                q.index != p.index
                    && Self::row_of(q.lane, rows.strip) == row
                    && q_first + i32::from(*q_cells) == *first
                    && q_plain == plain
            });
            if touches && lead == 0 {
                cx.fill(Rect::new(x, y, 1, 1), tones.step(color, tones.track, SEAM));
            }
            // The name keeps a cell of air on each side and never sits on a fading cell.
            let label = &block.label;
            let width = text::width(label);
            let (before, after) = (lead.max(1), trail.max(1));
            if width > 0 && *cells >= width.saturating_add(before).saturating_add(after) {
                cx.text(x + i32::from(before), y, label, CellStyle::fg(readable_on(cx, color)), width);
            }
        }

        if let Some(row) = rows.axis {
            let (from, span) = self.visible();
            let axis = Axis::hours(self.clock(from), self.clock(from + span)).faint(self.disabled);
            Widget::<Msg>::paint(&axis, cx, Rect::new(area.x, area.y + i32::from(row), area.width, 1));
        }
        if let Some(row) = rows.readout {
            let read = hovered.or(self.selected).and_then(|index| self.blocks.get(index));
            self.paint_readout(cx, Rect::new(area.x, area.y + i32::from(row), area.width, 1), read);
        }
    }

    /// The theme's tones for this timeline.
    fn tones(&self, cx: &mut PaintCx<'_>) -> Tones {
        let style = cx.style("timeline", None, &[]);
        let text = cx.color("text");
        Tones {
            track: style.color("track").unwrap_or_else(|| cx.color("raised")),
            fill: style.color("fill").unwrap_or_else(|| cx.color("accent")),
            muted: cx.color("muted"),
            text,
            hover: style.color("hover").unwrap_or(text),
            selected: style.color("selected").unwrap_or(text),
            depth: cx.env().depth(),
        }
    }

    /// The resting tone of `block`: its series tone, else the fill, muted while disabled, and
    /// never a tone the terminal would show as the track; a faint block stands halfway from that
    /// tone to the track.
    fn plain(&self, theme: &Theme, tones: &Tones, block: &TimeBlock) -> Rgb {
        let tone =
            if self.disabled { tones.muted } else { block.tone.map_or(tones.fill, |index| theme.series_color(index)) };
        let tone = if tones.depth.tells_apart(tone, tones.track) { tone } else { tones.text };
        if block.faint { tones.towards_track(tone, FAINT).unwrap_or(tone) } else { tone }
    }
}

/// The theme colour that reads best on `fill`: the ink meant for the accent, or the text colour
/// when the fill is dark enough for it.
fn readable_on(cx: &PaintCx<'_>, fill: Rgb) -> Rgb {
    let ink = cx.color("ink");
    let text = cx.color("text");
    if fill.contrast_ratio(ink) >= fill.contrast_ratio(text) { ink } else { text }
}
