//! How a picture's free cells become places, and how their crops meet.

use super::*;

/// A picture of `width` × `height` pixels whose `source` is stretched over `cells`.
fn picture(width: u32, height: u32, cells: Rect, source: (f64, f64, f64, f64)) -> Picture {
    let data = ImageData::from_rgb(width, height, &vec![90; (width * height * 3) as usize]).expect("pixels");
    let ground = Rgb::new(12, 12, 14);
    Picture { data, fit: Fit::Cover, area: cells, cells, source, visible: cells, marker: marker(ground), ground }
}

/// Every cell of `area` for which `free` holds, row by row.
fn cells_of(area: Rect, free: impl Fn(i32, i32) -> bool) -> Vec<(i32, i32)> {
    (area.y..area.bottom())
        .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
        .filter(|&(x, y)| free(x, y))
        .collect()
}

#[test]
fn free_cells_around_a_hole_are_four_rectangles() {
    let area = Rect::new(0, 0, 6, 4);
    let hole = Rect::new(2, 1, 2, 2);
    let rects = rectangles(&cells_of(area, |x, y| !hole.contains(x, y)), MOST_PLACES).expect("few enough");
    assert_eq!(rects, [Rect::new(0, 0, 6, 1), Rect::new(0, 1, 2, 2), Rect::new(4, 1, 2, 2), Rect::new(0, 3, 6, 1)]);
}

#[test]
fn one_rectangle_of_free_cells_is_one_place() {
    let area = Rect::new(3, 2, 5, 4);
    assert_eq!(rectangles(&cells_of(area, |_, _| true), MOST_PLACES), Some(vec![area]));
    assert_eq!(rectangles(&[], MOST_PLACES), Some(Vec::new()), "no free cell, no place");
}

#[test]
fn the_rectangles_cover_exactly_the_free_cells() {
    let area = Rect::new(0, 0, 9, 7);
    let free = |x: i32, y: i32| (x * 7 + y * 3) % 5 != 0;
    let cells = cells_of(area, free);
    let rects = rectangles(&cells, 1000).expect("under the cap");
    let mut covered: Vec<(i32, i32)> = rects
        .iter()
        .flat_map(|rect| (rect.y..rect.bottom()).flat_map(move |y| (rect.x..rect.right()).map(move |x| (x, y))))
        .collect();
    let total = covered.len();
    covered.sort_by_key(|&(x, y)| (y, x));
    covered.dedup();
    assert_eq!(covered.len(), total, "no cell in two rectangles");
    assert_eq!(covered, cells, "every free cell, and no other");
}

#[test]
fn more_rectangles_than_the_cap_give_none() {
    let area = Rect::new(0, 0, 8, 8);
    let board = cells_of(area, |x, y| (x + y) % 2 == 0);
    assert_eq!(rectangles(&board, 31), None, "32 lone cells over a cap of 31");
    assert_eq!(rectangles(&board, 32).map(|rects| rects.len()), Some(32));
}

/// Checks that the crops of the cells `0..count` along one side, one cell each, tile the
/// source's pixels from `first` to `end` with no gap and no overlap.
fn tiles(crops: &[(u32, u32)], first: u32, end: u32) {
    assert_eq!(crops[0].0, first, "the first crop starts where the source does: {crops:?}");
    for pair in crops.windows(2) {
        assert_eq!(pair[0].0 + pair[0].1, pair[1].0, "neighbours meet at one pixel: {crops:?}");
    }
    let last = crops[crops.len() - 1];
    assert_eq!(last.0 + last.1, end, "the last crop ends where the source does: {crops:?}");
}

#[test]
fn crops_of_neighbouring_rectangles_tile_the_source_exactly() {
    // 100 pixels over 7 columns and 37 over 5 rows: no cell edge falls on a whole pixel.
    let cells = Rect::new(4, 2, 7, 5);
    let whole = picture(100, 37, cells, (0.0, 0.0, 100.0, 37.0));
    let columns: Vec<(u32, u32)> = (0..7)
        .map(|i| {
            let (x, _, w, _) = crop(&whole, Rect::new(cells.x + i, cells.y, 1, 5));
            (x, w)
        })
        .collect();
    tiles(&columns, 0, 100);
    let rows: Vec<(u32, u32)> = (0..5)
        .map(|j| {
            let (_, y, _, h) = crop(&whole, Rect::new(cells.x, cells.y + j, 7, 1));
            (y, h)
        })
        .collect();
    tiles(&rows, 0, 37);
    // A cut source, as `Cover` leaves it: the edges are fractions of a pixel.
    let cut = picture(120, 40, cells, (10.3, 1.7, 99.4, 36.6));
    let columns: Vec<(u32, u32)> = [(0, 3), (3, 1), (4, 3)]
        .into_iter()
        .map(|(from, width)| {
            let (x, _, w, _) = crop(&cut, Rect::new(cells.x + from, cells.y, width, 5));
            (x, w)
        })
        .collect();
    tiles(&columns, 10, 110);
}
