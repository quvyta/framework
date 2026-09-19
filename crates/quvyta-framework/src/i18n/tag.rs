//! Language tags: the system's locale names (`pt_BR.UTF-8`, `zh_TW`) and the codes of locale
//! files (`pt-BR`, `zh-Hans`), reduced to the parts that choose a locale.

/// A locale name split into its parts. Everything is lowercase, since tags compare without case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Tag {
    /// The language, such as `pt` or `zh`.
    pub(super) language: String,
    /// The writing system, such as `hans`, when the name gives one.
    script: Option<String>,
    /// The country or area, such as `br` or `tw`, when the name gives one.
    region: Option<String>,
}

impl Tag {
    /// Reads `pt_BR.UTF-8`, `pt-BR`, `zh_Hans_CN` or `de_CH.UTF-8@euro`. The encoding after `.`
    /// and the modifier after `@` say nothing about the language and are dropped. `C` and
    /// `POSIX` name no language, and neither does anything whose first part is not two or three
    /// letters.
    pub(super) fn parse(name: &str) -> Option<Self> {
        let base = name.split(['.', '@']).next().unwrap_or_default();
        let mut parts = base.split(['_', '-']).filter(|part| !part.is_empty()).map(str::to_ascii_lowercase);
        let language = parts.next()?;
        let is_language = (2..=3).contains(&language.len()) && language.chars().all(|c| c.is_ascii_lowercase());
        if !is_language || language == "c" {
            return None;
        }
        let (mut script, mut region) = (None, None);
        for part in parts {
            let letters = part.chars().all(|c| c.is_ascii_lowercase());
            match part.len() {
                4 if letters && script.is_none() => script = Some(part),
                2 if letters && region.is_none() => region = Some(part),
                3 if part.chars().all(|c| c.is_ascii_digit()) && region.is_none() => region = Some(part),
                _ => {}
            }
        }
        Some(Self { language, script, region })
    }

    /// The whole tag in the form locale files use, `language-script-region`, lowercase.
    pub(super) fn full(&self) -> String {
        [Some(&self.language), self.script.as_ref(), self.region.as_ref()]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("-")
    }

    /// The writing system: the one the name gives, or for Chinese the one its region uses.
    /// Chinese is written in two scripts, and a system names a region (`zh_CN`, `zh_TW`) where a
    /// translation is made for a script (`zh-Hans`, `zh-Hant`).
    pub(super) fn script(&self) -> Option<&str> {
        if let Some(script) = &self.script {
            return Some(script);
        }
        match (self.language.as_str(), self.region.as_deref()?) {
            ("zh", "cn" | "sg" | "my") => Some("hans"),
            ("zh", "tw" | "hk" | "mo") => Some("hant"),
            _ => None,
        }
    }

    /// The region, lowercase, when the name gives one.
    pub(super) fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }
}

/// The language of a locale code, lowercase: `pt` for `pt-BR`, `zh` for `zh-Hans`. A code that
/// is not a language tag is taken whole, so a locale named `x` still has a language.
pub(super) fn language_of(code: &str) -> String {
    code.split(['-', '_']).next().unwrap_or(code).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_system_names_and_file_codes() {
        let tag = Tag::parse("pt_BR.UTF-8").expect("a language");
        assert_eq!((tag.language.as_str(), tag.region.as_deref(), tag.full()), ("pt", Some("br"), "pt-br".to_owned()));
        let tag = Tag::parse("zh_Hans_CN").expect("a language");
        assert_eq!((tag.script(), tag.full()), (Some("hans"), "zh-hans-cn".to_owned()));
        assert_eq!(Tag::parse("zh_TW").and_then(|tag| tag.script().map(str::to_owned)).as_deref(), Some("hant"));
        assert_eq!(Tag::parse("de_CH.UTF-8@euro").map(|tag| tag.full()).as_deref(), Some("de-ch"));
        assert_eq!(Tag::parse("es-419").and_then(|tag| tag.region).as_deref(), Some("419"));
        for none in ["C", "C.UTF-8", "POSIX", "", "english", "1a"] {
            assert_eq!(Tag::parse(none), None, "{none}");
        }
    }

    #[test]
    fn a_code_s_language_is_its_first_part() {
        assert_eq!(language_of("pt-BR"), "pt");
        assert_eq!(language_of("zh-Hans"), "zh");
        assert_eq!(language_of("TR"), "tr");
    }
}
