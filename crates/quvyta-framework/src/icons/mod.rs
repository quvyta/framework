//! Icon sets: every icon has a Nerd Font, a Unicode and an ASCII glyph, and the terminal's
//! capabilities decide which one is drawn.
//!
//! ```toml
//! [meta]
//! name = "Default"
//!
//! [icons]
//! check = { nerd = "", unicode = "✓", ascii = "v" }
//!
//! [animations.blink]
//! frames = [{ unicode = "●", ascii = "*" }, { unicode = "·", ascii = "." }]
//! ```
//!
//! An icon set also holds one-cell animations (see [`crate::animation`]); themes
//! replace single animations the way they replace single icons.

mod detect;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

use toml::de::DeTable;
use unicode_segmentation::UnicodeSegmentation;

pub use detect::{default_font_dirs, detect_glyph_mode};

use crate::animation::{self, CellAnimation};
use crate::assets;
use crate::diagnostics::Diagnostic;
use crate::doc::{self, Doc, Value};

/// The glyphs of one icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconGlyphs {
    /// Glyph for terminals with a Nerd Font.
    pub nerd: String,
    /// Glyph for UTF-8 terminals without a Nerd Font.
    pub unicode: String,
    /// Glyph for terminals that can only show ASCII.
    pub ascii: String,
}

impl IconGlyphs {
    /// The glyph for `mode`.
    #[must_use]
    pub fn for_mode(&self, mode: GlyphMode) -> &str {
        match mode {
            GlyphMode::Nerd => &self.nerd,
            GlyphMode::Unicode => &self.unicode,
            GlyphMode::Ascii => &self.ascii,
        }
    }
}

/// The icon preference a user or application chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconMode {
    /// Detect from the terminal and installed fonts.
    #[default]
    Auto,
    /// Always use Nerd Font glyphs.
    Nerd,
    /// Always use Unicode glyphs.
    Unicode,
    /// Always use ASCII glyphs.
    Ascii,
}

impl IconMode {
    /// Every mode, in the order a settings screen lists them.
    pub const ALL: [Self; 4] = [Self::Auto, Self::Nerd, Self::Unicode, Self::Ascii];

    /// The name used in settings and the `QUVYTA_ICONS` environment variable.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Nerd => "nerd",
            Self::Unicode => "unicode",
            Self::Ascii => "ascii",
        }
    }

    /// Looks a mode up by name, ignoring letter case.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }
}

/// The glyph column actually drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphMode {
    /// Nerd Font glyphs.
    Nerd,
    /// Unicode glyphs.
    Unicode,
    /// ASCII glyphs.
    Ascii,
}

/// Characters an ASCII glyph may not contain: glyphs must not fake shapes with brackets.
const BANNED_ASCII: [char; 6] = ['[', ']', '(', ')', '{', '}'];

/// Reads `{ nerd = "…", unicode = "…", ascii = "…" }`.
pub(crate) fn parse_glyphs(doc: &Doc<'_>, key: &str, value: &Value<'_>) -> Result<IconGlyphs, Diagnostic> {
    let table = doc.table(value, &format!("icon `{key}`"))?;
    let field = |name: &str| -> Result<String, Diagnostic> {
        let Some(entry) = doc::get(table, name) else {
            return Err(doc.error(&value.span(), format!("icon `{key}` is missing its `{name}` glyph")));
        };
        let text = doc.string(entry, &format!("icon `{key}`.{name}"))?;
        if text.is_empty() {
            return Err(doc.error(&entry.span(), format!("icon `{key}`.{name} must not be empty")));
        }
        Ok(text.to_owned())
    };
    let glyphs = IconGlyphs { nerd: field("nerd")?, unicode: field("unicode")?, ascii: field("ascii")? };
    if let Some((unknown, entry)) =
        table.iter().find(|(name, _)| !["nerd", "unicode", "ascii"].contains(&name.get_ref().as_ref()))
    {
        return Err(doc.error(
            &entry.span(),
            format!("icon `{key}` has unknown field `{}`; use nerd, unicode and ascii", unknown.get_ref()),
        ));
    }
    let ascii_ok = glyphs.ascii.chars().all(|c| c.is_ascii() && !c.is_ascii_control());
    if !ascii_ok {
        return Err(doc.error(&value.span(), format!("icon `{key}`.ascii must contain only printable ASCII")));
    }
    if let Some(bad) = glyphs.ascii.chars().find(|c| BANNED_ASCII.contains(c)) {
        return Err(
            doc.error(&value.span(), format!("icon `{key}`.ascii uses `{bad}`; brackets are not allowed as glyphs"))
        );
    }
    Ok(glyphs)
}

/// The icon drawn at the left of hovered, focused and selected rows, tabs, buttons and cards.
pub const PILLAR: &str = "pillar";

/// A pillar a user can choose at runtime, over whatever the theme draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillarStyle {
    /// A half block, `▌`: the default.
    Thick,
    /// A quarter block, `▎`.
    Thin,
}

impl PillarStyle {
    /// Every style, in the order a settings screen lists them.
    pub const ALL: [Self; 2] = [Self::Thick, Self::Thin];

    /// The name used in settings and theme files.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Thick => "thick",
            Self::Thin => "thin",
        }
    }

    /// Looks a style up by name, ignoring letter case.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|style| style.name() == name)
    }

    /// The glyph drawn for this style; ASCII terminals always draw a coloured cell.
    #[must_use]
    pub fn glyphs(self) -> IconGlyphs {
        pillar_glyphs(match self {
            Self::Thick => "▌",
            Self::Thin => "▎",
        })
    }
}

/// A pillar drawn with `glyph` wherever Unicode is available and as a coloured cell in ASCII.
fn pillar_glyphs(glyph: &str) -> IconGlyphs {
    IconGlyphs { nerd: glyph.to_owned(), unicode: glyph.to_owned(), ascii: " ".to_owned() }
}

/// Reads the shorthand of the pillar: `"thick"` (`▌`), `"thin"` (`▎`) or any single-cell
/// character. ASCII terminals always show the pillar as a coloured cell.
fn parse_pillar(doc: &Doc<'_>, value: &Value<'_>) -> Result<IconGlyphs, Diagnostic> {
    let text = doc.string(value, "icon `pillar`")?;
    if let Some(style) = PillarStyle::from_name(text) {
        return Ok(style.glyphs());
    }
    if crate::text::width(text) == 1 && text.chars().count() == 1 {
        Ok(pillar_glyphs(text))
    } else {
        Err(doc.error(
            &value.span(),
            format!("icon `pillar` is `{text}`; use \"thick\", \"thin\" or a single one-cell character"),
        ))
    }
}

/// Reads an `[icons]` table into `glyphs`, reporting and skipping broken entries. The pillar may
/// also be given in its short form, see [`PILLAR`].
pub(crate) fn read_icon_table(
    doc: &Doc<'_>,
    table: &DeTable<'_>,
    glyphs: &mut BTreeMap<String, IconGlyphs>,
    report: &mut Vec<Diagnostic>,
) {
    for (key, value) in table {
        let parsed = if key.get_ref() == PILLAR && value.get_ref().as_str().is_some() {
            parse_pillar(doc, value)
        } else {
            parse_glyphs(doc, key.get_ref(), value).and_then(|glyphs| legacy_glyphs(doc, key.get_ref(), value, glyphs))
        };
        match parsed {
            Ok(parsed) => {
                glyphs.insert(key.get_ref().to_string(), parsed);
            }
            Err(diagnostic) => report.push(diagnostic),
        }
    }
}

/// Checks the frames of a former spinner icon, which now replaces an animation's glyphs.
fn legacy_glyphs(doc: &Doc<'_>, key: &str, value: &Value<'_>, glyphs: IconGlyphs) -> Result<IconGlyphs, Diagnostic> {
    if !animation::LEGACY_ICONS.iter().any(|(icon, _)| *icon == key) {
        return Ok(glyphs);
    }
    animation::check_legacy(&glyphs).map_err(|message| doc.error(&value.span(), format!("icon `{key}`: {message}")))?;
    Ok(glyphs)
}

/// Adds one layer of animations over `animations`: first the former spinner icons among `glyphs`,
/// then the layer's own animations, which win over icons of the same layer.
fn layer_animations(
    animations: &mut BTreeMap<String, Arc<CellAnimation>>,
    glyphs: &BTreeMap<String, IconGlyphs>,
    own: impl IntoIterator<Item = (String, Arc<CellAnimation>)>,
) {
    for (icon, name) in animation::LEGACY_ICONS {
        if let Some(glyphs) = glyphs.get(icon) {
            animation::apply_legacy(animations, name, glyphs);
        }
    }
    animations.extend(own);
}

/// A loaded icon set file.
#[derive(Debug, Clone)]
struct IconSetSource {
    name: String,
    glyphs: BTreeMap<String, IconGlyphs>,
    animations: BTreeMap<String, Arc<CellAnimation>>,
}

/// All icon sets known to an application: the built-in ones plus any loaded from disk.
#[derive(Debug, Clone)]
pub struct IconSetRegistry {
    sets: BTreeMap<String, IconSetSource>,
    diagnostics: Vec<Diagnostic>,
}

impl IconSetRegistry {
    /// A registry holding the built-in icon sets.
    #[must_use]
    pub fn builtin() -> Self {
        let mut registry = Self { sets: BTreeMap::new(), diagnostics: Vec::new() };
        for (id, text) in assets::ICON_SETS {
            registry.add_source(id, &format!("{id}.toml"), text);
        }
        registry
    }

    /// Adds or replaces the icon set `id` from TOML text. Returns whether it was usable.
    pub fn add_source(&mut self, id: &str, file: &str, text: &str) -> bool {
        let doc = Doc::new(file, text);
        let root = match doc.parse() {
            Ok(root) => root,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                return false;
            }
        };
        for (key, value) in &root {
            if !["meta", "icons", "animations"].contains(&key.get_ref().as_ref()) {
                self.diagnostics.push(doc.error(
                    &value.span(),
                    format!("unknown section `{}`; expected meta, icons and animations", key.get_ref()),
                ));
            }
        }
        let name = self.read_name(&doc, &root).unwrap_or_else(|| id.to_owned());
        let mut glyphs = BTreeMap::new();
        match doc::get(&root, "icons") {
            Some(icons) => match doc.table(icons, "icons") {
                Ok(table) => read_icon_table(&doc, table, &mut glyphs, &mut self.diagnostics),
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            },
            None => self.diagnostics.push(Diagnostic::error(None, format!("{file}: missing [icons] table"))),
        }
        let mut animations = BTreeMap::new();
        if let Some(table) = doc::get(&root, "animations") {
            match doc.table(table, "animations") {
                Ok(table) => animation::read_animation_table(&doc, table, &mut animations, &mut self.diagnostics),
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }
        let animations = animations.into_iter().map(|(name, animation)| (name, Arc::new(animation))).collect();
        self.sets.insert(id.to_owned(), IconSetSource { name, glyphs, animations });
        true
    }

    /// The display name from `[meta] name`, reporting a malformed `[meta]` and unknown keys in it.
    fn read_name(&mut self, doc: &Doc<'_>, root: &DeTable<'_>) -> Option<String> {
        let meta = match doc.table(doc::get(root, "meta")?, "meta") {
            Ok(meta) => meta,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                return None;
            }
        };
        let mut name = None;
        for (key, value) in meta {
            if key.get_ref() != "name" {
                self.diagnostics.push(doc.error(&value.span(), format!("unknown key `meta.{}`", key.get_ref())));
                continue;
            }
            match doc.string(value, "meta.name") {
                Ok(text) => name = Some(text.to_owned()),
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }
        name
    }

    /// Loads every `*.toml` file in `dir`; the file stem is the set id.
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

    /// `(id, display name)` of every set, sorted by id.
    #[must_use]
    pub fn list(&self) -> Vec<(String, String)> {
        self.sets.iter().map(|(id, set)| (id.clone(), set.name.clone())).collect()
    }

    /// Problems found while loading.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Builds the icons for set `id` with `overrides` applied on top.
    ///
    /// An unknown id falls back to the built-in `default` set.
    #[must_use]
    pub fn icons(&self, id: &str, overrides: &BTreeMap<String, IconGlyphs>, mode: GlyphMode) -> Icons {
        self.icons_with_animations(id, overrides, &BTreeMap::new(), mode)
    }

    /// Builds the icons and animations for set `id`, with icon `overrides` and `animations` (such
    /// as a theme's) applied on top.
    ///
    /// Animations layer from the built-in `default` set, through set `id`, to the overrides, so a
    /// set or theme without animations still has every built-in one. Within a layer, a former
    /// spinner icon key such as `spinner-arc` first replaces the glyphs of its animation, then the
    /// layer's own animations apply. An unknown id falls back to the built-in `default` set.
    #[must_use]
    pub fn icons_with_animations(
        &self,
        id: &str,
        overrides: &BTreeMap<String, IconGlyphs>,
        animations: &BTreeMap<String, Arc<CellAnimation>>,
        mode: GlyphMode,
    ) -> Icons {
        let fallback = self.sets.get("default");
        let chosen = self.sets.get(id);
        let owned = |source: &IconSetSource| source.animations.clone();
        let mut layered = BTreeMap::new();
        if let Some(default) = fallback {
            layer_animations(&mut layered, &default.glyphs, owned(default));
        }
        if let Some(set) = chosen.filter(|_| id != "default") {
            layer_animations(&mut layered, &set.glyphs, owned(set));
        }
        layer_animations(&mut layered, overrides, animations.clone());
        let mut glyphs = chosen.or(fallback).map(|set| set.glyphs.clone()).unwrap_or_default();
        glyphs.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
        glyphs.retain(|key, _| !animation::LEGACY_ICONS.iter().any(|(icon, _)| icon == key));
        Icons { glyphs, animations: layered, mode }
    }
}

/// Icons ready to draw in one glyph mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icons {
    glyphs: BTreeMap<String, IconGlyphs>,
    animations: BTreeMap<String, Arc<CellAnimation>>,
    mode: GlyphMode,
}

impl Icons {
    /// The glyph mode in use.
    #[must_use]
    pub fn mode(&self) -> GlyphMode {
        self.mode
    }

    /// Switches glyph mode.
    pub fn set_mode(&mut self, mode: GlyphMode) {
        self.mode = mode;
    }

    /// The glyph for `key`. A missing icon is drawn as `⟦key⟧` so it is noticed.
    #[must_use]
    pub fn glyph(&self, key: &str) -> Cow<'_, str> {
        match self.glyphs.get(key) {
            Some(glyphs) => Cow::Borrowed(glyphs.for_mode(self.mode)),
            None => Cow::Owned(format!("⟦{key}⟧")),
        }
    }

    /// The glyph for `key` split into animation frames, one grapheme each.
    #[must_use]
    pub fn frames(&self, key: &str) -> Vec<String> {
        self.glyph(key).graphemes(true).map(str::to_owned).collect()
    }

    /// Whether `key` is defined.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.glyphs.contains_key(key)
    }

    /// Every icon key, sorted.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.glyphs.keys().map(String::as_str)
    }

    /// The animation named `name`, such as `"spinner-arc"`.
    #[must_use]
    pub fn animation(&self, name: &str) -> Option<&Arc<CellAnimation>> {
        self.animations.get(name)
    }

    /// Every animation name, sorted.
    pub fn animation_names(&self) -> impl Iterator<Item = &str> {
        self.animations.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: &str = r#"
[meta]
name = "Test"

[icons]
check = { nerd = "N", unicode = "✓", ascii = "v" }
spin = { nerd = "ab", unicode = "◐◓", ascii = "-/" }
bad = { nerd = "x", unicode = "x", ascii = "[x]" }
half = { nerd = "x" }
"#;

    fn registry() -> IconSetRegistry {
        let mut registry = IconSetRegistry::builtin();
        assert!(registry.add_source("test", "test.toml", SET));
        registry
    }

    #[test]
    fn picks_glyph_for_mode() {
        let mut icons = registry().icons("test", &BTreeMap::new(), GlyphMode::Nerd);
        assert_eq!(icons.glyph("check"), "N");
        icons.set_mode(GlyphMode::Unicode);
        assert_eq!(icons.glyph("check"), "✓");
        icons.set_mode(GlyphMode::Ascii);
        assert_eq!(icons.glyph("check"), "v");
        assert_eq!(icons.glyph("nope"), "⟦nope⟧");
    }

    #[test]
    fn splits_frames_by_grapheme() {
        let icons = registry().icons("test", &BTreeMap::new(), GlyphMode::Unicode);
        assert_eq!(icons.frames("spin"), vec!["◐", "◓"]);
    }

    #[test]
    fn broken_entries_are_reported_and_skipped() {
        let registry = registry();
        let messages: Vec<String> = registry.diagnostics().iter().map(ToString::to_string).collect();
        assert!(messages.iter().any(|m| m.contains("brackets are not allowed")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("missing its `unicode` glyph")), "{messages:?}");
        let icons = registry.icons("test", &BTreeMap::new(), GlyphMode::Ascii);
        assert!(!icons.contains("bad"));
        assert!(!icons.contains("half"));
    }

    #[test]
    fn unknown_sections_and_meta_keys_are_reported_with_their_line() {
        let mut registry = IconSetRegistry::builtin();
        let set = "[meta]\nname = 7\nauthor = \"x\"\n[icon]\ncheck = 1\n[icons]\n";
        assert!(registry.add_source("typo", "typo.toml", set));
        let lines: Vec<(usize, &str)> = registry
            .diagnostics()
            .iter()
            .map(|d| (d.location.as_ref().map_or(0, |l| l.line), d.message.as_str()))
            .collect();
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines.iter().any(|(line, m)| *line == 2 && m.contains("meta.name must be a string")), "{lines:?}");
        assert!(lines.iter().any(|(line, m)| *line == 3 && m.contains("unknown key `meta.author`")), "{lines:?}");
        assert!(lines.iter().any(|(line, m)| *line == 4 && m.contains("unknown section `icon`")), "{lines:?}");
        assert_eq!(registry.list().iter().find(|(id, _)| id == "typo").map(|(_, name)| name.as_str()), Some("typo"));
    }

    #[test]
    fn pillar_has_a_short_form_thick_thin_or_one_cell() {
        for (text, glyph) in [("thick", "▌"), ("thin", "▎"), ("▍", "▍")] {
            let mut registry = IconSetRegistry::builtin();
            let set = format!("[meta]\nname = \"P\"\n[icons]\npillar = \"{text}\"\n");
            assert!(registry.add_source("p", "p.toml", &set));
            let mut icons = registry.icons("p", &BTreeMap::new(), GlyphMode::Unicode);
            assert_eq!(icons.glyph(PILLAR), glyph);
            icons.set_mode(GlyphMode::Ascii);
            assert_eq!(icons.glyph(PILLAR), " ", "ASCII shows the pillar as a coloured cell");
        }
        let mut registry = IconSetRegistry::builtin();
        assert!(registry.add_source("p", "p.toml", "[meta]\nname = \"P\"\n[icons]\npillar = \"wide\"\n"));
        let messages: Vec<String> = registry.diagnostics().iter().map(ToString::to_string).collect();
        assert!(messages.iter().any(|m| m.contains("single one-cell character")), "{messages:?}");
    }

    #[test]
    fn overrides_replace_single_icons() {
        let overrides = BTreeMap::from([(
            "check".to_owned(),
            IconGlyphs { nerd: "C".to_owned(), unicode: "C".to_owned(), ascii: "C".to_owned() },
        )]);
        let icons = registry().icons("test", &overrides, GlyphMode::Nerd);
        assert_eq!(icons.glyph("check"), "C");
        assert_eq!(icons.glyph("spin"), "ab");
    }

    #[test]
    fn unknown_set_falls_back_to_default() {
        let icons = IconSetRegistry::builtin().icons("missing", &BTreeMap::new(), GlyphMode::Unicode);
        assert_eq!(icons.glyph("check"), "✓");
    }

    #[test]
    fn icon_mode_names_round_trip() {
        for mode in IconMode::ALL {
            assert_eq!(IconMode::from_name(mode.name()), Some(mode));
        }
        assert_eq!(IconMode::from_name(" NERD "), Some(IconMode::Nerd));
        assert_eq!(IconMode::from_name("emoji"), None);
    }
}
