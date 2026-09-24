//! The environment widgets draw in: active theme, icons, language, keymap and colour depth,
//! together with everything a settings screen needs to list the alternatives.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::color::ColorDepth;
use crate::diagnostics::{Diagnostic, Location};
use crate::graphics::{Graphics, GraphicsFacts};
use crate::i18n::I18n;
use crate::icons::{
    GlyphMode, IconMode, IconSetRegistry, Icons, PILLAR, PillarStyle, default_font_dirs, detect_glyph_mode,
};
use crate::keymap::Keymap;
use crate::theme::{Theme, ThemeRegistry};

/// Where an application's own theme, icon, locale and keymap files live.
///
/// Every kind of file can be given as text instead of as a path, for files compiled into the
/// binary with `include_str!`. An application that gives all of its files as text starts with
/// nothing beside it on disk, and a path it also names is then optional: when the path cannot be
/// read the text stands in for it and the reason becomes a [diagnostic](Env::diagnostics)
/// instead of stopping the program.
#[derive(Debug, Clone, Default)]
pub struct AssetDirs {
    /// Directory of `*.toml` theme files.
    pub themes: Option<PathBuf>,
    /// Theme files given as text, as `(file name, TOML text)`, loaded after `themes` so they
    /// win. The file stem is the theme id, as it is in a directory.
    pub theme_sources: Vec<(String, String)>,
    /// Directory of `*.toml` icon set files.
    pub icons: Option<PathBuf>,
    /// Icon set files given as text, as `(file name, TOML text)`, loaded after `icons` so they
    /// win. The file stem is the icon set id, as it is in a directory. Keys these sets add to the
    /// built-in set are drawn whatever set the theme chooses.
    pub icon_sources: Vec<(String, String)>,
    /// Directory of `*.toml` locale files.
    pub locales: Option<PathBuf>,
    /// Locale files given as text, as `(file name, TOML text)`, loaded after `locales` so they
    /// win. For files compiled into the binary with `include_str!`, which an installed program
    /// carries with it; the file name only labels diagnostics.
    pub locale_sources: Vec<(String, String)>,
    /// A keymap file layered over the built-in keymap.
    pub keymap: Option<PathBuf>,
    /// A keymap given as text, as `(file name, TOML text)`, layered over the built-in keymap and
    /// over `keymap`, so it wins. The file name only labels diagnostics.
    pub keymap_source: Option<(String, String)>,
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
    remote: bool,
    graphics: GraphicsFacts,
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
            remote: false,
            graphics: GraphicsFacts::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Loads the application's files over the built-ins and detects colour depth, glyphs,
    /// language and the kind of connection from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when a configured directory or file cannot be read and no text was
    /// given for that kind of file; with text given, an unreadable path is a diagnostic and the
    /// text stands in for it. Problems inside files are never errors; they are collected in
    /// [`Env::diagnostics`].
    pub fn load(dirs: &AssetDirs) -> io::Result<Self> {
        // Where the environment names no language, the operating system's own setting stands in
        // as the last of the variables a language is read from.
        Self::load_with(dirs, |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .or_else(|| (name == "LANG").then(sys_locale::get_locale).flatten())
        })
    }

    /// Loads the application's files like [`load`](Self::load), reading the variables it would
    /// read from the process environment (`LANG`, `LC_ALL`, `LC_TIME`, `TERM`, `COLORTERM`,
    /// `SSH_CONNECTION` and the rest) through `lookup` instead.
    ///
    /// For a test that runs an application with its real files: the machine's language and
    /// region would otherwise reach it, so the first day of the week, a number's decimal mark or
    /// the language itself would change from one machine to the next. `|_| None` is a machine
    /// with nothing set. Unlike `load`, the operating system's own language setting is never
    /// asked: only `lookup` answers.
    ///
    /// # Errors
    ///
    /// As for [`load`](Self::load).
    pub fn load_with(dirs: &AssetDirs, lookup: impl Fn(&str) -> Option<String>) -> io::Result<Self> {
        let mut env = Self::builtin();
        if let Some(dir) = &dirs.themes {
            let read = env.themes.load_dir(dir);
            stand_in(read, dir, !dirs.theme_sources.is_empty(), &mut env.diagnostics)?;
        }
        for (file, text) in &dirs.theme_sources {
            env.themes.add_source(&source_id(file), file, text);
        }
        if let Some(dir) = &dirs.icons {
            let read = env.icon_sets.load_dir(dir);
            stand_in(read, dir, !dirs.icon_sources.is_empty(), &mut env.diagnostics)?;
        }
        for (file, text) in &dirs.icon_sources {
            env.icon_sets.add_source(&source_id(file), file, text);
        }
        let mut i18n = I18n::builtin();
        if let Some(dir) = &dirs.locales {
            let read = i18n.load_dir(dir);
            stand_in(read, dir, !dirs.locale_sources.is_empty(), &mut env.diagnostics)?;
        }
        for (file, text) in &dirs.locale_sources {
            i18n.add_source(file, text);
        }
        if let Some(code) = i18n.detect_only(&lookup) {
            i18n.set_active(&code);
        }
        i18n.set_region(i18n.detect_region_only(&lookup).as_deref());
        if let Some(file) = &dirs.keymap {
            let read = load_keymap(file, &mut env.diagnostics);
            let has_source = dirs.keymap_source.is_some();
            if let Some(keymap) = stand_in(read, file, has_source, &mut env.diagnostics)? {
                env.keymap.overlay(&keymap);
            }
        }
        if let Some((file, text)) = &dirs.keymap_source {
            let keymap = Keymap::parse(file, text, &mut env.diagnostics);
            env.keymap.overlay(&keymap);
        }
        env.diagnostics.extend(env.themes.diagnostics().iter().cloned());
        env.diagnostics.extend(env.icon_sets.diagnostics().iter().cloned());
        env.diagnostics.extend(i18n.diagnostics().iter().cloned());
        env.diagnostics.extend(env.keymap.conflicts());
        env.i18n = Arc::new(i18n);
        env.depth = ColorDepth::detect(&lookup);
        env.force_reduced_motion(forced_reduced_motion(&lookup));
        env.icon_mode = IconMode::Auto;
        env.remote = detect_remote(&lookup);
        let (graphics, unknown) = GraphicsFacts::detect(&lookup);
        env.graphics = graphics;
        if let Some(value) = unknown {
            let known = Graphics::ALL.map(Graphics::name).join(", ");
            env.diagnostics.push(Diagnostic::warning(
                None,
                format!("unknown `{}` value `{value}`, expected one of {known}", crate::graphics::VARIABLE),
            ));
        }
        env.glyph_mode = detect_glyph_mode(IconMode::Auto, &lookup, &default_font_dirs(&lookup));
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

    /// `(id, name)` of every icon set, the way [`Env::themes`] lists the themes. A theme names
    /// the set it draws with, so this tells which sets a theme may name.
    #[must_use]
    pub fn icon_sets(&self) -> Vec<(String, String)> {
        self.icon_sets.list()
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

    /// Whether the terminal is at the other end of a remote connection, so every drawn frame
    /// travels over a network.
    ///
    /// True when `SSH_CONNECTION` or `SSH_TTY` is set and not empty, which is how an SSH server
    /// marks the session it started; an empty value counts as unset, the way an empty variable
    /// left over from another program does. Detected once by [`Env::load`], so it cannot change
    /// under a running application; [`Env::builtin`], the environment of tests, is never remote
    /// until [`Harness::set_remote`](crate::runtime::Harness::set_remote) says so.
    ///
    /// The runtime already uses it for the [`FrameLimit`](crate::runtime::FrameLimit) an
    /// application does not set. An application reads it to spend less on a slow link: fewer
    /// animations, smaller pictures, a plainer screen.
    #[must_use]
    pub fn remote(&self) -> bool {
        self.remote
    }

    /// Whether this process runs in a remote session, by the rule [`Env::remote`] uses, without
    /// loading an environment.
    ///
    /// It reads `SSH_CONNECTION` and `SSH_TTY` and nothing else, so it costs no file reads. An
    /// application calls it before [`Runtime::run`](crate::runtime::Runtime::run), where no
    /// `Env` is handed out yet, to choose what depends on the connection between frames, such as
    /// the size a picture is decoded at. In a view, [`Env::remote`] gives the same answer, and
    /// [`Harness::set_remote`](crate::runtime::Harness::set_remote) sets it in a test.
    #[must_use]
    pub fn remote_session() -> bool {
        detect_remote(|name: &str| std::env::var(name).ok())
    }

    /// Draws as a remote session would, instead of what was detected; see
    /// [`Harness::set_remote`](crate::runtime::Harness::set_remote).
    pub(crate) fn set_remote(&mut self, remote: bool) {
        self.remote = remote;
    }

    /// The way a picture can be drawn in this terminal.
    ///
    /// The runtime asks the terminal once, as it starts: a kitty graphics query and a request
    /// for its device attributes, with a wait of 150 ms at most that the attributes end, so a
    /// local terminal answers in milliseconds and starting never waits on the network. A kitty `OK` gives [`Graphics::Kitty`], attributes
    /// that list sixel give [`Graphics::Sixel`], and anything else, silence included, gives
    /// [`Graphics::HalfBlock`]. A kitty `OK` that arrives after the wait, over a very slow link,
    /// still gives [`Graphics::Kitty`] from then on. The answers never reach the
    /// application as keys. The terminal is not asked when its answer could not change the
    /// result, and never when it is not a terminal.
    ///
    /// Then the environment has its say:
    ///
    /// - 16 colours or ASCII glyphs give [`Graphics::None`]: no picture is drawn.
    /// - Inside tmux or GNU screen (`TMUX` or `STY` set and not empty) kitty and sixel become
    ///   half blocks, because the multiplexer does not pass them through.
    /// - The `QUVYTA_GRAPHICS` environment variable, set to `kitty`, `sixel`, `halfblock` or
    ///   `none`, wins over all of it, for a terminal the probe misjudges or a person who wants
    ///   something else. Any other value is ignored and becomes a [diagnostic](Env::diagnostics).
    ///
    /// [`Env::builtin`], the environment of tests, asks nothing and gives half blocks; see
    /// [`Harness::set_graphics`](crate::runtime::Harness::set_graphics) for the others.
    #[must_use]
    pub fn graphics(&self) -> Graphics {
        self.graphics.resolve(self.depth, self.glyph_mode)
    }

    /// Records what the terminal answered to the graphics probe; [`Env::graphics`] still applies
    /// its rules to it.
    pub(crate) fn set_terminal_graphics(&mut self, answer: Graphics) {
        self.graphics.answer = answer;
    }

    /// Whether the terminal's answer could change [`Env::graphics`], so the probe is worth its
    /// round trip.
    pub(crate) fn graphics_worth_asking(&self) -> bool {
        self.graphics.worth_asking(self.depth)
    }

    /// Draws as a terminal of `depth` would, instead of the depth that was detected. Lets a test
    /// see what a widget looks like where colours are scarce; see
    /// [`Harness::set_depth`](crate::runtime::Harness::set_depth).
    pub(crate) fn set_depth(&mut self, depth: ColorDepth) {
        self.depth = depth;
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
    pub(crate) fn force_reduced_motion(&mut self, forced: Option<bool>) {
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
        if i18n.select(code) {
            self.i18n = Arc::new(i18n);
        } else {
            self.diagnostics.push(Diagnostic::warning(None, format!("unknown locale `{code}`")));
        }
    }

    pub(crate) fn set_region(&mut self, region: Option<&str>) {
        let mut i18n = I18n::clone(&self.i18n);
        if i18n.set_region(region) {
            self.i18n = Arc::new(i18n);
        } else {
            let region = region.unwrap_or_default();
            self.diagnostics.push(Diagnostic::warning(None, format!("unknown region `{region}`")));
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

    /// Switches to the language, theme and icons the ecosystem's preferences resolved.
    pub(crate) fn apply_preferences(&mut self, preferences: &crate::storage::Preferences) {
        self.set_theme(&preferences.theme().value);
        self.set_locale(&preferences.language().value);
        self.set_icon_mode(preferences.icons().value);
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
/// Whether the variables an SSH server sets mark this session as remote: either of them set and
/// not empty. `lookup` reads the process environment in an application and a table in tests.
fn detect_remote(lookup: impl Fn(&str) -> Option<String>) -> bool {
    ["SSH_CONNECTION", "SSH_TTY"].iter().any(|name| lookup(name).is_some_and(|value| !value.is_empty()))
}

fn forced_reduced_motion(lookup: impl Fn(&str) -> Option<String>) -> Option<bool> {
    lookup("QUVYTA_REDUCED_MOTION").filter(|value| !value.is_empty()).map(|value| value != "0")
}

/// Lets text given for the same kind of file stand in for a path that cannot be read: with
/// `has_source` the reason becomes a warning and loading carries on, without it the I/O error
/// travels on, because then nothing would take the file's place.
fn stand_in<T>(
    read: io::Result<T>,
    path: &Path,
    has_source: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> io::Result<Option<T>> {
    match read {
        Ok(value) => Ok(Some(value)),
        Err(error) if has_source => {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            diagnostics.push(Diagnostic::warning(
                Some(Location::from_offset(name, "", 0)),
                format!("cannot read `{}`, the text given instead is used: {error}", path.display()),
            ));
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

/// The asset id of a file given as text: its stem, the way a directory names its files.
fn source_id(file: &str) -> String {
    Path::new(file).file_stem().and_then(|stem| stem.to_str()).unwrap_or(file).to_owned()
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
    fn a_test_can_load_the_files_without_the_machines_language_and_region() {
        let turkish = |name: &str| (name == "LANG").then(|| "tr_TR.UTF-8".to_owned());
        let env = Env::load_with(&AssetDirs::default(), turkish).expect("the built-in files load");
        assert_eq!(env.i18n().active(), "tr");
        assert_eq!(env.i18n().first_weekday(), crate::date::Weekday::Monday, "Turkey starts the week on Monday");
        let mut english = Env::load_with(&AssetDirs::default(), |_| None).expect("the built-in files load");
        assert_eq!(english.i18n().active(), "en", "a machine with nothing set");
        assert_eq!(english.i18n().first_weekday(), crate::date::Weekday::Sunday, "English alone starts on Sunday");
        english.set_locale("en");
        assert_eq!(english.i18n().first_weekday(), crate::date::Weekday::Sunday);
    }

    /// Every combination of the two variables an SSH server sets, with empty values among them.
    /// The process environment itself is never changed: `detect_remote` is given a table, which
    /// is what `Env::load` gives it in an application too.
    #[test]
    fn a_connection_is_remote_when_either_ssh_variable_carries_a_value() {
        let cases = [
            (None, None, false),
            (Some(""), None, false),
            (None, Some(""), false),
            (Some(""), Some(""), false),
            (Some("10.0.0.2 51150 10.0.0.9 22"), None, true),
            (None, Some("/dev/pts/3"), true),
            (Some("10.0.0.2 51150 10.0.0.9 22"), Some("/dev/pts/3"), true),
            (Some(""), Some("/dev/pts/3"), true),
            (Some("10.0.0.2 51150 10.0.0.9 22"), Some(""), true),
        ];
        for (connection, tty, remote) in cases {
            let lookup = |name: &str| match name {
                "SSH_CONNECTION" => connection.map(str::to_owned),
                "SSH_TTY" => tty.map(str::to_owned),
                _ => None,
            };
            assert_eq!(detect_remote(lookup), remote, "SSH_CONNECTION={connection:?} SSH_TTY={tty:?}");
        }
    }

    /// `Env::load_with` reads the multiplexer and the override through the same lookup as the
    /// rest, so a test decides them without touching the process environment.
    #[test]
    fn graphics_follow_the_multiplexer_and_the_override_the_lookup_gives() {
        /// A 256-colour UTF-8 terminal with Unicode glyphs, where half blocks can be drawn, plus
        /// `extra`.
        fn terminal(extra: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
            move |name: &str| {
                let base = [("LANG", "en_US.UTF-8"), ("TERM", "xterm-256color"), ("QUVYTA_ICONS", "unicode")];
                base.iter().chain(extra).find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned())
            }
        }
        let mut env = Env::load_with(&AssetDirs::default(), terminal(&[])).expect("the built-in files load");
        assert!(env.graphics_worth_asking());
        env.set_terminal_graphics(Graphics::Kitty);
        assert_eq!(env.graphics(), Graphics::Kitty, "outside a multiplexer the answer stands");

        let in_tmux = terminal(&[("TMUX", "/tmp/tmux-1000/default,4242,0")]);
        let mut env = Env::load_with(&AssetDirs::default(), in_tmux).expect("the built-in files load");
        assert!(!env.graphics_worth_asking(), "tmux answers for the terminal, so it is not asked");
        env.set_terminal_graphics(Graphics::Kitty);
        assert_eq!(env.graphics(), Graphics::HalfBlock);

        let forced = terminal(&[("QUVYTA_GRAPHICS", "sixel"), ("STY", "1234.pts-0.host")]);
        let env = Env::load_with(&AssetDirs::default(), forced).expect("the built-in files load");
        assert_eq!(env.graphics(), Graphics::Sixel, "the override wins over the multiplexer");

        let unknown = terminal(&[("QUVYTA_GRAPHICS", "pixels")]);
        let env = Env::load_with(&AssetDirs::default(), unknown).expect("the built-in files load");
        assert_eq!(env.graphics(), Graphics::HalfBlock);
        assert!(
            env.diagnostics().iter().any(|problem| problem.to_string().contains("`pixels`")),
            "{:?}",
            env.diagnostics()
        );
    }

    #[test]
    fn the_session_is_asked_by_the_same_rule_the_environment_uses() {
        let lookup = |name: &str| std::env::var(name).ok();
        assert_eq!(Env::remote_session(), detect_remote(lookup), "no file is read, the variables alone decide");
    }

    #[test]
    fn the_environment_of_tests_is_never_remote() {
        assert!(!Env::builtin().remote(), "a test must draw the same wherever it runs");
    }

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

    #[test]
    fn locales_given_as_text_load_over_the_built_ins_and_report_problems_by_file() {
        let english = "[meta]\nname = \"English\"\ncode = \"en\"\n[app]\ngreeting = \"Hello\"\n";
        let turkish = "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\nfallback = \"en\"\n[app]\ngreeting = \"Merhaba\"\n";
        let dirs = AssetDirs {
            locale_sources: vec![
                ("app-en.toml".to_owned(), english.to_owned()),
                ("app-tr.toml".to_owned(), turkish.to_owned()),
                ("broken.toml".to_owned(), "[meta\n".to_owned()),
            ],
            ..AssetDirs::default()
        };
        let env = Env::load(&dirs).expect("nothing to read from disk");
        let mut i18n = env.i18n().clone();
        assert!(i18n.set_active("tr"));
        assert_eq!(i18n.translate("app.greeting", &[]), "Merhaba");
        assert!(i18n.set_active("en"));
        assert_eq!(i18n.translate("app.greeting", &[]), "Hello");
        assert_eq!(i18n.translate("quvyta.keys.quit", &[]), "quit", "built-in text stays");
        assert!(
            env.diagnostics().iter().any(|problem| problem.to_string().contains("broken.toml")),
            "{:?}",
            env.diagnostics()
        );
    }

    /// A theme, an icon set and a keymap an application would compile into its binary.
    const BRAND_THEME: &str = "[meta]\nname = \"Brand\"\nextends = \"monochrome\"\nicon-set = \"brand\"\n\
                               [colors]\naccent = \"#FF8800\"\n";
    const BRAND_ICONS: &str =
        "[meta]\nname = \"Brand\"\n[icons]\ncheck = { nerd = \"!\", unicode = \"!\", ascii = \"!\" }\n";
    const BRAND_KEYS: &str = "[app]\nsave = \"ctrl+s\"\n";

    /// Everything an application gives as text, and nothing on disk.
    fn brand_sources() -> AssetDirs {
        AssetDirs {
            theme_sources: vec![("brand.toml".to_owned(), BRAND_THEME.to_owned())],
            icon_sources: vec![("brand.toml".to_owned(), BRAND_ICONS.to_owned())],
            keymap_source: Some(("keymap.toml".to_owned(), BRAND_KEYS.to_owned())),
            ..AssetDirs::default()
        }
    }

    fn chord(text: &str) -> crate::keymap::KeyChord {
        text.parse().expect("a chord")
    }

    #[test]
    fn a_theme_an_icon_set_and_a_keymap_given_as_text_load_with_no_files_on_disk() {
        let mut env = Env::load(&brand_sources()).expect("nothing to read from disk");
        assert!(env.diagnostics().is_empty(), "{:?}", env.diagnostics());
        assert!(env.themes().iter().any(|(id, name)| id == "brand" && name == "Brand"));
        env.set_theme("brand");
        assert_eq!(env.theme().id(), "brand");
        assert_eq!(env.theme().color("accent").map(|c| c.to_string()).as_deref(), Some("#ff8800"));
        env.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(env.icons().glyph("check"), "!", "the icon set the theme names came from text");
        assert_eq!(
            env.keymap().action_for(chord("ctrl+s")),
            Some((crate::keymap::Scope::App, "save")),
            "the keymap came from text"
        );
        assert_eq!(
            env.keymap().action_for(chord("ctrl+q")),
            Some((crate::keymap::Scope::Global, "quit")),
            "the built-in keymap is still under it"
        );
    }

    #[test]
    fn an_application_icon_is_found_in_every_theme_and_follows_the_icon_mode() {
        let app = "[icons]\n\"category.internet\" = { nerd = \"I\", unicode = \"◎\", ascii = \"@\" }\n";
        let mut dirs = brand_sources();
        dirs.icon_sources.push(("app.toml".to_owned(), app.to_owned()));
        let mut env = Env::load(&dirs).expect("nothing to read from disk");
        env.set_icon_mode(IconMode::Nerd);
        assert_eq!(env.icons().glyph("category.internet"), "I");
        for theme in ["nordic", "amber", "brand"] {
            env.set_theme(theme);
            assert_eq!(env.theme().id(), theme);
            env.set_icon_mode(IconMode::Unicode);
            assert_eq!(env.icons().glyph("category.internet"), "◎", "{theme}");
            env.set_icon_mode(IconMode::Ascii);
            assert_eq!(env.icons().glyph("category.internet"), "@", "{theme}");
        }
        assert_eq!(env.icons().glyph("check"), "!", "the brand theme's own set still restyles what it names");
    }

    #[test]
    fn a_missing_path_no_longer_stops_the_start_when_text_stands_in_for_it() {
        let missing = std::env::temp_dir().join("quvyta-not-installed");
        let dirs = AssetDirs {
            themes: Some(missing.join("themes")),
            icons: Some(missing.join("icons")),
            locales: Some(missing.join("locales")),
            locale_sources: vec![(
                "en.toml".to_owned(),
                "[meta]\nname = \"English\"\ncode = \"en\"\n[app]\ngreeting = \"Hello\"\n".to_owned(),
            )],
            keymap: Some(missing.join("keymap.toml")),
            ..brand_sources()
        };
        let mut env = Env::load(&dirs).expect("the text compiled in stands in for the files");
        env.set_theme("brand");
        assert_eq!(env.theme().id(), "brand");
        assert_eq!(env.keymap().action_for(chord("ctrl+s")), Some((crate::keymap::Scope::App, "save")));
        assert_eq!(env.i18n().translate("app.greeting", &[]), "Hello");
        for file in ["themes", "icons", "locales", "keymap.toml"] {
            assert!(
                env.diagnostics().iter().any(|problem| problem.to_string().contains(file)),
                "the unreadable {file} is reported: {:?}",
                env.diagnostics()
            );
        }
        let alone = AssetDirs { keymap: Some(missing.join("keymap.toml")), ..AssetDirs::default() };
        assert!(Env::load(&alone).is_err(), "without text to stand in for it a named file must be there");
    }

    #[test]
    fn broken_text_sources_are_skipped_with_located_diagnostics_and_the_built_ins_still_work() {
        let dirs = AssetDirs {
            theme_sources: vec![("brand.toml".to_owned(), "[meta\n".to_owned())],
            icon_sources: vec![("brand.toml".to_owned(), "[icons\n".to_owned())],
            keymap_source: Some(("keymap.toml".to_owned(), "[app\n".to_owned())),
            ..AssetDirs::default()
        };
        let mut env = Env::load(&dirs).expect("broken text is never an I/O error");
        for file in ["brand.toml", "keymap.toml"] {
            assert!(
                env.diagnostics().iter().any(|problem| problem
                    .location
                    .as_ref()
                    .is_some_and(|at| at.file == file && at.line > 0 && at.column > 0)),
                "{file} is reported with file, line and column: {:?}",
                env.diagnostics()
            );
        }
        assert_eq!(env.theme().id(), "monochrome");
        env.set_glyph_mode(GlyphMode::Unicode);
        assert_eq!(env.icons().glyph("check"), "✓", "the built-in icon set is still there");
        assert_eq!(env.keymap().action_for(chord("ctrl+q")), Some((crate::keymap::Scope::Global, "quit")));
        env.set_theme("brand");
        assert_eq!(env.theme().id(), "monochrome", "an unusable theme falls back to the default");
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
