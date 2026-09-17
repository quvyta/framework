//! Rows, columns and stacks.

use super::context::{MeasureCx, PaintCx};
use super::{Align, LayoutProps, Length, Node, Widget};
use crate::geometry::{Rect, Size, clamp_u16};

/// The direction a container lays its children out in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    Row,
    Column,
    Stack,
}

/// A container of child nodes.
pub(crate) struct Flex<Msg> {
    axis: Axis,
    children: Vec<Node<Msg>>,
}

impl<Msg> Flex<Msg> {
    pub(crate) fn new(axis: Axis, children: Vec<Node<Msg>>) -> Self {
        Self { axis, children }
    }
}

/// Main-axis and cross-axis extent of a size for `axis`.
fn split(axis: Axis, size: Size) -> (u16, u16) {
    match axis {
        Axis::Row => (size.width, size.height),
        Axis::Column | Axis::Stack => (size.height, size.width),
    }
}

fn join(axis: Axis, main: u16, cross: u16) -> Size {
    match axis {
        Axis::Row => Size::new(main, cross),
        Axis::Column | Axis::Stack => Size::new(cross, main),
    }
}

fn lengths(axis: Axis, layout: LayoutProps) -> (Length, Length) {
    match axis {
        Axis::Row => (layout.width, layout.height),
        Axis::Column | Axis::Stack => (layout.height, layout.width),
    }
}

fn offset(align: Align, free: u16) -> u16 {
    match align {
        Align::Start => 0,
        Align::Center => free / 2,
        Align::End => free,
    }
}

impl<Msg: 'static> Flex<Msg> {
    /// Cross-axis extent available to a child given its cross length.
    fn cross_available(cross: Length, available: u16) -> u16 {
        match cross {
            Length::Cells(cells) => cells.min(available),
            Length::Auto | Length::Fill(_) => available,
        }
    }

    /// The cells `gap` takes between all children.
    fn gaps(&self, gap: u16) -> u16 {
        gap.saturating_mul(clamp_u16(i32::try_from(self.children.len()).unwrap_or(i32::MAX) - 1))
    }

    /// Sizes of every child along the main axis within `main` cells.
    fn main_sizes(
        &self,
        measure: &mut dyn FnMut(&Node<Msg>, Size) -> Size,
        main: u16,
        cross: u16,
        gap: u16,
    ) -> Vec<u16> {
        let gaps = self.gaps(gap);
        let mut sizes = vec![0u16; self.children.len()];
        let mut used = gaps;
        let mut weights = 0u32;
        for (i, child) in self.children.iter().enumerate() {
            let (main_len, cross_len) = lengths(self.axis, child.layout);
            match main_len {
                Length::Cells(cells) => sizes[i] = cells,
                Length::Auto => {
                    let available = join(self.axis, main.saturating_sub(used), Self::cross_available(cross_len, cross));
                    sizes[i] = split(self.axis, measure(child, available)).0;
                }
                Length::Fill(weight) => weights += u32::from(weight.max(1)),
            }
            if !matches!(main_len, Length::Fill(_)) {
                used = used.saturating_add(sizes[i]);
            }
        }
        let remaining = u32::from(main.saturating_sub(used));
        let mut given = 0u32;
        let mut last_fill = None;
        for (i, child) in self.children.iter().enumerate() {
            if let (Length::Fill(weight), _) = lengths(self.axis, child.layout)
                && let Some(share) = (remaining * u32::from(weight.max(1))).checked_div(weights)
            {
                sizes[i] = u16::try_from(share).unwrap_or(u16::MAX);
                given += share;
                last_fill = Some(i);
            }
        }
        if let Some(i) = last_fill {
            let rest = u16::try_from(remaining - given).unwrap_or(0);
            sizes[i] = sizes[i].saturating_add(rest);
        }
        sizes
    }
}

impl<Msg: 'static> Widget<Msg> for Flex<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let layout = cx.layout();
        let (main_avail, cross_avail) = split(self.axis, available);
        if self.axis == Axis::Stack {
            let mut size = Size::default();
            for child in &self.children {
                let child_size = cx.measure_child(child, available);
                size = Size::new(size.width.max(child_size.width), size.height.max(child_size.height));
            }
            return size;
        }
        let mut main_total = 0u16;
        let mut cross_max = 0u16;
        let mut remaining = main_avail;
        for (i, child) in self.children.iter().enumerate() {
            let (main_len, cross_len) = lengths(self.axis, child.layout);
            let child_avail = join(self.axis, remaining, Self::cross_available(cross_len, cross_avail));
            let measured = split(self.axis, cx.measure_child(child, child_avail));
            let main_size = match main_len {
                Length::Cells(cells) => cells.min(remaining),
                Length::Auto | Length::Fill(_) => measured.0,
            };
            let cross_size = match cross_len {
                Length::Cells(cells) => cells.min(cross_avail),
                Length::Auto | Length::Fill(_) => measured.1,
            };
            let gap = if i + 1 < self.children.len() { layout.gap } else { 0 };
            main_total = main_total.saturating_add(main_size).saturating_add(gap);
            remaining = remaining.saturating_sub(main_size.saturating_add(gap));
            cross_max = cross_max.max(cross_size);
        }
        join(self.axis, main_total.min(main_avail), cross_max)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let layout = cx.layout();
        if self.axis == Axis::Stack {
            for child in &self.children {
                let measured = cx.measure_child(child, area.size());
                let width = match child.layout.width {
                    Length::Fill(_) => area.width,
                    Length::Cells(cells) => cells.min(area.width),
                    Length::Auto => measured.width,
                };
                let height = match child.layout.height {
                    Length::Fill(_) => area.height,
                    Length::Cells(cells) => cells.min(area.height),
                    Length::Auto => measured.height,
                };
                let rect = Rect::new(
                    area.x + i32::from(offset(layout.align, area.width - width)),
                    area.y + i32::from(offset(layout.justify, area.height - height)),
                    width,
                    height,
                );
                cx.paint_child(child, rect);
            }
            return;
        }

        let (main, cross) = split(self.axis, area.size());
        let sizes = {
            let mut measure_cx = MeasureCx::for_frame(cx.env, &mut cx.frame.measures);
            let mut measure = |node: &Node<Msg>, available: Size| measure_cx.measure_child(node, available);
            self.main_sizes(&mut measure, main, cross, layout.gap)
        };
        let total = sizes.iter().fold(self.gaps(layout.gap), |sum, size| sum.saturating_add(*size));
        let mut position = i32::from(offset(layout.justify, main.saturating_sub(total)));
        for (child, main_size) in self.children.iter().zip(sizes) {
            let (_, cross_len) = lengths(self.axis, child.layout);
            let cross_size = match cross_len {
                Length::Fill(_) => cross,
                Length::Cells(cells) => cells.min(cross),
                Length::Auto => {
                    let available = join(self.axis, main_size, cross);
                    split(self.axis, cx.measure_child(child, available)).1
                }
            };
            let cross_offset = i32::from(offset(layout.align, cross - cross_size));
            let rect = match self.axis {
                Axis::Row => Rect::new(area.x + position, area.y + cross_offset, main_size, cross_size),
                Axis::Column | Axis::Stack => {
                    Rect::new(area.x + cross_offset, area.y + position, cross_size, main_size)
                }
            };
            cx.paint_child(child, rect);
            position += i32::from(main_size) + i32::from(layout.gap);
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.children
    }
}
