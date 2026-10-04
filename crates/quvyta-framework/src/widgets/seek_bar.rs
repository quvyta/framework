//! Seek bars: a progress bar a person can click and drag to a position.

use std::time::Duration;

use super::placement::Placement;
use super::progress_bar;
use super::tooltip;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// How far one arrow key moves along the track, as a fraction of it.
const STEP: f32 = 0.05;

/// Builds a message from the fraction a person asked for.
type SeekMessage<Msg> = Box<dyn Fn(f32) -> Msg>;

/// Writes the label shown above the pointer.
type HoverLabel = Box<dyn Fn(f32) -> String>;

/// A bar a person can click and drag to a position, the way a music or video player offers.
///
/// It is drawn exactly as a [`ProgressBar`](super::ProgressBar) draws, so a player can put one
/// where the other stood without the picture changing. With [`on_seek`](Self::on_seek) the bar
/// answers the pointer: a press seeks to the cell it landed on, a drag seeks cell by cell for as
/// long as the button is held, even past either end of the track, and the release ends it. Without
/// it the bar is a picture, as a progress bar is: no focus, no hover, no pointer.
///
/// Left and Right move a twentieth of the track and Home and End go to its ends; the application
/// owns the fraction, so it decides what a seek means. Under the pointer the bar steps a tone
/// lighter and the cell the pointer is on takes the accent, and
/// [`hover_label`](Self::hover_label) writes a small label above the pointer, the time a player
/// would name the position with.
///
/// Style keys: `seek-bar` (`track`, `fill`) with the states `hover` and `focus` and the variants
/// `seek-bar.success`, `seek-bar.warning` and `seek-bar.danger`, as a progress bar has them; the
/// percentage written after the bar takes `progress-label`, the label a progress bar writes.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::widgets::SeekBar;
///
/// struct Player(f32);
///
/// impl App for Player {
///     type Msg = f32;
///     fn update(&mut self, seek: f32) -> Command<f32> {
///         self.0 = seek;
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, f32>) {
///         ui.add(SeekBar::new(self.0).on_seek(|fraction| fraction)).fill_width();
///     }
/// }
///
/// let mut app = Harness::new(Player(0.0), 25, 1);
/// app.click(4, 0);
/// assert_eq!(app.app().0, 0.225, "the fourth cell of twenty is where the press landed");
/// ```
pub struct SeekBar<Msg> {
    fraction: f32,
    percent: bool,
    variant: Option<String>,
    on_seek: Option<SeekMessage<Msg>>,
    hover_label: Option<HoverLabel>,
}

#[derive(Debug, Default)]
struct SeekBarMemory {
    /// Whether the pointer went down on the bar, which keeps a drag going past its ends.
    dragging: bool,
    /// The fraction the last seek sent, so a pointer resting on one cell does not send it again.
    sent: Option<f32>,
    /// When the pointer came onto the bar, which the label fades in from.
    hovered_since: Option<Duration>,
    /// The label the overlay paints, written while the bar was drawn.
    label: Option<String>,
}

impl<Msg> SeekBar<Msg> {
    /// A bar at `fraction` along its track, from 0 to 1. A value outside that range is pulled into
    /// it, and one that is not a number at all rests at the start of the track.
    #[must_use]
    pub fn new(fraction: f32) -> Self {
        let fraction = if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) };
        Self { fraction, percent: true, variant: None, on_seek: None, hover_label: None }
    }

    /// Theme variant, e.g. `"success"` once the work is done, as on
    /// [`ProgressBar`](super::ProgressBar).
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Shows or hides the percentage after the bar; shown by default, as on
    /// [`ProgressBar::percent`](super::ProgressBar::percent). A player that names the time beside
    /// the bar hides it, and the bar takes the cells it leaves.
    #[must_use]
    pub fn percent(mut self, show: bool) -> Self {
        self.percent = show;
        self
    }

    /// The message a press, a drag or an arrow key sends with the fraction it moved to. Without it
    /// the bar is drawn but answers nothing: it takes no focus, no hover and no pointer.
    #[must_use]
    pub fn on_seek(mut self, message: impl Fn(f32) -> Msg + 'static) -> Self {
        self.on_seek = Some(Box::new(message));
        self
    }

    /// Writes the label shown above the pointer while it rests on the bar, such as the time a
    /// player would name the position with. It is drawn as a tooltip of the library, anchored on
    /// the pointer's own cell, and comes at once rather than after the hover delay: it names where
    /// the pointer is, rather than explaining the widget.
    #[must_use]
    pub fn hover_label(mut self, label: impl Fn(f32) -> String + 'static) -> Self {
        self.hover_label = Some(Box::new(label));
        self
    }

    /// Whether the bar answers anything: without a message it is a picture, as a progress bar is.
    fn active(&self) -> bool {
        self.on_seek.is_some()
    }

    /// The cells a press and a drag land in: the fill, not the percentage written after it.
    fn bar(&self, area: Rect) -> Rect {
        progress_bar::bar_area(area, self.fraction, self.percent)
    }

    /// The fraction the cell at `x` stands for: the cell's own centre in the bar, so a press lands
    /// between two cells rather than on the seam between them, and a pointer past either end of
    /// the track clamps to that end. Nothing when the bar has no cells to divide by.
    fn fraction_at(&self, bar: Rect, x: i32) -> Option<f32> {
        (bar.width > 0).then(|| ((x - bar.x) as f32 + 0.5) / f32::from(bar.width)).map(|f| f.clamp(0.0, 1.0))
    }

    /// Sends the fraction at `x`, unless the pointer is still on the cell it last sent: a terminal
    /// that repeats one position would otherwise fill the application with the same seek.
    fn seek(&self, cx: &mut EventCx<'_, Msg>, x: i32) {
        let Some(fraction) = self.fraction_at(self.bar(cx.area()), x) else {
            return;
        };
        let memory = cx.memory::<SeekBarMemory>();
        if memory.sent == Some(fraction) {
            return;
        }
        memory.sent = Some(fraction);
        if let Some(on_seek) = &self.on_seek {
            cx.emit(on_seek(fraction));
        }
    }

    /// Remembers the label the overlay will paint and asks for the overlay, anchored on the
    /// pointer's own cell so the label stands over the position it names. Nothing to remember
    /// while the pointer is elsewhere, so the label leaves with it.
    fn schedule_label(&self, cx: &mut PaintCx<'_>, bar: Rect, pointer: Option<(i32, i32)>) {
        let now = cx.now();
        let memory = cx.memory::<SeekBarMemory>();
        let Some((label, (x, y))) = self.hover_label.as_ref().zip(pointer) else {
            memory.hovered_since = None;
            memory.label = None;
            return;
        };
        // The label fades in from the moment the pointer arrived, and stays where it is after.
        memory.hovered_since.get_or_insert(now);
        memory.label = Some(label(self.fraction_at(bar, x).unwrap_or(self.fraction)));
        cx.request_overlay(Rect::new(x, y, 1, 1));
    }
}

impl<Msg: 'static> Widget<Msg> for SeekBar<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(available.width, 1.min(available.height))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let bar = self.bar(area);
        // Without a message the bar is a picture, as a progress bar is: nothing answers the pointer.
        let states = if self.active() {
            cx.register_hit(bar);
            cx.pressable_states()
        } else {
            Vec::new()
        };
        let pointer = cx.pointer().filter(|&(x, y)| bar.contains(x, y));
        progress_bar::paint_bar(cx, area, self.fraction, self.percent, "seek-bar", self.variant.as_deref(), &states);
        // The cell under the pointer takes the accent, so it reads as where the seek would land.
        if let Some((x, _)) = pointer {
            cx.clear(Rect::new(x, bar.y, 1, 1), cx.color("accent"));
        }
        self.schedule_label(cx, bar, pointer);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let shown = {
            let memory = cx.memory::<SeekBarMemory>();
            memory.label.clone().zip(memory.hovered_since)
        };
        let Some((label, since)) = shown else {
            return;
        };
        tooltip::paint_tip(cx, anchor, &label, Placement::Above, since);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        match event {
            Event::Key(key) => {
                let target = if key.is_plain(Key::Left) {
                    self.fraction - STEP
                } else if key.is_plain(Key::Right) {
                    self.fraction + STEP
                } else if key.is_plain(Key::Home) {
                    0.0
                } else if key.is_plain(Key::End) {
                    1.0
                } else {
                    return false;
                };
                let target = target.clamp(0.0, 1.0);
                // At an end there is nowhere to move, so the bar says nothing about it.
                if target != self.fraction
                    && let Some(on_seek) = &self.on_seek
                {
                    cx.emit(on_seek(target));
                }
                true
            }
            Event::Mouse(mouse) => match mouse.kind {
                // A press seeks where it lands and takes the pointer, so the drags that follow are
                // heard however far from the bar they happen.
                MouseKind::Down(MouseButton::Left) => {
                    cx.capture_pointer();
                    cx.memory::<SeekBarMemory>().dragging = true;
                    self.seek(cx, mouse.x);
                    true
                }
                MouseKind::Drag(MouseButton::Left) if cx.memory::<SeekBarMemory>().dragging => {
                    self.seek(cx, mouse.x);
                    true
                }
                MouseKind::Up(MouseButton::Left) => {
                    let memory = cx.memory::<SeekBarMemory>();
                    memory.dragging = false;
                    memory.sent = None;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::theme::State;
    use crate::widget::View;
    use crate::widgets::{Button, ProgressBar, Text};

    /// A player: the bar shows where it is and remembers every seek it was asked for.
    struct Player {
        fraction: f32,
        seen: Vec<f32>,
    }

    impl Player {
        fn at(fraction: f32) -> Self {
            Self { fraction, seen: Vec::new() }
        }
    }

    impl App for Player {
        type Msg = f32;
        fn update(&mut self, seek: f32) -> Command<f32> {
            self.fraction = seek;
            self.seen.push(seek);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, f32>) {
            ui.add(SeekBar::new(self.fraction).on_seek(|fraction| fraction)).fill_width();
        }
    }

    /// A twenty-cell bar, as a player draws it: twenty-five cells less the percentage.
    fn player() -> Harness<Player> {
        Harness::new(Player::at(0.5), 25, 3)
    }

    #[test]
    fn a_press_seeks_the_centre_of_the_cell_it_landed_on() {
        for (cell, wanted) in [(0, 0.025), (4, 0.225), (19, 0.975)] {
            let mut h = player();
            h.click(cell, 0);
            assert_eq!(h.app().seen, [wanted], "cell {cell} of twenty");
        }
    }

    #[test]
    fn a_drag_seeks_every_cell_it_crosses_and_holds_at_the_ends() {
        let mut h = player();
        h.mouse(MouseKind::Down(MouseButton::Left), 2, 0);
        for cell in 3..=10 {
            h.mouse(MouseKind::Drag(MouseButton::Left), cell, 0);
        }
        // Past the last cell, and off the row the bar is on, the seek holds at the whole track.
        h.mouse(MouseKind::Drag(MouseButton::Left), 24, 2).mouse(MouseKind::Drag(MouseButton::Left), 24, 2);
        h.mouse(MouseKind::Up(MouseButton::Left), 24, 2);
        assert_eq!(h.app().seen, [0.125, 0.175, 0.225, 0.275, 0.325, 0.375, 0.425, 0.475, 0.525, 1.0]);
        h.mouse(MouseKind::Drag(MouseButton::Left), 8, 0);
        assert_eq!(h.app().seen.len(), 10, "nothing is heard after the release");
    }

    #[test]
    fn the_keys_step_along_the_track_and_jump_to_its_ends() {
        let mut h = Harness::new(Player::at(0.5), 25, 1);
        h.press("tab").press("right");
        assert_eq!(h.app().seen, [0.55]);
        h.press("home");
        assert_eq!(h.app().seen, [0.55, 0.0]);
        h.press("left").press("right");
        assert_eq!(h.app().seen, [0.55, 0.0, 0.05], "the start of the track holds still");
        h.press("end").press("right");
        assert_eq!(h.app().seen, [0.55, 0.0, 0.05, 1.0], "and so does the end of it");
    }

    #[test]
    fn the_cell_under_the_pointer_takes_the_accent_and_the_bar_steps_lighter() {
        let mut h = player();
        let theme = h.env().theme();
        let accent = theme.color("accent");
        let lighter = theme.style("seek-bar", None, &[State::Hover]).paint("track").map(|paint| paint.at(0.0));
        let at_rest: Vec<_> = (10..20).map(|x| h.bg(x, 0)).collect();
        h.hover(15, 0);
        assert_eq!(h.bg(15, 0), accent, "the cell under the pointer takes the accent");
        assert_eq!(h.bg(12, 0), lighter, "the rest of the bar steps a tone lighter");
        assert_ne!(at_rest[0], lighter, "and is not the tone it had at rest");
        h.hover(15, 2);
        for (x, before) in (10..20).zip(at_rest) {
            assert_eq!(h.bg(x, 0), before, "cell {x} is as it was once the pointer left");
        }
    }

    #[test]
    fn the_label_under_the_pointer_names_the_position() {
        /// A bar with a label, under a row of its own so the label has somewhere to stand.
        struct Labelled;
        impl App for Labelled {
            type Msg = f32;
            fn update(&mut self, _: f32) -> Command<f32> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, f32>) {
                ui.column(|ui| {
                    ui.add(Text::new("track"));
                    ui.add(SeekBar::new(0.0).on_seek(|f| f).hover_label(|f| format!("{:.1} s", f * 225.0)))
                        .fill_width();
                });
            }
        }
        let mut h = Harness::new(Labelled, 30, 3);
        h.hover(7, 1).advance(Duration::from_millis(200));
        let (x, y) = h.find("67.5 s").unwrap_or_else(|| panic!("the label names the cell:\n{}", h.screen()));
        assert_eq!((x, y), (8, 0), "just above the cell the pointer is on:\n{}", h.screen());
        h.hover(7, 2);
        assert!(!h.screen().contains("67.5 s"), "the label leaves with the pointer:\n{}", h.screen());
    }

    /// A bar with no message beside a button, to see where the keyboard goes.
    struct Row;

    impl App for Row {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(SeekBar::new(0.4)).fill_width();
            ui.add(Button::new("Play").on_press(())).id("play");
        }
    }

    #[test]
    fn a_bar_without_a_message_is_drawn_and_answers_nothing() {
        let mut h = Harness::new(Row, 25, 2);
        let drawn = h.screen();
        // On the track, past the fill, where the accent of a marked cell would show.
        let resting = h.bg(15, 0);
        h.hover(15, 0);
        assert_eq!(h.bg(15, 0), resting, "the pointer marks no cell on a bar that seeks nowhere");
        h.click(4, 0).press("right").press("home");
        assert_eq!(h.screen(), drawn, "nothing the bar saw changed it");
        h.press("tab");
        assert!(h.is_focused("play"), "Tab went past the bar to the button");
    }

    #[test]
    fn the_bar_is_drawn_exactly_as_a_progress_bar() {
        /// The two bars one over the other, so every cell can be compared with its neighbour.
        struct Both;
        impl App for Both {
            type Msg = f32;
            fn update(&mut self, _: f32) -> Command<f32> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, f32>) {
                ui.add(ProgressBar::new(0.4)).fill_width();
                ui.add(SeekBar::new(0.4).on_seek(|f| f)).fill_width();
            }
        }
        let h = Harness::new(Both, 20, 2);
        let screen = h.screen();
        let rows: Vec<&str> = screen.lines().collect();
        assert!(rows[0].contains("40%"), "{screen}");
        assert_eq!(rows[0], rows[1], "the same picture in both cells and tones");
        for x in 0..20 {
            assert_eq!(h.bg(x, 0), h.bg(x, 1), "cell {x}");
        }
    }

    #[test]
    fn a_variant_colours_the_bar_as_it_colours_a_progress_bar() {
        /// The two bars in `variant`, one over the other.
        struct Both(&'static str);
        impl App for Both {
            type Msg = f32;
            fn update(&mut self, _: f32) -> Command<f32> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, f32>) {
                ui.add(ProgressBar::new(0.4).variant(self.0)).fill_width();
                ui.add(SeekBar::new(0.4).variant(self.0).on_seek(|f| f)).fill_width();
            }
        }
        let plain = Harness::new(Both(""), 20, 2);
        for variant in ["success", "warning", "danger"] {
            let h = Harness::new(Both(variant), 20, 2);
            for x in 0..20 {
                assert_eq!((h.fg(x, 0), h.bg(x, 0)), (h.fg(x, 1), h.bg(x, 1)), "{variant}, cell {x}");
            }
            // The first cell is filled, so a variant that reached the seek bar shows there.
            assert_ne!(h.bg(0, 1), plain.bg(0, 1), "{variant} colours the fill of the seek bar");
        }
    }

    #[test]
    fn without_the_percentage_the_bar_takes_the_whole_row_and_a_press_lands_on_it() {
        struct Bare(f32);
        impl App for Bare {
            type Msg = f32;
            fn update(&mut self, seek: f32) -> Command<f32> {
                self.0 = seek;
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, f32>) {
                ui.add(SeekBar::new(self.0).percent(false).on_seek(|fraction| fraction)).fill_width();
            }
        }
        let mut h = Harness::new(Bare(0.0), 20, 1);
        assert!(!h.screen().contains('%'), "no percentage is written:\n{}", h.screen());
        h.click(19, 0);
        assert_eq!(h.app().0, 0.975, "the last of twenty cells is the bar's own, not the label's");
    }
}
