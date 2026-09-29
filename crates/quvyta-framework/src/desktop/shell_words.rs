//! Splitting a command line such as `$EDITOR` into its words, the way a POSIX shell would before
//! it runs anything.

/// The words of `line` as a POSIX shell splits them, without expanding anything: blanks separate
/// words, single quotes keep everything up to the next single quote, double quotes keep
/// everything up to the next double quote except that a backslash there escapes `"`, `\`, `$`
/// and `` ` ``, and a backslash outside quotes keeps the next character. `None` when a quote is
/// never closed or the line ends in a lone backslash.
///
/// For a program named in a variable the person set, such as `EDITOR` or `VISUAL`, where a path
/// with a space must stay one word. `$`, `~` and globs are kept as written, since no shell reads
/// the words; a line that needs one should be run through `sh -c` instead.
///
/// ```
/// use qframe::desktop::shell_words;
///
/// assert_eq!(
///     shell_words(r#""/opt/My Editor/bin/ed" -w"#),
///     Some(vec!["/opt/My Editor/bin/ed".to_owned(), "-w".to_owned()]),
/// );
/// assert_eq!(shell_words(r"vim\ x -p"), Some(vec!["vim x".to_owned(), "-p".to_owned()]));
/// assert_eq!(shell_words("code 'unclosed"), None);
/// ```
#[must_use]
pub fn shell_words(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    // A word is open from its first character, even an empty pair of quotes, which is a word.
    let mut word: Option<String> = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => words.extend(word.take()),
            '\'' => {
                let word = word.get_or_insert_with(String::new);
                loop {
                    match chars.next()? {
                        '\'' => break,
                        other => word.push(other),
                    }
                }
            }
            '"' => {
                let word = word.get_or_insert_with(String::new);
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            next @ ('"' | '\\' | '$' | '`') => word.push(next),
                            // A backslash before a line break joins the lines.
                            '\n' => {}
                            next => {
                                word.push('\\');
                                word.push(next);
                            }
                        },
                        other => word.push(other),
                    }
                }
            }
            '\\' => match chars.next()? {
                '\n' => {}
                next => word.get_or_insert_with(String::new).push(next),
            },
            other => word.get_or_insert_with(String::new).push(other),
        }
    }
    words.extend(word);
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Option<Vec<String>> {
        shell_words(line)
    }

    fn some(words: &[&str]) -> Option<Vec<String>> {
        Some(words.iter().map(|word| (*word).to_owned()).collect())
    }

    #[test]
    fn a_quoted_path_with_spaces_is_one_word() {
        assert_eq!(words(r#""/opt/My Editor/bin/ed" -w"#), some(&["/opt/My Editor/bin/ed", "-w"]));
        assert_eq!(words("'/opt/My Editor/ed'  --wait"), some(&["/opt/My Editor/ed", "--wait"]));
    }

    #[test]
    fn escapes_follow_the_shell() {
        assert_eq!(words(r"a\ b c"), some(&["a b", "c"]));
        assert_eq!(words(r#"'x "y"'"#), some(&[r#"x "y""#]));
        assert_eq!(words(r#""a \"b\" \n""#), some(&[r#"a "b" \n"#]), "only four escapes inside double quotes");
        assert_eq!(words(r"'a\b'"), some(&[r"a\b"]), "nothing escapes inside single quotes");
        assert_eq!(words("ed'it'or\"s\""), some(&["editors"]), "quotes join into one word");
    }

    #[test]
    fn empty_quotes_are_a_word_and_blanks_are_not() {
        assert_eq!(words("prog '' x"), some(&["prog", "", "x"]));
        assert_eq!(words("   "), some(&[]));
        assert_eq!(words(""), some(&[]));
    }

    #[test]
    fn nothing_is_expanded() {
        assert_eq!(words("$HOME/bin/ed ~/notes *.md"), some(&["$HOME/bin/ed", "~/notes", "*.md"]));
    }

    #[test]
    fn an_unclosed_quote_or_a_trailing_backslash_is_no_command() {
        assert_eq!(words("code 'unclosed"), None);
        assert_eq!(words("code \"unclosed"), None);
        assert_eq!(words("code \\"), None);
    }
}
