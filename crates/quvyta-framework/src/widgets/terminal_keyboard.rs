//! The keyboard a program in a terminal asked for: the flags of the kitty keyboard protocol,
//! which a program pushes with `CSI > flags u`, pops with `CSI < n u`, changes with
//! `CSI = flags ; mode u` and asks about with `CSI ? u`.
//!
//! Only the first flag, "disambiguate escape codes", is kept, because it is the one that lets a
//! program tell Shift+Enter from Enter, and that is what programs turn it on for. Anything else a
//! program asks for is left out of what is kept, so the answer to `CSI ? u` names only what the
//! keys will really do and a program asking for more learns it got this much. Each screen keeps
//! its own stack, as the protocol asks: a full-screen program's flags end with its screen.

/// Report keys that legacy encoding cannot tell apart as `CSI … u`.
const DISAMBIGUATE: u16 = 1;

/// The flags this terminal acts on.
const SUPPORTED: u16 = DISAMBIGUATE;

/// The deepest a stack grows; pushing onto a full one drops the oldest entry, as kitty does, so a
/// program pushing without end cannot make it grow.
const DEPTH: usize = 16;

/// The protocol state of one session; see the module documentation.
#[derive(Debug, Default)]
pub(crate) struct Keyboard {
    main: Vec<u16>,
    alternate: Vec<u16>,
    /// Answers owed to the program, written back by the session's reading thread.
    replies: Vec<u8>,
}

impl Keyboard {
    /// Whether Enter with a modifier is sent as `CSI 13 ; modifier u` on the given screen.
    pub(crate) fn disambiguates(&self, alternate: bool) -> bool {
        self.flags(alternate) & DISAMBIGUATE != 0
    }

    /// The answers owed to the program since the last call.
    pub(crate) fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }

    /// Follows a CSI sequence ending in `u` whose first intermediate byte is `marker`; other
    /// sequences are none of its business.
    pub(crate) fn csi(&mut self, alternate: bool, marker: Option<u8>, params: &[&[u16]], end: char) {
        if end != 'u' {
            return;
        }
        // A missing parameter and an explicit 0 read the same way.
        let param = |index: usize| params.get(index).and_then(|param| param.first()).copied().unwrap_or(0);
        match marker {
            Some(b'?') => {
                let flags = self.flags(alternate);
                self.replies.extend_from_slice(format!("\x1b[?{flags}u").as_bytes());
            }
            Some(b'>') => {
                let stack = self.stack(alternate);
                if stack.len() == DEPTH {
                    stack.remove(0);
                }
                stack.push(param(0) & SUPPORTED);
            }
            Some(b'<') => {
                let stack = self.stack(alternate);
                let count = usize::from(param(0).max(1)).min(stack.len());
                stack.truncate(stack.len() - count);
            }
            Some(b'=') => {
                let flags = param(0) & SUPPORTED;
                let mode = param(1).max(1);
                let stack = self.stack(alternate);
                if stack.is_empty() {
                    stack.push(0);
                }
                if let Some(top) = stack.last_mut() {
                    *top = match mode {
                        2 => *top | flags,
                        3 => *top & !flags,
                        _ => flags,
                    };
                }
            }
            _ => {}
        }
    }

    fn flags(&self, alternate: bool) -> u16 {
        let stack = if alternate { &self.alternate } else { &self.main };
        stack.last().copied().unwrap_or(0)
    }

    fn stack(&mut self, alternate: bool) -> &mut Vec<u16> {
        if alternate { &mut self.alternate } else { &mut self.main }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keyboard after `bytes` were parsed, with the screen they switched to.
    fn after(bytes: &[u8]) -> (Keyboard, bool) {
        let mut parser = vt100::Parser::new_with_callbacks(4, 40, 0, Probe::default());
        parser.process(bytes);
        let alternate = parser.screen().alternate_screen();
        (std::mem::take(&mut parser.callbacks_mut().0), alternate)
    }

    #[derive(Default)]
    struct Probe(Keyboard);

    impl vt100::Callbacks for Probe {
        fn unhandled_csi(
            &mut self,
            screen: &mut vt100::Screen,
            i1: Option<u8>,
            _: Option<u8>,
            params: &[&[u16]],
            end: char,
        ) {
            self.0.csi(screen.alternate_screen(), i1, params, end);
        }
    }

    #[test]
    fn push_pop_and_query() {
        let (mut keyboard, _) = after(b"\x1b[?u");
        assert_eq!(keyboard.take_replies(), b"\x1b[?0u", "a terminal that knows the protocol answers");
        assert!(!keyboard.disambiguates(false));

        let (mut keyboard, _) = after(b"\x1b[>1u\x1b[?u");
        assert!(keyboard.disambiguates(false));
        assert_eq!(keyboard.take_replies(), b"\x1b[?1u");
        assert!(keyboard.take_replies().is_empty(), "an answer is written once");

        let (keyboard, _) = after(b"\x1b[>1u\x1b[<u");
        assert!(!keyboard.disambiguates(false), "a pop without a count takes one entry");
        let (keyboard, _) = after(b"\x1b[>1u\x1b[>0u\x1b[<u");
        assert!(keyboard.disambiguates(false), "the entry below comes back");
        let (keyboard, _) = after(b"\x1b[>1u\x1b[<9u");
        assert!(!keyboard.disambiguates(false), "popping past the bottom empties the stack");
    }

    #[test]
    fn only_the_flags_acted_on_are_kept_and_reported() {
        let (mut keyboard, _) = after(b"\x1b[>31u\x1b[?u");
        assert_eq!(keyboard.take_replies(), b"\x1b[?1u");
        let (mut keyboard, _) = after(b"\x1b[>8u\x1b[?u");
        assert_eq!(keyboard.take_replies(), b"\x1b[?0u");
    }

    #[test]
    fn setting_changes_the_top_entry() {
        let (keyboard, _) = after(b"\x1b[=1u");
        assert!(keyboard.disambiguates(false), "mode 1 sets");
        let (keyboard, _) = after(b"\x1b[>1u\x1b[=1;3u");
        assert!(!keyboard.disambiguates(false), "mode 3 clears");
        let (keyboard, _) = after(b"\x1b[>0u\x1b[=1;2u");
        assert!(keyboard.disambiguates(false), "mode 2 adds");
    }

    #[test]
    fn each_screen_has_its_own_stack() {
        let (keyboard, alternate) = after(b"\x1b[?1049h\x1b[>1u");
        assert!(alternate);
        assert!(keyboard.disambiguates(true));
        assert!(!keyboard.disambiguates(false), "the main screen's flags are untouched");
        let (keyboard, alternate) = after(b"\x1b[?1049h\x1b[>1u\x1b[?1049l");
        assert!(!alternate);
        assert!(!keyboard.disambiguates(false), "back on the main screen its own flags apply");
    }

    #[test]
    fn a_stack_pushed_without_end_stays_bounded() {
        let (keyboard, _) = after(&b"\x1b[>1u".repeat(DEPTH * 4));
        assert_eq!(keyboard.main.len(), DEPTH);
    }

    #[test]
    fn other_sequences_are_left_alone() {
        let (mut keyboard, _) = after(b"\x1b[?25l\x1b[>1c\x1b[u");
        assert!(keyboard.take_replies().is_empty());
        assert!(!keyboard.disambiguates(false));
    }
}
