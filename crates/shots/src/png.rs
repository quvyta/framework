//! Rasterising the SVG at twice its size.

use std::io;

use resvg::{tiny_skia, usvg};

/// Pixels per SVG unit in the PNG: sharp on high-density screens, and whole pixels for every
/// cell edge.
pub(crate) const SCALE: f32 = 2.0;

/// Renders `svg` to PNG bytes at [`SCALE`]. The picture holds only paths, so nothing is looked
/// up on the machine: no font, no file. The encoder's settings are fixed, and the same picture
/// gives the same bytes.
pub(crate) fn render(svg: &str) -> io::Result<Vec<u8>> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(io::Error::other)?;
    let size = tree.size().to_int_size().scale_by(SCALE).ok_or_else(|| io::Error::other("picture too large"))?;
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or_else(|| io::Error::other("picture has no area"))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(SCALE, SCALE), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(io::Error::other)
}
