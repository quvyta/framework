//! Adding up cell widths without overflowing.
//!
//! Text widths are `u16` cells that saturate at `u16::MAX` for text wider than any screen, so a
//! plain `+` over widths panics in debug builds and wraps to a tiny width in release builds. Sums
//! of widths saturate instead: a result that large is cut to the screen anyway.

/// The saturating sum of `widths`.
pub(crate) fn sum(widths: impl IntoIterator<Item = u16>) -> u16 {
    widths.into_iter().fold(0, u16::saturating_add)
}

#[cfg(test)]
mod tests {
    #[test]
    fn sums_saturate_at_the_widest_width() {
        assert_eq!(super::sum([1, 2, 3]), 6);
        assert_eq!(super::sum([u16::MAX, 2]), u16::MAX);
        assert_eq!(super::sum([]), 0);
    }
}
