//! Animation studio: every built-in one-cell animation in the three glyph modes at once, and a
//! studio that edits a working copy frame by frame, previews it live and copies it as TOML.

mod draft;
mod editor;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::Duration;

use qframe::animation::{CellAnimation, ColorMode, Playback, parse_animations};
use qframe::icons::{GlyphMode, IconSetRegistry};
use qframe::prelude::*;
use qframe::storage::Settings;
use qframe::style::CellStyle;
use qframe::text;
use qframe::widget::{MeasureCx, PaintCx, Widget};

use super::{PageMsg, storage};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;
use draft::{Draft, FrameDraft, MODES, Problem, TimeDraft};

const PAGE: &str = "cell-animation";

/// Where the studio keeps its working animations: one TOML text of `[animations.<name>]` tables,
/// under the open `studio` prefix of the showcase settings.
pub const STUDIO_KEY: &str = "studio.animations";

/// How long a single-play animation rests on its last frame before the demo plays it again.
const REPLAY_PAUSE: Duration = Duration::from_millis(1200);

/// The studio: the built-in animations to start from, the working copies and the one being edited.
#[derive(Debug)]
pub struct State {
    builtin: Vec<(String, Arc<CellAnimation>)>,
    drafts: Vec<Draft>,
    current: usize,
    /// The current draft built, its problems and each frame alone for the frame rows.
    built: Arc<CellAnimation>,
    problems: Vec<Problem>,
    frame_cells: Vec<Option<Arc<CellAnimation>>>,
    /// Problems found in the saved studio when the showcase started.
    restored: Vec<String>,
    /// Bumped by Replay, so single-play previews start again.
    replay: u32,
}

impl Default for State {
    fn default() -> Self {
        let icons = IconSetRegistry::builtin().icons("default", &std::collections::BTreeMap::new(), GlyphMode::Unicode);
        let builtin: Vec<(String, Arc<CellAnimation>)> = icons
            .animation_names()
            .filter_map(|name| icons.animation(name).map(|animation| (name.to_owned(), Arc::clone(animation))))
            .collect();
        let first = builtin
            .first()
            .map_or_else(|| Draft::new("my-animation".to_owned()), |(name, a)| Draft::from_animation(name, a));
        let mut state = Self {
            builtin,
            drafts: vec![first],
            current: 0,
            built: Arc::new(CellAnimation::new()),
            problems: Vec::new(),
            frame_cells: Vec::new(),
            restored: Vec::new(),
            replay: 0,
        };
        state.refresh();
        state
    }
}

impl State {
    // region: studio-restore
    /// The studio with the working animations saved in `settings`.
    #[must_use]
    pub fn restore(settings: &Settings) -> Self {
        let mut state = Self::default();
        let Some(saved) = settings.get::<String>(STUDIO_KEY) else {
            return state;
        };
        let (animations, diagnostics) = parse_animations("studio", &saved);
        state.restored = diagnostics.iter().map(ToString::to_string).collect();
        if !animations.is_empty() {
            state.drafts = animations.iter().map(|(name, animation)| Draft::from_animation(name, animation)).collect();
            state.refresh();
        }
        state
    }
    // endregion

    fn draft(&self) -> &Draft {
        &self.drafts[self.current]
    }

    fn draft_mut(&mut self) -> &mut Draft {
        &mut self.drafts[self.current]
    }

    /// The built-in animation called `name`.
    fn builtin(&self, name: &str) -> Option<&Arc<CellAnimation>> {
        self.builtin.iter().find(|(builtin, _)| builtin == name).map(|(_, animation)| animation)
    }

    /// Rebuilds the current animation and its frame rows after a change.
    fn refresh(&mut self) {
        let draft = self.draft();
        let (built, problems) = draft.build();
        let frame_cells = draft
            .frames
            .iter()
            .map(|frame| {
                let alone = Draft { frames: vec![frame.clone()], rest: None, ..draft.clone() };
                let (animation, _) = alone.build();
                (!animation.frames().is_empty()).then(|| Arc::new(animation))
            })
            .collect();
        self.built = Arc::new(built);
        self.problems = problems;
        self.frame_cells = frame_cells;
    }

    /// Whether `draft` differs from the built-in animation of its name, or has no built-in.
    fn changed(&self, draft: &Draft) -> bool {
        self.builtin(&draft.name).is_none_or(|builtin| draft.build().0 != **builtin)
    }

    /// The names the start list offers after "New": built-ins, then the user's own animations.
    fn names(&self) -> Vec<String> {
        let own = self.drafts.iter().filter(|d| self.builtin(&d.name).is_none()).map(|d| d.name.clone());
        self.builtin.iter().map(|(name, _)| name.clone()).chain(own).collect()
    }
}

/// Studio messages.
#[derive(Debug, Clone)]
pub enum Msg {
    /// Opens the start list's entry: 0 is a new animation, then [`State::names`].
    Open(usize),
    Name(String),
    TimeKind(usize),
    MotionKey(usize),
    Millis(f64),
    Playback(usize),
    Colors(usize),
    Rest(usize),
    Glyph(usize, usize, String),
    Color(usize, String),
    Duration(usize, String),
    Add,
    Remove(usize),
    Up(usize),
    Down(usize),
    Copy,
    Replay,
    Reset,
}

pub(crate) fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::CellAnimation(message))
}

// region: studio-save
/// The TOML of every working animation that is the user's own or differs from its built-in; a
/// draft is kept once its name is valid.
fn saved_text(state: &State) -> String {
    let changed = state.drafts.iter().filter(|draft| state.changed(draft));
    let valid = changed.filter(|draft| qframe::animation::is_valid_name(&draft.name));
    valid.map(|draft| draft.build().0.to_toml(&draft.name)).collect::<Vec<_>>().join("\n")
}

/// Remembers the working animations, or forgets them when nothing differs from the built-ins.
fn save(state: &State, settings: &mut Settings) -> Command<AppMsg> {
    let text = saved_text(state);
    if text.is_empty() {
        if settings.remove(STUDIO_KEY) {
            return settings.save_command(|result| AppMsg::Page(PageMsg::Storage(storage::Msg::Saved(result))));
        }
        return Command::none();
    }
    storage::remember(settings, STUDIO_KEY, text)
}
// endregion

/// Applies a studio message, logs it and saves the working animations.
pub fn update(state: &mut State, settings: &mut Settings, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    let name = state.draft().name.clone();
    let source = format!("Studio#{name}");
    match message {
        Msg::Open(0) => {
            let taken: Vec<String> = state.names();
            let fresh = (1..).map(|n| if n == 1 { "my-animation".to_owned() } else { format!("my-animation-{n}") });
            let name = fresh.into_iter().find(|name| !taken.contains(name)).unwrap_or_default();
            log.push(PAGE, "Select#start", format!("new animation {name}"));
            state.drafts.push(Draft::new(name));
            state.current = state.drafts.len() - 1;
        }
        Msg::Open(index) => {
            let Some(name) = state.names().get(index - 1).cloned() else {
                return Command::none();
            };
            log.push(PAGE, "Select#start", format!("open {name}"));
            match state.drafts.iter().position(|draft| draft.name == name) {
                Some(found) => state.current = found,
                None => {
                    let Some(builtin) = state.builtin(&name) else {
                        return Command::none();
                    };
                    state.drafts.push(Draft::from_animation(&name, builtin));
                    state.current = state.drafts.len() - 1;
                }
            }
        }
        Msg::Name(text) => {
            log.push(PAGE, "TextInput#name", format!("name = {text:?}"));
            state.draft_mut().name = text;
        }
        Msg::TimeKind(kind) => {
            let time = state.frame_time_kind(kind);
            log.push(
                PAGE,
                "Segmented#time",
                format!("{name} frame = {}", Draft { time, ..state.draft().clone() }.frame_time()),
            );
            state.draft_mut().time = time;
        }
        Msg::MotionKey(index) => {
            let key = draft::MOTION_KEYS.get(index).copied().unwrap_or("spinner");
            log.push(PAGE, "Select#motion-key", format!("{name} frame = {key}"));
            state.draft_mut().time = TimeDraft::Motion(index.min(draft::MOTION_KEYS.len() - 1));
        }
        Msg::Millis(millis) => {
            // The field's range keeps the value between 10 and 5000.
            let millis = millis.round().clamp(1.0, 60_000.0) as u32;
            log.push(PAGE, "NumberInput#millis", format!("{name} frame = {millis}ms"));
            state.draft_mut().time = TimeDraft::Millis(millis);
        }
        Msg::Playback(index) => {
            let playback = Playback::ALL.get(index).copied().unwrap_or_default();
            log.push(PAGE, "Segmented#playback", format!("{name} playback = {}", playback.name()));
            state.draft_mut().playback = playback;
        }
        Msg::Colors(index) => {
            let colors = ColorMode::ALL.get(index).copied().unwrap_or_default();
            log.push(PAGE, "Segmented#colors", format!("{name} colors = {}", colors.name()));
            state.draft_mut().colors = colors;
        }
        Msg::Rest(index) => {
            let rest = index.checked_sub(1);
            let shown = rest.map_or_else(|| "auto".to_owned(), |rest| (rest + 1).to_string());
            log.push(PAGE, "Select#rest", format!("{name} rest = {shown}"));
            state.draft_mut().rest = rest;
        }
        Msg::Glyph(frame, column, text) => {
            let Some(target) = state.draft_mut().frames.get_mut(frame) else {
                return Command::none();
            };
            let mode = ["nerd", "unicode", "ascii"].get(column).copied().unwrap_or("ascii");
            log.push(
                PAGE,
                format!("TextInput#frame-{}-{mode}", frame + 1),
                format!("{name} frame {} {mode} = {text:?}", frame + 1),
            );
            if let Some(glyph) = target.glyphs.get_mut(column) {
                *glyph = text;
            }
        }
        Msg::Color(frame, text) => {
            let Some(target) = state.draft_mut().frames.get_mut(frame) else {
                return Command::none();
            };
            log.push(
                PAGE,
                format!("TextInput#frame-{}-color", frame + 1),
                format!("{name} frame {} color = {text:?}", frame + 1),
            );
            target.color = text;
        }
        Msg::Duration(frame, text) => {
            let Some(target) = state.draft_mut().frames.get_mut(frame) else {
                return Command::none();
            };
            log.push(
                PAGE,
                format!("TextInput#frame-{}-duration", frame + 1),
                format!("{name} frame {} duration = {text:?}", frame + 1),
            );
            target.duration = text;
        }
        Msg::Add => {
            let frames = &mut state.draft_mut().frames;
            let copy = frames.last().cloned().unwrap_or_else(|| FrameDraft {
                glyphs: [String::new(), String::new(), "*".to_owned()],
                ..FrameDraft::default()
            });
            frames.push(copy);
            log.push(PAGE, "Button#add-frame", format!("{name} frame {} added", frames.len()));
        }
        Msg::Remove(frame) => {
            let draft = state.draft_mut();
            if frame >= draft.frames.len() || draft.frames.len() == 1 {
                return Command::none();
            }
            draft.frames.remove(frame);
            draft.rest = draft.rest.filter(|rest| *rest < draft.frames.len());
            log.push(PAGE, source, format!("frame {} removed", frame + 1));
        }
        Msg::Up(frame) | Msg::Down(frame) => {
            let down = matches!(message, Msg::Down(_));
            let draft = state.draft_mut();
            let other = if down { frame + 1 } else { frame.wrapping_sub(1) };
            if frame >= draft.frames.len() || other >= draft.frames.len() {
                return Command::none();
            }
            draft.frames.swap(frame, other);
            log.push(PAGE, source, format!("frame {} moved to {}", frame + 1, other + 1));
        }
        Msg::Copy => {
            let text = state.built.to_toml(&name);
            log.push(PAGE, "Button#copy-toml", format!("copied [animations.{name}], {} lines", text.lines().count()));
            return Command::copy(text);
        }
        Msg::Replay => {
            state.replay = state.replay.wrapping_add(1);
            log.push(PAGE, "Button#replay", format!("{name} plays again"));
            return Command::none();
        }
        Msg::Reset => {
            let Some(builtin) = state.builtin(&name).cloned() else {
                return Command::none();
            };
            log.push(PAGE, "Button#reset", format!("{name} reset to the built-in"));
            *state.draft_mut() = Draft::from_animation(&name, &builtin);
        }
    }
    state.refresh();
    save(state, settings)
}

impl State {
    /// The frame time after choosing between a theme motion key (0) and fixed milliseconds (1),
    /// keeping the current choice when it already is of that kind.
    fn frame_time_kind(&self, kind: usize) -> TimeDraft {
        let time = self.draft().time;
        match (kind, time) {
            (0, TimeDraft::Millis(_)) => TimeDraft::Motion(0),
            (1, TimeDraft::Motion(index)) => {
                let key = draft::MOTION_KEYS.get(index).copied().unwrap_or("spinner");
                // Start from what the key lasts in the default theme, so the pace does not jump.
                let motion = qframe::env::Env::builtin().theme().motion();
                let millis = motion.duration(key).map_or(80, |duration| duration.as_millis());
                TimeDraft::Millis(u32::try_from(millis).unwrap_or(80))
            }
            _ => time,
        }
    }
}

// region: preview-widget
/// One animation in one glyph mode, whatever mode the terminal uses: a custom widget sampling a
/// `CellAnimation` itself. Widgets drawing a named animation in the terminal's own mode call
/// `cx.animation(name, style, since)` instead.
struct AnimationCell {
    animation: Arc<CellAnimation>,
    mode: GlyphMode,
    /// Drawn on a raised tile, three rows tall, instead of in a line.
    large: bool,
    /// Text after the cell, like a spinner's label.
    label: Option<String>,
    /// Bumped to play a single-play animation again.
    replay: u32,
    /// Plays a single-play animation again after a pause, on its own.
    repeat: bool,
}

/// When a single-play preview started, and for which replay.
#[derive(Default)]
struct Started(Option<(u32, Duration)>);

/// The size of a large preview tile.
const TILE: Size = Size::new(7, 3);

impl<Msg: 'static> Widget<Msg> for AnimationCell {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.large {
            return TILE.min(available);
        }
        let label = self.label.as_deref().map_or(0, |label| text::width(label).saturating_add(2));
        Size::new(label.saturating_add(1), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let style = cx.style("spinner", None, &[]).text();
        let fg = style.fg.unwrap_or_else(|| cx.color("accent"));
        let now = cx.now();
        let since = self.since(cx, now).filter(|_| !cx.reduced_motion());
        let frame = self.animation.sample(cx.env().theme(), Some(fg), now, since);
        if since.is_some() {
            match frame.next {
                _ if frame.smooth => cx.request_frame_in(Duration::from_millis(16)),
                Some(next) => cx.request_frame_in(next),
                None if self.repeat => cx.request_frame_in(REPLAY_PAUSE),
                None => {}
            }
        }
        let glyph = self.animation.glyph(frame.index, self.mode).to_owned();
        let cell = CellStyle { fg: frame.color.or(Some(fg)), ..style };
        if self.large {
            let tile = cx.color("raised");
            cx.clear(Rect::new(area.x, area.y, TILE.width.min(area.width), TILE.height.min(area.height)), tile);
            cx.text(area.x + 3, area.y + 1, &glyph, CellStyle { bg: Some(tile), ..cell }, 1);
            return;
        }
        cx.text(area.x, area.y, &glyph, cell, 1);
        if let Some(label) = &self.label {
            let label_style = cx.style("spinner-label", None, &[]).text();
            cx.text(area.x + 2, area.y, label, label_style, area.width.saturating_sub(2));
        }
    }
}
// endregion

impl AnimationCell {
    /// When the animation started: loops share the clock so they turn in step; a single play
    /// starts when first drawn or replayed, or again after each pause when it repeats.
    fn since(&self, cx: &mut PaintCx<'_>, now: Duration) -> Option<Duration> {
        if self.animation.play_mode() != Playback::Once {
            return Some(Duration::ZERO);
        }
        if self.repeat {
            let motion = cx.env().theme().motion();
            let total: Duration = self
                .animation
                .frames()
                .iter()
                .map(|frame| frame.frame_duration().unwrap_or(self.animation.time()).resolve(&motion))
                .sum();
            let period = (total + REPLAY_PAUSE).as_millis().max(1);
            let into = u64::try_from(now.as_millis() % period).unwrap_or(0);
            return Some(now.saturating_sub(Duration::from_millis(into)));
        }
        let started = cx.memory::<Started>();
        match started.0 {
            Some((replay, at)) if replay == self.replay => Some(at),
            _ => {
                started.0 = Some((self.replay, now));
                Some(now)
            }
        }
    }
}

/// The names of the glyph modes, as column titles.
fn mode_titles() -> [String; 3] {
    [t!("cell-animation.nerd"), t!("cell-animation.unicode"), t!("cell-animation.ascii")]
}

/// The live demo and the studio.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("cell-animation.builtins")).gap(0), |ui| {
        ui.add(Text::new(t!("cell-animation.builtins-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.row(|ui| {
            ui.spacer().width(Length::Cells(22));
            for title in mode_titles() {
                ui.add(Text::new(title).role("faint").no_wrap()).width(Length::Cells(12));
            }
        });
        for (name, animation) in &state.builtin {
            ui.row(|ui| {
                ui.add(
                    Button::new(name.clone())
                        .on_press(send(Msg::Open(1 + state.names().iter().position(|n| n == name).unwrap_or(0)))),
                )
                .width(Length::Cells(22))
                .id(format!("open-{name}"));
                for mode in MODES {
                    // region: builtins
                    ui.add(AnimationCell {
                        animation: Arc::clone(animation),
                        mode,
                        large: false,
                        label: None,
                        replay: 0,
                        repeat: true,
                    })
                    .width(Length::Cells(12));
                    // endregion
                }
                let detail = t!(
                    "cell-animation.detail",
                    playback = animation.play_mode().name(),
                    time = animation.time().to_string(),
                    n = animation.frames().len()
                );
                ui.add(Text::new(detail).role("faint").no_wrap());
            })
            .align(Align::Center);
        }
    })
    .fill_width();

    editor::studio(state, ui);
}
