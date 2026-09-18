//! What every modal layer shares: the dimmed screen, the overlay surface that pops in with a
//! pillar down its left edge, the bold title, the faint key hint line, and dismissal: Esc, the
//! close mark at the top right and, where the layer asks for it, a click on the dimmed screen.
//! A dismissable layer has all of them and a layer that is not has none.

use crate::color::Rgb;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Padding, Rect, Size};
use crate::keymap::Key;
use crate::motion::{Easing, steps};
use crate::text;
use crate::widget::{EventCx, Grounds, PaintCx};

use super::cells;
use super::close_mark;

/// How far the screen behind a layer is blended towards the scrim colour, in percent, when the
/// theme does not say.
const DEFAULT_STRENGTH: u16 = 55;

/// Cells a popping surface grows by on each side while it enters: columns, then rows.
const POP: (u16, u16) = (2, 1);

/// Where a layer's surface sits on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SurfacePosition {
    /// In the middle, for dialogs.
    Center,
    /// Centred horizontally in the upper part of the screen, for pickers: the top stays put
    /// while filtering changes the height.
    Top,
}

/// State of a layer between frames.
#[derive(Debug, Default)]
pub(crate) struct LayerMemory {
    /// The whole surface, used to tell clicks on the dimmed screen from clicks inside.
    pub(crate) surface: Rect,
    /// The close mark's cells while the layer is dismissable.
    close: Option<Rect>,
    /// When the layer that owns this memory last opened.
    opened: Option<std::time::Duration>,
}

/// The surface of a layer being painted.
#[derive(Debug, Clone)]
pub(crate) struct Surface {
    /// Whether the layer opened in this frame; widgets reset what they typed or scrolled, which
    /// memory in a persistent page would otherwise keep from the last time.
    pub(crate) fresh: bool,
    /// The part shown in this frame of the entrance.
    pub(crate) shown: Rect,
    /// The full surface without its padding.
    pub(crate) inner: Rect,
    /// The surface colour.
    pub(crate) background: Rgb,
    /// Where the close mark goes, when the layer is dismissable.
    close: Option<Rect>,
    /// How far the entrance has come in whole cell steps, 0 to 1.
    entered: f32,
    /// The dimmed screen around the surface, which the surface keeps apart from.
    grounds: Grounds,
}

/// The padding of the surface style `style`, `[1, 3]` when the theme does not say. A dismissable
/// layer keeps at least one row on top, where the close mark sits in the top right corner, and
/// at least the mark's width on the right, so the content column never reaches under the mark.
pub(crate) fn padding(cx: &mut PaintCx<'_>, style: &str, dismissable: bool) -> Padding {
    let pair = cx.env().theme().style(style, None, &[]).pair("padding").unwrap_or((1, 3));
    let mut padding = Padding::symmetric(pair.0, pair.1);
    if dismissable {
        padding.right = padding.right.max(close_mark::WIDTH);
        padding.top = padding.top.max(1);
    }
    padding
}

/// How a layer's surface looks and closes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look<'a> {
    /// The surface style, such as `modal`.
    pub(crate) style: &'a str,
    /// The theme variant, such as `danger`, which colours the pillar.
    pub(crate) variant: Option<&'a str>,
    /// Whether Esc and the close mark close the layer; the mark is drawn only then.
    pub(crate) dismissable: bool,
}

/// Opens the layer for this frame: dims the screen, blocks the pointer from the widgets
/// beneath, clears the part of a surface of `size` (padding included) that has entered so far
/// and draws the pillar down its left edge. The padding must be [`padding`] with the same
/// `dismissable`. Paint the content inside [`Surface::shown`], then call [`finish`].
///
/// The pillar is `▌` on every row in the first padding column, coloured by the `pillar` key of
/// the style and its variant (`modal` is accent-muted, `modal.danger` danger).
pub(crate) fn open(cx: &mut PaintCx<'_>, size: Size, placement: SurfacePosition, look: Look<'_>) -> Surface {
    let opened = cx.open_layer();
    let screen = cx.clip();
    let enter = cx.env().theme().motion().enter;
    let progress = cx.progress_since(opened, enter, Easing::EaseOut);

    let backdrop = cx.style("layer-backdrop", None, &[]);
    let scrim = backdrop.color("scrim").unwrap_or_else(|| cx.color("canvas"));
    let strength = f32::from(backdrop.cells("strength").unwrap_or(DEFAULT_STRENGTH).min(100)) / 100.0;
    cx.tint(screen, scrim, strength * progress);
    cx.register_hit(screen);

    let size = size.min(Size::new(screen.width.saturating_sub(2), screen.height.saturating_sub(2)));
    let full = match placement {
        SurfacePosition::Center => screen.centered(size),
        SurfacePosition::Top => {
            let top = (screen.height / 6).min(screen.height.saturating_sub(size.height) / 2);
            let left = (screen.width - size.width) / 2;
            Rect::new(screen.x + i32::from(left), screen.y + i32::from(top), size.width, size.height)
        }
    };
    // Toasts keep clear of where the surface ends up, not of the part popped in so far.
    cx.set_layer_surface(full);
    let grounds = cx.grounds_around(full);
    // The surface grows by whole cells; its colours follow the same steps.
    let columns = steps(1.0 - progress, POP.0).min(full.width / 2);
    let rows = steps(1.0 - progress, POP.1).min(full.height / 2);
    let shown = Rect::new(
        full.x + i32::from(columns),
        full.y + i32::from(rows),
        full.width - columns * 2,
        full.height - rows * 2,
    );
    let entered = 1.0 - f32::from(columns) / f32::from(POP.0);

    let surface_style = cx.style(look.style, look.variant, &[]);
    let background = surface_style.text().bg.unwrap_or_else(|| cx.color("overlay"));
    let dimmed = cx.color("canvas").mix(background, entered);
    cx.clear(shown, dimmed);
    let pad = padding(cx, look.style, look.dismissable);
    if let Some(pillar) = surface_style.color("pillar").filter(|_| pad.left >= 1) {
        // Painted in its own colour; `finish` fades it in with the rest of the surface.
        for y in shown.y..shown.bottom() {
            cx.pillar(shown.x, y, pillar);
        }
    }
    let inner = full.inset(pad);
    // The mark's three cells are the top right corner of the surface, on the top padding row.
    let close = look.dismissable.then(|| close_mark::rect(full.right() - i32::from(close_mark::WIDTH), full.y));
    let memory = cx.memory::<LayerMemory>();
    memory.surface = full;
    memory.close = close;
    let fresh = memory.opened != Some(opened);
    memory.opened = Some(opened);
    Surface { fresh, shown, inner, background, close, entered, grounds }
}

/// Draws the close mark of a dismissable layer over the content, keeps the surface apart from
/// the dimmed screen around it (see [`PaintCx::floating`]), then fades everything painted on
/// `surface` in with the entrance steps.
pub(crate) fn finish(cx: &mut PaintCx<'_>, surface: &Surface) {
    if let Some(close) = surface.close {
        cx.with_clip(surface.shown, |cx| {
            close_mark::paint(cx, close.x, close.y, true);
        });
    }
    // Decided on the surface's own tone, so the lift stays the same through the entrance.
    let lift = cx.lift_for(surface.shown, &surface.grounds, Some(surface.background));
    if let Some(lift) = lift {
        cx.lift(surface.shown, lift);
    }
    if surface.entered < 1.0 {
        let background = lift.map_or(surface.background, |lift| lift.apply(surface.background));
        let dimmed = cx.color("canvas").mix(background, surface.entered);
        cx.tint(surface.shown, dimmed, 1.0 - surface.entered);
    }
}

/// Draws a layer title at `(x, y)`, bold.
pub(crate) fn title(cx: &mut PaintCx<'_>, x: i32, y: i32, width: u16, title: &str) {
    let style = cx.style("modal-title", None, &[]);
    let shown = text::truncate(title, width).into_owned();
    cx.text(x, y, &shown, style.text(), width);
}

/// A key and what it does, for the faint hint line of a layer.
pub(crate) type Hint = (String, String);

/// The hint for a keymap-independent key: `key` and the translated label `quvyta.layer.<label>`.
pub(crate) fn hint(cx: &PaintCx<'_>, key: &str, label: &str) -> Hint {
    (key.to_owned(), cx.env().i18n().translate(&format!("quvyta.layer.{label}"), &[]))
}

/// Cells between two hints.
const HINT_SPACING: u16 = 3;

/// Width of `hints` drawn side by side.
pub(crate) fn hints_width(hints: &[Hint]) -> u16 {
    let count = u16::try_from(hints.len()).unwrap_or(u16::MAX);
    cells::sum(hints.iter().map(|(key, label)| cells::sum([text::width(key), 1, text::width(label)])))
        + HINT_SPACING * count.saturating_sub(1)
}

/// Draws as many `hints` as fit in `width` cells from `(x, y)`: keys slightly brighter than
/// their labels, both faint, nothing bracketed.
pub(crate) fn paint_hints(cx: &mut PaintCx<'_>, x: i32, y: i32, width: u16, hints: &[Hint]) {
    let key_style = cx.style("layer-hint-key", None, &[]).text();
    let label_style = cx.style("layer-hint-label", None, &[]).text();
    let mut shown = hints.len();
    while shown > 0 && hints_width(&hints[..shown]) > width {
        shown -= 1;
    }
    let mut column = x;
    for (key, label) in &hints[..shown] {
        column += i32::from(cx.text(column, y, key, key_style, text::width(key))) + 1;
        column += i32::from(cx.text(column, y, label, label_style, text::width(label))) + i32::from(HINT_SPACING);
    }
}

/// Lets the pointer carry the one highlight of a list in a layer, but only when it moves: a
/// pointer resting where a list unfolds from the keyboard, or resting on a row while the keyboard
/// moves the highlight on, must not pull the highlight back to itself.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PointerGate {
    last: Option<(i32, i32)>,
}

impl PointerGate {
    /// A gate for a layer opening while the pointer is at `pointer`.
    pub(crate) fn new(pointer: Option<(i32, i32)>) -> Self {
        Self { last: pointer }
    }

    /// Whether the pointer, now at `pointer`, moved since the layer opened or since the last call.
    pub(crate) fn moved(&mut self, pointer: Option<(i32, i32)>) -> bool {
        let moved = pointer.is_some() && pointer != self.last;
        self.last = pointer.or(self.last);
        moved
    }
}

/// The scrollbar of a list inside a layer, which the pointer can press and drag like a list's:
/// a press on the bar moves the thumb there and the pointer holds it until release.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BarDrag {
    /// The bar's column as painted last, while the list scrolls.
    bar: Option<Rect>,
    dragging: bool,
}

impl BarDrag {
    /// Records where the bar is painted this frame; `None` while the list fits.
    pub(crate) fn place(&mut self, bar: Option<Rect>) {
        self.bar = bar;
        self.dragging &= bar.is_some();
    }

    /// Whether the bar should look active: dragged, or the pointer is on it.
    pub(crate) fn active(self, pointer: Option<(i32, i32)>) -> bool {
        self.dragging || self.bar.zip(pointer).is_some_and(|(bar, (x, y))| bar.contains(x, y))
    }

    /// Handles a pointer event for the bar of a list at `metrics`. Returns the offset to scroll
    /// to when the bar used the event.
    pub(crate) fn event<Msg>(
        &mut self,
        cx: &mut EventCx<'_, Msg>,
        mouse: &crate::event::MouseEvent,
        metrics: super::scrollbar::ScrollMetrics,
    ) -> Option<usize> {
        let bar = self.bar?;
        let row = crate::geometry::clamp_u16(mouse.y - bar.y);
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) if bar.contains(mouse.x, mouse.y) => {
                cx.capture_pointer();
                self.dragging = true;
                Some(metrics.offset_at(row, bar.height))
            }
            MouseKind::Drag(MouseButton::Left) if self.dragging => Some(metrics.offset_at(row, bar.height)),
            MouseKind::Up(MouseButton::Left) if self.dragging => {
                self.dragging = false;
                Some(metrics.offset)
            }
            _ => None,
        }
    }
}

/// What an event did to a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backdrop {
    /// The layer should close: Esc, a click on the close mark, or a click on the dimmed screen
    /// when that closes it. Only ever for a dismissable layer.
    Close,
    /// A pointer event on the dimmed screen, which the layer swallows so nothing beneath reacts
    /// and no text selection starts there.
    Swallowed,
    /// A pointer event on the surface that no widget inside used; the layer may leave it unused
    /// so text on the surface can be selected.
    Inside,
    /// Not the backdrop's business.
    Ignored,
}

/// Reads the events every layer handles the same way. Nothing closes a layer that is not
/// `dismissable`; a dismissable one closes on Esc and on its close mark, and on a click on the
/// dimmed screen when `click_outside_closes`.
pub(crate) fn backdrop_event<Msg>(
    cx: &mut EventCx<'_, Msg>,
    event: &Event,
    dismissable: bool,
    click_outside_closes: bool,
) -> Backdrop {
    match event {
        Event::Key(key) if key.is_plain(Key::Esc) && dismissable => Backdrop::Close,
        Event::Mouse(mouse) => {
            let (surface, close) = {
                let memory = cx.memory::<LayerMemory>();
                (memory.surface, memory.close.filter(|_| dismissable))
            };
            let outside = !surface.contains(mouse.x, mouse.y);
            let left_down = mouse.kind == MouseKind::Down(MouseButton::Left);
            let on_close_mark = close.is_some_and(|close| close.contains(mouse.x, mouse.y));
            if left_down && (on_close_mark || (outside && click_outside_closes && dismissable)) {
                Backdrop::Close
            } else if outside {
                Backdrop::Swallowed
            } else {
                Backdrop::Inside
            }
        }
        _ => Backdrop::Ignored,
    }
}
