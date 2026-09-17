//! Colour expressions used in theme files, and the paints they resolve to.
//!
//! Grammar:
//!
//! ```text
//! expr  = token | hex | mix | pulse
//! token = "$" name                       e.g. $accent-2
//! hex   = "#" 3 or 6 hex digits          e.g. #38BDF8
//! mix   = "mix(" expr "," expr "," number "%" ")"
//! pulse = "pulse(" expr "," expr ")"
//! ```
//!
//! `mix(a, b, 30%)` is 30% of `a` blended into `b`. `pulse(a, b)` breathes between `a` and
//! `b`; it can only appear at the top of an expression.

use std::collections::BTreeMap;
use std::f32::consts::TAU;

use crate::color::Rgb;

/// A resolved colour, possibly animated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Paint {
    /// A single colour.
    Solid(Rgb),
    /// A colour that breathes between two ends over the theme's pulse period.
    Pulse(Rgb, Rgb),
}

impl Paint {
    /// The colour at `phase` of the pulse cycle (`0.0..1.0`, wrapping).
    ///
    /// The pulse starts at its first colour, reaches the second at `0.5` and returns.
    #[must_use]
    pub fn at(self, phase: f32) -> Rgb {
        match self {
            Self::Solid(color) => color,
            Self::Pulse(from, to) => {
                let t = (1.0 - (phase.rem_euclid(1.0) * TAU).cos()) / 2.0;
                from.mix(to, t)
            }
        }
    }

    /// Whether drawing this paint needs animation frames.
    #[must_use]
    pub fn is_animated(self) -> bool {
        matches!(self, Self::Pulse(..))
    }
}

/// A parsed, unresolved colour expression.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr {
    Hex(Rgb),
    Token(String),
    Mix(Box<Expr>, Box<Expr>, f32),
    Pulse(Box<Expr>, Box<Expr>),
}

// Percentages are checked to lie in 0..=100 while parsing, so no expression holds a NaN.
impl Eq for Expr {}

impl Expr {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut parser = Parser { text, pos: 0, depth: 0 };
        let expr = parser.expr()?;
        parser.skip_space();
        if parser.pos != text.len() {
            return Err(format!("unexpected `{}` in colour `{text}`", &text[parser.pos..]));
        }
        Ok(expr)
    }

    /// Resolves to a paint. `tokens` holds already resolved colour tokens.
    pub(crate) fn resolve(&self, tokens: &BTreeMap<String, Rgb>) -> Result<Paint, String> {
        match self {
            Self::Pulse(a, b) => Ok(Paint::Pulse(a.solid(tokens)?, b.solid(tokens)?)),
            other => other.solid(tokens).map(Paint::Solid),
        }
    }

    /// Resolves to a single colour; `pulse()` is not allowed here.
    pub(crate) fn solid(&self, tokens: &BTreeMap<String, Rgb>) -> Result<Rgb, String> {
        self.solid_by(&|name| tokens.get(name).copied())
    }

    /// Resolves to a paint, looking tokens up with `lookup`.
    pub(crate) fn resolve_by(&self, lookup: &dyn Fn(&str) -> Option<Rgb>) -> Result<Paint, String> {
        match self {
            Self::Pulse(a, b) => Ok(Paint::Pulse(a.solid_by(lookup)?, b.solid_by(lookup)?)),
            other => other.solid_by(lookup).map(Paint::Solid),
        }
    }

    /// Resolves to a single colour, looking tokens up with `lookup`; `pulse()` is not allowed here.
    fn solid_by(&self, lookup: &dyn Fn(&str) -> Option<Rgb>) -> Result<Rgb, String> {
        match self {
            Self::Hex(color) => Ok(*color),
            Self::Token(name) => lookup(name).ok_or_else(|| format!("unknown colour token `${name}`")),
            Self::Mix(a, b, percent) => Ok(b.solid_by(lookup)?.mix(a.solid_by(lookup)?, percent / 100.0)),
            Self::Pulse(..) => Err("pulse() can only be used as a whole value, not inside another colour".to_owned()),
        }
    }

    /// The error for a `pulse()` nested inside another colour, which no lookup can resolve.
    pub(crate) fn nested_pulse(&self) -> Option<String> {
        let inner = match self {
            Self::Pulse(a, b) | Self::Mix(a, b, _) => [a, b],
            Self::Hex(_) | Self::Token(_) => return None,
        };
        if inner.iter().any(|expr| expr.contains_pulse()) {
            Some("pulse() can only be used as a whole value, not inside another colour".to_owned())
        } else {
            None
        }
    }

    fn contains_pulse(&self) -> bool {
        match self {
            Self::Pulse(..) => true,
            Self::Mix(a, b, _) => a.contains_pulse() || b.contains_pulse(),
            Self::Hex(_) | Self::Token(_) => false,
        }
    }

    /// Names of the tokens this expression reads.
    pub(crate) fn tokens(&self) -> Vec<&str> {
        match self {
            Self::Hex(_) => Vec::new(),
            Self::Token(name) => vec![name.as_str()],
            Self::Mix(a, b, _) | Self::Pulse(a, b) => {
                let mut names = a.tokens();
                names.extend(b.tokens());
                names
            }
        }
    }
}

/// How deeply `mix()` and `pulse()` may nest. Real themes use two or three levels; the limit keeps
/// a hostile file from recursing the parser into a stack overflow.
const MAX_NESTING: usize = 16;

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn skip_space(&mut self) {
        let trimmed = self.rest().trim_start();
        self.pos = self.text.len() - trimmed.len();
    }

    fn eat(&mut self, literal: &str) -> bool {
        self.skip_space();
        if self.rest().starts_with(literal) {
            self.pos += literal.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, literal: &str) -> Result<(), String> {
        if self.eat(literal) { Ok(()) } else { Err(format!("expected `{literal}` in colour `{}`", self.text)) }
    }

    fn take_while(&mut self, keep: impl Fn(char) -> bool) -> &str {
        let start = self.pos;
        let len = self.rest().find(|c: char| !keep(c)).unwrap_or(self.rest().len());
        self.pos += len;
        &self.text[start..self.pos]
    }

    /// Enters one more level of `mix()` or `pulse()`.
    fn nest(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_NESTING {
            return Err(format!("colour `{}` nests more than {MAX_NESTING} levels of mix() or pulse()", self.text));
        }
        Ok(())
    }

    fn expr(&mut self) -> Result<Expr, String> {
        if self.eat("$") {
            let name = self.take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if name.is_empty() {
                return Err(format!("missing token name after `$` in colour `{}`", self.text));
            }
            return Ok(Expr::Token(name.to_owned()));
        }
        if self.eat("#") {
            let digits = self.take_while(|c| c.is_ascii_hexdigit()).to_owned();
            return Rgb::parse_hex(&format!("#{digits}"))
                .map(Expr::Hex)
                .ok_or_else(|| format!("`#{digits}` is not a valid colour; use #RRGGBB or #RGB"));
        }
        if self.eat("mix(") {
            self.nest()?;
            let a = self.expr()?;
            self.expect(",")?;
            let b = self.expr()?;
            self.expect(",")?;
            self.skip_space();
            let number = self.take_while(|c| c.is_ascii_digit() || c == '.').to_owned();
            let percent: f32 =
                number.parse().map_err(|_| format!("mix() needs a percentage like 30% in colour `{}`", self.text))?;
            if !(0.0..=100.0).contains(&percent) {
                return Err(format!("mix() percentage must be between 0% and 100%, got {number}%"));
            }
            self.expect("%")?;
            self.expect(")")?;
            self.depth -= 1;
            return Ok(Expr::Mix(Box::new(a), Box::new(b), percent));
        }
        if self.eat("pulse(") {
            self.nest()?;
            let a = self.expr()?;
            self.expect(",")?;
            let b = self.expr()?;
            self.expect(")")?;
            self.depth -= 1;
            return Ok(Expr::Pulse(Box::new(a), Box::new(b)));
        }
        Err(format!("`{}` is not a colour; use $token, #RRGGBB, mix(a, b, N%) or pulse(a, b)", self.text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens() -> BTreeMap<String, Rgb> {
        BTreeMap::from([("accent".to_owned(), Rgb::new(255, 255, 255)), ("surface".to_owned(), Rgb::new(0, 0, 0))])
    }

    #[test]
    fn parses_every_form() {
        assert_eq!(Expr::parse("#fff"), Ok(Expr::Hex(Rgb::new(255, 255, 255))));
        assert_eq!(Expr::parse(" $accent-2 "), Ok(Expr::Token("accent-2".to_owned())));
        let mix = Expr::parse("mix($accent, $surface, 34%)").expect("valid mix");
        assert_eq!(mix.tokens(), vec!["accent", "surface"]);
        assert!(matches!(Expr::parse("pulse($accent, #000)"), Ok(Expr::Pulse(..))));
    }

    #[test]
    fn explains_bad_input() {
        assert_eq!(Expr::parse("#38BDZ8").err().as_deref(), Some("`#38BD` is not a valid colour; use #RRGGBB or #RGB"));
        assert!(Expr::parse("mix($a, $b)").is_err());
        assert!(Expr::parse("mix($a, $b, 140%)").is_err());
        assert!(Expr::parse("red").is_err());
        assert!(Expr::parse("$accent extra").is_err());
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_stack_overflow() {
        let nested = |levels: usize| format!("{}$accent{}", "mix(".repeat(levels), ", $surface, 50%)".repeat(levels));
        assert!(Expr::parse(&nested(MAX_NESTING)).is_ok());
        assert!(Expr::parse(&nested(MAX_NESTING + 1)).is_err_and(|message| message.contains("nests more than")));
        assert!(Expr::parse(&nested(200_000)).is_err());
    }

    #[test]
    fn resolves_mix_as_share_of_first_colour() {
        let paint = Expr::parse("mix($accent, $surface, 25%)").and_then(|e| e.resolve(&tokens())).expect("resolves");
        assert_eq!(paint, Paint::Solid(Rgb::new(64, 64, 64)));
    }

    #[test]
    fn pulse_only_at_top_level() {
        let nested = Expr::parse("mix(pulse($accent, $surface), $surface, 50%)").expect("parses");
        assert!(nested.resolve(&tokens()).is_err());
        let unknown = Expr::parse("$missing").expect("parses");
        assert_eq!(unknown.resolve(&tokens()).err().as_deref(), Some("unknown colour token `$missing`"));
    }

    #[test]
    fn pulse_breathes_between_ends() {
        let paint = Paint::Pulse(Rgb::new(0, 0, 0), Rgb::new(200, 200, 200));
        assert_eq!(paint.at(0.0), Rgb::new(0, 0, 0));
        assert_eq!(paint.at(0.5), Rgb::new(200, 200, 200));
        assert_eq!(paint.at(1.0), Rgb::new(0, 0, 0));
        assert!(paint.is_animated());
        assert!(!Paint::Solid(Rgb::new(1, 2, 3)).is_animated());
    }
}
