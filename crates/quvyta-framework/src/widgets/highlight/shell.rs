//! Shell scripts in bash syntax: comments, quoting, expansions, keywords, function definitions
//! and here-documents.

use std::ops::Range;

use super::{Scanner, Token, is_ident, is_ident_start};

const KEYWORDS: [&str; 20] = [
    "if", "then", "elif", "else", "fi", "for", "while", "until", "do", "done", "case", "esac", "in", "function",
    "return", "local", "export", "declare", "readonly", "select",
];

/// Characters that end an unquoted word.
fn ends_word(c: char) -> bool {
    c.is_whitespace() || matches!(c, ';' | '|' | '&' | '<' | '>' | '(' | ')' | '\'' | '"' | '$' | '`' | '{' | '}')
}

/// The one-character parameters: `$1`, `$@`, `$?` and their kin.
fn is_special_parameter(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '@' | '*' | '#' | '?' | '$' | '!' | '-')
}

/// Whether the `$` under the scanner starts an expansion rather than standing for itself.
fn expands(s: &Scanner<'_>) -> bool {
    s.peek_at(1).is_some_and(|c| matches!(c, '(' | '{') || is_ident_start(c) || is_special_parameter(c))
}

/// Consumes up to the `close` that balances the `open` under the scanner, stepping over quoted
/// text and escapes, or to the end when it never comes.
fn balanced(s: &mut Scanner<'_>, open: char, close: char) {
    let mut depth = 0usize;
    while let Some(c) = s.peek() {
        match c {
            '\\' => {
                s.bump();
                s.bump();
                continue;
            }
            '\'' | '"' if open == '(' => {
                s.quoted(c);
                continue;
            }
            _ => {}
        }
        s.bump();
        if c == open {
            depth += 1;
        } else if c == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                break;
            }
        }
    }
}

/// Consumes a whole expansion starting at its `$`: `$name`, `$1`, `${…}` or `$(…)`.
fn expansion(s: &mut Scanner<'_>) {
    s.bump();
    match s.peek() {
        Some('{') => balanced(s, '{', '}'),
        Some('(') => balanced(s, '(', ')'),
        Some(c) if is_ident_start(c) => s.eat_while(is_ident),
        Some(_) => {
            s.bump();
        }
        None => {}
    }
}

/// A single-quoted string, where nothing is special, not even a backslash.
fn single_quoted(s: &mut Scanner<'_>) {
    s.bump();
    s.pos = s.text[s.pos..].find('\'').map_or(s.text.len(), |end| s.pos + end + 1);
}

/// A double-quoted string: the quoted text is a string, the expansions inside it variables.
fn double_quoted(s: &mut Scanner<'_>, out: &mut Vec<(Range<usize>, Token)>) {
    let mut piece = s.pos;
    s.bump();
    while let Some(c) = s.peek() {
        match c {
            '\\' => {
                s.bump();
                s.bump();
            }
            '"' => {
                s.bump();
                break;
            }
            '$' if expands(s) => {
                out.push((piece..s.pos, Token::String));
                let start = s.pos;
                expansion(s);
                out.push((start..s.pos, Token::Variable));
                piece = s.pos;
            }
            _ => {
                s.bump();
            }
        }
    }
    out.push((piece..s.pos, Token::String));
}

/// Reads the delimiter of a here-document after `<<` or `<<-`, pushing it as a string. Returns
/// the word its terminator line must equal.
fn heredoc_delimiter(s: &mut Scanner<'_>, out: &mut Vec<(Range<usize>, Token)>) -> Option<String> {
    s.eat_while(|c| c == ' ' || c == '\t');
    let start = s.pos;
    let word = match s.peek() {
        Some(quote @ ('\'' | '"')) => {
            if quote == '\'' {
                single_quoted(s);
            } else {
                s.quoted(quote);
            }
            s.text[start..s.pos].trim_matches(quote).to_owned()
        }
        _ => {
            s.eat_while(|c| !ends_word(c) || c == '\'' || c == '"');
            s.text[start..s.pos].replace(['\\', '\'', '"'], "")
        }
    };
    out.push((start..s.pos, Token::String));
    (!word.is_empty()).then_some(word)
}

/// Consumes the body of a here-document from the start of its first line through its
/// terminator line, which is left without its newline.
fn heredoc_body(s: &mut Scanner<'_>, delimiter: &str, strip_tabs: bool, out: &mut Vec<(Range<usize>, Token)>) {
    let start = s.pos;
    let len = s.text.len();
    while s.pos < len {
        let end = s.text[s.pos..].find('\n').map_or(len, |offset| s.pos + offset);
        let line = s.text[s.pos..end].trim_end_matches('\r');
        let line = if strip_tabs { line.trim_start_matches('\t') } else { line };
        let terminates = line == delimiter;
        s.pos = end;
        if terminates || s.pos == len {
            break;
        }
        s.pos += 1;
    }
    out.push((start..s.pos, Token::String));
}

/// Consumes an unquoted word, with the escapes inside it, and returns its text.
fn word<'a>(s: &mut Scanner<'a>) -> &'a str {
    let start = s.pos;
    while let Some(c) = s.peek() {
        if c == '\\' {
            if s.peek_at(1) == Some('\n') {
                break;
            }
            s.bump();
            s.bump();
        } else if ends_word(c) {
            break;
        } else {
            s.bump();
        }
    }
    &s.text[start..s.pos]
}

pub(super) fn shell(code: &str) -> Vec<(Range<usize>, Token)> {
    let mut s = Scanner { text: code, pos: 0 };
    let mut out = Vec::new();
    // Whether each open parenthesis began a command substitution, so its `)` is coloured alike.
    let mut parens: Vec<bool> = Vec::new();
    let mut heredocs: Vec<(String, bool)> = Vec::new();
    // Whether the next word is where a command starts: assignments and function definitions
    // are only recognised there.
    let mut command_start = true;
    let mut names_function = false;
    while let Some(c) = s.peek() {
        let start = s.pos;
        match c {
            '\n' => {
                s.bump();
                command_start = true;
                for (index, (delimiter, strip_tabs)) in std::mem::take(&mut heredocs).into_iter().enumerate() {
                    if index > 0 && s.peek() == Some('\n') {
                        s.bump();
                    }
                    heredoc_body(&mut s, &delimiter, strip_tabs, &mut out);
                }
                continue;
            }
            c if c.is_whitespace() => {
                s.bump();
                continue;
            }
            '\\' if s.peek_at(1) == Some('\n') => {
                s.bump();
                s.bump();
                continue;
            }
            '#' => {
                s.skip_line();
                out.push((start..s.pos, Token::Comment));
                continue;
            }
            '\'' => {
                single_quoted(&mut s);
                out.push((start..s.pos, Token::String));
            }
            '"' => double_quoted(&mut s, &mut out),
            '$' if s.peek_at(1) == Some('(') => {
                s.bump();
                s.bump();
                out.push((start..s.pos, Token::Variable));
                parens.push(true);
                command_start = true;
                continue;
            }
            '$' if s.peek_at(1) == Some('\'') => {
                s.bump();
                s.quoted('\'');
                out.push((start..s.pos, Token::String));
            }
            '$' if expands(&s) => {
                expansion(&mut s);
                out.push((start..s.pos, Token::Variable));
            }
            '(' | '{' => {
                s.bump();
                if c == '(' {
                    parens.push(false);
                }
                out.push((start..s.pos, Token::Punctuation));
                command_start = true;
                continue;
            }
            ')' => {
                s.bump();
                let token = if parens.pop() == Some(true) { Token::Variable } else { Token::Punctuation };
                out.push((start..s.pos, token));
            }
            ';' | '|' | '&' => {
                s.eat_while(|c| matches!(c, ';' | '|' | '&'));
                out.push((start..s.pos, Token::Punctuation));
                command_start = true;
                continue;
            }
            '<' if s.starts_with("<<") && !s.starts_with("<<<") => {
                s.bump();
                s.bump();
                let strip_tabs = s.peek() == Some('-');
                if strip_tabs {
                    s.bump();
                }
                out.push((start..s.pos, Token::Punctuation));
                if let Some(delimiter) = heredoc_delimiter(&mut s, &mut out) {
                    heredocs.push((delimiter, strip_tabs));
                }
            }
            '<' | '>' | '`' | '}' | '$' => {
                s.bump();
                if c != '`' && c != '}' && c != '$' {
                    s.eat_while(|c| matches!(c, '<' | '>' | '&' | '|'));
                }
                let token = if c == '$' { Token::Plain } else { Token::Punctuation };
                out.push((start..s.pos, token));
            }
            c if command_start && is_ident_start(c) && assignment(&s).is_some() => {
                let (name_end, operator_end) = assignment(&s).unwrap_or((s.pos, s.pos));
                out.push((start..name_end, Token::Variable));
                out.push((name_end..operator_end, Token::Punctuation));
                s.pos = operator_end;
                command_start = false;
                continue;
            }
            _ => {
                let text = word(&mut s);
                if text.is_empty() {
                    s.bump();
                    continue;
                }
                let rest = s.text[s.pos..].trim_start_matches([' ', '\t']);
                let token = if names_function {
                    names_function = false;
                    Token::Function
                } else if KEYWORDS.contains(&text) {
                    names_function = text == "function";
                    command_start = text != "in";
                    out.push((start..s.pos, Token::Keyword));
                    continue;
                } else if text.starts_with(|c: char| c.is_ascii_digit())
                    && text.chars().all(|c| c.is_ascii_digit() || c == '.')
                {
                    Token::Number
                } else if command_start && text.chars().all(is_ident) && rest.starts_with("()") {
                    Token::Function
                } else {
                    Token::Plain
                };
                out.push((start..s.pos, token));
            }
        }
        command_start = false;
    }
    out
}

/// Where the name and the `=` or `+=` of an assignment such as `pkgver=1.2` end, when one
/// starts under the scanner.
fn assignment(s: &Scanner<'_>) -> Option<(usize, usize)> {
    let rest = &s.text[s.pos..];
    let name = rest.find(|c: char| !is_ident(c)).unwrap_or(rest.len());
    let after = &rest[name..];
    let operator = if after.starts_with('=') {
        1
    } else if after.starts_with("+=") {
        2
    } else {
        return None;
    };
    Some((s.pos + name, s.pos + name + operator))
}
