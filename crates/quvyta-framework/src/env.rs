//! The environment widgets draw in: active theme, icons, language, keymap and colour depth,
//! together with everything a settings screen needs to list the alternatives.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::color::ColorDepth;
use crate::diagnostics::Diagnostic;
use crate::i18n::I18n;
use crate::icons::{
    GlyphMode, IconMode, IconSetRegistry, Icons, PILLAR, PillarStyle, default_font_dirs, detect_glyph_mode,
};
use crate::keymap::Keymap;
use crate::theme::{Theme, ThemeRegistry};

/// Where an application's own theme, icon, locale and keymap files live.
#[derive(Debug, Clone, Default)]
pub struct AssetDirs {
    /// Directory of `*.toml` theme files.
    pub themes: Option<PathBuf>,
    /// Directory of `*.toml` icon set files.
    pub icons: Option<PathBuf>,
    /// Directory of `*.toml` locale files.
    pub locales: Option<PathBuf>,
    /// A keymap file layered over the built-in keymap.
    pub keymap: Option<PathBuf>,
}

/// Everything widgets need to know about how to draw and label themselves.
#[derive(Debug, Clone)]
pub struct Env {
    themes: ThemeRegistry,
    theme: Theme,
    icon_sets: IconSetRegistry,
    icon_mode: IconMode,
    glyph_mode: GlyphMode,
    icons: Icons,
    i18n: Arc<I18n>,
    keymap: Keymap,
    depth: ColorDepth,
    reduced_motion: bool,
    /// Reduced motion as the `QUVYTA_REDUCED_MOTION` environment variable forces it, if set.
    forced_reduced_motion: Option<bool>,
    pillar: Option<PillarStyle>,
    slide: Option<bool>,
    diagnostics: Vec<Diagnostic>,
}

impl Env {
    /// Built-in files only, the `monochrome` theme, Unicode glyphs, English and 24-bit colour.
    /// Deterministic, which makes it the environment for tests.
    #[must_use]
    pub fn builtin() -> Self {
        let themes = ThemeRegistry::builtin();
        let (theme, _) = themes.resolve_or_default("monochrome");
        let icon_sets = IconSetRegistry::builtin();
        let icons = icon_sets.icons(theme.icon_set(), theme.icon_overrides(), GlyphMode::Unicode);
        Self {
            themes,
            theme,
            icon_sets,
            icon_mode: IconMode::Unicode,
            glyph_mode: GlyphMode::Unicode,
            icons,
            i18n: Arc::new(I18n::builtin()),
            keymap: Keymap::builtin(),
            depth: ColorDepth::TrueColor,
            reduced_motion: false,
            forced_reduced_motion: None,
            pillar: None,
            slide: None,
            diagnostics: Vec::new(),
        }
    }

    /// Loads the application's files over the built-ins and detects colour depth, glyphs and
    /// language from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when a configured directory or file cannot be read. Problems inside
    /// files are not errors; they are collected in [`Env::diagnostics`].
    pub fn load(dirs: &AssetDirs) -> io::Result<Self> {
        let lookup = |name: &str| std::env::var(name).ok();
        let mut env = Self::builtin();
        if let Some(dir) = &dirs.themes {
            env.themes.load_dir(dir)?;
        }
        if let Some(dir) = &dirs.icons {
            env.icon_sets.load_dir(dir)?;
        }
        let mut i18n = I18n::builtin();
        if let Some(dir) = &dirs.locales {
            i18n.load_dir(dir)?;
        }
        if let Some(code) = i18n.detect(lookup) {
            i18n.set_active(&code);
        }
        if let Some(file) = &dirs.keymap {
            env.keymap.overlay(&load_keymap(file, &mut env.diagnostics)?);
        }
        env.diagnostics.extend(env.themes.diagnostics().iter().cloned());
        env.diagnostics.extend(env.icon_sets.diagnostics().iter().cloned());
        env.diagnostics.extend(i18n.diagnostics().iter().cloned());
        env.diagnostics.extend(env.keymap.conflicts());
        env.i18n = Arc::new(i18n);
        env.depth = ColorDepth::detect(lookup);
        env.force_reduced_motion(forced_reduced_motion(lookup));
        env.icon_mode = IconMode::Auto;
        env.glyph_mode = detect_glyph_mode(IconMode::Auto, lookup, &default_font_dirs(lookup));
        env.rebuild_icons();
        Ok(env)
    }

    /// The active theme.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// `(id, name)` of every theme.
    #[must_use]
    pub fn themes(&self) -> Vec<(String, String)> {
        self.themes.list()
    }

    /// The icons in the active glyph mode.
    #[must_use]
    pub fn icons(&self) -> &Icons {
        &self.icons
    }

    /// The chosen icon mode.
    #[must_use]
    pub fn icon_mode(&self) -> IconMode {
        self.icon_mode
    }

    /// The glyph column actually drawn.
    #[must_use]
    pub fn glyph_mode(&self) -> GlyphMode {
        self.glyph_mode
    }

    /// The translator.
    #[must_use]
    pub fn i18n(&self) -> &I18n {
        &self.i18n
    }

    /// The keymap.
    #[must_use]
    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    /// The keymap, to bind actions in code, e.g. before handing the environment to a
    /// [`Harness`](crate::runtime::Harness).
    pub fn keymap_mut(&mut self) -> &mut Keymap {
        &mut self.keymap
    }

    /// The terminal's colour depth.
    #[must_use]
    pub fn depth(&self) -> ColorDepth {
        self.depth
    }

    /// Whether animations are reduced: layers appear at once, nothing breathes or spins.
    ///
    /// The `QUVYTA_REDUCED_MOTION` environment variable, read by [`Env::load`], decides when it
    /// is set: `0` keeps motion, any other non-empty value reduces it. It wins over a saved
    /// `reduced-motion` setting and over `Command::set_reduced_motion`, because a choice made in
    /// the user's shell is the stronger signal, the way accessibility overrides work. Unset or
    /// empty, the saved setting and the application decide.
    #[must_use]
    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    /// Whether the `QUVYTA_REDUCED_MOTION` environment variable decides reduced motion, so neither
    /// a saved setting nor `Command::set_reduced_motion` can change it. A settings screen uses it to
    /// show its reduced-motion switch as decided by the environment instead of letting the switch
    /// snap back when pressed.
    #[must_use]
    pub fn reduced_motion_forced(&self) -> bool {
        self.forced_reduced_motion.is_some()
    }

    /// Lets `forced` decide reduced motion from now on, whatever is set or saved later; `None`
    /// leaves the decision to settings and commands.
    fn force_reduced_motion(&mut self, forced: Option<bool>) {
        self.forced_reduced_motion = forced;
        if let Some(reduced) = forced {
            self.reduced_motion = reduced;
        }
    }

    /// Reduces motion or brings it back, unless the environment variable already decided.
    pub(crate) fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = self.forced_reduced_motion.unwrap_or(reduced);
    }

    /// The pillar the user chose over the theme's, if any.
    #[must_use]
    pub fn pillar_style(&self) -> Option<PillarStyle> {
        self.pillar
    }

    pub(crate) fn set_pillar_style(&mut self, style: PillarStyle) {
        self.pillar = Some(style);
        self.rebuild_icons();
    }

    /// Whether list structures (lists, menus, trees, tables, tab strips and rails, dropdown options)
    /// slide the leading text of hovered and selected rows one cell; buttons and fields never do.
    /// The user's choice when made, otherwise the theme's `motion.slide`.
    #[must_use]
    pub fn slide(&self) -> bool {
        self.slide.unwrap_or(self.theme.motion().slide)
    }

    pub(crate) fn set_slide(&mut self, slide: bool) {
        self.slide = Some(slide);
    }

    /// Problems found in theme, icon, locale and keymap files, including theme switches that
    /// fell back to the default.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub(crate) fn i18n_arc(&self) -> Arc<I18n> {
        Arc::clone(&self.i18n)
    }

    /// Activates theme `id`; falls back to the built-in default and records why when it
    /// cannot be loaded.
    pub(crate) fn set_theme(&mut self, id: &str) {
        let (theme, diagnostics) = self.themes.resolve_or_default(id);
        self.diagnostics.extend(diagnostics);
        self.theme = theme;
        self.rebuild_icons();
    }

    pub(crate) fn set_locale(&mut self, code: &str) {
        let mut i18n = I18n::clone(&self.i18n);
        if i18n.set_active(code) {
            self.i18n = Arc::new(i18n);
        } else {
            self.diagnostics.push(Diagnostic::warning(None, format!("unknown locale `{code}`")));
        }
    }

    pub(crate) fn set_icon_mode(&mut self, mode: IconMode) {
        let lookup = |name: &str| std::env::var(name).ok();
        self.icon_mode = mode;
        self.glyph_mode = match mode {
            IconMode::Nerd => GlyphMode::Nerd,
            IconMode::Unicode => GlyphMode::Unicode,
            IconMode::Ascii => GlyphMode::Ascii,
            IconMode::Auto => detect_glyph_mode(IconMode::Auto, lookup, &default_font_dirs(lookup)),
        };
        self.rebuild_icons();
    }

    /// Sets glyph mode directly; used by tests to render every mode.
    pub(crate) fn set_glyph_mode(&mut self, mode: GlyphMode) {
        self.glyph_mode = mode;
        self.rebuild_icons();
    }

    /// Switches to the theme, language, icon mode, reduced motion, pillar and slide saved in
    /// `settings`; `QUVYTA_REDUCED_MOTION`, when set, still decides reduced motion.
    pub(crate) fn apply_settings(&mut self, settings: &crate::storage::Settings) {
        if let Some(theme) = settings.theme() {
            self.set_theme(&theme);
        }
        if let Some(language) = settings.language() {
            self.set_locale(&language);
        }
        if let Some(mode) = settings.icon_mode() {
            self.set_icon_mode(mode);
        }
        if let Some(reduced) = settings.reduced_motion() {
            self.set_reduced_motion(reduced);
        }
        if let Some(style) = settings.pillar_style() {
            self.set_pillar_style(style);
        }
        if let Some(slide) = settings.slide() {
            self.set_slide(slide);
        }
    }

    fn rebuild_icons(&mut self) {
        let mut overrides: BTreeMap<_, _> = self.theme.icon_overrides().clone();
        if let Some(style) = self.pillar {
            overrides.insert(PILLAR.to_owned(), style.glyphs());
        }
        self.icons = self.icon_sets.icons_with_animations(
            self.theme.icon_set(),
            &overrides,
            self.theme.animation_overrides(),
            self.glyph_mode,
        );
    }
}

/// What `QUVYTA_REDUCED_MOTION` forces: nothing when unset or empty, motion for `0`, reduced
/// motion for any other value.
fn forced_reduced_motion(lookup: impl Fn(&str) -> Option<String>) -> Option<bool> {
    lookup("QUVYTA_REDUCED_MOTION").filter(|value| !value.is_empty()).map(|value| value != "0")
}

fn load_keymap(file: &Path, diagnostics: &mut Vec<Diagnostic>) -> io::Result<Keymap> {
    let text = std::fs::read_to_string(file)?;
    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("keymap.toml");
    Ok(Keymap::parse(name, &text, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_choices_for_pillar_and_slide_win_over_the_theme_and_survive_a_theme_switch() {
        let mut env = Env::builtin();
        assert_eq!(env.icons().glyph(PILLAR), "▌");
        assert!(env.slide());
        env.set_pillar_style(PillarStyle::Thin);
        env.set_slide(false);
        env.set_theme("amber");
        assert_eq!(env.icons().glyph(PILLAR), "▎");
        assert!(!env.slide());
        let mut settings = crate::storage::Settings::in_memory();
        settings.set(crate::storage::Settings::PILLAR, "thick".to_owned());
        settings.set(crate::storage::Settings::SLIDE, true);
        env.apply_settings(&settings);
        assert_eq!(env.icons().glyph(PILLAR), "▌");
        assert!(env.slide());
    }

    /// Looks names up in `vars` instead of the process environment.
    fn vars(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: Vec<(String, String)> = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    /// The built-in environment as `Env::load` leaves it for these variables.
    fn env_with(variables: &[(&str, &str)]) -> Env {
        let mut env = Env::builtin();
        env.force_reduced_motion(forced_reduced_motion(vars(variables)));
        env
    }

    fn saved_reduced_motion(reduced: bool) -> crate::storage::Settings {
        let mut settings = crate::storage::Settings::in_memory();
        settings.set(crate::storage::Settings::REDUCED_MOTION, reduced);
        settings
    }

    #[test]
    fn reads_the_reduced_motion_variable() {
        assert_eq!(forced_reduced_motion(vars(&[])), None);
        assert_eq!(forced_reduced_motion(vars(&[("QUVYTA_REDUCED_MOTION", "")])), None);
        assert_eq!(forced_reduced_motion(vars(&[("QUVYTA_REDUCED_MOTION", "0")])), Some(false));
        assert_eq!(forced_reduced_motion(vars(&[("QUVYTA_REDUCED_MOTION", "1")])), Some(true));
        assert_eq!(forced_reduced_motion(vars(&[("QUVYTA_REDUCED_MOTION", "yes")])), Some(true));
    }

    #[test]
    fn tells_whether_the_variable_decides() {
        assert!(!Env::builtin().reduced_motion_forced());
        assert!(!env_with(&[]).reduced_motion_forced());
        assert!(!env_with(&[("QUVYTA_REDUCED_MOTION", "")]).reduced_motion_forced(), "empty is unset");
        let mut env = env_with(&[("QUVYTA_REDUCED_MOTION", "1")]);
        env.apply_settings(&saved_reduced_motion(false));
        assert!(env.reduced_motion_forced() && env.reduced_motion());
        let env = env_with(&[("QUVYTA_REDUCED_MOTION", "0")]);
        assert!(env.reduced_motion_forced() && !env.reduced_motion(), "forced to keep motion counts too");
    }

    #[test]
    fn the_variable_wins_over_the_saved_setting_in_both_directions() {
        let mut env = env_with(&[("QUVYTA_REDUCED_MOTION", "1")]);
        env.apply_settings(&saved_reduced_motion(false));
        assert!(env.reduced_motion(), "the shell asked for reduced motion; the saved `false` loses");
        let mut env = env_with(&[("QUVYTA_REDUCED_MOTION", "0")]);
        env.apply_settings(&saved_reduced_motion(true));
        assert!(!env.reduced_motion(), "the shell asked for motion; the saved `true` loses");
        let mut env = env_with(&[]);
        env.apply_settings(&saved_reduced_motion(true));
        assert!(env.reduced_motion(), "without the variable the saved setting decides");
    }

    #[test]
    fn the_variable_wins_when_the_setting_was_applied_first() {
        let mut env = Env::builtin();
        env.apply_settings(&saved_reduced_motion(false));
        env.force_reduced_motion(forced_reduced_motion(vars(&[("QUVYTA_REDUCED_MOTION", "1")])));
        assert!(env.reduced_motion());
        let mut env = Env::builtin();
        env.apply_settings(&saved_reduced_motion(true));
        env.force_reduced_motion(forced_reduced_motion(vars(&[("QUVYTA_REDUCED_MOTION", "0")])));
        assert!(!env.reduced_motion());
        let mut env = Env::builtin();
        env.apply_settings(&saved_reduced_motion(true));
        env.force_reduced_motion(forced_reduced_motion(vars(&[])));
        assert!(env.reduced_motion(), "an unset variable leaves the saved choice alone");
    }

    #[test]
    fn the_variable_wins_over_settings_applied_as_commands_after_start() {
        use crate::runtime::{App, Command, Harness};
        use crate::widget::View;

        struct Saved(crate::storage::Settings);
        impl App for Saved {
            type Msg = ();
            fn update(&mut self, (): ()) -> Command<()> {
                self.0.apply()
            }
            fn view(&self, _: &mut View<'_, ()>) {}
        }

        let mut h =
            Harness::with_env(Saved(saved_reduced_motion(false)), env_with(&[("QUVYTA_REDUCED_MOTION", "1")]), 10, 1);
        h.send(());
        assert!(h.env().reduced_motion());
        let mut h =
            Harness::with_env(Saved(saved_reduced_motion(true)), env_with(&[("QUVYTA_REDUCED_MOTION", "0")]), 10, 1);
        h.send(());
        assert!(!h.env().reduced_motion());
        let mut h = Harness::with_env(Saved(saved_reduced_motion(true)), env_with(&[]), 10, 1);
        h.send(());
        assert!(h.env().reduced_motion(), "without the variable the saved setting decides");
    }

    #[test]
    fn switches_theme_locale_and_icons() {
        let mut env = Env::builtin();
        assert_eq!(env.theme().id(), "monochrome");
        env.set_theme("nordic");
        assert_eq!(env.theme().id(), "nordic");
        env.set_theme("missing");
        assert_eq!(env.theme().id(), "monochrome");
        assert!(env.diagnostics().iter().any(|d| d.message.contains("`missing`")));
        env.set_locale("tr");
        assert_eq!(env.i18n().active(), "tr");
        env.set_icon_mode(IconMode::Ascii);
        assert_eq!(env.icons().glyph("check"), "v");
        env.set_glyph_mode(GlyphMode::Unicode);
        assert_eq!(env.icons().glyph("check"), "✓");
    }
}
