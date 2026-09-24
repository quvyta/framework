//! A desktop entry's `Exec` line turned into a command for one file.
//!
//! The line is split into arguments here and never handed to a shell, so no file name, however
//! strange, can change what is run: the file is always one argument of its own.

use std::ffi::{OsStr, OsString};
use std::path::Path;

#[cfg(test)]
mod tests;

/// What the field codes of an `Exec` line stand for.
pub(super) struct Fields<'a> {
    /// The file being opened (`%f %F %u %U`).
    pub(super) file: &'a Path,
    /// The program's name in the person's language (`%c`).
    pub(super) name: &'a str,
    /// The program's icon (`%i`).
    pub(super) icon: Option<&'a str>,
    /// The desktop entry's own path (`%k`).
    pub(super) entry: &'a Path,
}

/// A run of an argument as written: plain text, or a field code still to be filled in.
#[derive(Debug, PartialEq)]
enum Piece {
    Text(String),
    /// A field code's letter, and whether it stood inside double quotes.
    Code(char, bool),
}

/// One argument as written.
#[derive(Debug, Default)]
struct Word {
    pieces: Vec<Piece>,
    /// Whether any part was quoted: `""` is an argument even though it is empty.
    quoted: bool,
}

impl Word {
    fn text(&mut self, c: char) {
        if let Some(Piece::Text(text)) = self.pieces.last_mut() {
            text.push(c);
        } else {
            self.pieces.push(Piece::Text(c.to_string()));
        }
    }
}

/// The command for opening `fields.file`, program first; `None` when the line is empty or broken
/// (a quote never closed, a `%` at the very end, a field code the standard does not know).
///
/// `exec` is the value with the key file's own escapes already resolved.
pub(super) fn expand(exec: &str, fields: &Fields<'_>) -> Option<Vec<OsString>> {
    let mut args = Vec::new();
    let mut takes_file = false;
    for word in words(exec)? {
        if word.pieces == [Piece::Code('i', false)] {
            if let Some(icon) = fields.icon {
                args.push(OsString::from("--icon"));
                args.push(OsString::from(icon));
            }
            continue;
        }
        let mut arg = OsString::new();
        let mut keep = word.quoted;
        for piece in word.pieces {
            match piece {
                Piece::Text(text) => {
                    arg.push(text);
                    keep = true;
                }
                Piece::Code(code, quoted) => {
                    let value = match code {
                        'f' | 'F' | 'u' | 'U' => {
                            takes_file = true;
                            fields.file.as_os_str().to_owned()
                        }
                        'c' => OsString::from(fields.name),
                        'k' => fields.entry.as_os_str().to_owned(),
                        'i' | 'd' | 'D' | 'n' | 'N' | 'v' | 'm' => continue,
                        _ => return None,
                    };
                    arg.push(if quoted { shell_quoted(&value) } else { value });
                    keep = true;
                }
            }
        }
        if keep {
            args.push(arg);
        }
    }
    if args.first().is_none_or(|program| program.is_empty()) {
        return None;
    }
    // A program that names no file field code still opens a file given to it: the standard says
    // the file then goes last.
    if !takes_file {
        args.push(fields.file.as_os_str().to_owned());
    }
    Some(args)
}

/// The arguments of an `Exec` line as written, or `None` when a quote is never closed or a line
/// ends in a lone `%`.
fn words(exec: &str) -> Option<Vec<Word>> {
    let mut words = Vec::new();
    let mut word: Option<Word> = None;
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => words.extend(word.take()),
            '"' => {
                let word = word.get_or_insert_with(Word::default);
                word.quoted = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => {
                            let next = chars.next()?;
                            if !matches!(next, '"' | '`' | '$' | '\\') {
                                word.text('\\');
                            }
                            word.text(next);
                        }
                        '%' => match chars.next()? {
                            '%' => word.text('%'),
                            code => word.pieces.push(Piece::Code(code, true)),
                        },
                        other => word.text(other),
                    }
                }
            }
            '%' => {
                let word = word.get_or_insert_with(Word::default);
                match chars.next()? {
                    '%' => word.text('%'),
                    code => word.pieces.push(Piece::Code(code, false)),
                }
            }
            other => word.get_or_insert_with(Word::default).text(other),
        }
    }
    words.extend(word);
    Some(words)
}

/// A value in single quotes, the way a shell reads it back as one word.
///
/// A field code inside a quoted argument is, in practice, part of a shell command line
/// (`sh -c "exec viewer %f"`). The value goes in single-quoted, as other desktops do it, so that a
/// quote, a semicolon or a `$` in a file name stays part of the name when that shell reads it.
#[cfg(unix)]
fn shell_quoted(value: &OsStr) -> OsString {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let mut out = vec![b'\''];
    for &byte in value.as_bytes() {
        if byte == b'\'' {
            out.extend_from_slice(b"'\\''");
        } else {
            out.push(byte);
        }
    }
    out.push(b'\'');
    OsString::from_vec(out)
}

/// A value in single quotes, the way a shell reads it back as one word. Off Unix a name that is
/// not Unicode cannot be written byte for byte, so its lossy text is quoted.
#[cfg(not(unix))]
fn shell_quoted(value: &OsStr) -> OsString {
    OsString::from(format!("'{}'", value.to_string_lossy().replace('\'', "'\\''")))
}
