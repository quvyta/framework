//! Drawing a [`Table`]: the header with its sort and scroll arrows, and the rows.

use crate::geometry::Rect;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{Align, PaintCx};

use super::super::row::{self, LEAD};
use super::super::rows::{self, RowScroll};
use super::layout::Placed;
use super::{MARK, SortDirection, Table, TableCell};

/// What every row of one frame is painted with: whether the table has the focus, whether an
/// activation is flashing, and which row's context menu is open.
#[derive(Debug, Clone, Copy)]
pub(super) struct RowPaint {
    pub(super) focused: bool,
    pub(super) pressed: bool,
    pub(super) menu_row: Option<usize>,
}

impl<Msg: 'static> Table<Msg> {
    pub(super) fn paint_header(
        &self,
        cx: &mut PaintCx<'_>,
        area: Rect,
        placed: &[Placed],
        column_offset: usize,
        more: bool,
    ) {
        let header = Rect::new(area.x, area.y, area.width, 1);
        let base = cx.style("table-header", None, &[]).text();
        if let Some(bg) = base.bg {
            cx.fill(header, bg);
        }
        let pointer = cx.pointer();
        let sorting = self.on_sort.is_some();
        for place in placed {
            let column = &self.columns[place.column];
            let mut states = Vec::new();
            if sorting && column.sortable && pointer.is_some_and(|(x, y)| y == area.y && Self::spans(place, x)) {
                states.push(State::Hover);
            }
            let sorted = self.sort.filter(|(index, _)| *index == place.column).map(|(_, direction)| direction);
            if sorted.is_some() {
                states.push(State::Selected);
            }
            let mut style = cx.style("table-header", None, &states).text();
            style.bg = None;
            let arrow = sorted.map(|direction| {
                let key = if direction == SortDirection::Ascending { "arrow-up" } else { "arrow-down" };
                cx.env().icons().glyph(key).into_owned()
            });
            let arrow_style = cx.style("table-sort", None, &states).text();
            let arrow_width = if arrow.is_some() { 2 } else { 0 };
            let budget = place.width.saturating_sub(arrow_width);
            let title = text::truncate(&column.title, budget).into_owned();
            let title_width = text::width(&title);
            if column.align == Align::End {
                let right = place.x + i32::from(place.width);
                let title_x = right - i32::from(title_width);
                cx.text(title_x, area.y, &title, style, title_width);
                if let Some(arrow) = &arrow {
                    cx.text(title_x - 2, area.y, arrow, arrow_style, 1);
                }
            } else {
                cx.text(place.x, area.y, &title, style, budget);
                if let Some(arrow) = &arrow {
                    cx.text(place.x + i32::from(title_width) + 1, area.y, arrow, arrow_style, 1);
                }
            }
        }
        // The arrows scroll the columns when clicked, so they light up under the pointer.
        let arrows = [(column_offset > 0, area.x, "chevron-left"), (more, area.right() - 1, "chevron-right")];
        for (shown, x, key) in arrows {
            if !shown {
                continue;
            }
            let hovered = pointer == Some((x, area.y));
            let style = cx.style("table-scroll", None, if hovered { &[State::Hover] } else { &[] }).text();
            if let Some(bg) = style.bg {
                cx.fill(Rect::new(x, area.y, 1, 1), bg);
            }
            let glyph = text::truncate(&cx.env().icons().glyph(key), 1).into_owned();
            cx.text(x, area.y, &glyph, CellStyle { bg: None, ..style }, 1);
        }
    }

    pub(super) fn paint_row(&self, cx: &mut PaintCx<'_>, rect: Rect, index: usize, placed: &[Placed], frame: RowPaint) {
        let row = &self.rows[index];
        let hovered = match frame.menu_row {
            Some(menu) => menu == index,
            None => cx.pointer().is_some_and(|(x, y)| rect.contains(x, y)),
        };
        let flashed = cx.memory::<RowScroll>().flashed == Some(index);
        let states = rows::row_states(hovered, Some(index) == self.selected, frame.focused, frame.pressed && flashed);
        let style = cx.style("list-item", row.faint.then_some("faint"), &states);
        let text_style = rows::paint_row(cx, rect, &style);
        let shift = rows::slide(cx, &states);

        // The check mark is a fixed mark: it keeps its column while the first cell slides.
        if let Some(checked) = &self.checked {
            let (glyph, style) = row::check(cx, checked.get(index).copied().unwrap_or(false));
            let glyph = text::truncate(&glyph, 1).into_owned();
            row::paint_marks(cx, rect.x + i32::from(LEAD), rect.y, &[(glyph, style)], MARK);
        }
        for (position, place) in placed.iter().enumerate() {
            let Some(cell) = row.cells.get(place.column) else {
                continue;
            };
            // Only the first visible cell slides; it keeps one spare cell so it never runs into
            // the next column.
            let (x, width) = if position == 0 {
                (place.x + i32::from(shift), place.width.saturating_sub(1))
            } else {
                (place.x, place.width)
            };
            let selected = Some(index) == self.selected;
            Self::paint_cell(cx, cell, (x, rect.y), width, self.columns[place.column].align, text_style, selected);
        }
    }

    /// Paints `cell` at `(x, y)` within `width` cells, aligned by `align`, in the row's text style.
    /// A glyph and its space always keep their cells; only the text is cut.
    fn paint_cell(
        cx: &mut PaintCx<'_>,
        cell: &TableCell,
        (x, y): (i32, i32),
        width: u16,
        align: Align,
        row_style: CellStyle,
        selected: bool,
    ) {
        let style = match &cell.color {
            Some(token) => CellStyle { fg: Some(cx.color(token)), ..row_style },
            None => row_style,
        };
        let icon = cell.icon.as_ref().map(|glyph| {
            let glyph = glyph.resolve(cx.env().icons()).into_owned();
            let quiet = if selected { row_style.fg } else { Some(cx.color("muted")) };
            let fg = cell.icon_color.as_deref().map_or(quiet, |token| Some(cx.color(token)));
            (glyph, CellStyle { fg, ..CellStyle::default() })
        });
        let glyph_width = icon.as_ref().map_or(0, |(glyph, _)| text::width(glyph));
        let prefix = if icon.is_some() { glyph_width.saturating_add(1) } else { 0 };
        let budget = width.saturating_sub(prefix);
        let label = text::truncate(&cell.text, budget).into_owned();
        let content = prefix.saturating_add(text::width(&label));
        let left = match align {
            Align::End => x + i32::from(width) - i32::from(content),
            Align::Center => x + i32::from(width.saturating_sub(content) / 2),
            Align::Start => x,
        };
        // A column narrower than the glyph and its space still starts them at its left edge.
        let mut left = left.max(x);
        if let Some((glyph, icon_style)) = &icon {
            cx.text(left, y, glyph, *icon_style, glyph_width);
            left += i32::from(prefix);
        }
        cx.text(left, y, &label, style, budget);
    }
}
