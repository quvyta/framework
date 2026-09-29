//! The focus a program in a terminal asked to hear about: xterm's mode 1004, turned on with
//! `CSI ? 1004 h` and off with `CSI ? 1004 l`.
//!
//! One switch for the whole session, not one per screen as the kitty keyboard protocol keeps its
//! flags: mode 1004 says whether this window has the focus, which does not change when a program
//! switches screens. A program that turns it on while the terminal already has the focus is told
//! so at once, which is what the widget does by reporting the focus as it stands rather than
//! waiting for it to change.

/// The mode number a program names to be told about the focus.
const REPORTS: u16 = 1004;

/// What the terminal sends when the program asked for it and the focus arrives.
const GAINED: &[u8] = b"\x1b[I";

/// What it sends when the focus leaves.
const LOST: &[u8] = b"\x1b[O";

/// Whether one program in one session wants to hear about the focus.
#[derive(Debug, Default)]
pub(crate) struct Focus {
    reporting: bool,
}

impl Focus {
    /// The bytes telling the program that the focus `gained`, or `None` while it has not asked to
    /// hear: the widget reports a change and nothing else, so a frame never repeats an answer.
    pub(crate) fn report(&self, gained: bool) -> Option<&'static [u8]> {
        self.reporting.then_some(if gained { GAINED } else { LOST })
    }

    /// Whether the program asked to hear about the focus.
    pub(crate) fn reporting(&self) -> bool {
        self.reporting
    }

    /// Follows the mode in a private sequence that sets or resets modes; every other sequence is
    /// none of its business.
    pub(crate) fn csi(&mut self, marker: Option<u8>, params: &[&[u16]], end: char) {
        if marker != Some(b'?') || !matches!(end, 'h' | 'l') {
            return;
        }
        // The whole list of modes comes at once, so `CSI ? 1;1004 h` names this one among others.
        // A mode with no number is a zero, which is no mode at all.
        if params.iter().any(|param| param.first() == Some(&REPORTS)) {
            self.reporting = end == 'h';
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_reported_until_a_program_asks() {
        let focus = Focus::default();
        assert_eq!(focus.report(true), None);
        assert_eq!(focus.report(false), None);
    }

    #[test]
    fn the_mode_says_which_way_the_focus_went() {
        let mut focus = Focus::default();
        focus.csi(Some(b'?'), &[&[REPORTS]], 'h');
        assert_eq!(focus.report(true), Some(&b"\x1b[I"[..]));
        assert_eq!(focus.report(false), Some(&b"\x1b[O"[..]));
        focus.csi(Some(b'?'), &[&[REPORTS]], 'l');
        assert_eq!(focus.report(true), None, "and off again");
    }

    #[test]
    fn the_mode_is_found_among_the_others_of_one_sequence() {
        let mut focus = Focus::default();
        focus.csi(Some(b'?'), &[&[1], &[REPORTS], &[1049]], 'h');
        assert_eq!(focus.report(true), Some(&b"\x1b[I"[..]));
        focus.csi(Some(b'?'), &[&[1], &[1049]], 'l');
        assert_eq!(focus.report(true), Some(&b"\x1b[I"[..]), "a sequence without it leaves it alone");
        focus.csi(Some(b'?'), &[&[1], &[REPORTS], &[1049]], 'l');
        assert_eq!(focus.report(true), None, "and naming it in the same sequence turns it off");
    }

    #[test]
    fn other_sequences_are_left_alone() {
        let reports: &[&[u16]] = &[&[REPORTS]];
        let mouse: &[&[u16]] = &[&[1002]];
        let empty: &[&[u16]] = &[&[]];
        let mut focus = Focus::default();
        let others = [
            (None, reports, 'h'),
            (Some(b'>'), reports, 'h'),
            (Some(b'?'), reports, 'u'),
            (Some(b'?'), mouse, 'h'),
            (Some(b'?'), empty, 'h'),
        ];
        for (marker, params, end) in others {
            focus.csi(marker, params, end);
            assert_eq!(focus.report(true), None, "{marker:?} {params:?} {end}");
        }
    }
}
