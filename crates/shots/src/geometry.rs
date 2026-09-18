//! Sizes of the picture in pixels at 1x, and the one number format every coordinate uses.

/// Width of one terminal cell. The font is scaled so that its advance is exactly this.
pub(crate) const CELL_W: f32 = 9.0;
/// Height of one terminal cell: close to the font's own line height, and whole at 1x and 2x so
/// rows never fall between pixels.
pub(crate) const CELL_H: f32 = 20.0;
/// Even margin between the rounded ground and the terminal grid.
pub(crate) const MARGIN: f32 = 16.0;
/// Corner radius of the ground.
pub(crate) const RADIUS: f32 = 10.0;
/// Height of the title strip, when there is a title.
pub(crate) const TITLE_H: f32 = 32.0;

/// A coordinate with at most two decimals and no trailing zeros, the same on every machine.
pub(crate) fn num(value: f32) -> String {
    let text = format!("{value:.2}");
    let text = if text.contains('.') { text.trim_end_matches('0').trim_end_matches('.') } else { &text };
    if text == "-0" { "0".to_owned() } else { text.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::num;

    #[test]
    fn numbers_are_short_and_stable() {
        assert_eq!(num(9.0), "9");
        assert_eq!(num(2.5), "2.5");
        assert_eq!(num(1.005_f32), "1");
        assert_eq!(num(-0.001), "0");
        assert_eq!(num(-3.456), "-3.46");
        assert_eq!(num(120.0), "120");
    }
}
