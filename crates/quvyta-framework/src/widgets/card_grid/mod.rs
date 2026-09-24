//! Card grids: cards in as many columns as fit, moved through with the keys in two dimensions.

mod layout;
#[cfg(test)]
mod tests;

use crate::env::Env;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Padding, Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{Axis, EventCx, Flex, IdleScope, Length, MeasureCx, Node, PaintCx, View, Widget};

use super::click::Click;
use super::empty_state::EmptyState;
use super::row_menu::{self, RowAnchor, RowMenuItems};
use super::row_pointer::{self, PickedRows, Picking, RowDrop, Spot};
use super::scrollbar::{self, ScrollbarStyle};
use super::select_box;
use super::{ContextItem, IndexMessage};
use layout::{Layout, Sizing, Step};

/// Builds what one card shows.
type CardBuilder<Msg> = Box<dyn Fn(&mut View<'_, Msg>, usize)>;

/// Cards laid out in as many columns as fit, for a store's apps, a launcher's programs or a
/// choice of profiles. Only the cards on screen are built and drawn, so a grid of ten thousand
/// cards costs what one screen of them costs.
///
/// Every card is a surface one step above its background, with no frame. Under the pointer a
/// card rises one tone with a soft pillar `▌` down its left edge; the selected card takes the
/// selected surface, and its pillar breathes while the grid has focus reached with the keyboard.
/// Nothing slides: a card is a surface, not a list row. Only one card is lit at a time: while
/// the pointer moves over the grid it carries the highlight and the selected card rests; the
/// next key goes on from the card the pointer is on.
///
/// The column count follows the width: cards are at least [`card_width`](Self::card_width)'s
/// least width and share the room left, up to the widest. An area narrower than one card shows
/// one column as wide as the area, and the card's content is cut there (build it with
/// [`Text::no_wrap`](super::Text::no_wrap) so it ends in `…`).
///
/// The application owns the selection and the checked cards; the grid reports changes through
/// messages. Keys while focused: arrows move between cards and stop at the edges (Right on the
/// last card of a row stays there), Home and End go to the first and last card, PgUp and PgDn
/// move a screen of rows, Enter activates, and Space toggles the check when checks are on,
/// activating otherwise. A click selects and activates a card; with checks on, a click on the
/// mark in a card's top right corner only toggles it. The wheel scrolls a row of cards at a
/// time, and the scrollbar can be pressed and dragged.
///
/// Four capabilities make the cards work the way the icons of a file explorer do, each off until
/// asked for: [`activate_on(Click::Double)`](Self::activate_on) selects on a click and activates
/// on a double click; [`multi_select`](Self::multi_select) selects several cards with Ctrl+click,
/// Shift+click, Shift+arrows, Ctrl+A and Space; [`box_select`](Self::box_select) draws a box from the free space between
/// and after the cards and selects the cards it touches; [`droppable`](Self::droppable) drags the
/// selection onto a card that takes it.
///
/// ```
/// use std::rc::Rc;
///
/// use qframe::prelude::*;
/// use qframe::widgets::CardGrid;
///
/// struct Store {
///     apps: Rc<[(String, String)]>,
///     selected: Option<usize>,
/// }
///
/// #[derive(Clone)]
/// enum Msg {
///     Select(usize),
///     Open(usize),
/// }
///
/// impl App for Store {
///     type Msg = Msg;
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         if let Msg::Select(index) | Msg::Open(index) = msg {
///             self.selected = Some(index);
///         }
///         Command::none()
///     }
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         let apps = Rc::clone(&self.apps);
///         let grid = CardGrid::new(self.apps.len())
///             .card_width(24, 32)
///             .card_height(2)
///             .selected(self.selected)
///             .on_select(Msg::Select)
///             .on_activate(Msg::Open)
///             .card(move |ui, index| {
///                 let (name, summary) = &apps[index];
///                 ui.add(Text::new(name.as_str()).role("title").no_wrap());
///                 ui.add(Text::new(summary.as_str()).role("secondary").no_wrap());
///             });
///         ui.add(grid).fill();
///     }
/// }
///
/// let apps: Rc<[(String, String)]> = (0..40).map(|n| (format!("App {n}"), "Does a thing".to_owned())).collect();
/// let mut store = Harness::new(Store { apps, selected: None }, 80, 10);
/// store.press("tab").press("right").press("right").press("down");
/// assert_eq!(store.app().selected, Some(4));
/// ```
///
/// Cards are built while the grid paints, once for each card on screen, by the closure given to
/// [`card`](Self::card). It lives as long as the widget, so it owns what it reads, e.g. an
/// `Rc<[App]>` cloned from the state. Widgets inside a card are drawn but take no input of their
/// own: the card is the pressable surface, and a press anywhere on it is the card's. Idle
/// watches ([`View::on_idle`]) belong in the application's own view, not in a card.
///
/// Style keys: `card` (`bg`, `padding`, `pillar`) with `hover`, `selected`, `focus`, `pressed`;
/// `card-mark` for a checked card's mark and `card-mark.off` for the faint mark a lit card
/// offers while checks are on; `tree-drop` (`bg`) for the card a drag would drop on;
/// `text-selection` (`bg`) for the selection box; `scrollbar`.
pub struct CardGrid<Msg> {
    count: usize,
    min_width: u16,
    max_width: u16,
    height: u16,
    column_gap: u16,
    row_gap: u16,
    selected: Option<usize>,
    checked: Option<Vec<bool>>,
    disabled: bool,
    scrollbar: Option<ScrollbarStyle>,
    on_select: Option<IndexMessage<Msg>>,
    on_activate: Option<IndexMessage<Msg>>,
    on_toggle: Option<IndexMessage<Msg>>,
    card: Option<CardBuilder<Msg>>,
    menu: Option<RowMenuItems<Msg>>,
    empty: Vec<Node<Msg>>,
    picking: Picking<Msg>,
}

/// What a grid remembers between frames.
#[derive(Debug, Default)]
struct GridMemory {
    /// First row of cards shown.
    offset: usize,
    /// The selection and column count the offset last followed: a new selection, or a resize
    /// that moves it to another row, scrolls it into view once.
    followed: Option<(Option<usize>, usize)>,
    /// Whether the scrollbar thumb is being dragged.
    dragging: bool,
    /// The card activated last, for the press flash.
    flashed: Option<usize>,
    /// Where the pointer was when the grid was painted last.
    pointer: Option<(i32, i32)>,
    /// Whether the pointer moved over the grid since the last key, so it carries the highlight.
    pointed: bool,
    /// The layout painted last, which input is read against.
    layout: Layout,
}

impl<Msg: 'static> CardGrid<Msg> {
    /// A grid of `count` cards, 24 to 32 cells wide and three rows of content high, two cells
    /// apart in a row and one row apart between rows. Give it what cards show with
    /// [`card`](Self::card).
    #[must_use]
    pub fn new(count: usize) -> Self {
        Self {
            count,
            min_width: 24,
            max_width: 32,
            height: 3,
            column_gap: 2,
            row_gap: 1,
            selected: None,
            checked: None,
            disabled: false,
            scrollbar: None,
            on_select: None,
            on_activate: None,
            on_toggle: None,
            card: None,
            menu: None,
            empty: Vec::new(),
            picking: Picking::default(),
        }
    }

    /// The narrowest and the widest a card gets, in cells. As many columns as fit at `min`
    /// share the width, each at most `max`.
    #[must_use]
    pub fn card_width(mut self, min: u16, max: u16) -> Self {
        self.min_width = min.max(1);
        self.max_width = max.max(self.min_width);
        self
    }

    /// Rows of content in every card, without the card's padding.
    #[must_use]
    pub fn card_height(mut self, rows: u16) -> Self {
        self.height = rows.max(1);
        self
    }

    /// Cells between two cards of a row, and rows between two rows of cards.
    #[must_use]
    pub fn gap(mut self, columns: u16, rows: u16) -> Self {
        self.column_gap = columns;
        self.row_gap = rows;
        self
    }

    /// The selected card's index.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Turns checks on: `checked[i]` tells whether card `i` is checked, and a checked card
    /// carries a mark in its top right corner. Space and a click on the mark report toggles
    /// through [`on_toggle`](Self::on_toggle).
    #[must_use]
    pub fn checked(mut self, checked: Vec<bool>) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Keeps the grid from being hovered, focused or pressed; its messages are not sent. The
    /// cards fade and the selected card still shows.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Draws the scrollbar in `style` whatever the theme chooses.
    #[must_use]
    pub fn scrollbar(mut self, style: ScrollbarStyle) -> Self {
        self.scrollbar = Some(style);
        self
    }

    /// Message for moving the selection to a card.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// Message for opening a card (Enter, a click).
    #[must_use]
    pub fn on_activate(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_activate = Some(Box::new(message));
        self
    }

    /// Message for checking or unchecking a card while checks are on (Space, a click on the
    /// mark).
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(message));
        self
    }

    /// Builds what card `index` shows, into the card's padded content area. Called only for the
    /// cards on screen, every time the grid paints.
    #[must_use]
    pub fn card(mut self, build: impl Fn(&mut View<'_, Msg>, usize) + 'static) -> Self {
        self.card = Some(Box::new(build));
        self
    }

    /// Gives every card a context menu: `items(index)` builds the entries for the card of that
    /// index, and the menu acts on the card it was opened on rather than on the selected one.
    ///
    /// A right press on a card opens the menu at the pointer; the menu key or Shift+F10 opens the
    /// menu of the card the keys are on, scrolling it into view first. That card stays raised
    /// while the menu is open, so it is clear what the entries act on. A right press on a card
    /// that is not checked makes it the selection first, so a menu never acts on cards the person
    /// did not mean.
    #[must_use]
    pub fn context_menu(mut self, items: impl Fn(usize) -> Vec<ContextItem<Msg>> + 'static) -> Self {
        self.menu = Some(Box::new(items));
        self
    }

    /// How many clicks activate a card: [`Click::Single`], the default, selects and activates at
    /// once; [`Click::Double`] only selects on a click and activates on a second press on the same
    /// card within [`Click::INTERVAL`]. Enter activates either way.
    #[must_use]
    pub fn activate_on(mut self, click: Click) -> Self {
        self.picking.activate_on = click;
        self
    }

    /// Lets several cards be selected at once: `selected` holds their indexes and
    /// `message(cards)` asks the application to make `cards` the whole new selection.
    ///
    /// The card given to [`selected`](Self::selected) stays the one the keys move from, while
    /// every selected card takes the selected surface. Ctrl+click adds a card or takes it out,
    /// Shift+click selects the cards from the last plain or Ctrl click to this one in reading
    /// order; Shift with the arrows, PgUp/PgDn or Home/End extends that range the same way, Ctrl+A
    /// selects every card, Space adds or takes out the card the keys are on, Esc reduces several
    /// selected cards to that one, and a plain click or arrow selects that one card. A right click on a selected card keeps the selection for its menu; on another card it
    /// makes that card the selection first.
    #[must_use]
    pub fn multi_select(mut self, selected: &[usize], message: impl Fn(Vec<usize>) -> Msg + 'static) -> Self {
        self.picking.chosen = selected.to_vec();
        self.picking.on_choose = Some(Box::new(message));
        self
    }

    /// Lets a drag from the free space between and after the cards draw a box: every card it
    /// touches becomes the selection while it is drawn, or joins it when Ctrl was held at the
    /// press, and a click there without a drag clears the selection. The box is a tone laid over
    /// the cells it covers, never a frame. It needs [`multi_select`](Self::multi_select) and does
    /// nothing without it.
    #[must_use]
    pub fn box_select(mut self, on: bool) -> Self {
        self.picking.box_select = on;
        self
    }

    /// Lets cards be dragged onto other cards, such as files onto a folder: `accepts(index)` tells
    /// whether a card takes drops and `message(RowDrop)` asks the application to move the cards.
    ///
    /// A drag carries the pressed card, or the whole [selection](Self::multi_select) when it is
    /// pressed on one of its cards; a click on a selected card without a drag makes it the one
    /// selected card on release. The card under the pointer takes the accent tone while it can
    /// take the drag. A release anywhere else, or on one of the dragged cards, does nothing. With
    /// [`Click::Single`] a card activates on release rather than on press, so pressing a card to
    /// drag it does not activate it.
    #[must_use]
    pub fn droppable(
        mut self,
        message: impl Fn(RowDrop) -> Msg + 'static,
        accepts: impl Fn(usize) -> bool + 'static,
    ) -> Self {
        self.picking.dropping = Some((Box::new(message), Box::new(accepts)));
        self
    }

    /// A drop released with Ctrl held asks for a copy with `message` instead of the move of
    /// [`droppable`](Self::droppable), the way a file explorer copies. A terminal that does not
    /// report Ctrl with the pointer always moves. It does nothing without `droppable`.
    #[must_use]
    pub fn on_copy_drop(mut self, message: impl Fn(RowDrop) -> Msg + 'static) -> Self {
        self.picking.copy_drop = Some(Box::new(message));
        self
    }

    /// Offers `event` to the card menu; see [`context_menu`](Self::context_menu).
    fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        row_menu::event(
            cx,
            event,
            self.menu.as_ref(),
            self.count,
            |cx, x, y| {
                let memory = cx.memory::<GridMemory>();
                let (layout, offset) = (memory.layout, memory.offset);
                let index = layout.index_at(x, y, offset)?;
                if self.picking.is_multi() {
                    if !self.picking.is_chosen(index) {
                        self.picking.select_one(cx, self, index);
                    }
                } else if !self.is_checked(index).unwrap_or(false) {
                    self.select(cx, index);
                }
                Some(RowAnchor { row: index, at: Rect::new(x, y, 1, 1), keyboard: false })
            },
            |cx| {
                let index = self.current(cx)?;
                let memory = cx.memory::<GridMemory>();
                let layout = memory.layout;
                memory.offset = layout.reveal(index, memory.offset);
                let at = layout.card_rect(index, memory.offset);
                Some(RowAnchor { row: index, at, keyboard: true })
            },
        )
    }

    fn active(&self) -> bool {
        !self.disabled && self.count > 0
    }

    fn is_checked(&self, index: usize) -> Option<bool> {
        self.checked.as_ref().map(|checked| checked.get(index).copied().unwrap_or(false))
    }

    fn toggles(&self) -> bool {
        self.checked.is_some() && self.on_toggle.is_some()
    }

    fn sizing(&self, padding: Padding) -> Sizing {
        Sizing {
            min_width: self.min_width,
            max_width: self.max_width,
            height: self.height.saturating_add(padding.vertical()),
            column_gap: self.column_gap,
            row_gap: self.row_gap,
        }
    }

    fn select(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        if Some(index) != self.selected
            && let Some(message) = &self.on_select
        {
            cx.emit(message(index));
        }
    }

    fn activate(&self, cx: &mut EventCx<'_, Msg>, index: usize) -> bool {
        let Some(message) = &self.on_activate else {
            return false;
        };
        cx.memory::<GridMemory>().flashed = Some(index);
        cx.flash();
        cx.emit(message(index));
        true
    }

    fn toggle(&self, cx: &mut EventCx<'_, Msg>, index: usize) -> bool {
        match (&self.checked, &self.on_toggle) {
            (Some(_), Some(message)) => {
                cx.emit(message(index));
                true
            }
            _ => false,
        }
    }

    /// The card whose check mark is at `(x, y)`, when one is.
    fn mark_at(&self, cx: &mut EventCx<'_, Msg>, x: i32, y: i32) -> Option<usize> {
        let memory = cx.memory::<GridMemory>();
        let (layout, offset) = (memory.layout, memory.offset);
        let index = layout.index_at(x, y, offset)?;
        let env = cx.env();
        mark_zone(env, layout.card_rect(index, offset), card_padding(env)).contains(x, y).then_some(index)
    }

    /// The card the keys act on: the one the pointer carries the highlight to, else the
    /// selected one.
    fn current(&self, cx: &mut EventCx<'_, Msg>) -> Option<usize> {
        let memory = cx.memory::<GridMemory>();
        let pointed = memory.pointed.then_some(memory.pointer).flatten();
        pointed.and_then(|(x, y)| memory.layout.index_at(x, y, memory.offset)).or(self.selected)
    }

    fn on_key(&self, cx: &mut EventCx<'_, Msg>, key: &crate::event::KeyEvent) -> bool {
        let steps = [
            (Key::Left, Step::Left),
            (Key::Right, Step::Right),
            (Key::Up, Step::Up),
            (Key::Down, Step::Down),
            (Key::PageUp, Step::PageUp),
            (Key::PageDown, Step::PageDown),
            (Key::Home, Step::Home),
            (Key::End, Step::End),
        ];
        let extend = |cx: &mut EventCx<'_, Msg>, plain: &crate::event::KeyEvent| {
            let step = steps.into_iter().find(|(k, _)| plain.is_plain(*k)).map(|(_, step)| step)?;
            let current = self.current(cx);
            Some(cx.memory::<GridMemory>().layout.step(step, current))
        };
        if self.picking.selection_key(cx, key, self, self.count, extend) {
            cx.memory::<GridMemory>().pointed = false;
            return true;
        }
        let step = steps.into_iter().find(|(k, _)| key.is_plain(*k)).map(|(_, step)| step);
        let activates = key.is_plain(Key::Enter) || key.is_plain(Key::Space);
        if step.is_none() && !activates {
            return false;
        }
        let current = self.current(cx);
        // A key takes the highlight back from a pointer resting on the grid, until it moves.
        cx.memory::<GridMemory>().pointed = false;
        if let Some(step) = step {
            let target = cx.memory::<GridMemory>().layout.step(step, current);
            if let Some(target) = target {
                if self.picking.is_multi() {
                    self.picking.select_one(cx, self, target);
                } else {
                    self.select(cx, target);
                }
            }
            return target.is_some();
        }
        let Some(index) = current else {
            return false;
        };
        self.select(cx, index);
        if key.is_plain(Key::Space) && (self.picking.toggle(cx, self, index) || self.toggle(cx, index)) {
            return true;
        }
        self.activate(cx, index)
    }

    fn on_mouse(&self, cx: &mut EventCx<'_, Msg>, mouse: &crate::event::MouseEvent) -> bool {
        // A press on a card's check mark only toggles it.
        let marked = self.toggles().then(|| self.mark_at(cx, mouse.x, mouse.y)).flatten();
        let memory = cx.memory::<GridMemory>();
        let layout = memory.layout;
        let bar = layout.bar();
        let on_bar = bar.is_some_and(|bar| bar.contains(mouse.x, mouse.y));
        let track = |y: i32| clamp_u16(y - layout.body.y);
        match mouse.kind {
            MouseKind::ScrollUp | MouseKind::ScrollDown => {
                memory.offset = if mouse.kind == MouseKind::ScrollUp {
                    memory.offset.saturating_sub(1)
                } else {
                    (memory.offset + 1).min(layout.max_offset())
                };
                // The wheel is the pointer at work: the card now under it takes the highlight.
                memory.pointed = true;
                true
            }
            MouseKind::Down(MouseButton::Left) if on_bar => {
                memory.dragging = true;
                memory.offset = layout.metrics(memory.offset).offset_at(track(mouse.y), layout.body.height);
                cx.capture_pointer();
                true
            }
            MouseKind::Drag(MouseButton::Left) if memory.dragging => {
                memory.offset = layout.metrics(memory.offset).offset_at(track(mouse.y), layout.body.height);
                true
            }
            MouseKind::Up(MouseButton::Left) if memory.dragging => {
                memory.dragging = false;
                true
            }
            MouseKind::Down(MouseButton::Left) if let Some(index) = marked => self.toggle(cx, index),
            _ => self.picking.mouse(cx, mouse, self).unwrap_or(false),
        }
    }
}

/// The card surface's padding from the theme.
fn card_padding(env: &Env) -> Padding {
    let style = env.theme().style("card", None, &[]);
    style.pair("padding").map_or(Padding::default(), |(vertical, horizontal)| Padding::symmetric(vertical, horizontal))
}

/// Width of the check mark glyph.
fn mark_width(env: &Env) -> u16 {
    text::width(&env.icons().glyph("check")).max(1)
}

/// Where a card's check mark sits: in its top right corner, with a cell of air to its right.
fn mark_cell(env: &Env, card: Rect, padding: Padding) -> Rect {
    let width = mark_width(env);
    Rect::new(card.right() - 1 - i32::from(width), card.y + i32::from(padding.top), width, 1)
}

/// The cells a click toggles the check in: the mark and a cell on each side of it.
fn mark_zone(env: &Env, card: Rect, padding: Padding) -> Rect {
    let mark = mark_cell(env, card, padding);
    Rect::new(mark.x - 1, mark.y, mark.width + 2, 1)
}

impl<Msg: Clone + 'static> CardGrid<Msg> {
    /// What the grid shows when it has no cards, e.g. "No apps match" with a way out. Without it
    /// an empty grid draws nothing. Set the count with [`new`](Self::new) first; a grid with
    /// cards leaves the empty state out.
    #[must_use]
    pub fn empty(mut self, state: EmptyState<Msg>) -> Self {
        if self.count > 0 {
            return self;
        }
        let mut node = Node::new(state, 0);
        node.layout.width = Length::Fill(1);
        node.layout.height = Length::Fill(1);
        self.empty = vec![node];
        self
    }

    /// Paints card `index` in `rect` in `states`.
    fn paint_card(&self, cx: &mut PaintCx<'_>, rect: Rect, index: usize, states: &[State], padding: Padding) {
        let style = cx.style("card", None, states);
        let background = style.text().bg.unwrap_or_else(|| cx.color("raised"));
        cx.clear(rect, background);
        if let Some(pillar) = style.color("pillar") {
            // A card is a whole surface, so its mark runs down its full left edge.
            for row in 0..rect.height {
                cx.pillar(rect.x, rect.y + i32::from(row), pillar);
            }
        }
        let env = cx.env;
        let mut inner = rect.inset(padding);
        if self.checked.is_some() {
            // The mark's corner is kept free on every row, so content never runs under it.
            let reserve = mark_width(env) + 2;
            inner.width = inner.width.saturating_sub(reserve.saturating_sub(padding.right));
        }
        if let Some(build) = &self.card {
            let mut children = Vec::new();
            let screen = Size::new(cx.buf.area.width, cx.buf.area.height);
            let idle = IdleScope::new(cx.idle);
            build(&mut View::new(&mut children, env, screen, &idle), index);
            // Keyed by the card's index, so what a card's widgets remember follows the card.
            let mut node = Node::new(Flex::new(Axis::Column, children), index);
            node.layout.width = Length::Fill(1);
            node.assign_ids(cx.id());
            cx.paint_child(&node, inner);
        }
        if let Some(checked) = self.is_checked(index) {
            let lit = states.iter().any(|state| matches!(state, State::Hover | State::Selected));
            if checked || (lit && self.on_toggle.is_some() && !self.disabled) {
                let variant = (!checked).then_some("off");
                let fg = cx.style("card-mark", variant, &[]).text().fg.unwrap_or_else(|| cx.color("accent"));
                let mark = mark_cell(env, rect, padding);
                cx.text(mark.x, mark.y, &env.icons().glyph("check"), CellStyle::fg(fg), mark.width);
            }
        }
        if self.disabled {
            cx.tint(rect, background, 0.5);
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for CardGrid<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.count == 0 {
            return self.empty.first().map_or(Size::default(), |empty| cx.measure_child(empty, available));
        }
        let sizing = self.sizing(card_padding(cx.env()));
        let (columns, _) = layout::columns(available.width, sizing);
        let height = layout::height_of(self.count.div_ceil(columns), sizing);
        Size::new(available.width, height).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        if self.count == 0 {
            if let Some(empty) = self.empty.first() {
                cx.paint_child(empty, area);
            }
            return;
        }
        cx.register_hit(area);
        let padding = cx.style("card", None, &[]).padding();
        let layout = Layout::new(area, self.count, self.sizing(padding));
        let active = self.active();
        let focus_visible = active && cx.is_focus_visible();
        let pressed = cx.is_pressed();
        let pointer = if active { cx.pointer_within() } else { None };
        let (offset, pointed, flashed, dragging) = {
            let memory = cx.memory::<GridMemory>();
            if pointer != memory.pointer {
                // Only a moving pointer takes the highlight; one resting where the keys left it
                // does not pull it back. Leaving the grid hands it back to the selection.
                memory.pointer = pointer;
                memory.pointed = pointer.is_some();
            }
            let follow = (self.selected, layout.columns);
            if memory.followed != Some(follow) {
                if let Some(selected) = self.selected.filter(|index| *index < self.count) {
                    memory.offset = layout.reveal(selected, memory.offset);
                }
                memory.followed = Some(follow);
            }
            memory.offset = memory.offset.min(layout.max_offset());
            memory.layout = layout;
            (memory.offset, memory.pointed && pointer.is_some(), memory.flashed, memory.dragging)
        };
        // An open card menu takes the pointer: only the card it acts on stays raised, so the menu
        // and the card it belongs to are read together.
        let menu_card = row_menu::open_row(cx, self.menu.as_ref());
        if menu_card.is_some() {
            cx.request_overlay(area);
        }
        let hovered = match menu_card {
            Some(card) => Some(card),
            None => pointer.filter(|_| pointed).and_then(|(x, y)| layout.index_at(x, y, offset)),
        };

        // The card a drag is over takes the accent tone when it can take what is dragged.
        let target = row_pointer::dragged(cx).and_then(|((x, y), carried)| {
            layout.index_at(x, y, offset).filter(|index| self.picking.takes_drop(&carried, *index))
        });
        let drawn = row_pointer::drawn_box(cx);
        cx.with_clip(layout.body, |cx| {
            for index in layout.shown(offset) {
                let rect = layout.card_rect(index, offset);
                let is_hovered = hovered == Some(index);
                // While the pointer carries the highlight, the selected card rests unless it is
                // the one under the pointer.
                let cursor = self.selected == Some(index) && (!pointed || is_hovered);
                // With several selected, the selected cards take the selected surface and the
                // card the keys are on is raised like a hovered one when it is not among them,
                // so the keys still show where they start.
                let (lit, raised) = if self.picking.is_multi() {
                    let chosen = self.picking.is_chosen(index);
                    (chosen, is_hovered || (cursor && !chosen))
                } else {
                    (cursor, is_hovered)
                };
                let mut states = Vec::new();
                if raised {
                    states.push(State::Hover);
                }
                if lit {
                    states.push(State::Selected);
                    if focus_visible && (cursor || !self.picking.is_multi()) {
                        states.push(State::Focus);
                    }
                }
                if pressed && flashed == Some(index) {
                    states.push(State::Pressed);
                }
                self.paint_card(cx, rect, index, &states, padding);
                if target == Some(index) {
                    // The card keeps what it shows and takes the tone of a place that takes a drop.
                    let drop = cx.style("tree-drop", None, &[]).text();
                    let bg = drop.bg.unwrap_or_else(|| cx.color("accent"));
                    let readable = drop.fg.unwrap_or_else(|| cx.color("text"));
                    cx.fill_keeping_text_readable(rect, bg, readable);
                }
            }
            if let Some(drawn) = drawn {
                select_box::paint(cx, drawn, layout.body);
            }
        });

        if let Some(bar) = layout.bar() {
            let lit = dragging || pointer.is_some_and(|(x, y)| bar.contains(x, y));
            scrollbar::paint(cx, bar, layout.metrics(offset), lit, self.scrollbar);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        row_menu::paint(cx, self.menu.as_ref(), anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        if self.menu_event(cx, event) {
            return true;
        }
        match event {
            Event::Key(key) => self.on_key(cx, key),
            Event::Mouse(mouse) => self.on_mouse(cx, mouse),
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.active()
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.empty
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.empty
    }
}

impl<Msg: 'static> PickedRows<Msg> for CardGrid<Msg> {
    fn spot(&self, cx: &mut EventCx<'_, Msg>, x: i32, y: i32) -> Spot {
        let memory = cx.memory::<GridMemory>();
        if !memory.layout.body.contains(x, y) {
            return Spot::Outside;
        }
        memory.layout.index_at(x, y, memory.offset).map_or(Spot::Free, Spot::Row)
    }

    fn covered(&self, cx: &mut EventCx<'_, Msg>, rect: Rect) -> Vec<usize> {
        let memory = cx.memory::<GridMemory>();
        let (layout, offset) = (memory.layout, memory.offset);
        layout.shown(offset).filter(|index| !layout.card_rect(*index, offset).intersect(rect).is_empty()).collect()
    }

    fn cursor(&self) -> Option<usize> {
        self.selected
    }

    fn select(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        CardGrid::select(self, cx, index);
    }

    fn open(&self, cx: &mut EventCx<'_, Msg>, index: usize, _at: (i32, i32)) {
        self.activate(cx, index);
    }
}
