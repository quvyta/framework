//! Where the tabs, the scroll arrows, the menu control and the add button of a [`Tabs`] strip sit.

use crate::geometry::{Rect, clamp_u16};
use crate::text;

use super::super::cells;
use super::super::tab_model::TabHit;
use super::{ADD, ARROW, Arrow, CLOSE, FILL_MIN, GAP, Overflow, PAD, TabWidth, Tabs, close_mark};

/// Where everything of a strip sits for one scroll offset.
#[derive(Debug, Default)]
pub(super) struct Strip {
    /// Visible tabs in display order.
    pub(super) tabs: Vec<(usize, Rect)>,
    /// The back and forward arrows, when the strip overflows with `Overflow::Arrows`.
    pub(super) arrows: Option<(Rect, Rect)>,
    /// The hidden-tabs control, when the strip overflows with `Overflow::Menu`.
    pub(super) menu: Option<Rect>,
    /// The add button, when the strip has one: right after the last tab, or at the strip's right
    /// end while tabs hide.
    pub(super) add: Option<Rect>,
}

impl Strip {
    /// The part of `area` the tabs are laid out in: between the arrows, or before the menu
    /// control.
    pub(super) fn lane(&self, area: Rect) -> Rect {
        let (mut left, mut right) = (area.x, area.right());
        if let Some((back, forward)) = self.arrows {
            left = back.right() + i32::from(GAP);
            right = forward.x - i32::from(GAP);
        }
        if let Some(menu) = self.menu {
            right = menu.x - i32::from(GAP);
        }
        if let Some(add) = self.add {
            right = right.min(add.x - i32::from(GAP));
        }
        Rect::new(left, area.y, clamp_u16(right - left), area.height)
    }

    /// The arrow at `(x, y)`, if any.
    pub(super) fn arrow_at(&self, x: i32, y: i32) -> Option<Arrow> {
        let (back, forward) = self.arrows?;
        if back.contains(x, y) {
            Some(Arrow::Back)
        } else if forward.contains(x, y) {
            Some(Arrow::Forward)
        } else {
            None
        }
    }
}

impl<Msg: 'static> Tabs<Msg> {
    fn number_width(&self, position: usize) -> u16 {
        if self.numbered { cells::sum([text::width(&(position + 1).to_string()), 1]) } else { 0 }
    }

    /// Where the close mark of a tab at `rect` starts: its last cell is the first of the right
    /// padding, so the lit mark keeps one cell of tab surface after it.
    pub(super) fn close_x(rect: Rect) -> i32 {
        rect.right() - i32::from(PAD + close_mark::WIDTH - 1)
    }

    pub(super) fn close_width(&self, index: usize) -> u16 {
        if self.model.closable(index) { CLOSE } else { 0 }
    }

    /// The width that fits tab `index`. Every tab keeps one spare cell at its right: with slide on a
    /// resting label moves one cell left into it. The cell is there whether slide is on or not, so
    /// turning slide on or off never changes a tab's width.
    fn fit_width(&self, index: usize) -> u16 {
        cells::sum([self.number_width(index), text::width(&self.labels[index]), PAD * 2, self.close_width(index), 1])
    }

    fn min_width(&self, index: usize) -> u16 {
        cells::sum([self.number_width(index), 1, PAD * 2, self.close_width(index)])
    }

    /// The room every tab needs to show at all, cut short.
    fn narrowest_room(&self) -> u16 {
        (0..self.labels.len()).map(|i| self.min_width(i)).max().unwrap_or(0)
    }

    /// The width of every tab when the strip is `available` cells wide.
    pub(super) fn widths(&self, available: u16) -> Vec<u16> {
        let count = self.labels.len();
        match self.width {
            TabWidth::Fit => (0..count).map(|i| self.fit_width(i)).collect(),
            TabWidth::Fixed(cells) => (0..count).map(|i| cells.max(self.min_width(i))).collect(),
            TabWidth::Fill => {
                let count16 = u16::try_from(count).unwrap_or(u16::MAX).max(1);
                let room = available.saturating_sub(self.add_room()).saturating_sub(count16.saturating_sub(1) * GAP);
                let (share, extra) = (room / count16, room % count16);
                (0..count)
                    .map(|i| {
                        let bonus = u16::from(u16::try_from(i).unwrap_or(u16::MAX) < extra);
                        let floor = self.fit_width(i).min(FILL_MIN).max(self.min_width(i));
                        (share + bonus).max(floor)
                    })
                    .collect()
            }
        }
    }

    pub(super) fn total_width(widths: &[u16]) -> u16 {
        let count = u16::try_from(widths.len()).unwrap_or(u16::MAX);
        widths.iter().fold(0u16, |sum, w| sum.saturating_add(*w)).saturating_add(count.saturating_sub(1) * GAP)
    }

    /// The cells the add button keeps at the strip's end beside the tabs: the button and the gap
    /// before it. With no tabs it only needs its own cells.
    pub(super) fn add_room(&self) -> u16 {
        match (&self.on_add, self.labels.is_empty()) {
            (None, _) => 0,
            (Some(_), true) => ADD,
            (Some(_), false) => ADD + GAP,
        }
    }

    /// The strip for tabs shown in `order`, scrolled to position `offset`.
    pub(super) fn strip(&self, whole: Rect, order: &[usize], offset: usize, widths: &[u16]) -> Strip {
        // The tabs, the arrows and the menu control are laid out as if the add button's room were
        // not there, so the button never pushes them around and never falls off the strip.
        let area = Rect::new(whole.x, whole.y, whole.width.saturating_sub(self.add_room()), whole.height);
        let overflows = Self::total_width(widths) > area.width;
        let mut strip = Strip::default();
        let (mut x, mut limit) = (area.x, area.right());
        match self.overflow {
            // Arrows need room for themselves and for any tab, cut short, between them; a strip
            // narrower than that shows the open tab cut to its width instead.
            Overflow::Arrows if overflows && area.width >= 2 * (ARROW + GAP) + self.narrowest_room() => {
                strip.arrows = Some((
                    Rect::new(area.x, area.y, ARROW, 1),
                    Rect::new(area.right() - i32::from(ARROW), area.y, ARROW, 1),
                ));
                x += i32::from(ARROW + GAP);
                limit -= i32::from(ARROW + GAP);
            }
            Overflow::Menu if overflows => {
                // A space, the chevron, a space, the count and a space.
                let digits = text::width(&self.labels.len().to_string());
                let width = 4 + digits;
                let rect = Rect::new(area.right() - i32::from(width), area.y, width, 1);
                strip.menu = Some(rect);
                limit = rect.x - i32::from(GAP);
            }
            Overflow::Arrows | Overflow::Menu => {}
        }
        for &index in order.iter().skip(offset) {
            let width = widths[index];
            let room = limit - x;
            if i32::from(width) > room {
                // A tab wider than all the room there is still shows, cut to the room, so a narrow
                // strip is never empty. With no room even for that, a menu control lists every tab.
                if strip.tabs.is_empty() && room >= i32::from(self.min_width(index)) {
                    strip.tabs.push((index, Rect::new(x, area.y, clamp_u16(room), 1)));
                }
                break;
            }
            strip.tabs.push((index, Rect::new(x, area.y, width, 1)));
            x += i32::from(width) + i32::from(GAP);
        }
        if self.on_add.is_some() && whole.width >= ADD {
            let end = whole.right() - i32::from(ADD);
            let cut = strip.tabs.last().is_some_and(|(index, rect)| rect.width < widths[*index]);
            let x = if strip.tabs.len() < order.len() || cut {
                end
            } else {
                strip.tabs.last().map_or(whole.x, |(_, rect)| (rect.right() + i32::from(GAP)).min(end))
            };
            strip.add = Some(Rect::new(x, whole.y, ADD, 1));
        }
        strip
    }

    /// Whether every tab from position `offset` on shows at its full width.
    fn shows_the_rest(&self, area: Rect, order: &[usize], offset: usize, widths: &[u16]) -> bool {
        let strip = self.strip(area, order, offset, widths);
        strip.tabs.len() == order.len().saturating_sub(offset)
            && strip.tabs.last().is_none_or(|(index, rect)| rect.width == widths[*index])
    }

    /// Scrolls back while the tabs before `offset` fit too, so closing tabs or a wider strip never
    /// leaves empty room at the end while tabs hide at the start.
    pub(super) fn settle(&self, area: Rect, order: &[usize], mut offset: usize, widths: &[u16]) -> usize {
        while offset > 0 && self.shows_the_rest(area, order, offset - 1, widths) {
            offset -= 1;
        }
        offset
    }

    /// The scroll offset that keeps position `target` visible, starting from `offset`.
    pub(super) fn follow(&self, area: Rect, order: &[usize], offset: usize, target: usize, widths: &[u16]) -> usize {
        let mut offset = offset.min(target);
        while offset < target && !self.strip(area, order, offset, widths).tabs.iter().any(|(i, _)| order[target] == *i)
        {
            offset += 1;
        }
        offset
    }

    /// Where a dragged tab can land: the tabs of `strip`, and the add button, which stands for the
    /// end of the strip even when the last tab is scrolled out of view.
    pub(super) fn drop_slots(&self, strip: &Strip) -> Vec<(usize, Rect)> {
        let mut slots = strip.tabs.clone();
        if let Some(add) = strip.add
            && let Some(last) = self.labels.len().checked_sub(1)
        {
            slots.push((last, add));
        }
        slots
    }

    pub(super) fn identity(&self) -> Vec<usize> {
        (0..self.labels.len()).collect()
    }

    pub(super) fn hit(&self, strip: &Strip, x: i32, y: i32) -> Option<TabHit> {
        let (index, rect) = strip.tabs.iter().find(|(_, rect)| rect.contains(x, y)).copied()?;
        if self.model.closable(index) && close_mark::rect(Self::close_x(rect), rect.y).contains(x, y) {
            return Some(TabHit::Close(index));
        }
        Some(TabHit::Tab(index, rect))
    }
}
