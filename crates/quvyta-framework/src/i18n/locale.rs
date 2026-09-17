//! Reading locale files into flat `section.key` messages.

use std::collections::BTreeMap;

use toml::de::DeTable;

use super::plural::PluralCategory;
use crate::diagnostics::Diagnostic;
use crate::doc::{self, Doc};

/// One piece of a message template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    Text(String),
    Arg(String),
}

/// A message with `{name}` placeholders. `{{` and `}}` write literal braces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Template(pub(crate) Vec<Piece>);

impl Template {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    literal.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    literal.push('}');
                }
                '{' => {
                    let mut name = String::new();
                    let mut closed = false;
                    for c in chars.by_ref() {
                        if c == '}' {
                            closed = true;
                            break;
                        }
                        name.push(c);
                    }
                    let valid = closed
                        && !name.is_empty()
                        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                    if !valid {
                        return Err(format!("invalid placeholder `{{{name}` in `{text}`; write it like {{count}}"));
                    }
                    if !literal.is_empty() {
                        pieces.push(Piece::Text(std::mem::take(&mut literal)));
                    }
                    pieces.push(Piece::Arg(name));
                }
                '}' => {
                    return Err(format!("unmatched `}}` in `{text}`; write }}}} for a literal brace"));
                }
                other => literal.push(other),
            }
        }
        if !literal.is_empty() {
            pieces.push(Piece::Text(literal));
        }
        Ok(Self(pieces))
    }
}

/// A message: plain, or one template per plural category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
    Plain(Template),
    Plural(BTreeMap<PluralCategory, Template>),
}

/// A loaded locale file.
#[derive(Debug, Clone)]
pub(crate) struct Locale {
    pub(crate) code: String,
    pub(crate) name: String,
    pub(crate) fallback: Option<String>,
    pub(crate) messages: BTreeMap<String, Message>,
}

/// Parses a locale file. Returns `None` when `[meta]` with `name` and `code` is unusable.
pub(crate) fn parse(file: &str, text: &str, report: &mut Vec<Diagnostic>) -> Option<Locale> {
    let doc = Doc::new(file, text);
    let root = match doc.parse() {
        Ok(root) => root,
        Err(diagnostic) => {
            report.push(diagnostic);
            return None;
        }
    };
    let Some(meta_value) = doc::get(&root, "meta") else {
        report.push(Diagnostic::error(None, format!("{file}: missing [meta] table with `name` and `code`")));
        return None;
    };
    let meta = match doc.table(meta_value, "meta") {
        Ok(meta) => meta,
        Err(diagnostic) => {
            report.push(diagnostic);
            return None;
        }
    };
    let field = |key: &str, report: &mut Vec<Diagnostic>| -> Option<String> {
        let value = doc::get(meta, key)?;
        match doc.string(value, &format!("meta.{key}")) {
            Ok(text) => Some(text.to_owned()),
            Err(diagnostic) => {
                report.push(diagnostic);
                None
            }
        }
    };
    let (Some(name), Some(code)) = (field("name", report), field("code", report)) else {
        report.push(Diagnostic::error(None, format!("{file}: [meta] needs `name` and `code`")));
        return None;
    };
    let fallback = field("fallback", report);
    for (key, value) in meta {
        if !["name", "code", "fallback"].contains(&key.get_ref().as_ref()) {
            report.push(doc.error(&value.span(), format!("unknown key `meta.{}`", key.get_ref())));
        }
    }

    let mut messages = BTreeMap::new();
    for (key, value) in &root {
        let key = key.get_ref().as_ref();
        if key == "meta" {
            continue;
        }
        match doc.table(value, key) {
            Ok(table) => collect(&doc, key, table, &mut messages, report),
            Err(_) => report.push(doc.error(
                &value.span(),
                format!("`{key}` must be a section such as [{key}]; messages live inside sections"),
            )),
        }
    }
    Some(Locale { code, name, fallback, messages })
}

fn collect(
    doc: &Doc<'_>,
    prefix: &str,
    table: &DeTable<'_>,
    messages: &mut BTreeMap<String, Message>,
    report: &mut Vec<Diagnostic>,
) {
    for (key, value) in table {
        let full_key = format!("{prefix}.{}", key.get_ref());
        if let Some(text) = value.get_ref().as_str() {
            match Template::parse(text) {
                Ok(template) => {
                    messages.insert(full_key, Message::Plain(template));
                }
                Err(message) => report.push(doc.error(&value.span(), message)),
            }
            continue;
        }
        let Some(inner) = value.get_ref().as_table() else {
            report
                .push(doc.error(&value.span(), format!("`{full_key}` must be a string, a plural table or a section")));
            continue;
        };
        let is_plural = !inner.is_empty()
            && inner.iter().all(|(k, v)| PluralCategory::from_name(k.get_ref()).is_some() && v.get_ref().is_str());
        if !is_plural {
            collect(doc, &full_key, inner, messages, report);
            continue;
        }
        let mut forms = BTreeMap::new();
        for (category, form) in inner {
            let (Some(category), Some(text)) = (PluralCategory::from_name(category.get_ref()), form.get_ref().as_str())
            else {
                continue;
            };
            match Template::parse(text) {
                Ok(template) => {
                    forms.insert(category, template);
                }
                Err(message) => report.push(doc.error(&form.span(), message)),
            }
        }
        if forms.contains_key(&PluralCategory::Other) {
            messages.insert(full_key, Message::Plural(forms));
        } else {
            report.push(doc.error(&value.span(), format!("plural message `{full_key}` needs an `other` form")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_split_text_and_args() {
        let template = Template::parse("Hi {name}, {{literal}}").expect("valid");
        assert_eq!(
            template.0,
            vec![Piece::Text("Hi ".to_owned()), Piece::Arg("name".to_owned()), Piece::Text(", {literal}".to_owned()),]
        );
        assert!(Template::parse("broken {").is_err());
        assert!(Template::parse("Hello {name").is_err(), "a placeholder must be closed");
        assert!(Template::parse("stray }").is_err());
    }

    #[test]
    fn flattens_sections_and_detects_plurals() {
        let text = r#"
[meta]
name = "English"
code = "en"

[files]
count = { one = "{n} file", other = "{n} files" }
[files.actions]
open = "Open"
bad = { one = "only one" }
"#;
        let mut report = Vec::new();
        let locale = parse("en.toml", text, &mut report).expect("usable");
        assert_eq!(locale.code, "en");
        assert!(matches!(locale.messages.get("files.count"), Some(Message::Plural(_))));
        assert!(matches!(locale.messages.get("files.actions.open"), Some(Message::Plain(_))));
        assert!(!locale.messages.contains_key("files.actions.bad"));
        assert_eq!(report.len(), 1);
        assert!(report[0].message.contains("needs an `other` form"));
        assert_eq!(report[0].location.as_ref().map(|l| l.line), Some(10));
    }

    #[test]
    fn unknown_meta_keys_are_reported() {
        let mut report = Vec::new();
        let locale = parse("x.toml", "[meta]\nname = \"X\"\ncode = \"x\"\nfallbak = \"en\"\n", &mut report);
        assert!(locale.is_some_and(|locale| locale.fallback.is_none()));
        assert_eq!(report.len(), 1);
        assert!(report[0].message.contains("unknown key `meta.fallbak`"));
        assert_eq!(report[0].location.as_ref().map(|l| l.line), Some(4));
    }

    #[test]
    fn meta_is_required() {
        let mut report = Vec::new();
        assert!(parse("x.toml", "[a]\nb = \"c\"\n", &mut report).is_none());
        assert!(parse("x.toml", "[meta]\nname = \"X\"\n", &mut report).is_none());
    }
}
