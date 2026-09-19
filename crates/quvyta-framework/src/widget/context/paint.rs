//! The painting context.

use std::time::Duration;

use ratatui_core::buffer::{Buffer, Cell};
use ratatui_core::style::Color;
use unicode_segmentation::UnicodeSegmentation;

use super::MeasureCx;
use super::frame::{FocusRequest, Frame, Interaction, LayerEntry};
use crate::color::Rgb;
use crate::env::Env;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::PILLAR;
use crate::keymap::KeyChord;
use crate::motion::{Easing, Tweens};
use crate::style::{CellStyle, WidgetStyle, to_color};
use crate::text;
use crate::theme::State;
use crate::widget::memory::Memory;
use crate::widget::{Key, LayoutProps, Node, WidgetId};

/// Painting context: draws into the frame, clipped to the widget's visible area.
pub struct PaintCx<'a> {
    pub(crate) buf: &'a mut Buffer,
    pub(crate) env: &'a Env,
    pub(crate) frame: &'a mut Frame,
    pub(crate) memory: &'a mut Memory,
    pub(crate) interaction: &'a Interaction,
    pub(crate) now: Duration,
    pub(crate) clip: Rect,
    pub(crate) id: WidgetId,
    pub(crate) layout: LayoutProps,
    pub(crate) scope: Option<WidgetId>,
    /// How long no input has arrived, for views a widget builds while it paints.
    pub(crate) idle: Duration,
}

impl PaintCx<'_> {
    /// The id of the widget being painted.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// The environment.
    #[must_use]
    pub fn env(&self) -> &Env {
        self.env
    }

    /// Layout properties of the widget being painted.
    #[must_use]
    pub fn layout(&self) -> LayoutProps {
        self.layout
    }

    /// Time since the runtime started; drives animations.
    #[must_use]
    pub fn now(&self) -> Duration {
        self.now
    }

    /// Whether the pointer is over this widget.
    #[must_use]
    pub fn is_hovered(&self) -> bool {
        self.interaction.hovered == Some(self.id)
    }

    /// The widget that has keyboard focus, if any.
    #[must_use]
    pub fn focused(&self) -> Option<WidgetId> {
        self.interaction.focused
    }

    /// The pointer cell wherever it is on screen, unlike [`PaintCx::pointer`].
    #[must_use]
    pub fn pointer_anywhere(&self) -> Option<(i32, i32)> {
        self.interaction.pointer
    }

    /// Once this frame is painted, moves keyboard focus to the first focusable widget inside
    /// `ancestor`, unless focus is already inside. Used by layers that take focus when they open.
    pub fn request_focus_within(&mut self, ancestor: WidgetId) {
        self.frame.focus_request = Some(FocusRequest::Within(ancestor));
    }

    /// Once this frame is painted, moves keyboard focus to `id` if that widget is focusable.
    /// Used by layers to give focus back when they close.
    pub fn request_focus(&mut self, id: WidgetId) {
        self.frame.focus_request = Some(FocusRequest::Exact(id));
    }

    /// Makes this widget a dismissable layer for this frame: a pointer press outside it and its
    /// descendants sends it [`Event::PointerOutside`](crate::event::Event::PointerOutside) and
    /// then still reaches what was pressed, unless that is the widget whose press opened the
    /// layer (then the press only closes it); an Esc key no focused widget used is sent to it.
    /// Layers registered later are on top and are asked first. Dismissable layers share the
    /// layer stack with modal layers ([`PaintCx::open_layer`]): a press on a modal layer above a
    /// dismissable one lands inside the modal layer, so the dismissable layer beneath stays.
    pub fn register_dismissable(&mut self) {
        self.frame.layers.push(LayerEntry { id: self.id, modal: false, surface: None });
    }

    /// Whether this widget has keyboard focus.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.interaction.focused == Some(self.id)
    }

    /// Whether this widget has focus that should be shown loudly: it was reached with the
    /// keyboard rather than clicked. Buttons and cards breathe their pillar only then, so a
    /// clicked button stays calm under the pointer.
    #[must_use]
    pub fn is_focus_visible(&self) -> bool {
        self.is_focused() && !self.interaction.focus_by_pointer
    }

    /// The pointer cell, when the pointer is over this widget.
    #[must_use]
    pub fn pointer(&self) -> Option<(i32, i32)> {
        self.interaction.pointer.filter(|_| self.is_hovered())
    }

    /// Whether this widget is flashing after being activated. Schedules the frame that ends
    /// the flash.
    pub fn is_pressed(&mut self) -> bool {
        let Some((id, at)) = self.interaction.pressed else {
            return false;
        };
        let end = at + self.env.theme().motion().flash;
        if id != self.id || self.now >= end {
            return false;
        }
        self.frame.schedule(end);
        true
    }

    /// Hover, focus and pressed states of a pressable widget such as a button, card, tab or
    /// toggle: like [`states`](Self::states), but focus counts only when it is
    /// [visible](Self::is_focus_visible), so a clicked control stays calm under the pointer.
    pub fn pressable_states(&mut self) -> Vec<State> {
        let visible = self.is_focus_visible();
        let mut states = self.states();
        if !visible {
            states.retain(|state| *state != State::Focus);
        }
        states
    }

    /// Hover, focus and pressed states of this widget.
    pub fn states(&mut self) -> Vec<State> {
        let mut states = Vec::new();
        if self.is_hovered() {
            states.push(State::Hover);
        }
        if self.is_focused() {
            states.push(State::Focus);
        }
        if self.is_pressed() {
            states.push(State::Pressed);
        }
        states
    }

    /// The theme style of `widget.variant` in `states`, evaluated for this frame. Animated
    /// styles schedule the next frame.
    pub fn style(&mut self, widget: &str, variant: Option<&str>, states: &[State]) -> WidgetStyle {
        let props = self.env.theme().style(widget, variant, states);
        let style = WidgetStyle::new(props, self.pulse_phase());
        if style.is_animated() && !self.env.reduced_motion() {
            self.request_frame_in(PULSE_FRAME);
        }
        style
    }

    /// A theme colour token such as `"accent"`; black when the token does not exist.
    #[must_use]
    pub fn color(&self, token: &str) -> Rgb {
        self.env.theme().color(token).unwrap_or(Rgb::new(0, 0, 0))
    }

    /// Asks for another frame after `delay`.
    pub fn request_frame_in(&mut self, delay: Duration) {
        self.frame.schedule(self.now + delay);
    }

    /// Whether the user asked for reduced motion; animations should show their end state.
    #[must_use]
    pub fn reduced_motion(&self) -> bool {
        self.env.reduced_motion()
    }

    /// A value of this widget that moves towards `target` over `duration`. The value named
    /// `name` starts at its first target without animating; later target changes animate from
    /// wherever the value is. Schedules frames while it moves; returns `target` at once when
    /// motion is reduced.
    pub fn animate(&mut self, name: &'static str, target: f32, duration: Duration, easing: Easing) -> f32 {
        if self.env.reduced_motion() {
            return target;
        }
        let now = self.now;
        let persistent = self.scope.is_some();
        let tween = self.memory.get::<Tweens>(self.id, persistent).drive(name, target, now, duration, easing);
        if tween.is_running(now) {
            self.request_frame_in(ANIMATION_FRAME);
        }
        tween.value(now)
    }

    /// Eased progress from 0 to 1 of something that started at `start` (a time from
    /// [`PaintCx::now`] or [`EventCx::now`](super::EventCx::now)) and takes `duration`. Schedules frames until it
    /// completes; is 1 at once when motion is reduced.
    pub fn progress_since(&mut self, start: Duration, duration: Duration, easing: Easing) -> f32 {
        if self.env.reduced_motion() || duration.is_zero() {
            return 1.0;
        }
        let elapsed = self.now.saturating_sub(start);
        if elapsed >= duration {
            return 1.0;
        }
        self.request_frame_in(ANIMATION_FRAME);
        easing.apply(elapsed.as_secs_f32() / duration.as_secs_f32())
    }

    /// Where a repeating animation of length `period` is, `0.0..1.0`. Schedules smooth frames;
    /// always 0 when motion is reduced.
    pub fn cycle(&mut self, period: Duration) -> f32 {
        let period = period.as_millis().max(1);
        if self.env.reduced_motion() {
            return 0.0;
        }
        self.request_frame_in(ANIMATION_FRAME);
        (self.now.as_millis() % period) as f32 / period as f32
    }

    /// How many whole `interval`s have passed, for animations that jump between frames such as
    /// spinners. Schedules a frame exactly at the next step; always 0 when motion is reduced.
    pub fn ticks(&mut self, interval: Duration) -> u128 {
        let interval_ms = interval.as_millis().max(1);
        if self.env.reduced_motion() {
            return 0;
        }
        let elapsed = self.now.as_millis();
        let into = u64::try_from(elapsed % interval_ms).unwrap_or(0);
        self.request_frame_in(Duration::from_millis(u64::try_from(interval_ms).unwrap_or(u64::MAX) - into));
        elapsed / interval_ms
    }

    /// Where the theme pulse is, `0.0..1.0`; always 0 when motion is reduced.
    #[must_use]
    pub fn pulse_phase(&self) -> f32 {
        if self.env.reduced_motion() {
            return 0.0;
        }
        let period = self.env.theme().motion().pulse_period.as_secs_f64();
        let phase = self.now.as_secs_f64().rem_euclid(period) / period;
        // `phase` is in 0..1, which f32 represents closely enough for colour blending.
        phase as f32
    }

    /// The visible area of this widget.
    #[must_use]
    pub fn clip(&self) -> Rect {
        self.clip
    }

    /// Whether pasting would insert text, as far as the runtime knows; see `probe_clipboard` on
    /// [`EventCx`](super::EventCx).
    pub(crate) fn can_paste(&self) -> bool {
        self.interaction.can_paste
    }

    /// Makes `rect` clickable for this widget. Later registrations are on top.
    pub fn register_hit(&mut self, rect: Rect) {
        self.register_hit_as(rect, self.id);
    }

    /// Makes `rect` clickable for `id`, a target the runtime handles itself (such as a toast)
    /// rather than a widget in the view.
    pub(crate) fn register_hit_as(&mut self, rect: Rect, id: WidgetId) {
        if let Some(visible) = self.visible_part(rect) {
            self.frame.hits.push((visible, id));
        }
    }

    /// Keeps mouse text selection from starting in `rect`, even inside a selectable area, e.g.
    /// for a secret shown in a selectable log.
    pub fn unselectable(&mut self, rect: Rect) {
        if let Some(visible) = self.visible_part(rect) {
            self.frame.unselectable.push(visible);
        }
    }

    /// Makes the visible part of `rect` a text selection region of this widget: a mouse drag
    /// that starts inside it, and that no widget uses, selects text, and the selection stays
    /// within this widget. Nothing is selectable unless a widget or
    /// [`NodeMut::selectable`](crate::widget::NodeMut::selectable) asks; widgets whose text is
    /// content to copy (code, documents, terminal output) call this while painting. The
    /// innermost region under a press wins.
    pub fn selectable(&mut self, rect: Rect) {
        if let Some(visible) = self.visible_part(rect) {
            self.frame.selectable.push((visible, self.id));
        }
    }

    /// Marks the cells of `rect` as decoration rather than content, such as a scrollbar a widget
    /// draws itself: a clean copy of a text selection leaves them out, a raw copy keeps them.
    /// [`PaintCx::pillar`] marks its cell by itself.
    pub fn decoration(&mut self, rect: Rect) {
        if let Some(visible) = self.visible_part(rect) {
            self.frame.decorations.push(visible);
        }
    }

    /// Asks for [`MouseKind::Moved`](crate::event::MouseKind::Moved) events in this frame: the
    /// pointer moving with no button held over this widget's hit area, or over a child of it.
    /// Other widgets never see plain moves. Widgets that follow the pointer without a button,
    /// such as an embedded terminal whose program asked for every motion,
    /// call this while painting, and only while they need it.
    pub fn track_pointer_moves(&mut self) {
        self.frame.pointer_moves.push(self.id);
    }

    /// Adds this widget to the keyboard focus order.
    pub fn register_focusable(&mut self) {
        self.frame.focusable.push(self.id);
    }

    /// Paints this widget's overlay after the rest of the view.
    pub fn request_overlay(&mut self, anchor: Rect) {
        self.frame.overlays.push((self.id, anchor));
    }

    /// Makes this widget a modal layer for this frame and returns the time the layer opened.
    ///
    /// Call it from [`Widget::paint_overlay`](crate::widget::Widget::paint_overlay) every frame
    /// the layer is shown, before painting what is inside it. Modal and dismissable layers
    /// ([`PaintCx::register_dismissable`]) share one stack in paint order; the rules of a modal
    /// layer are:
    ///
    /// - **Focus stays inside.** Each frame the layer asks, like
    ///   [`PaintCx::request_focus_within`], for focus to be inside it; a request made later in
    ///   the frame by a widget inside the layer (a popover opening) wins. Tab and Shift+Tab
    ///   cycle only through the layer's widgets.
    /// - **Input stops at the layer.** Keys and pointer events bubble from their target up to
    ///   the topmost modal layer and no further; a press outside it lands on the layer itself.
    ///   Paint a hit area over the whole screen and use presses on it, so nothing beneath
    ///   reacts and no text selection starts there.
    /// - **Shortcuts pause.** Application keymap actions (and global ones the runtime passes to
    ///   the application) do not run; quit, focus moves, debug, copy and paste still do. Key
    ///   listeners outside the layer are not heard.
    /// - **Above it:** dismissable layers opened inside it, later modal layers, and the
    ///   runtime's toasts, which stay clickable.
    /// - **Focus comes back.** When the layer is no longer painted, the runtime requests focus
    ///   for the widget that had it when the layer opened.
    ///
    /// The opening time stays the same for as long as the layer is shown, which makes it the
    /// start of an entrance animation, even for layers inside persistent pages.
    pub fn open_layer(&mut self) -> Duration {
        self.frame.layers.push(LayerEntry { id: self.id, modal: true, surface: None });
        self.request_focus_within(self.id);
        self.interaction.layers.iter().find(|layer| layer.id == self.id).map_or(self.now, |layer| layer.opened)
    }

    /// Tells the runtime where the surface of the modal layer this widget opened with
    /// [`PaintCx::open_layer`] sits, so toasts can keep clear of it.
    pub(crate) fn set_layer_surface(&mut self, surface: Rect) {
        let id = self.id;
        if let Some(layer) = self.frame.layers.iter_mut().rev().find(|layer| layer.id == id) {
            layer.surface = Some(surface);
        }
    }

    /// The areas of the modal layers open in this frame: each layer's surface, or all of
    /// `screen` for a layer that did not say where its surface is.
    pub(crate) fn modal_surfaces(&self, screen: Rect) -> Vec<Rect> {
        self.frame.layers.iter().filter(|layer| layer.modal).map(|layer| layer.surface.unwrap_or(screen)).collect()
    }

    /// Delivers `chord` to this widget even when it is not focused, for as long as the widget
    /// is painted: after the focused widgets had their chance and before the keymap. Repeats
    /// and releases of the key (which keyboards report while it is held) are delivered too,
    /// including repeats of Enter and Space that focused widgets never see. Inside a modal
    /// layer only listeners within the topmost layer hear keys.
    pub fn listen_key(&mut self, chord: KeyChord) {
        self.frame.listeners.push((chord, self.id));
    }

    /// Blends the text and background colours already drawn in `rect` towards `color` by
    /// `amount` (0 keeps them, 1 replaces them), e.g. to dim the screen behind a dialog. Cells
    /// drawn with reduced colour depth take `color` once `amount` passes one half.
    pub fn tint(&mut self, rect: Rect, color: Rgb, amount: f32) {
        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            return;
        }
        let depth = self.env.depth();
        let blend = |current: Color| match current {
            Color::Rgb(r, g, b) => Some(to_color(Rgb::new(r, g, b).mix(color, amount), depth)),
            _ if amount > 0.5 => Some(to_color(color, depth)),
            _ => None,
        };
        self.each_cell(rect, |cell| {
            if let Some(fg) = blend(cell.fg) {
                cell.fg = fg;
            }
            if let Some(bg) = blend(cell.bg) {
                cell.bg = bg;
            }
        });
    }

    /// This widget's state of type `T`.
    pub fn memory<T: Default + 'static>(&mut self) -> &mut T {
        self.memory.get::<T>(self.id, self.scope.is_some())
    }

    /// Fills `rect` with `color`, keeping text.
    pub fn fill(&mut self, rect: Rect, color: Rgb) {
        let bg = to_color(color, self.env.depth());
        self.each_cell(rect, |cell| cell.bg = bg);
    }

    /// Puts `color` behind the cells of `rect`, keeping their glyphs. A cell whose text would no
    /// longer read on it (contrast below 3:1) takes `readable` instead, so faint text stays legible
    /// under a highlight such as a text selection.
    pub(crate) fn fill_keeping_text_readable(&mut self, rect: Rect, color: Rgb, readable: Rgb) {
        let depth = self.env.depth();
        let (bg, text) = (to_color(color, depth), to_color(readable, depth));
        self.each_cell(rect, |cell| {
            cell.bg = bg;
            if let Color::Rgb(r, g, b) = cell.fg
                && cell.symbol().trim() != ""
                && Rgb::new(r, g, b).contrast_ratio(color) < 3.0
            {
                cell.fg = text;
            }
        });
    }

    /// Clears `rect` to spaces on `color`.
    pub fn clear(&mut self, rect: Rect, color: Rgb) {
        // An empty cell reads as a space and equals a cell holding one; copying a prepared blank
        // cell is cheaper than resetting, writing and styling every cell.
        let mut blank = Cell::EMPTY;
        blank.bg = to_color(color, self.env.depth());
        // Cells inside the rectangle are all replaced; only a wide character crossing its left
        // or right edge would be cut in half.
        let area = rect.intersect(self.clip);
        if !area.is_empty() {
            for y in area.y..area.bottom() {
                self.release(area.x, y);
                self.release(area.right() - 1, y);
            }
        }
        self.each_cell(rect, |cell| cell.clone_from(&blank));
    }

    /// Draws the theme's pillar (`[icons] pillar`, e.g. `▌`) at `(x, y)` in `color`, over whatever
    /// surface is already there. A blank glyph, as in ASCII mode, becomes a cell of `color`. The
    /// cell is [decoration](PaintCx::decoration): clean copies of a text selection skip it.
    pub fn pillar(&mut self, x: i32, y: i32, color: Rgb) {
        self.decoration(Rect::new(x, y, 1, 1));
        let env = self.env;
        let glyph = env.icons().glyph(PILLAR);
        if glyph.trim().is_empty() {
            self.fill(Rect::new(x, y, 1, 1), color);
        } else {
            self.text(x, y, &glyph, CellStyle::fg(color), 1);
        }
    }

    /// Draws `text` starting at `(x, y)`, at most `max` cells wide, clipped to the visible area.
    /// Returns the number of cells the text occupies (before clipping).
    pub fn text(&mut self, x: i32, y: i32, text: &str, style: CellStyle, max: u16) -> u16 {
        let paint = style.for_depth(self.env.depth());
        let clip = self.clip;
        let limit = x + i32::from(max);
        let mut column = x;
        // Printable ASCII is one cell per byte. The byte after the last one that fits is checked
        // too: a combining mark there would join the last character drawn.
        let drawn = text.len().min(usize::from(max));
        if text.get(..text.len().min(drawn + 1)).is_some_and(text::is_printable_ascii) {
            for index in 0..drawn {
                if clip.contains(column, y) {
                    self.release(column, y);
                }
                if clip.contains(column, y)
                    && let Some(cell) = self.cell_mut(column, y)
                {
                    cell.set_symbol(&text[index..=index]);
                    paint.apply(cell);
                }
                column += 1;
            }
            return clamp_u16(column - x);
        }
        for grapheme in text.graphemes(true) {
            let width = i32::from(text::grapheme_width(grapheme));
            if width == 0 {
                continue;
            }
            if column + width > limit {
                break;
            }
            // Every cell of the grapheme is released before any is written, so its own second
            // half is not mistaken for part of a character it cut.
            for offset in 0..width {
                if clip.contains(column + offset, y) {
                    self.release(column + offset, y);
                }
            }
            for offset in 0..width {
                let cx = column + offset;
                if !clip.contains(cx, y) {
                    continue;
                }
                let whole = clip.contains(column, y) && clip.contains(column + width - 1, y);
                if let Some(cell) = self.cell_mut(cx, y) {
                    let symbol = match (offset, whole) {
                        (0, true) => grapheme,
                        (_, true) => "",
                        _ => " ",
                    };
                    cell.set_symbol(symbol);
                    paint.apply(cell);
                }
            }
            column += width;
        }
        clamp_u16(column - x)
    }

    /// Paints a child node into `rect`, applying its padding.
    pub fn paint_child<M: 'static>(&mut self, node: &Node<M>, rect: Rect) {
        let saved = (self.id, self.layout, self.clip, self.scope);
        self.id = node.id;
        self.layout = node.layout;
        if node.persistent {
            self.scope = Some(node.id);
        }
        self.clip = saved.2.intersect(rect);
        self.frame.rects.insert(node.id, rect);
        self.frame.parents.insert(node.id, saved.0);
        if let Key::Named(name) = &node.key {
            self.frame.names.insert(node.id, name.clone());
        }
        if let Some(scope) = self.scope {
            self.frame.scopes.insert(node.id, scope);
        }
        self.memory.touch(node.id, self.scope.is_some());
        match node.selectable {
            Some(true) => self.selectable(rect),
            Some(false) => self.unselectable(rect),
            None => {}
        }
        if node.widget.focusable() {
            self.register_focusable();
        }
        node.widget.paint(self, rect.inset(node.layout.padding));
        (self.id, self.layout, self.clip, self.scope) = saved;
    }

    /// Paints a child node that does not take keyboard focus itself, not even when it is
    /// normally focusable. For composite widgets that take focus as one control and pass keys on
    /// to the child with [`EventCx::forward`](super::EventCx::forward), such as a settings row's switch.
    pub fn paint_child_unfocusable<M: 'static>(&mut self, node: &Node<M>, rect: Rect) {
        let registered = self.frame.focusable.len();
        self.paint_child(node, rect);
        self.frame.focusable.truncate(registered);
    }

    /// The pointer cell, when the pointer is over this widget or over a widget inside it, for
    /// containers whose rows light up while the pointer is on a control within them.
    #[must_use]
    pub fn pointer_within(&self) -> Option<(i32, i32)> {
        self.interaction.pointer.filter(|_| self.interaction.hovered_chain.contains(&self.id))
    }

    /// Whether keyboard focus is on this widget or on a widget painted inside it so far in this
    /// frame. Paint children before asking, e.g. to brighten a field label while its control has
    /// focus.
    #[must_use]
    pub fn has_focus_within(&self) -> bool {
        self.interaction.focused.is_some_and(|focused| self.frame.is_within(focused, self.id))
    }

    /// Measures a child node with the same rules as layout.
    pub fn measure_child<M: 'static>(&mut self, node: &Node<M>, available: Size) -> Size {
        MeasureCx::for_frame(self.env, &mut self.frame.measures).measure_child(node, available)
    }

    /// Runs `paint` with drawing limited to `rect` (and the current visible area).
    pub fn with_clip(&mut self, rect: Rect, paint: impl FnOnce(&mut Self)) {
        let saved = self.clip;
        self.clip = saved.intersect(rect);
        paint(self);
        self.clip = saved;
    }

    /// The part of `rect` inside the visible area, when there is one.
    fn visible_part(&self, rect: Rect) -> Option<Rect> {
        Some(rect.intersect(self.clip)).filter(|visible| !visible.is_empty())
    }

    /// Runs `change` on every cell of `rect` inside the visible area and the screen.
    pub(super) fn each_cell(&mut self, rect: Rect, mut change: impl FnMut(&mut Cell)) {
        let area = rect.intersect(self.clip);
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if let Some(cell) = self.cell_mut(x, y) {
                    change(cell);
                }
            }
        }
    }

    /// Keeps wide characters whole before a new symbol lands on `(x, y)`.
    ///
    /// A terminal draws a double-width character from its first cell across the next one, and the
    /// buffer holds it as the glyph followed by an empty second half. Replacing only one half, as a
    /// dialog's pillar or edge drawn over text does, leaves a pair the terminal cannot show: the
    /// glyph spills over the new symbol, or the rest of the row shifts by a column. So when the
    /// cell belongs to a wide character that reaches beyond it, every other cell of that
    /// character becomes a blank in the colours it already had. The cell may lie outside the
    /// visible area: a layer's edge decides what happens to the character it cuts.
    fn release(&mut self, x: i32, y: i32) {
        // The character this cell is the second half of: the nearest non-empty cell to the left.
        let mut lead = None;
        for back in 1..=MAX_GLYPH_CELLS {
            let Some(cell) = self.cell_mut(x - back, y) else { break };
            let symbol = cell.symbol();
            if symbol.is_empty() {
                continue;
            }
            let cells = i32::from(text::width(symbol));
            if cells > back {
                lead = Some((x - back, cells));
            }
            break;
        }
        if let Some((start, cells)) = lead {
            self.blank_cells(start, start + cells, y);
        }
        let own = self.cell_mut(x, y).map_or(1, |cell| i32::from(text::width(cell.symbol())));
        if own > 1 {
            self.blank_cells(x + 1, x + own, y);
        }
    }

    /// Turns the cells from `start` up to `end` on row `y` into spaces, keeping their colours.
    fn blank_cells(&mut self, start: i32, end: i32, y: i32) {
        for x in start..end {
            if let Some(cell) = self.cell_mut(x, y) {
                cell.set_symbol(" ");
            }
        }
    }

    /// The screen cell at `(x, y)`, when it is on screen.
    fn cell_mut(&mut self, x: i32, y: i32) -> Option<&mut Cell> {
        self.buf.cell_mut((u16::try_from(x).ok()?, u16::try_from(y).ok()?))
    }
}

/// The most cells one grapheme covers on screen; how far to look left for the start of the
/// character a cell belongs to.
const MAX_GLYPH_CELLS: i32 = 4;

/// Frame interval while something moves: about 60 frames a second.
pub(crate) const ANIMATION_FRAME: Duration = Duration::from_millis(16);

/// How often animated theme colours are redrawn.
pub(crate) const PULSE_FRAME: Duration = Duration::from_millis(50);
