//! Loading theme files and resolving `extends` chains into [`Theme`]s.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

use super::cache::StyleCache;
use super::motion::Motion;
use super::paint::Expr;
use super::source::{self, MotionValue, RawProps, ThemeSource};
use super::style::{PropValue, RawProp, StyleProps};
use super::{REQUIRED_COLORS, Theme, validate};
use crate::assets;
use crate::color::Rgb;
use crate::diagnostics::{Diagnostic, Location};

/// The id of the built-in default theme.
const DEFAULT_THEME: &str = "monochrome";

/// The result of resolving a theme.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// The theme, when it could be built.
    pub theme: Option<Theme>,
    /// Errors and warnings found while resolving, including readability warnings.
    pub diagnostics: Vec<Diagnostic>,
}

/// All themes known to an application: the built-in ones plus any loaded from disk.
///
/// A loaded file with the same id as a built-in theme replaces it.
#[derive(Debug, Clone)]
pub struct ThemeRegistry {
    sources: BTreeMap<String, ThemeSource>,
    diagnostics: Vec<Diagnostic>,
}

impl ThemeRegistry {
    /// A registry holding the built-in themes.
    #[must_use]
    pub fn builtin() -> Self {
        let mut registry = Self { sources: BTreeMap::new(), diagnostics: Vec::new() };
        for (id, text) in assets::THEMES {
            registry.add_source(id, &format!("{id}.toml"), text);
        }
        registry
    }

    /// Adds or replaces theme `id` from TOML text. Returns whether the file was usable.
    pub fn add_source(&mut self, id: &str, file: &str, text: &str) -> bool {
        match source::parse(file, text, &mut self.diagnostics) {
            Some(parsed) => {
                self.sources.insert(id.to_owned(), parsed);
                true
            }
            None => false,
        }
    }

    /// Loads every `*.toml` file in `dir`; the file stem is the theme id.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the directory cannot be read. A file that cannot be read is
    /// skipped and reported in the diagnostics.
    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        let found = assets::read_toml_dir(dir)?;
        self.diagnostics.extend(found.skipped);
        for (id, file, text) in found.files {
            self.add_source(&id, &file, &text);
        }
        Ok(())
    }

    /// `(id, display name)` of every theme, sorted by id; for a settings screen.
    #[must_use]
    pub fn list(&self) -> Vec<(String, String)> {
        self.sources.iter().map(|(id, s)| (id.clone(), s.name.clone())).collect()
    }

    /// Problems found while loading files.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Resolves theme `id` through its `extends` chain.
    #[must_use]
    pub fn resolve(&self, id: &str) -> Resolved {
        let mut diagnostics = Vec::new();
        let theme = self.build(id, &mut diagnostics);
        if let Some(theme) = &theme {
            diagnostics.extend(validate::validate(theme));
        }
        Resolved { theme, diagnostics }
    }

    /// Resolves theme `id`, falling back to the pristine built-in default theme when `id`
    /// cannot be built. Diagnostics explain why a fallback happened.
    #[must_use]
    pub fn resolve_or_default(&self, id: &str) -> (Theme, Vec<Diagnostic>) {
        let Resolved { theme, mut diagnostics } = self.resolve(id);
        if let Some(theme) = theme {
            return (theme, diagnostics);
        }
        diagnostics.push(Diagnostic::warning(
            None,
            format!("theme `{id}` could not be loaded; using the built-in `{DEFAULT_THEME}` theme"),
        ));
        let pristine = Self::builtin()
            .build(DEFAULT_THEME, &mut Vec::new())
            .expect("the built-in default theme is complete; the builtin_assets test guarantees it");
        (pristine, diagnostics)
    }

    fn chain(&self, id: &str, diagnostics: &mut Vec<Diagnostic>) -> Option<Vec<&ThemeSource>> {
        let mut chain: Vec<&ThemeSource> = Vec::new();
        let mut visited: Vec<&str> = Vec::new();
        let mut current = id;
        let mut referenced_at: Option<&Location> = None;
        loop {
            if visited.contains(&current) {
                visited.push(current);
                diagnostics.push(Diagnostic::error(
                    referenced_at.cloned(),
                    format!("theme `extends` forms a cycle: {}", visited.join(" -> ")),
                ));
                return None;
            }
            let Some(source) = self.sources.get(current) else {
                diagnostics.push(Diagnostic::error(referenced_at.cloned(), format!("unknown theme `{current}`")));
                return None;
            };
            visited.push(current);
            chain.push(source);
            match &source.extends {
                Some((parent, location)) => {
                    current = parent;
                    referenced_at = Some(location);
                }
                None => break,
            }
        }
        chain.reverse();
        Some(chain)
    }

    fn build(&self, id: &str, diagnostics: &mut Vec<Diagnostic>) -> Option<Theme> {
        let chain = self.chain(id, diagnostics)?;

        let mut color_exprs: BTreeMap<String, (Expr, Location)> = BTreeMap::new();
        let mut motion_values: BTreeMap<String, MotionValue> = BTreeMap::new();
        let mut typography_raw: BTreeMap<String, RawProps> = BTreeMap::new();
        let mut rules_raw: Vec<(&super::Selector, &RawProps)> = Vec::new();
        let mut icon_set = None;
        let mut icons = BTreeMap::new();
        let mut animations = BTreeMap::new();
        for source in &chain {
            for (name, expr, location) in &source.colors {
                color_exprs.insert(name.clone(), (expr.clone(), location.clone()));
            }
            for (name, value) in &source.motion {
                motion_values.insert(name.clone(), *value);
            }
            for (role, props) in &source.typography {
                typography_raw.entry(role.clone()).or_default().extend(props.iter().cloned());
            }
            rules_raw.extend(source.styles.iter().map(|(selector, props)| (selector, props)));
            if source.icon_set.is_some() {
                icon_set.clone_from(&source.icon_set);
            }
            icons.extend(source.icons.iter().map(|(k, v)| (k.clone(), v.clone())));
            animations.extend(source.animations.iter().map(|(k, v)| (k.clone(), Arc::new(v.clone()))));
        }

        let colors = resolve_colors(&color_exprs, diagnostics);
        let missing: Vec<&str> = REQUIRED_COLORS.iter().copied().filter(|c| !colors.contains_key(*c)).collect();
        if !missing.is_empty() {
            diagnostics.push(Diagnostic::error(
                None,
                format!("theme `{id}` is missing colour tokens: {}", missing.join(", ")),
            ));
            return None;
        }

        let motion = build_motion(id, &motion_values, diagnostics)?;
        let typography =
            typography_raw.iter().map(|(role, raw)| (role.clone(), resolve_props(raw, &colors, diagnostics))).collect();
        let rules = rules_raw
            .into_iter()
            .map(|(selector, raw)| (selector.clone(), resolve_props(raw, &colors, diagnostics)))
            .collect();

        Some(Theme {
            id: id.to_owned(),
            name: chain.last().map(|s| s.name.clone()).unwrap_or_default(),
            colors,
            motion,
            typography,
            rules,
            icon_set: icon_set.unwrap_or_else(|| "default".to_owned()),
            icons,
            animations,
            cache: StyleCache::default(),
        })
    }
}

/// How many colour tokens may refer to one another in a row. Real themes use two or three; the
/// limit keeps a generated or hostile file from exhausting the stack.
const MAX_TOKEN_CHAIN: usize = 64;

/// Resolves colour tokens that may reference each other, reporting unknown names and cycles.
fn resolve_colors(
    exprs: &BTreeMap<String, (Expr, Location)>,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Rgb> {
    let mut resolved: BTreeMap<String, Rgb> = BTreeMap::new();
    let mut failed: Vec<String> = Vec::new();
    for name in exprs.keys() {
        let mut stack = Vec::new();
        resolve_color(name, exprs, &mut resolved, &mut failed, &mut stack, diagnostics);
    }
    resolved
}

fn resolve_color(
    name: &str,
    exprs: &BTreeMap<String, (Expr, Location)>,
    resolved: &mut BTreeMap<String, Rgb>,
    failed: &mut Vec<String>,
    stack: &mut Vec<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Rgb> {
    if let Some(color) = resolved.get(name) {
        return Some(*color);
    }
    if failed.iter().any(|f| f == name) {
        return None;
    }
    let (expr, location) = exprs.get(name)?;
    if stack.iter().any(|s| s == name) {
        stack.push(name.to_owned());
        diagnostics.push(Diagnostic::error(
            Some(location.clone()),
            format!("colour tokens reference each other in a cycle: {}", stack.join(" -> ")),
        ));
        failed.push(name.to_owned());
        return None;
    }
    if stack.len() >= MAX_TOKEN_CHAIN {
        diagnostics.push(Diagnostic::error(
            Some(location.clone()),
            format!("colour token `{name}` sits at the end of a chain longer than {MAX_TOKEN_CHAIN} tokens"),
        ));
        failed.push(name.to_owned());
        return None;
    }
    stack.push(name.to_owned());
    for dependency in expr.tokens() {
        if exprs.contains_key(dependency) {
            resolve_color(dependency, exprs, resolved, failed, stack, diagnostics);
        }
    }
    stack.pop();
    match expr.solid(resolved) {
        Ok(color) => {
            resolved.insert(name.to_owned(), color);
            Some(color)
        }
        Err(message) => {
            if !failed.iter().any(|f| f == name) {
                diagnostics.push(Diagnostic::error(Some(location.clone()), format!("colour `{name}`: {message}")));
                failed.push(name.to_owned());
            }
            None
        }
    }
}

/// Motion keys every resolved theme defines; `page` and `hover-delay` have defaults.
const REQUIRED_MOTION: [&str; 8] =
    ["pulse-period", "flash", "cursor-blink", "step", "slide", "enter", "spinner", "shimmer"];

/// Tooltip delay for themes whose `[motion]` does not say.
const DEFAULT_HOVER_DELAY: std::time::Duration = std::time::Duration::from_millis(450);

fn build_motion(id: &str, values: &BTreeMap<String, MotionValue>, diagnostics: &mut Vec<Diagnostic>) -> Option<Motion> {
    let duration = |key: &str| match values.get(key) {
        Some(MotionValue::Duration(d)) => Some(*d),
        _ => None,
    };
    let (
        Some(pulse_period),
        Some(flash),
        Some(cursor_blink),
        Some(step),
        Some(MotionValue::Flag(slide)),
        Some(enter),
        Some(spinner),
        Some(shimmer),
    ) = (
        duration("pulse-period"),
        duration("flash"),
        duration("cursor-blink"),
        duration("step"),
        values.get("slide"),
        duration("enter"),
        duration("spinner"),
        duration("shimmer"),
    )
    else {
        let missing: Vec<&str> = REQUIRED_MOTION.into_iter().filter(|key| !values.contains_key(*key)).collect();
        diagnostics.push(Diagnostic::error(None, format!("theme `{id}` must define motion {}", missing.join(", "))));
        return None;
    };
    if pulse_period.is_zero() || spinner.is_zero() || shimmer.is_zero() {
        diagnostics.push(Diagnostic::error(
            None,
            format!("theme `{id}`: motion.pulse-period, spinner and shimmer must be longer than 0ms"),
        ));
        return None;
    }
    let page = duration("page").unwrap_or(enter * 2);
    let hover_delay = duration("hover-delay").unwrap_or(DEFAULT_HOVER_DELAY);
    Some(Motion { pulse_period, flash, cursor_blink, step, slide: *slide, enter, spinner, shimmer, page, hover_delay })
}

fn resolve_props(raw: &RawProps, colors: &BTreeMap<String, Rgb>, diagnostics: &mut Vec<Diagnostic>) -> StyleProps {
    let mut props = StyleProps::default();
    for (key, value, location) in raw {
        let resolved = match value {
            RawProp::Expr(expr) => match expr.resolve(colors) {
                Ok(paint) => PropValue::Paint(paint),
                Err(message) => {
                    diagnostics.push(Diagnostic::error(Some(location.clone()), format!("`{key}`: {message}")));
                    continue;
                }
            },
            RawProp::Flag(flag) => PropValue::Flag(*flag),
            RawProp::Cells(n) => PropValue::Cells(*n),
            RawProp::Pair(v, h) => PropValue::Pair(*v, *h),
            RawProp::Word(word) => PropValue::Word(word),
        };
        props.set(key, resolved);
    }
    props
}

#[cfg(test)]
mod tests {
    use super::super::{Paint, State};
    use super::*;

    fn registry_with(files: &[(&str, &str)]) -> ThemeRegistry {
        let mut registry = ThemeRegistry::builtin();
        for (id, text) in files {
            registry.add_source(id, &format!("{id}.toml"), text);
        }
        registry
    }

    #[test]
    fn child_overrides_parent_tokens_and_rules_use_final_tokens() {
        let registry = registry_with(&[(
            "child",
            "[meta]\nname = \"Child\"\nextends = \"monochrome\"\n[colors]\naccent = \"#ff0000\"\n[style.button]\nbg = \"$accent\"\n",
        )]);
        let resolved = registry.resolve("child");
        let theme = resolved.theme.expect("child resolves");
        assert_eq!(theme.name(), "Child");
        assert_eq!(theme.color("accent"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(
            theme.color("canvas"),
            ThemeRegistry::builtin().resolve("monochrome").theme.and_then(|t| t.color("canvas"))
        );
        assert_eq!(theme.style("button", None, &[]).paint("bg"), Some(Paint::Solid(Rgb::new(255, 0, 0))));
    }

    #[test]
    fn word_properties_resolve_and_inherit() {
        let registry = registry_with(&[(
            "thin",
            "[meta]\nname = \"Thin\"\nextends = \"monochrome\"\n[style.scrollbar]\nstyle = \"thin\"\n",
        )]);
        let theme = registry.resolve("thin").theme.expect("resolves");
        assert_eq!(theme.style("scrollbar", None, &[]).word("style"), Some("thin"));
        assert_eq!(theme.style("scrollbar", None, &[State::Hover]).word("style"), Some("thin"));
        let monochrome = ThemeRegistry::builtin().resolve("monochrome").theme.expect("resolves");
        assert_eq!(monochrome.style("scrollbar", None, &[]).word("style"), Some("block"));
    }

    #[test]
    fn specificity_then_order_decides() {
        let registry = registry_with(&[(
            "rules",
            r##"[meta]
name = "Rules"
extends = "monochrome"
[style."probe:hover"]
bg = "#000003"
[style."probe.primary"]
bg = "#000002"
[style.probe]
bg = "#000001"
bold = true
[style."probe.primary:hover"]
fg = "#0000ff"
"##,
        )]);
        let theme = registry.resolve("rules").theme.expect("resolves");
        let paint = |variant, states: &[State]| theme.style("probe", variant, states).paint("bg");
        assert_eq!(paint(None, &[]), Some(Paint::Solid(Rgb::new(0, 0, 1))));
        assert_eq!(paint(Some("primary"), &[]), Some(Paint::Solid(Rgb::new(0, 0, 2))));
        assert_eq!(paint(Some("primary"), &[State::Hover]), Some(Paint::Solid(Rgb::new(0, 0, 3))));
        let hovered = theme.style("probe", Some("primary"), &[State::Hover]);
        assert!(hovered.flag("bold"));
        assert_eq!(hovered.paint("fg"), Some(Paint::Solid(Rgb::new(0, 0, 255))));
    }

    #[test]
    fn unknown_parent_and_cycles_fail_with_diagnostics() {
        let registry = registry_with(&[
            ("orphan", "[meta]\nname = \"O\"\nextends = \"ghost\"\n"),
            ("a", "[meta]\nname = \"A\"\nextends = \"b\"\n"),
            ("b", "[meta]\nname = \"B\"\nextends = \"a\"\n"),
        ]);
        let orphan = registry.resolve("orphan");
        assert!(orphan.theme.is_none());
        assert!(orphan.diagnostics[0].message.contains("unknown theme `ghost`"));
        let cycle = registry.resolve("a");
        assert!(cycle.theme.is_none());
        assert!(cycle.diagnostics[0].message.contains("a -> b -> a"));
    }

    #[test]
    fn token_cycles_and_missing_tokens_are_reported() {
        let registry = registry_with(&[(
            "loop",
            "[meta]\nname = \"L\"\nextends = \"monochrome\"\n[colors]\naccent = \"$ink\"\nink = \"mix($accent, #000, 50%)\"\n",
        )]);
        let resolved = registry.resolve("loop");
        assert!(resolved.theme.is_none());
        let text: Vec<String> = resolved.diagnostics.iter().map(ToString::to_string).collect();
        assert!(text.iter().any(|m| m.contains("cycle")), "{text:?}");
        assert!(text.iter().any(|m| m.contains("missing colour tokens: accent, ink")), "{text:?}");
    }

    #[test]
    fn an_endless_token_chain_is_a_diagnostic_not_a_stack_overflow() {
        let chain: String = (0..200).map(|i| format!("t{i} = \"$t{}\"\n", i + 1)).collect();
        let text = format!("[meta]\nname = \"Deep\"\nextends = \"monochrome\"\n[colors]\n{chain}t200 = \"#808080\"\n");
        let resolved = registry_with(&[("deep", &text)]).resolve("deep");
        let text: Vec<String> = resolved.diagnostics.iter().map(ToString::to_string).collect();
        assert!(text.iter().any(|m| m.contains("longer than 64 tokens")), "{:?}", text.first());
    }

    #[test]
    fn missing_motion_keys_are_named() {
        let colors: String = REQUIRED_COLORS.iter().map(|token| format!("{token} = \"#808080\"\n")).collect();
        let text = format!("[meta]\nname = \"Still\"\n[colors]\n{colors}[motion]\nflash = \"90ms\"\nslide = true\n");
        let resolved = registry_with(&[("still", &text)]).resolve("still");
        assert!(resolved.theme.is_none());
        let text: Vec<String> = resolved.diagnostics.iter().map(ToString::to_string).collect();
        assert_eq!(
            text,
            vec!["error: theme `still` must define motion pulse-period, cursor-blink, step, enter, spinner, shimmer"]
        );
    }

    #[test]
    fn falls_back_to_pristine_default() {
        let registry = registry_with(&[("monochrome", "[meta]\nname = \"Broken\"\n")]);
        let (theme, diagnostics) = registry.resolve_or_default("monochrome");
        assert_eq!(theme.name(), "Monochrome");
        assert!(diagnostics.iter().any(|d| d.message.contains("using the built-in `monochrome` theme")));
    }

    #[test]
    fn readability_problems_are_warnings() {
        let registry = registry_with(&[(
            "murky",
            "[meta]\nname = \"Murky\"\nextends = \"monochrome\"\n[colors]\ntext = \"#222222\"\nwarning = \"$success\"\n",
        )]);
        let resolved = registry.resolve("murky");
        assert!(resolved.theme.is_some());
        let text: Vec<String> = resolved.diagnostics.iter().map(ToString::to_string).collect();
        assert!(text.iter().any(|m| m.contains("`text` on `canvas`")), "{text:?}");
        assert!(text.iter().any(|m| m.contains("`success` and `warning` look too similar")), "{text:?}");
    }

    #[test]
    fn lists_themes_for_settings() {
        let names: Vec<String> = ThemeRegistry::builtin().list().into_iter().map(|(id, _)| id).collect();
        assert_eq!(names, vec!["amber", "iris", "monochrome", "nordic"]);
    }
}
