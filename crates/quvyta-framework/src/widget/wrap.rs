//! Breaking the children of a wrapping row into lines.

/// One child of a wrapping row as line breaking sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Piece {
    /// The width the child measures, at most the row's.
    pub(crate) width: u16,
    /// Whether the child is a spacer: it fills and measures nothing.
    pub(crate) spacer: bool,
}

/// Fills lines of `width` cells with `pieces` in order, `gap` cells apart, and returns the
/// indices on each line.
///
/// A piece that does not fit after the ones already on the line starts the next line; one
/// wider than the row still goes on a line, alone. A spacer that does not fit, which only its
/// gap can cause, is left out: at the start of the next line it would push that line away from
/// the edge.
pub(crate) fn break_lines(pieces: &[Piece], width: u16, gap: u16) -> Vec<Vec<usize>> {
    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut used = 0u32;
    for (i, piece) in pieces.iter().enumerate() {
        let needed = used + u32::from(gap) + u32::from(piece.width);
        if line.is_empty() {
            used = u32::from(piece.width);
        } else if needed <= u32::from(width) {
            used = needed;
        } else if piece.spacer {
            continue;
        } else {
            lines.push(std::mem::take(&mut line));
            used = u32::from(piece.width);
        }
        line.push(i);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{Piece, break_lines};

    fn pieces(widths: &[u16]) -> Vec<Piece> {
        widths.iter().map(|&width| Piece { width, spacer: false }).collect()
    }

    #[test]
    fn pieces_fill_a_line_before_the_next_one_starts() {
        assert_eq!(break_lines(&pieces(&[9, 9, 11, 9]), 25, 1), [vec![0, 1], vec![2, 3]]);
        assert_eq!(break_lines(&pieces(&[9, 9, 11, 9]), 41, 1), [vec![0, 1, 2, 3]]);
        assert_eq!(break_lines(&pieces(&[9, 9, 11, 9]), 40, 1), [vec![0, 1, 2], vec![3]]);
    }

    #[test]
    fn a_piece_wider_than_the_row_is_alone_on_its_line() {
        assert_eq!(break_lines(&pieces(&[5, 20, 5]), 20, 1), [vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn a_spacer_that_does_not_fit_is_left_out() {
        let mut row = pieces(&[9, 9, 0, 11]);
        row[2].spacer = true;
        assert_eq!(break_lines(&row, 19, 1), [vec![0, 1], vec![3]]);
        assert_eq!(break_lines(&row, 20, 1), [vec![0, 1, 2], vec![3]]);
    }

    #[test]
    fn nothing_makes_no_lines() {
        assert!(break_lines(&[], 10, 1).is_empty());
    }
}
