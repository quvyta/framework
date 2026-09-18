//! Actions confirmed by holding a key or a mouse button.

use std::time::Duration;

use crate::color::Rgb;
use crate::event::{Event, KeyKind, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::{Key, KeyChord};
use crate::motion::Easing;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::cells;

/// How long the key or button must be held when no duration is set: three bars of 400 ms.
const DEFAULT_DURATION: Duration = Duration::from_millis(1200);

/// Keyboards wait this long before repeating a held key. Without a repeat or a release in
/// this time, the key counts as let go.
const INITIAL_TIMEOUT: Duration = Duration::from_millis(650);

/// Once a key repeats, a longer gap than this between repeats means it was let go.
const REPEAT_TIMEOUT: Duration = Duration::from_millis(350);

/// How often a held mouse button is looked at.
const POINTER_REPEAT: Duration = Duration::from_millis(40);

/// Time between frames while the bars fill.
const FILL_FRAME: Duration = Duration::from_millis(16);

/// Number of bars.
const BARS: u16 = 3;

/// Cells of one bar.
const BAR: u16 = 3;

/// Width of the bars with the cell between them.
const BARS_WIDTH: u16 = BARS * BAR + BARS - 1;

/// A control that sends its message only after a key or the mouse button is held on it.
///
/// While held, three bars fill one after another. Each bar is one block of colour: over its third
/// of the duration the whole bar blends from the track colour to the theme's target colour
/// (`to`, the warning tone in the built-in themes). When the third bar reaches the full colour the
/// message is sent, once; nothing more happens until the key or button is let go and pressed
/// again. Letting go empties the bars quickly, over `motion.enter`. With reduced motion each bar
/// switches to the full colour at the end of its third and letting go empties them at once.
///
/// With no options it is a focusable chip: hold Enter or Space while it has focus, or press
/// the mouse button on it. `key` makes a chord work from anywhere on the screen, e.g. holding
/// `ctrl+q` to quit; `floating` draws nothing until the hold starts and then shows the label
/// and bars on a small card in the top left corner of the screen.
///
/// Held keys arrive as repeats: with the kitty keyboard protocol as repeat and release events,
/// elsewhere as the same key pressed again every few dozen milliseconds after the keyboard's
/// repeat delay. The hold therefore counts as released on a release event, or when no repeat
/// arrives within 650 ms of the press or 350 ms of the last repeat. A held mouse button is
/// followed until it is released or leaves the control.
///
/// Hovered and focused controls show the pillar in their first cell, like buttons.
///
/// The bars fill towards the theme's `to` colour unless [`HoldToConfirm::color`] names another
/// one, preferably a theme token such as `"$danger"` so the control follows the theme.
///
/// Style keys: `hold` (`bg`, `fg`, `bold`, `padding`, `track`, `to`, `pillar`) with states
/// `hover`, `focus`, `active` (held), `disabled`; `hold-card` (`bg`, `padding`, `pillar`). Without
/// a card `pillar`, the card's pillar blends from `muted` to `to` as the hold goes on.
pub struct HoldToConfirm<Msg> {
    label: String,
    duration: Duration,
    key: Option<KeyChord>,
    floating: bool,
    disabled: bool,
    color: Option<String>,
    on_confirm: Option<Msg>,
}

/// What is being held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Key(Key),
    Pointer,
}

#[derive(Debug, Default)]
struct HoldMemory {
    /// When the current hold started.
    start: Option<Duration>,
    /// The last sign that the hold goes on.
    last: Duration,
    repeated: bool,
    source: Option<Source>,
    /// Completed; waits for the key or button to be let go.
    done: bool,
    /// When the last hold was let go and how far it had come, so the bars can empty.
    released: Option<(Duration, f32)>,
}

impl HoldMemory {
    /// Lets the hold go at `now`; the bars empty from where they were.
    fn release(&mut self, now: Duration, duration: Duration) {
        let reached = self.progress(now, duration);
        *self = Self::default();
        if reached > 0.0 {
            self.released = Some((now, reached));
        }
    }

    fn timeout(&self) -> Option<Duration> {
        match self.source {
            Some(Source::Key(_)) => Some(self.last + if self.repeated { REPEAT_TIMEOUT } else { INITIAL_TIMEOUT }),
            _ => None,
        }
    }

    fn progress(&self, now: Duration, duration: Duration) -> f32 {
        if self.done {
            return 1.0;
        }
        let Some(start) = self.start else {
            return 0.0;
        };
        if duration.is_zero() {
            return 1.0;
        }
        (now.saturating_sub(start).as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
    }
}

/// How filled bar `bar` is at `progress` of the whole hold: it blends during its own third. With
/// reduced motion it jumps to full at the end of its third.
fn bar_fill(progress: f32, bar: u16, reduced_motion: bool) -> f32 {
    let within = (progress * f32::from(BARS) - f32::from(bar)).clamp(0.0, 1.0);
    // The small nudge keeps a bar from missing its end by a rounding error in `progress`.
    if reduced_motion { (within + 0.001).floor().min(1.0) } else { within }
}

impl<Msg: Clone + 'static> HoldToConfirm<Msg> {
    /// A control labelled `label`, e.g. "Hold to delete".
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            duration: DEFAULT_DURATION,
            key: None,
            floating: false,
            disabled: false,
            color: None,
            on_confirm: None,
        }
    }

    /// The message sent once the hold completes.
    #[must_use]
    pub fn on_confirm(mut self, message: Msg) -> Self {
        self.on_confirm = Some(message);
        self
    }

    /// How long to hold; 1.2 s by default. It does not shorten with reduced motion: the wait
    /// protects the action, it is not decoration.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Holding `chord` anywhere on the screen confirms too, while the control is in the view.
    ///
    /// # Panics
    ///
    /// Panics when `chord` is not a valid chord such as `"ctrl+q"`; chords are fixed in code.
    #[must_use]
    pub fn key(mut self, chord: &str) -> Self {
        self.key = Some(chord.parse().unwrap_or_else(|message| panic!("invalid chord `{chord}`: {message}")));
        self
    }

    /// Takes no room and draws nothing until a hold starts, then shows a card in the top left
    /// corner of the screen. Meant together with [`HoldToConfirm::key`].
    #[must_use]
    pub fn floating(mut self, floating: bool) -> Self {
        self.floating = floating;
        self
    }

    /// Greys the control out; holding does nothing.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The colour the bars fill towards, written like a theme colour: a token such as
    /// `"$danger"`, `"$success"` or `"$accent"`, a blend such as `"mix($accent, $danger, 50%)"`, or a
    /// fixed `"#RRGGBB"`. Tokens are the intended use: they follow the theme, a fixed colour does
    /// not. An expression that is not a single colour of the current theme (a typo, an unknown
    /// token, `pulse()`) falls back to the theme's `to`, as if no colour were set; check one with
    /// [`Theme::solid`](crate::theme::Theme::solid). Default: the theme's `to`, the warning tone.
    #[must_use]
    pub fn color(mut self, paint: impl Into<String>) -> Self {
        self.color = Some(paint.into());
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_confirm.is_some()
    }

    /// The colour of a full bar: the chosen colour when it resolves, else the theme's `to`.
    fn target(&self, cx: &PaintCx<'_>, hold: Option<Rgb>) -> Rgb {
        self.color
            .as_deref()
            .and_then(|paint| cx.env().theme().solid(paint).ok())
            .or(hold)
            .unwrap_or_else(|| cx.color("warning"))
    }

    fn content_width(&self) -> u16 {
        cells::sum([text::width(&self.label), 2, BARS_WIDTH])
    }

    /// Paints the label and the bars from `(x, y)`.
    fn paint_content(&self, cx: &mut PaintCx<'_>, x: i32, y: i32, width: u16, states: &[State], progress: f32) {
        let style = cx.style("hold", None, states);
        let mut label_style = style.text();
        label_style.bg = None;
        let track = style.color("track").unwrap_or_else(|| cx.color("active"));
        let to = self.target(cx, style.color("to"));
        let label_budget = width.saturating_sub(BARS_WIDTH + 2);
        let label = text::truncate(&self.label, label_budget).into_owned();
        cx.text(x, y, &label, label_style, label_budget);
        let bars_x = x + i32::from(width.saturating_sub(BARS_WIDTH));
        for bar in 0..BARS {
            // Each bar owns one third of the hold and blends as a whole within it.
            let fill = bar_fill(progress, bar, cx.reduced_motion());
            let column = bars_x + i32::from(bar * (BAR + 1));
            cx.clear(Rect::new(column, y, BAR, 1), track.mix(to, fill));
        }
    }

    /// How far the bars are shown at `now`: the hold's progress while held, and afterwards the
    /// progress at release emptying over `motion.enter`.
    fn shown_progress(&self, cx: &mut PaintCx<'_>) -> f32 {
        let now = cx.now();
        let (progress, released) = {
            let memory = cx.memory::<HoldMemory>();
            (memory.progress(now, self.duration), memory.released)
        };
        match released {
            Some((at, reached)) if progress == 0.0 => {
                let emptied = cx.progress_since(at, cx.env().theme().motion().enter, Easing::Linear);
                if emptied >= 1.0 {
                    cx.memory::<HoldMemory>().released = None;
                }
                reached * (1.0 - emptied)
            }
            _ => progress,
        }
    }

    fn start(&self, cx: &mut EventCx<'_, Msg>, source: Source) {
        let now = cx.now();
        let memory = cx.memory::<HoldMemory>();
        *memory = HoldMemory { start: Some(now), last: now, source: Some(source), ..HoldMemory::default() };
    }

    fn release(&self, cx: &mut EventCx<'_, Msg>) {
        let now = cx.now();
        cx.memory::<HoldMemory>().release(now, self.duration);
    }

    /// Notes that the hold goes on and sends the message when it is complete.
    fn keep(&self, cx: &mut EventCx<'_, Msg>) {
        let now = cx.now();
        let duration = self.duration;
        let complete = {
            let memory = cx.memory::<HoldMemory>();
            memory.last = now;
            let complete = !memory.done && memory.progress(now, duration) >= 1.0;
            if complete {
                memory.done = true;
            }
            complete
        };
        if complete && let Some(message) = &self.on_confirm {
            cx.emit(message.clone());
        }
    }

    fn is_trigger(&self, cx: &EventCx<'_, Msg>, chord: KeyChord, release: bool) -> bool {
        let focused_key = cx.is_focused() && !self.floating && matches!(chord.key, Key::Enter | Key::Space);
        let focused_key = focused_key && (release || chord.mods == crate::keymap::Modifiers::default());
        let bound = self.key.is_some_and(|key| if release { key.key == chord.key } else { key == chord });
        focused_key || bound
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for HoldToConfirm<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.floating {
            return Size::default();
        }
        let style = cx.env().theme().style("hold", None, &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((0, 2));
        Size::new(
            self.content_width().saturating_add(horizontal.saturating_mul(2)),
            vertical.saturating_mul(2).saturating_add(1),
        )
        .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let now = cx.now();
        let (holding, progress) = {
            let memory = cx.memory::<HoldMemory>();
            if let Some(timeout) = memory.timeout().filter(|timeout| now > *timeout) {
                // A silent key was let go when its repeats stopped, not when this frame noticed.
                memory.release(timeout, self.duration);
            }
            (memory.start.is_some() || memory.done, memory.progress(now, self.duration))
        };
        if self.active() {
            if let Some(key) = self.key {
                cx.listen_key(key);
            }
            if holding || (cx.is_focused() && !self.floating) {
                // Repeats of Enter and Space never reach focused widgets; listening hears them.
                cx.listen_key(KeyChord::plain(Key::Enter));
                cx.listen_key(KeyChord::plain(Key::Space));
            }
        }
        if holding {
            if progress < 1.0 {
                cx.request_frame_in(FILL_FRAME);
            }
            if let Some(timeout) = cx.memory::<HoldMemory>().timeout() {
                cx.request_frame_in(timeout.saturating_sub(now) + Duration::from_millis(1));
            }
        }
        let shown = self.shown_progress(cx);
        if self.floating {
            if holding || shown > 0.0 {
                cx.request_overlay(area);
            }
            return;
        }
        let mut states = if self.active() { cx.states() } else { Vec::new() };
        if self.disabled {
            states.push(State::Disabled);
        }
        if holding {
            states.push(State::Active);
        }
        let style = cx.style("hold", None, &states);
        let background = style.text().bg.unwrap_or_else(|| cx.color("raised"));
        cx.clear(area, background);
        if self.active() {
            cx.register_hit(area);
        }
        let padding = style.padding();
        let inner = area.inset(padding);
        // Like a button, hover and focus raise the pillar in the first cell of the padding.
        if let Some(color) = style.color("pillar").filter(|_| padding.left >= 1) {
            cx.pillar(area.x, inner.y, color);
        }
        self.paint_content(cx, inner.x, inner.y, inner.width, &states, shown);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, _anchor: Rect) {
        let screen = cx.clip();
        let progress = self.shown_progress(cx);
        let card = cx.style("hold-card", None, &[]);
        let background = card.text().bg.unwrap_or_else(|| cx.color("overlay"));
        let padding = card.padding();
        let width = self.content_width().saturating_add(padding.horizontal()).min(screen.width.saturating_sub(4));
        let rect = Rect::new(screen.x + 2, screen.y + 1, width, padding.vertical().saturating_add(1));
        cx.floating(rect, |cx| {
            cx.clear(rect, background);
            let hold = cx.style("hold", None, &[State::Active]);
            let to = self.target(cx, hold.color("to"));
            let pillar: Rgb = card.color("pillar").unwrap_or_else(|| cx.color("muted").mix(to, progress));
            for row in 0..rect.height {
                cx.pillar(rect.x, rect.y + i32::from(row), pillar);
            }
            let inner = rect.inset(padding);
            self.paint_content(cx, inner.x, inner.y, inner.width, &[State::Active], progress);
        });
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        let now = cx.now();
        match event {
            Event::Key(key) => {
                let release = key.kind == KeyKind::Release;
                if !self.is_trigger(cx, key.chord, release) {
                    return false;
                }
                let (source, timed_out) = {
                    let memory = cx.memory::<HoldMemory>();
                    (memory.source, memory.timeout().is_some_and(|timeout| now > timeout))
                };
                if release {
                    if source == Some(Source::Key(key.chord.key)) {
                        self.release(cx);
                    }
                    return true;
                }
                if timed_out || source != Some(Source::Key(key.chord.key)) {
                    self.start(cx, Source::Key(key.chord.key));
                    return true;
                }
                cx.memory::<HoldMemory>().repeated = true;
                self.keep(cx);
                true
            }
            Event::Mouse(mouse) if !self.floating => match mouse.kind {
                MouseKind::Down(MouseButton::Left) => {
                    cx.capture_pointer();
                    cx.repeat_pointer(POINTER_REPEAT);
                    self.start(cx, Source::Pointer);
                    true
                }
                MouseKind::Drag(MouseButton::Left) if cx.memory::<HoldMemory>().source == Some(Source::Pointer) => {
                    if cx.area().contains(mouse.x, mouse.y) {
                        self.keep(cx);
                    } else {
                        self.release(cx);
                    }
                    true
                }
                MouseKind::Up(MouseButton::Left) => {
                    self.release(cx);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.active() && !self.floating
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::KeyEvent;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::TextInput;

    #[derive(Default)]
    struct Demo {
        confirmed: u32,
        floating: bool,
        typed: String,
        color: Option<&'static str>,
    }

    #[derive(Clone)]
    enum Msg {
        Confirm,
        Typed(String),
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Confirm => self.confirmed += 1,
                Msg::Typed(text) => self.typed = text,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                let mut hold = HoldToConfirm::new("Hold to delete").key("ctrl+d").floating(self.floating);
                if let Some(color) = self.color {
                    hold = hold.color(color);
                }
                ui.add(hold.on_confirm(Msg::Confirm)).id("hold");
                ui.add(TextInput::new(&self.typed).on_change(Msg::Typed)).id("field");
            });
        }
    }

    fn repeat(chord: &str) -> KeyEvent {
        KeyEvent { kind: KeyKind::Repeat, ..KeyEvent::press(chord) }
    }

    fn release(chord: &str) -> KeyEvent {
        KeyEvent { kind: KeyKind::Release, ..KeyEvent::press(chord) }
    }

    /// Holds `chord` for `total` with keep-alive presses every 30 ms, like a keyboard without
    /// the kitty protocol.
    fn hold_with_presses(h: &mut Harness<Demo>, chord: &str, total: Duration) {
        h.key(KeyEvent::press(chord));
        let mut held = Duration::ZERO;
        while held < total {
            h.advance(Duration::from_millis(30));
            held += Duration::from_millis(30);
            h.key(KeyEvent::press(chord));
        }
    }

    #[test]
    fn draws_label_and_empty_bars_without_brackets() {
        let h = Harness::new(Demo::default(), 40, 2);
        assert_eq!(h.screen(), "  Hold to delete\n  ❯\n");
        let track = h.env().theme().color("active");
        assert_eq!(h.bg(18, 0), track);
        assert_eq!(h.bg(21, 0), h.env().theme().color("raised"), "one cell between bars");
    }

    /// The colour of every cell of the three bars, bar by bar, checking that a bar is one colour.
    fn bars(h: &Harness<Demo>) -> [Option<Rgb>; 3] {
        [18, 22, 26].map(|x| {
            let cells = [h.bg(x, 0), h.bg(x + 1, 0), h.bg(x + 2, 0)];
            assert!(cells.iter().all(|cell| *cell == cells[0]), "a bar blends as a whole: {cells:?}");
            cells[0]
        })
    }

    /// Whether two colours match within one step of rounding per channel.
    fn near(a: Option<Rgb>, b: Rgb) -> bool {
        a.is_some_and(|a| a.r.abs_diff(b.r) <= 1 && a.g.abs_diff(b.g) <= 1 && a.b.abs_diff(b.b) <= 1)
    }

    #[test]
    fn bars_blend_whole_one_after_another_to_the_theme_colour() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        let theme = h.env().theme().clone();
        let track = theme.color("active").expect("track");
        let to = theme.color("warning").expect("the target is the warning tone");
        let half = track.mix(to, 0.5);
        h.mouse(MouseKind::Down(MouseButton::Left), 4, 0);
        assert_eq!(bars(&h), [Some(track); 3]);
        h.advance(Duration::from_millis(200));
        let [first, second, third] = bars(&h);
        assert!(near(first, half), "1/6: the first bar is halfway: {first:?}");
        assert_eq!((second, third), (Some(track), Some(track)), "1/6: the others wait");
        h.advance(Duration::from_millis(400));
        let [first, second, third] = bars(&h);
        assert_eq!(first, Some(to), "1/2: the first bar is full");
        assert!(near(second, half), "1/2: the second bar is halfway: {second:?}");
        assert_eq!(third, Some(track));
        h.advance(Duration::from_millis(400));
        let [first, second, third] = bars(&h);
        assert_eq!((first, second), (Some(to), Some(to)), "5/6: two bars are full");
        assert!(near(third, half), "5/6: the third bar is halfway: {third:?}");
        assert_eq!(h.app().confirmed, 0, "nothing fires before the last bar is full");
        h.advance(Duration::from_millis(200));
        assert_eq!(bars(&h), [Some(to); 3]);
        assert_eq!(h.app().confirmed, 1, "the full third bar fires");
        h.advance(Duration::from_millis(400));
        assert_eq!(h.app().confirmed, 1, "a completed hold sends once");
    }

    /// Holds the mouse button and checks the bars blend from the track to `to` at 1/6, 1/2 and
    /// 5/6 of the hold and are all `to` at the end.
    fn fills_towards(h: &mut Harness<Demo>, to: Rgb, label: &str) {
        let track = h.env().theme().color("active").expect("track");
        let half = track.mix(to, 0.5);
        h.mouse(MouseKind::Down(MouseButton::Left), 4, 0);
        h.advance(Duration::from_millis(200));
        let [first, second, third] = bars(h);
        assert!(near(first, half) && second == Some(track) && third == Some(track), "{label} 1/6: {first:?}");
        h.advance(Duration::from_millis(400));
        let [first, second, third] = bars(h);
        assert!(first == Some(to) && near(second, half) && third == Some(track), "{label} 1/2: {second:?}");
        h.advance(Duration::from_millis(400));
        let [first, second, third] = bars(h);
        assert!(first == Some(to) && second == Some(to) && near(third, half), "{label} 5/6: {third:?}");
        h.advance(Duration::from_millis(200));
        assert_eq!(bars(h), [Some(to); 3], "{label}: full");
        h.mouse(MouseKind::Up(MouseButton::Left), 4, 0);
        h.advance(Duration::from_millis(500));
    }

    #[test]
    fn a_theme_colour_can_be_chosen_in_every_theme() {
        for (token, color) in
            [("warning", "$warning"), ("danger", "$danger"), ("success", "$success"), ("accent", "$accent")]
        {
            let mut h = Harness::new(Demo { color: Some(color), ..Demo::default() }, 40, 2);
            for id in ["monochrome", "iris", "nordic", "amber"] {
                h.set_theme(id);
                let to = h.env().theme().color(token).expect("token");
                fills_towards(&mut h, to, &format!("{id} {color}"));
            }
            assert_eq!(h.app().confirmed, 4);
        }
    }

    #[test]
    fn a_blend_or_a_fixed_hex_colour_works_too() {
        let mut h = Harness::new(Demo { color: Some("#38BDF8"), ..Demo::default() }, 40, 2);
        fills_towards(&mut h, Rgb::new(0x38, 0xBD, 0xF8), "hex");
        let mut h = Harness::new(Demo { color: Some("mix($accent, $danger, 50%)"), ..Demo::default() }, 40, 2);
        let theme = h.env().theme().clone();
        let blend = theme.color("danger").expect("danger").mix(theme.color("accent").expect("accent"), 0.5);
        fills_towards(&mut h, blend, "mix");
    }

    #[test]
    fn an_invalid_colour_falls_back_to_the_theme_target() {
        for invalid in ["$dangr", "red", "#12", "pulse($accent, $danger)", ""] {
            let mut h = Harness::new(Demo { color: Some(invalid), ..Demo::default() }, 40, 2);
            let theme = h.env().theme().clone();
            let error = theme.solid(invalid).expect_err("not a single colour");
            assert!(!error.is_empty(), "{invalid}: the reason is reported");
            fills_towards(&mut h, theme.color("warning").expect("warning"), invalid);
        }
    }

    #[test]
    fn the_target_colour_comes_from_the_theme() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        for id in ["monochrome", "iris", "nordic", "amber"] {
            h.set_theme(id);
            let theme = h.env().theme().clone();
            h.mouse(MouseKind::Down(MouseButton::Left), 4, 0);
            h.advance(Duration::from_millis(1250));
            assert_eq!(bars(&h), [theme.color("warning"); 3], "{id}");
            h.mouse(MouseKind::Up(MouseButton::Left), 4, 0);
            h.advance(Duration::from_millis(500));
            assert_eq!(bars(&h), [theme.color("active"); 3], "{id}: empty again");
        }
    }

    #[test]
    fn the_key_holds_while_it_repeats_and_letting_go_empties_the_bars_quickly() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        h.press("tab");
        assert!(h.is_focused("hold"));
        h.key(KeyEvent::press("enter"));
        for _ in 0..25 {
            h.advance(Duration::from_millis(30));
            h.key(repeat("enter"));
        }
        let theme = h.env().theme().clone();
        let (track, to) = (theme.color("active").expect("track"), theme.color("warning").expect("warning"));
        assert_eq!(bars(&h)[0], Some(to), "750 ms fill the first bar");
        h.key(release("enter"));
        let enter = theme.motion().enter;
        h.advance(enter / 2);
        let [first, second, _] = bars(&h);
        assert!(first != Some(track) || second != Some(track), "the bars empty over a moment, not at once");
        h.advance(enter);
        assert_eq!(bars(&h), [Some(track); 3], "empty after motion.enter");
        assert_eq!(h.app().confirmed, 0);
    }

    #[test]
    fn releasing_early_resets() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        h.press("tab");
        hold_with_presses(&mut h, "space", Duration::from_millis(600));
        h.key(release("space"));
        h.advance(h.env().theme().motion().enter);
        assert_eq!(h.bg(18, 0), h.env().theme().color("active"));
        hold_with_presses(&mut h, "space", Duration::from_millis(600));
        assert_eq!(h.app().confirmed, 0, "the hold started over after the release");
        hold_with_presses(&mut h, "space", Duration::from_millis(700));
        assert_eq!(h.app().confirmed, 1);
    }

    #[test]
    fn a_silent_key_counts_as_released() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        h.press("tab").key(KeyEvent::press("enter"));
        h.advance(Duration::from_millis(900));
        assert_eq!(h.bg(18, 0), h.env().theme().color("active"), "no repeat within the delay: released");
        h.key(KeyEvent::press("enter"));
        h.advance(Duration::from_millis(1250)).key(KeyEvent::press("enter"));
        assert_eq!(h.app().confirmed, 0, "a new press starts a new hold");
    }

    #[test]
    fn a_chord_works_from_anywhere_and_floats_a_card() {
        let mut h = Harness::new(Demo { floating: true, ..Demo::default() }, 40, 4);
        h.click(3, 0).type_text("x");
        assert_eq!(h.app().typed, "x");
        assert!(!h.screen().contains("Hold to delete"));
        hold_with_presses(&mut h, "ctrl+d", Duration::from_millis(300));
        let screen = h.screen();
        assert!(screen.contains("Hold to delete"), "{screen}");
        assert!(screen.lines().nth(1).is_some_and(|line| line.starts_with("  ▌")), "the card has a pillar: {screen}");
        hold_with_presses(&mut h, "ctrl+d", Duration::from_millis(1300));
        assert_eq!(h.app().confirmed, 1);
        h.key(release("d")).advance(Duration::from_millis(10));
        assert!(h.screen().contains("Hold to delete"), "the card stays while its bars empty");
        h.advance(h.env().theme().motion().enter);
        assert!(!h.screen().contains("Hold to delete"));
    }

    #[test]
    fn holding_the_mouse_button_confirms_and_leaving_cancels() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        h.mouse(MouseKind::Down(MouseButton::Left), 4, 0);
        h.advance(Duration::from_millis(600));
        assert_eq!(h.app().confirmed, 0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 4, 1);
        h.advance(Duration::from_millis(900));
        assert_eq!(h.app().confirmed, 0, "leaving the control cancels");
        h.mouse(MouseKind::Up(MouseButton::Left), 4, 1);
        h.mouse(MouseKind::Down(MouseButton::Left), 4, 0);
        for _ in 0..40 {
            h.advance(Duration::from_millis(40));
        }
        assert_eq!(h.app().confirmed, 1, "the held button is followed without events");
    }

    #[test]
    fn reduced_motion_switches_each_bar_at_the_end_of_its_third_and_empties_at_once() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        h.set_reduced_motion(true);
        let theme = h.env().theme().clone();
        let (track, to) = (theme.color("active"), theme.color("warning"));
        h.mouse(MouseKind::Down(MouseButton::Left), 4, 0);
        h.advance(Duration::from_millis(390));
        assert_eq!(bars(&h), [track; 3], "just before a third nothing shows");
        h.advance(Duration::from_millis(10));
        assert_eq!(bars(&h), [to, track, track], "a third switches the first bar at once");
        h.advance(Duration::from_millis(600));
        assert_eq!(bars(&h), [to, to, track], "5/6: the third bar waits for its end");
        h.mouse(MouseKind::Up(MouseButton::Left), 4, 0);
        assert_eq!(bars(&h), [track; 3], "letting go empties at once");
    }

    #[test]
    fn hover_and_focus_raise_the_pillar_in_the_first_cell() {
        let mut h = Harness::new(Demo::default(), 40, 2);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        assert!(h.screen().starts_with("  Hold to delete"));
        h.hover(8, 0);
        assert!(h.screen().starts_with("▌ Hold to delete"), "{}", h.screen());
        h.hover(39, 1).press("tab");
        assert!(h.screen().starts_with("▌ Hold to delete"), "{}", h.screen());
    }

    #[test]
    fn a_narrow_control_cuts_the_label_and_keeps_the_bars() {
        let h = Harness::new(Demo::default(), 20, 2);
        let theme = h.env().theme();
        assert!(h.screen().starts_with("  Ho…"), "{}", h.screen());
        assert_eq!(h.bg(17, 0), theme.color("active"), "the last bar is still drawn");
    }
}
