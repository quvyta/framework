//! Syntax highlighting for code shown in the terminal: Rust and TOML.

use std::ops::Range;

/// A language [`CodeView`](crate::widgets::CodeView) can colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Rust source.
    Rust,
    /// TOML files such as themes and locales.
    Toml,
    /// No highlighting.
    Plain,
}

impl Language {
    /// The language for a fenced code block tag such as `rust` or `toml`.
    #[must_use]
    pub fn from_tag(tag: &str) -> Self {
        match tag.trim().to_ascii_lowercase().as_str() {
            "rust" | "rs" => Self::Rust,
            "toml" => Self::Toml,
            _ => Self::Plain,
        }
    }
}

/// What a highlighted piece of code is; also the theme variant of `code-token`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token {
    Keyword,
    Type,
    Function,
    Macro,
    String,
    Number,
    Comment,
    Attribute,
    Lifetime,
    Punctuation,
    Table,
    Key,
    Plain,
}

impl Token {
    pub(crate) fn variant(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Type => "type",
            Self::Function => "function",
            Self::Macro => "macro",
            Self::String => "string",
            Self::Number => "number",
            Self::Comment => "comment",
            Self::Attribute => "attribute",
            Self::Lifetime => "lifetime",
            Self::Punctuation => "punctuation",
            Self::Table => "table",
            Self::Key => "key",
            Self::Plain => "plain",
        }
    }
}

const RUST_KEYWORDS: [&str; 41] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "yield", "gen", "try",
];

/// Splits `code` into highlighted byte ranges covering the whole text.
pub(crate) fn highlight(code: &str, language: Language) -> Vec<(Range<usize>, Token)> {
    let tokens = match language {
        Language::Rust => rust(code),
        Language::Toml => toml(code),
        Language::Plain => vec![(0..code.len(), Token::Plain)],
    };
    fill_gaps(code.len(), tokens)
}

fn fill_gaps(len: usize, tokens: Vec<(Range<usize>, Token)>) -> Vec<(Range<usize>, Token)> {
    let mut out = Vec::with_capacity(tokens.len() * 2);
    let mut position = 0;
    for (range, token) in tokens {
        if range.start > position {
            out.push((position..range.start, Token::Plain));
        }
        if range.end > range.start {
            position = range.end;
            out.push((range, token));
        }
    }
    if position < len {
        out.push((position..len, Token::Plain));
    }
    out
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.text[self.pos..].chars().nth(n)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.text[self.pos..].starts_with(s)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat_while(&mut self, keep: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&keep) {
            self.bump();
        }
    }

    fn skip_line(&mut self) {
        self.eat_while(|c| c != '\n');
    }

    /// Consumes a quoted string starting at the opening quote, honouring backslash escapes.
    fn quoted(&mut self, quote: char) {
        self.bump();
        while let Some(c) = self.bump() {
            if c == '\\' {
                self.bump();
            } else if c == quote {
                break;
            }
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn rust(code: &str) -> Vec<(Range<usize>, Token)> {
    let mut s = Scanner { text: code, pos: 0 };
    let mut out = Vec::new();
    let mut previous_word = String::new();
    while let Some(c) = s.peek() {
        let start = s.pos;
        if s.starts_with("//") {
            s.skip_line();
            out.push((start..s.pos, Token::Comment));
        } else if s.starts_with("/*") {
            match code[s.pos + 2..].find("*/") {
                Some(end) => s.pos += 2 + end + 2,
                None => s.pos = code.len(),
            }
            out.push((start..s.pos, Token::Comment));
        } else if s.starts_with("#[") || s.starts_with("#![") {
            let mut depth = 0;
            while let Some(c) = s.bump() {
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    '\n' => break,
                    _ => {}
                }
            }
            out.push((start..s.pos, Token::Attribute));
        } else if c == '"' || ((c == 'b') && s.peek_at(1) == Some('"')) {
            if c == 'b' {
                s.bump();
            }
            s.quoted('"');
            out.push((start..s.pos, Token::String));
        } else if c == 'r' && (s.peek_at(1) == Some('"') || (s.peek_at(1) == Some('#') && s.peek_at(2) != Some('['))) {
            s.bump();
            let mut hashes = 0;
            while s.peek() == Some('#') {
                s.bump();
                hashes += 1;
            }
            if s.peek() == Some('"') {
                let closing = format!("\"{}", "#".repeat(hashes));
                s.bump();
                match code[s.pos..].find(&closing) {
                    Some(end) => s.pos += end + closing.len(),
                    None => s.pos = code.len(),
                }
                out.push((start..s.pos, Token::String));
            } else {
                s.eat_while(is_ident);
            }
        } else if c == '\'' {
            let is_char = s.peek_at(1) == Some('\\') || s.peek_at(2) == Some('\'');
            if is_char {
                s.quoted('\'');
                out.push((start..s.pos, Token::String));
            } else {
                s.bump();
                s.eat_while(is_ident);
                out.push((start..s.pos, Token::Lifetime));
            }
        } else if c.is_ascii_digit() {
            s.eat_while(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
            if code[start..s.pos].ends_with('.') {
                s.pos -= 1;
            }
            out.push((start..s.pos, Token::Number));
        } else if is_ident_start(c) {
            s.eat_while(is_ident);
            let word = &code[start..s.pos];
            let token = if RUST_KEYWORDS.contains(&word) {
                Token::Keyword
            } else if s.peek() == Some('!') && s.peek_at(1) != Some('=') {
                s.bump();
                Token::Macro
            } else if previous_word == "fn" || s.peek() == Some('(') || s.starts_with("::<") {
                Token::Function
            } else if word.chars().next().is_some_and(char::is_uppercase) {
                Token::Type
            } else {
                Token::Plain
            };
            previous_word = word.to_owned();
            out.push((start..s.pos, token));
            continue;
        } else if c.is_whitespace() {
            s.bump();
            continue;
        } else {
            s.bump();
            out.push((start..s.pos, Token::Punctuation));
        }
        previous_word.clear();
    }
    out
}

fn toml(code: &str) -> Vec<(Range<usize>, Token)> {
    let mut out = Vec::new();
    let mut line_start = 0;
    for line in code.split_inclusive('\n') {
        let mut s = Scanner { text: code, pos: line_start };
        let end = line_start + line.len();
        s.eat_while(|c| c == ' ' || c == '\t');
        if s.peek() == Some('[') {
            let header_start = s.pos;
            while s.pos < end && s.peek() != Some(']') {
                if s.peek() == Some('"') {
                    s.quoted('"');
                } else {
                    s.bump();
                }
            }
            s.eat_while(|c| c == ']');
            out.push((header_start..s.pos, Token::Table));
        } else if s.peek().is_some_and(|c| is_ident(c) || c == '"' || c == '-') {
            let key_start = s.pos;
            while s.pos < end && !matches!(s.peek(), Some('=') | Some('\n') | Some('#')) {
                if s.peek() == Some('"') {
                    s.quoted('"');
                } else {
                    s.bump();
                }
            }
            let key_end = key_start + code[key_start..s.pos].trim_end().len();
            if s.peek() == Some('=') {
                out.push((key_start..key_end, Token::Key));
            } else {
                s.pos = key_start;
            }
        }
        while s.pos < end {
            let start = s.pos;
            match s.peek() {
                Some('#') => {
                    s.skip_line();
                    out.push((start..s.pos, Token::Comment));
                }
                Some(q @ ('"' | '\'')) => {
                    s.quoted(q);
                    out.push((start..s.pos.min(end), Token::String));
                    s.pos = s.pos.min(end);
                }
                Some(c) if c.is_ascii_digit() || (c == '-' && s.peek_at(1).is_some_and(|d| d.is_ascii_digit())) => {
                    s.bump();
                    s.eat_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-'));
                    out.push((start..s.pos, Token::Number));
                }
                Some(c) if is_ident_start(c) => {
                    s.eat_while(is_ident);
                    let token = match &code[start..s.pos] {
                        "true" | "false" => Token::Keyword,
                        _ => Token::Plain,
                    };
                    out.push((start..s.pos, token));
                }
                Some(c) if c.is_whitespace() => {
                    s.bump();
                }
                Some(_) => {
                    s.bump();
                    out.push((start..s.pos, Token::Punctuation));
                }
                None => break,
            }
        }
        line_start = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(code: &str, language: Language) -> Vec<(&str, Token)> {
        highlight(code, language)
            .into_iter()
            .filter(|(r, t)| *t != Token::Plain || !code[r.clone()].trim().is_empty())
            .map(|(r, t)| (&code[r], t))
            .collect()
    }

    #[test]
    fn rust_tokens() {
        let code = "#[derive(Debug)]\nfn view(&self, ui: &mut View<'_, Msg>) {\n    ui.add(Text::new(t!(\"hi\")) // note\n        .width(3));\n}";
        let tokens = kinds(code, Language::Rust);
        assert!(tokens.contains(&("#[derive(Debug)]", Token::Attribute)));
        assert!(tokens.contains(&("fn", Token::Keyword)));
        assert!(tokens.contains(&("view", Token::Function)));
        assert!(tokens.contains(&("View", Token::Type)));
        assert!(tokens.contains(&("'_", Token::Lifetime)));
        assert!(tokens.contains(&("t!", Token::Macro)));
        assert!(tokens.contains(&("\"hi\"", Token::String)));
        assert!(tokens.contains(&("// note", Token::Comment)));
        assert!(tokens.contains(&("3", Token::Number)));
        assert!(tokens.contains(&("new", Token::Function)));
    }

    #[test]
    fn toml_tokens() {
        let code = "[style.\"button:hover\"]\nbg = \"$raised\" # surface\npadding = [0, 2]\nslide = true\n";
        let tokens = kinds(code, Language::Toml);
        assert!(tokens.contains(&("[style.\"button:hover\"]", Token::Table)));
        assert!(tokens.contains(&("bg", Token::Key)));
        assert!(tokens.contains(&("\"$raised\"", Token::String)));
        assert!(tokens.contains(&("# surface", Token::Comment)));
        assert!(tokens.contains(&("2", Token::Number)));
        assert!(tokens.contains(&("true", Token::Keyword)));
    }

    #[test]
    fn ranges_cover_the_whole_text() {
        let code = "let x = 'a'; r#\"raw\"#";
        let ranges = highlight(code, Language::Rust);
        let joined: String = ranges.iter().map(|(r, _)| &code[r.clone()]).collect();
        assert_eq!(joined, code);
        assert!(ranges.iter().any(|(r, t)| &code[r.clone()] == "r#\"raw\"#" && *t == Token::String));
    }
}
