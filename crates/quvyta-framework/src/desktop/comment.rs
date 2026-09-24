//! What a kind of file is called in words: "Rust source code" beside `text/x-rust`, from the
//! `<comment>` lines of shared-mime-info's `mime/<kind>.xml` files.
//!
//! `update-mime-database` writes those files in one fixed shape, one element to a line, so a
//! reader of that shape is enough and no XML parser is pulled in for one sentence.

use std::path::Path;

use super::mime::MimeDb;
use super::read_small;
use crate::diagnostics::{Diagnostic, Location};

/// One `<comment>` of a kind's file: its language, when it has one, and its words.
type Comment = (Option<String>, String);

impl MimeDb {
    /// What `mime` is called in words in `lang`: "Rust source code" for `text/x-rust`.
    ///
    /// `lang` is a language code as a locale gives it (`tr`, `pt-BR`, or `tr_TR.UTF-8` as in
    /// `LANG`). The comment in that language wins, then the one in the language without its region
    /// (`pt` for `pt-BR`), then the one with no language. An alias is followed to the kind's own
    /// name first, and the first data folder that describes the kind wins, the person's own
    /// before the system's.
    ///
    /// `None` when no folder describes the kind, or it is not a plain `group/name`: nothing
    /// outside the `mime` folders is ever read. A file that cannot be read is left out quietly;
    /// [`MimeDb::comment_with_diagnostics`] says why.
    #[must_use]
    pub fn comment(&self, mime: &str, lang: &str) -> Option<String> {
        self.comment_with_diagnostics(mime, lang, &mut Vec::new())
    }

    /// [`MimeDb::comment`], with every problem found in the kind's files added to `diagnostics`:
    /// a file that is not UTF-8, a `<comment>` never closed, a file too large or not a regular
    /// file. A broken file is skipped, never a panic, and a less important folder may still
    /// describe the kind.
    #[must_use]
    pub fn comment_with_diagnostics(
        &self,
        mime: &str,
        lang: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        let mime = self.canonical(mime);
        // A kind is two plain words around one slash; anything else could reach outside the folder.
        let (group, name) = mime.split_once('/')?;
        if !plain(group) || !plain(name) {
            return None;
        }
        let comments = self.data.iter().find_map(|dir| {
            let file = dir.join("mime").join(group).join(format!("{name}.xml"));
            let bytes = read_small(&file, diagnostics)?;
            let comments = comments(&bytes, &file, diagnostics);
            (!comments.is_empty()).then_some(comments)
        })?;
        in_language(&comments, lang)
    }
}

/// Whether a part of a kind's name is a plain word that stays inside its folder.
fn plain(part: &str) -> bool {
    !part.is_empty() && !part.starts_with('.') && part.chars().all(|c| c.is_ascii_alphanumeric() || "+-._".contains(c))
}

/// The words in `lang`, then in its base language, then with no language.
fn in_language(comments: &[Comment], lang: &str) -> Option<String> {
    let lang = normal(lang);
    let base = lang.split('-').next().unwrap_or(&lang).to_owned();
    let find = |wanted: Option<&str>| {
        comments
            .iter()
            .find(|(said, _)| said.as_deref().map(normal).as_deref() == wanted)
            .map(|(_, words)| words.clone())
    };
    find(Some(&lang)).or_else(|| find(Some(&base))).or_else(|| find(None))
}

/// A language code in one spelling: `tr_TR.UTF-8` and `tr-tr` are both `tr-tr`.
fn normal(code: &str) -> String {
    let code = code.split(['.', '@']).next().unwrap_or(code);
    code.replace('_', "-").to_ascii_lowercase()
}

/// Every `<comment>` of a kind's file, in order. A file that is not UTF-8 gives none; a comment
/// that is never closed ends the reading there, and what came before it is kept.
fn comments(bytes: &[u8], file: &Path, diagnostics: &mut Vec<Diagnostic>) -> Vec<Comment> {
    let name = file.display().to_string();
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let good = std::str::from_utf8(&bytes[..error.valid_up_to()]).unwrap_or_default();
            let location = Location::from_offset(&name, good, good.len());
            diagnostics.push(Diagnostic::warning(Some(location), "the file is not UTF-8; it is skipped"));
            return Vec::new();
        }
    };
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(start) = text[at..].find("<comment").map(|offset| at + offset) {
        at = start + "<comment".len();
        // `<commentary>` is another element; only `<comment>` and `<comment xml:lang=..>` count.
        if !text[at..].starts_with(['>', ' ', '\t', '\n', '\r']) {
            continue;
        }
        let closed = text[at..].find('>').map(|offset| at + offset).and_then(|close| {
            let end = text[close + 1..].find("</comment>").map(|offset| close + 1 + offset)?;
            Some((close, end))
        });
        let Some((close, end)) = closed else {
            let location = Location::from_offset(&name, text, start);
            diagnostics.push(Diagnostic::warning(Some(location), "a <comment> is never closed; the rest is skipped"));
            break;
        };
        let lang = attribute(&text[at..close], "xml:lang");
        let words = unescape(text[close + 1..end].trim());
        at = end + "</comment>".len();
        if !words.is_empty() {
            found.push((lang, words));
        }
    }
    found
}

/// The value of attribute `key` among `attributes`, in double or single quotes.
fn attribute(attributes: &str, key: &str) -> Option<String> {
    let mut rest = attributes;
    while let Some(index) = rest.find(key) {
        let after = rest[index + key.len()..].trim_start();
        rest = &rest[index + key.len()..];
        let Some(value) = after.strip_prefix('=') else { continue };
        let value = value.trim_start();
        let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        let value = &value[1..];
        return value.find(quote).map(|end| value[..end].to_owned());
    }
    None
}

/// Text with XML's character references turned back: the five named ones and `&#NN;` / `&#xNN;`.
/// One that is not known stays as written.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest.find(';').and_then(|semi| {
            let c = match &rest[1..semi] {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                code => {
                    let number = code.strip_prefix('#')?;
                    let value = match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => number.parse().ok()?,
                    };
                    char::from_u32(value)?
                }
            };
            Some((c, semi + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::fixture::{self, Tree};

    /// A database whose system data folder describes Markdown, with an alias to it.
    fn markdown(tree: &Tree) -> MimeDb {
        tree.write("usr/mime/text/markdown.xml", fixture::MARKDOWN_XML);
        tree.write("usr/mime/aliases", "text/x-markdown text/markdown\n");
        MimeDb::load(&tree.dirs())
    }

    #[test]
    fn a_kind_is_described_in_the_language_asked_for_then_without_its_region_then_plainly() {
        let tree = Tree::new();
        let db = markdown(&tree);
        assert_eq!(db.comment("text/markdown", "pt-BR").as_deref(), Some("Documento Markdown"));
        assert_eq!(db.comment("text/markdown", "pt_BR.UTF-8").as_deref(), Some("Documento Markdown"));
        assert_eq!(db.comment("text/markdown", "pt-PT").as_deref(), Some("Documento Markdown (pt)"), "the base");
        assert_eq!(db.comment("text/markdown", "tr_TR.UTF-8").as_deref(), Some("Markdown belgesi & not"));
        assert_eq!(db.comment("text/markdown", "de").as_deref(), Some("Markdown document"), "no language");
        assert_eq!(db.comment("text/markdown", "sr-Cyrl").as_deref(), Some("Маркдаун документ"));
        assert_eq!(db.comment("text/x-nothing", "en"), None, "a kind no folder describes");
    }

    #[test]
    fn an_alias_is_followed_to_the_kind_it_names() {
        let tree = Tree::new();
        let db = markdown(&tree);
        assert_eq!(db.comment("text/x-markdown", "tr").as_deref(), Some("Markdown belgesi & not"));
    }

    #[test]
    fn the_persons_own_description_comes_before_the_systems() {
        let tree = Tree::new();
        tree.write("usr/mime/text/markdown.xml", fixture::MARKDOWN_XML);
        tree.write("home/data/mime/text/markdown.xml", "<comment>My notes</comment>\n");
        let db = MimeDb::load(&tree.dirs());
        assert_eq!(db.comment("text/markdown", "en").as_deref(), Some("My notes"));
    }

    #[test]
    fn a_kind_never_reaches_outside_the_mime_folders() {
        let tree = Tree::new();
        tree.write("usr/secret.xml", "<comment>outside</comment>");
        let db = MimeDb::load(&tree.dirs());
        assert_eq!(db.comment("../secret", "en"), None);
        assert_eq!(db.comment("mime/../../secret", "en"), None);
        assert_eq!(db.comment("text/..", "en"), None);
    }

    #[test]
    fn references_are_turned_back_and_unknown_ones_stay() {
        assert_eq!(unescape("a &lt;b&gt; &quot;c&quot; &apos;d&apos; &amp;amp;"), "a <b> \"c\" 'd' &amp;");
        assert_eq!(unescape("&#233;t&#xE9; &nope; & alone"), "été &nope; & alone");
    }

    #[test]
    fn a_broken_file_gives_nothing_and_a_diagnostic_and_a_lower_folder_still_answers() {
        let tree = Tree::new();
        tree.write("home/data/mime/text/markdown.xml", fixture::UNCLOSED_XML);
        tree.write("home/data/mime/text/x-rust.xml", b"<comment>Rust \xff source</comment>\n");
        let db = MimeDb::load(&tree.dirs());
        let mut diagnostics = Vec::new();
        assert_eq!(db.comment_with_diagnostics("text/markdown", "en", &mut diagnostics), None);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].to_string().contains("markdown.xml:2:"), "the line is named: {}", diagnostics[0]);
        let mut diagnostics = Vec::new();
        assert_eq!(db.comment_with_diagnostics("text/x-rust", "en", &mut diagnostics), None);
        assert!(diagnostics[0].to_string().contains("not UTF-8"), "{}", diagnostics[0]);

        tree.write("usr/mime/text/markdown.xml", fixture::MARKDOWN_XML);
        let db = MimeDb::load(&tree.dirs());
        assert_eq!(db.comment("text/markdown", "en").as_deref(), Some("Markdown document"), "the system's still does");
    }

    #[test]
    fn a_folder_in_place_of_a_file_is_reported_and_never_read() {
        let tree = Tree::new();
        std::fs::create_dir_all(tree.path("home/data/mime/text/markdown.xml")).expect("the folder can be made");
        let db = MimeDb::load(&tree.dirs());
        let mut diagnostics = Vec::new();
        assert_eq!(db.comment_with_diagnostics("text/markdown", "en", &mut diagnostics), None);
        assert!(diagnostics[0].to_string().contains("not a regular file"), "{diagnostics:?}");
    }

    #[test]
    fn a_similar_element_is_not_a_comment() {
        let tree = Tree::new();
        tree.write(
            "usr/mime/text/plain.xml",
            "<commentary>no</commentary>\n<comment xml:lang='tr'>Düz metin</comment>\n",
        );
        let db = MimeDb::load(&tree.dirs());
        assert_eq!(db.comment("text/plain", "en"), None, "no comment without a language");
        assert_eq!(db.comment("text/plain", "tr").as_deref(), Some("Düz metin"));
    }
}
