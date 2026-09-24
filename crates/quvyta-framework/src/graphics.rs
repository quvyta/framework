//! Which way the terminal can show a picture: the [`Graphics`] an [`Env`](crate::env::Env)
//! reports, and the rules that decide it.
//!
//! The runtime asks the terminal once, as it starts (see
//! [`Env::graphics`](crate::env::Env::graphics)); what the terminal answered is then weighed
//! against what is known of the environment. This module holds the parts that need no terminal:
//! reading the answers and applying the rules.

/// The way a picture can be drawn in this terminal, sharpest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Graphics {
    /// The kitty graphics protocol: real pixels, sent once and placed again by number. Kitty,
    /// WezTerm, Ghostty and Konsole answer to it.
    Kitty,
    /// DEC sixel: real pixels, written out whole each time. foot, WezTerm, mlterm, Windows
    /// Terminal and a configured xterm answer to it.
    Sixel,
    /// Half blocks: each cell shows two pixels, the upper one as the colour of `▀`, the lower one
    /// as the cell's ground. Every terminal with 256 or more colours shows it, over any link.
    HalfBlock,
    /// No picture at all: the terminal has 16 colours or draws in ASCII, where a picture cannot
    /// be told apart from noise. A picture's place shows what it is instead.
    None,
}

impl Graphics {
    /// Every value, sharpest first.
    pub const ALL: [Self; 4] = [Self::Kitty, Self::Sixel, Self::HalfBlock, Self::None];

    /// The name the `QUVYTA_GRAPHICS` environment variable takes for this value: `kitty`,
    /// `sixel`, `halfblock` or `none`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Kitty => "kitty",
            Self::Sixel => "sixel",
            Self::HalfBlock => "halfblock",
            Self::None => "none",
        }
    }

    /// The value `name` stands for, as [`Graphics::name`] writes it; case and surrounding space
    /// do not matter. `None` for any other name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        Self::ALL.into_iter().find(|graphics| graphics.name().eq_ignore_ascii_case(name))
    }
}

/// The environment variable that decides the graphics, whatever the terminal answered.
pub(crate) const VARIABLE: &str = "QUVYTA_GRAPHICS";

/// What the environment knows about pictures before the terminal is asked, and what it answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphicsFacts {
    /// What the terminal answered to the probe; [`Graphics::HalfBlock`] until it is asked, and
    /// when it did not answer.
    pub(crate) answer: Graphics,
    /// What `QUVYTA_GRAPHICS` forces, if it is set to a known name.
    pub(crate) forced: Option<Graphics>,
    /// Whether the application runs inside tmux or GNU screen.
    pub(crate) multiplexed: bool,
}

impl Default for GraphicsFacts {
    fn default() -> Self {
        Self { answer: Graphics::HalfBlock, forced: None, multiplexed: false }
    }
}

impl GraphicsFacts {
    /// Reads `QUVYTA_GRAPHICS`, `TMUX` and `STY` through `lookup`. An unknown name in
    /// `QUVYTA_GRAPHICS` is returned as the second value, so the caller can report it.
    pub(crate) fn detect(lookup: impl Fn(&str) -> Option<String>) -> (Self, Option<String>) {
        let set = |name: &str| lookup(name).filter(|value| !value.trim().is_empty());
        let multiplexed = set("TMUX").is_some() || set("STY").is_some();
        let (forced, unknown) = match set(VARIABLE) {
            Some(value) => match Graphics::from_name(&value) {
                Some(graphics) => (Some(graphics), None),
                None => (None, Some(value)),
            },
            None => (None, None),
        };
        (Self { answer: Graphics::HalfBlock, forced, multiplexed }, unknown)
    }

    /// The graphics in force for a terminal of `depth` drawing in `glyphs`.
    ///
    /// `QUVYTA_GRAPHICS` wins over everything, because the person who set it knows their
    /// terminal. Otherwise 16 colours and ASCII glyphs show no picture, a multiplexer turns kitty
    /// and sixel into half blocks because it does not pass them through, and what the terminal
    /// answered decides the rest.
    pub(crate) fn resolve(self, depth: crate::color::ColorDepth, glyphs: crate::icons::GlyphMode) -> Graphics {
        if let Some(forced) = self.forced {
            return forced;
        }
        if depth == crate::color::ColorDepth::Ansi16 || glyphs == crate::icons::GlyphMode::Ascii {
            return Graphics::None;
        }
        match self.answer {
            Graphics::Kitty | Graphics::Sixel if self.multiplexed => Graphics::HalfBlock,
            answer => answer,
        }
    }

    /// Whether asking the terminal could change the result: not when the variable decides, not
    /// inside a multiplexer, and not at 16 colours, which a running application never leaves.
    /// ASCII glyphs do not count, because a settings screen can switch them off while it runs.
    pub(crate) fn worth_asking(self, depth: crate::color::ColorDepth) -> bool {
        self.forced.is_none() && !self.multiplexed && depth != crate::color::ColorDepth::Ansi16
    }
}

/// The question the runtime sends as it starts: a kitty graphics query for a one-pixel image
/// (`a=q` only asks, nothing is stored or shown), then a primary device attributes request (DA1).
/// Every terminal answers DA1 and answers in order, so its reply marks the end of the answers.
pub(crate) const QUERY: &str = "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c";

/// Reads the terminal's answers to [`QUERY`]: an `OK` to the kitty query means kitty, a DA1
/// answer that lists attribute `4` means sixel, and anything else, no answer included, means half
/// blocks. Kitty wins when both are there, being the sharper and the cheaper to redraw.
pub(crate) fn classify(replies: &[u8]) -> Graphics {
    if kitty_ok(replies) {
        Graphics::Kitty
    } else if primary_attributes(replies).is_some_and(|attributes| attributes.contains(&"4")) {
        Graphics::Sixel
    } else {
        Graphics::HalfBlock
    }
}

/// Whether `replies` hold the terminal's primary device attributes in full, which is the last
/// answer to [`QUERY`].
pub(crate) fn answered(replies: &[u8]) -> bool {
    primary_attributes(replies).is_some()
}

/// Whether `replies` hold `ESC _ G i=31 … ; OK ESC \`, the kitty answer to the query's image 31.
fn kitty_ok(replies: &[u8]) -> bool {
    let mut rest = replies;
    while let Some(start) = find(rest, b"\x1b_G") {
        let after = &rest[start + 3..];
        let Some(end) = find(after, b"\x1b\\") else {
            return false;
        };
        let body = &after[..end];
        if let Some(split) = body.iter().position(|&byte| byte == b';') {
            let (keys, message) = (&body[..split], &body[split + 1..]);
            if keys.split(|&byte| byte == b',').any(|key| key == b"i=31") && message == b"OK" {
                return true;
            }
        }
        rest = &after[end + 2..];
    }
    false
}

/// The attributes of a DA1 answer, `ESC [ ? 62 ; 4 ; 22 c`, when `replies` hold a whole one.
fn primary_attributes(replies: &[u8]) -> Option<Vec<&str>> {
    let mut rest = replies;
    while let Some(start) = find(rest, b"\x1b[?") {
        let body = &rest[start + 3..];
        let length = body.iter().position(|&byte| !(byte.is_ascii_digit() || byte == b';'))?;
        if body[length] == b'c' {
            let text = std::str::from_utf8(&body[..length]).ok()?;
            return Some(text.split(';').collect());
        }
        rest = &body[length..];
    }
    None
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::ColorDepth;
    use crate::icons::GlyphMode;

    const KITTY_OK: &[u8] = b"\x1b_Gi=31;OK\x1b\\";
    /// What kitty itself answers to DA1: a VT220 without sixel.
    const DA1_PLAIN: &[u8] = b"\x1b[?62;c";
    /// What foot answers: a VT220 with sixel (4) and ANSI colour (22).
    const DA1_SIXEL: &[u8] = b"\x1b[?62;4;22c";

    #[test]
    fn a_kitty_ok_before_the_attributes_means_kitty() {
        assert_eq!(classify(&[KITTY_OK, DA1_PLAIN].concat()), Graphics::Kitty);
        assert_eq!(classify(&[KITTY_OK, DA1_SIXEL].concat()), Graphics::Kitty, "the sharper of the two wins");
    }

    #[test]
    fn attributes_listing_4_mean_sixel() {
        assert_eq!(classify(DA1_SIXEL), Graphics::Sixel);
        assert_eq!(classify(b"\x1b[?4c"), Graphics::Sixel, "4 alone");
        assert_eq!(classify(b"\x1b[?64;1;2;4;6;9;15;18;21;22c"), Graphics::Sixel, "xterm as a VT340");
    }

    #[test]
    fn attributes_without_4_mean_half_blocks() {
        assert_eq!(classify(DA1_PLAIN), Graphics::HalfBlock);
        assert_eq!(classify(b"\x1b[?1;2c"), Graphics::HalfBlock, "a VT100 with advanced video");
        assert_eq!(classify(b"\x1b[?64;14;22c"), Graphics::HalfBlock, "14 is not 4");
    }

    #[test]
    fn a_kitty_error_or_another_image_is_not_kitty() {
        let refused = [&b"\x1b_Gi=31;ENOTSUPPORTED:no\x1b\\"[..], DA1_PLAIN].concat();
        assert_eq!(classify(&refused), Graphics::HalfBlock);
        let other = [&b"\x1b_Gi=7;OK\x1b\\"[..], DA1_PLAIN].concat();
        assert_eq!(classify(&other), Graphics::HalfBlock);
    }

    #[test]
    fn garbage_and_silence_mean_half_blocks() {
        assert_eq!(classify(b""), Graphics::HalfBlock);
        assert_eq!(classify(b"hello \x1b[?62;4"), Graphics::HalfBlock, "an unfinished answer");
        assert_eq!(classify(b"\x1b_Gi=31;OK"), Graphics::HalfBlock, "an unterminated kitty answer");
        assert_eq!(classify(b"\x1b[?6x4c\x1b\x1b_G;"), Graphics::HalfBlock);
        assert_eq!(classify(&[0xff, 0x1b, b'[', b'?', 0xfe]), Graphics::HalfBlock);
    }

    #[test]
    fn the_attributes_mark_the_end_of_the_answers() {
        assert!(!answered(b""));
        assert!(!answered(KITTY_OK), "the kitty answer comes first; the attributes are still due");
        assert!(!answered(b"\x1b[?62;4"));
        assert!(answered(&[KITTY_OK, DA1_PLAIN].concat()));
        assert!(answered(DA1_SIXEL));
    }

    #[test]
    fn a_multiplexer_turns_kitty_and_sixel_into_half_blocks() {
        for variable in ["TMUX", "STY"] {
            let lookup = |name: &str| (name == variable).then(|| "/tmp/tmux-1000/default,1234,0".to_owned());
            let (mut facts, unknown) = GraphicsFacts::detect(lookup);
            assert!(facts.multiplexed && unknown.is_none(), "{variable}");
            for answer in [Graphics::Kitty, Graphics::Sixel, Graphics::HalfBlock] {
                facts.answer = answer;
                let graphics = facts.resolve(ColorDepth::TrueColor, GlyphMode::Unicode);
                assert_eq!(graphics, Graphics::HalfBlock, "{variable} with {answer:?}");
            }
            assert!(!facts.worth_asking(ColorDepth::TrueColor), "a multiplexer needs no question");
        }
        let empty = |name: &str| (name == "TMUX").then(String::new);
        assert!(!GraphicsFacts::detect(empty).0.multiplexed, "an empty variable is unset");
    }

    #[test]
    fn outside_a_multiplexer_the_answer_decides() {
        let (mut facts, _) = GraphicsFacts::detect(|_| None);
        for answer in [Graphics::Kitty, Graphics::Sixel, Graphics::HalfBlock] {
            facts.answer = answer;
            assert_eq!(facts.resolve(ColorDepth::TrueColor, GlyphMode::Unicode), answer);
            assert_eq!(facts.resolve(ColorDepth::Ansi256, GlyphMode::Nerd), answer);
        }
        assert!(facts.worth_asking(ColorDepth::Ansi256));
    }

    #[test]
    fn sixteen_colours_and_ascii_show_no_picture() {
        let facts = GraphicsFacts { answer: Graphics::Kitty, ..GraphicsFacts::default() };
        assert_eq!(facts.resolve(ColorDepth::Ansi16, GlyphMode::Unicode), Graphics::None);
        assert_eq!(facts.resolve(ColorDepth::TrueColor, GlyphMode::Ascii), Graphics::None);
        assert!(!facts.worth_asking(ColorDepth::Ansi16), "16 colours never change while it runs");
        assert!(facts.worth_asking(ColorDepth::TrueColor), "ASCII glyphs can be switched off while it runs");
    }

    #[test]
    fn the_variable_wins_over_everything() {
        for forced in Graphics::ALL {
            let lookup = |name: &str| match name {
                VARIABLE => Some(format!(" {} ", forced.name().to_uppercase())),
                "TMUX" => Some("/tmp/tmux".to_owned()),
                _ => None,
            };
            let (mut facts, unknown) = GraphicsFacts::detect(lookup);
            assert_eq!((facts.forced, unknown), (Some(forced), None));
            facts.answer = Graphics::Sixel;
            assert_eq!(facts.resolve(ColorDepth::Ansi16, GlyphMode::Ascii), forced, "{forced:?}");
            assert_eq!(facts.resolve(ColorDepth::TrueColor, GlyphMode::Unicode), forced, "{forced:?}");
            assert!(!facts.worth_asking(ColorDepth::TrueColor), "the variable already decided");
        }
    }

    #[test]
    fn an_unknown_name_is_reported_and_ignored() {
        let lookup = |name: &str| (name == VARIABLE).then(|| "pixels".to_owned());
        let (facts, unknown) = GraphicsFacts::detect(lookup);
        assert_eq!(facts.forced, None);
        assert_eq!(unknown.as_deref(), Some("pixels"));
    }

    #[test]
    fn names_read_back() {
        for graphics in Graphics::ALL {
            assert_eq!(Graphics::from_name(graphics.name()), Some(graphics));
        }
        assert_eq!(Graphics::from_name("half-block"), None);
    }
}
