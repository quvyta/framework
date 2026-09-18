//! A short list of choices unfolding as a layer under a widget, such as the hidden tabs of a tab
//! strip or the collapsed middle of a breadcrumb, and the option list it shares with the open
//! list of a [`Select`](super::Select).
//!
//! The owning widget keeps the state in its memory, asks for an overlay while it is open and
//! passes its events here first.

use std::time::Duration;

use crate::event::{Event, MouseButton, MouseEvent, MouseKind};
use crate::geometry::{Rect, clamp_u16};
use crate::keymap::Key;
use crate::motion::Easing;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, PaintCx};

use super::layer::{BarDrag, PointerGate};
use super::placement::{self, Placement};
use super::scrollbar::{self, ScrollMetrics};

/// Rows shown before the list scrolls.
const MAX_ROWS: usize = 8;

/// Cells a row keeps beside its label: the pillar and a space before it, the spare cell the label
/// slides into, and the check column after it.
const ROW_CHROME: u16 = 2 + 3 + 1;

/// The style keys of an option list: the surface, the rows and the check of the current option.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OptionStyles {
    /// The surface (`bg`), such as `select-menu`.
    pub(crate) menu: &'static str,
    /// A row with `hover` and `checked` (`bg`, `fg`, `pillar`), such as `select-option`.
    pub(crate) item: &'static str,
    /// The check of the current option, such as `select-check`.
    pub(crate) check: &'static str,
}

/// The highlight, scroll position, pointer gate and scrollbar of a list of options unfolding in
/// a layer. The keyboard and the pointer move the one highlight.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct OptionList {
    /// The highlighted option.
    pub(crate) highlight: usize,
    /// The first option shown.
    pub(crate) offset: usize,
    /// The pointer moves the highlight only once it moves after the list opened.
    pointer: PointerGate,
    bar: BarDrag,
}

impl OptionList {
    /// A list opening with `highlight` highlighted and in view among `visible` rows, while the
    /// pointer rests at `pointer`.
    pub(crate) fn open(highlight: usize, visible: usize, pointer: Option<(i32, i32)>) -> Self {
        Self {
            highlight,
            offset: highlight.saturating_sub(visible.saturating_sub(1)),
            pointer: PointerGate::new(pointer),
            bar: BarDrag::default(),
        }
    }

    /// Keeps the highlight and the scroll position inside `len` options of which `visible` show;
    /// the application may have removed options since the highlight last moved.
    pub(crate) fn clamp(&mut self, len: usize, visible: usize) {
        self.highlight = self.highlight.min(len.saturating_sub(1));
        self.offset = self.offset.min(len.saturating_sub(visible));
    }

    /// Moves the highlight to `target`, kept inside `len` options, and scrolls it into the
    /// `visible` rows.
    pub(crate) fn move_to(&mut self, target: usize, len: usize, visible: usize) {
        self.highlight = target.min(len.saturating_sub(1));
        if self.highlight < self.offset {
            self.offset = self.highlight;
        } else if self.highlight >= self.offset + visible {
            self.offset = self.highlight + 1 - visible;
        }
    }

    /// Scrolls one row up or down within `len` options of which `visible` show.
    pub(crate) fn scroll(&mut self, up: bool, len: usize, visible: usize) {
        self.offset =
            if up { self.offset.saturating_sub(1) } else { (self.offset + 1).min(len.saturating_sub(visible)) };
    }

    /// Offers a pointer event to the scrollbar of `len` options of which `visible` show. Returns
    /// whether the bar used it.
    pub(crate) fn bar_event<Msg>(
        &mut self,
        cx: &mut EventCx<'_, Msg>,
        mouse: &MouseEvent,
        len: usize,
        visible: usize,
    ) -> bool {
        let metrics = ScrollMetrics { total: len, visible, offset: self.offset };
        let Some(offset) = self.bar.event(cx, mouse, metrics) else {
            return false;
        };
        self.offset = offset.min(metrics.max_offset());
        true
    }

    /// Paints `labels` into `popup`, the part shown so far of a list `full_height` rows tall, on
    /// the `menu` surface. `current` is marked with a check. Rows show the pillar and slide on
    /// the highlight, which the pointer carries once it moves; the scrollbar shows from the start
    /// when the unfolded list scrolls and never when it fits.
    pub(crate) fn paint(
        &mut self,
        cx: &mut PaintCx<'_>,
        popup: Rect,
        full_height: u16,
        labels: &[String],
        current: Option<usize>,
        styles: OptionStyles,
    ) {
        let menu = cx.style(styles.menu, None, &[]).text();
        let background = menu.bg.unwrap_or_else(|| cx.color("overlay"));
        let grounds = cx.grounds_around(popup);
        cx.clear(popup, background);
        cx.register_hit(popup);
        let slide = cx.env().slide();
        let check = cx.env().icons().glyph("check").into_owned();
        let metrics = ScrollMetrics { total: labels.len(), visible: usize::from(popup.height), offset: self.offset };
        // Whether the list scrolls is decided by its unfolded height, so a list that fits never
        // flashes a scrollbar while it opens, and one that does not shows it from the start.
        let overflows = labels.len() > usize::from(full_height);
        let content_width = if overflows { popup.width.saturating_sub(1) } else { popup.width };
        // The scrollbar column is not a row: dragging it must not move the highlight.
        let hovered = cx
            .pointer()
            .filter(|(x, y)| popup.contains(*x, *y) && *x < popup.x + i32::from(content_width))
            .and_then(|(_, y)| usize::try_from(y - popup.y).ok())
            .map(|row| self.offset + row)
            .filter(|index| *index < labels.len());
        let anywhere = cx.pointer_anywhere();
        // A pointer resting where the list unfolds, or on a row while the keyboard moves on,
        // waits until it moves.
        let moved = self.pointer.moved(anywhere);
        if let Some(index) = hovered.filter(|_| moved) {
            self.highlight = index;
        }
        for (row, index) in (self.offset..labels.len()).take(usize::from(popup.height)).enumerate() {
            let rect = Rect::new(popup.x, popup.y + i32::try_from(row).unwrap_or(0), content_width, 1);
            let mut states = Vec::new();
            if index == self.highlight {
                states.push(State::Hover);
            }
            if Some(index) == current {
                states.push(State::Checked);
            }
            let style = cx.style(styles.item, None, &states);
            let text_style = style.text();
            if let Some(bg) = text_style.bg {
                cx.fill(rect, bg);
            }
            if let Some(color) = style.color("pillar") {
                cx.pillar(rect.x, rect.y, color);
            }
            let shift = u16::from(slide && states.contains(&State::Hover));
            let budget = content_width.saturating_sub(ROW_CHROME);
            // Cut at the same place resting and raised: the slide moves the label into its spare cell.
            let label = text::truncate(&labels[index], budget).into_owned();
            cx.text(rect.x + 2 + i32::from(shift), rect.y, &label, CellStyle { bg: None, ..text_style }, budget);
            if Some(index) == current {
                let check_style = cx.style(styles.check, None, &states).text();
                cx.text(rect.right() - 2, rect.y, &check, check_style, 1);
            }
        }
        let bar = Rect::new(popup.right() - 1, popup.y, 1, popup.height);
        self.bar.place(overflows.then_some(bar));
        if overflows {
            let active = self.bar.active(anywhere);
            scrollbar::paint(cx, bar, metrics, active, None);
        }
        cx.stand_apart(popup, &grounds, Some(background));
    }
}

/// Open state and list of a popup menu.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PopupMenu {
    open: bool,
    opened_at: Duration,
    rect: Rect,
    /// The control the popup unfolds from; a press on it only closes the popup.
    anchor: Rect,
    list: OptionList,
}

/// What an event meant to an open popup menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PopupAction {
    /// Not for the popup; the widget may handle it.
    Ignored,
    /// Used by the popup, e.g. moving the highlight.
    Used,
    /// Choice `index` was taken; the popup closed.
    Chosen(usize),
    /// The popup closed without a choice.
    Closed,
}

/// The popup menu's style keys.
const POPUP_STYLES: OptionStyles = OptionStyles { menu: "popup-menu", item: "popup-item", check: "popup-check" };

impl PopupMenu {
    /// Whether the popup of the widget handling `cx` is open.
    pub(crate) fn is_open<Msg>(cx: &mut EventCx<'_, Msg>) -> bool {
        cx.memory::<Self>().open
    }

    /// Whether the popup of the widget being painted is open.
    pub(crate) fn is_open_paint(cx: &mut PaintCx<'_>) -> bool {
        cx.memory::<Self>().open
    }

    /// Opens the popup with row `highlight` highlighted and takes the keyboard.
    pub(crate) fn open<Msg>(cx: &mut EventCx<'_, Msg>, highlight: usize) {
        let now = cx.now();
        let pointer = cx.interaction.pointer;
        let memory = cx.memory::<Self>();
        memory.open = true;
        memory.opened_at = now;
        memory.list = OptionList::open(highlight, MAX_ROWS, pointer);
        cx.capture_keys(true);
    }

    /// Closes the popup and gives the keyboard back.
    pub(crate) fn close<Msg>(cx: &mut EventCx<'_, Msg>) {
        cx.memory::<Self>().open = false;
        cx.capture_keys(false);
    }

    fn visible(len: usize) -> usize {
        len.min(MAX_ROWS)
    }

    /// Handles `event` while the popup is open; `labels` are the choices.
    pub(crate) fn event<Msg>(cx: &mut EventCx<'_, Msg>, event: &Event, labels: &[String]) -> PopupAction {
        let len = labels.len();
        let mut state = *cx.memory::<Self>();
        if !state.open || len == 0 {
            return PopupAction::Ignored;
        }
        let visible = Self::visible(len);
        let list = &mut state.list;
        list.clamp(len, visible);
        let action = match event {
            Event::PointerOutside => PopupAction::Closed,
            Event::Key(key) if key.is_plain(Key::Esc) => PopupAction::Closed,
            Event::Key(key) if key.is_plain(Key::Tab) => {
                Self::close(cx);
                return PopupAction::Ignored;
            }
            Event::Key(key) if key.is_plain(Key::Enter) || key.is_plain(Key::Space) => {
                PopupAction::Chosen(list.highlight)
            }
            Event::Key(key) => {
                let last = len - 1;
                if key.is_plain(Key::Up) {
                    list.move_to(list.highlight.saturating_sub(1), len, visible);
                } else if key.is_plain(Key::Down) {
                    list.move_to((list.highlight + 1).min(last), len, visible);
                } else if key.is_plain(Key::Home) {
                    list.move_to(0, len, visible);
                } else if key.is_plain(Key::End) {
                    list.move_to(last, len, visible);
                } else if let (Some(typed), false) = (key.text, key.chord.mods.ctrl || key.chord.mods.alt)
                    && let Some(next) = type_ahead(labels, list.highlight, typed)
                {
                    list.move_to(next, len, visible);
                }
                PopupAction::Used
            }
            Event::Mouse(mouse) if list.bar_event(cx, mouse, len, visible) => PopupAction::Used,
            Event::Mouse(mouse) => {
                let inside = state.rect.contains(mouse.x, mouse.y);
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) if inside => {
                        let row = usize::try_from(mouse.y - state.rect.y).unwrap_or(0);
                        PopupAction::Chosen((list.offset + row).min(len - 1))
                    }
                    MouseKind::ScrollUp | MouseKind::ScrollDown if inside => {
                        list.scroll(mouse.kind == MouseKind::ScrollUp, len, visible);
                        PopupAction::Used
                    }
                    // A press on the control the popup belongs to only closes it; a press anywhere
                    // else in the owning widget closes it and the widget still handles the press.
                    MouseKind::Down(_) if state.anchor.contains(mouse.x, mouse.y) => PopupAction::Closed,
                    MouseKind::Down(_) if !inside => {
                        Self::close(cx);
                        return PopupAction::Ignored;
                    }
                    _ if inside => PopupAction::Used,
                    _ => PopupAction::Ignored,
                }
            }
            Event::Paste(_) => PopupAction::Used,
        };
        *cx.memory::<Self>() = state;
        if matches!(action, PopupAction::Chosen(_) | PopupAction::Closed) {
            Self::close(cx);
        }
        if matches!(action, PopupAction::Chosen(_)) {
            cx.flash();
        }
        action
    }

    /// Paints the open popup under `anchor` (above it when there is no room below). `current`
    /// is marked with a check. The pointer, once it moves, carries the one highlight.
    pub(crate) fn paint(cx: &mut PaintCx<'_>, anchor: Rect, labels: &[String], current: Option<usize>) {
        let state = *cx.memory::<Self>();
        if !state.open || labels.is_empty() {
            return;
        }
        let screen = cx.clip();
        let longest = labels.iter().map(|label| text::width(label)).max().unwrap_or(0);
        let width = longest.saturating_add(7).max(anchor.width).max(14).min(screen.width);
        let height = clamp_u16(i32::try_from(Self::visible(labels.len())).unwrap_or(i32::MAX));
        let x = (anchor.right() - i32::from(width)).max(anchor.x.min(screen.right() - i32::from(width))).max(screen.x);
        let below = anchor.bottom() + i32::from(height) <= screen.bottom();
        let (y, side) = if below {
            (anchor.bottom(), Placement::Below)
        } else {
            ((anchor.y - i32::from(height)).max(screen.y), Placement::Above)
        };

        // The list unfolds from the anchor over the theme's `motion.enter`.
        let enter = cx.env().theme().motion().enter;
        let shown = cx.progress_since(state.opened_at, enter, Easing::EaseOut);
        let popup = placement::unfold(Rect::new(x, y, width, height), side, shown);
        let mut list = state.list;
        list.clamp(labels.len(), usize::from(height));
        list.paint(cx, popup, height, labels, current, POPUP_STYLES);
        let memory = cx.memory::<Self>();
        memory.rect = popup;
        memory.anchor = anchor;
        memory.list = list;
    }
}

/// The next label after `from` starting with `typed`, wrapping around; case is ignored.
pub(crate) fn type_ahead(labels: &[String], from: usize, typed: char) -> Option<usize> {
    type_ahead_by(labels.len(), from, typed, |index| Some(&labels[index]))
}

/// The next index after `from` among `len` rows whose label starts with `typed`, wrapping
/// around; case is ignored. `label` gives a row's label, or `None` for a row that cannot be
/// highlighted.
pub(crate) fn type_ahead_by<'a>(
    len: usize,
    from: usize,
    typed: char,
    label: impl Fn(usize) -> Option<&'a str>,
) -> Option<usize> {
    let typed: String = typed.to_lowercase().collect();
    (1..=len)
        .map(|step| (from + step) % len)
        .find(|index| label(*index).is_some_and(|label| label.to_lowercase().starts_with(&typed)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};
    use crate::widgets::Breadcrumb;

    /// A breadcrumb whose collapsed middle opens a popup menu.
    struct Path {
        segments: Vec<String>,
        opened: Vec<usize>,
    }

    impl App for Path {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.opened.push(index);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(Breadcrumb::new(self.segments.clone()).on_select(|i| i)).width(Length::Cells(30)).id("path");
        }
    }

    /// A path of `levels` segments; all but the first and the last two collapse at 30 cells.
    fn path(levels: usize) -> Harness<Path> {
        let mut segments = vec!["workspace".to_owned()];
        segments.extend((1..levels - 2).map(|i| format!("level-{i}")));
        segments.extend(["src".to_owned(), "widgets".to_owned()]);
        Harness::new(Path { segments, opened: Vec::new() }, 30, 16)
    }

    /// Whether a scrollbar shows beside the popup rows, drawn with glyphs or with colours only:
    /// any cell on those rows whose background is neither a row's own nor the canvas.
    fn scrollbar(h: &Harness<Path>) -> bool {
        let screen = h.screen();
        if screen.lines().skip(1).any(|line| line.contains('▐') || line.contains('▕')) {
            return true;
        }
        let canvas = h.env().theme().color("canvas");
        let rows: Vec<(u16, u16)> = screen
            .lines()
            .enumerate()
            .skip(1)
            .filter_map(|(y, line)| {
                let x = line.find("level-").or_else(|| line.find("src"))?;
                Some((u16::try_from(line[..x].chars().count()).ok()?, u16::try_from(y).ok()?))
            })
            .collect();
        rows.iter().any(|(label, y)| {
            let own = h.bg(*label, *y);
            (0..30).any(|x| h.bg(x, *y) != own && h.bg(x, *y) != canvas)
        })
    }

    fn lit_rows(h: &Harness<Path>) -> Vec<String> {
        h.screen()
            .lines()
            .skip(1)
            .filter(|line| line.contains('▌'))
            .map(|line| line.replace('▌', "").trim().to_owned())
            .collect()
    }

    #[test]
    fn the_scrollbar_is_decided_by_the_unfolded_height() {
        let enter = Duration::from_millis(150);
        let mut fits = path(6);
        fits.click_text("…").advance(enter / 2);
        assert!(fits.screen().contains("level-1"), "half unfolded:\n{}", fits.screen());
        assert!(!scrollbar(&fits), "a list that fits never shows one while unfolding:\n{}", fits.screen());
        let mut scrolls = path(14);
        scrolls.click_text("…");
        for _ in 0..6 {
            assert!(scrolls.screen().contains("level-"), "{}", scrolls.screen());
            assert!(scrollbar(&scrolls), "a list that scrolls shows one from the start:\n{}", scrolls.screen());
            fits.advance(Duration::from_millis(30));
            assert!(!scrollbar(&fits), "{}", fits.screen());
            scrolls.advance(Duration::from_millis(30));
        }
        // Without glyphs the scrollbar is colour alone, and it is still found.
        scrolls.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert!(scrollbar(&scrolls), "{}", scrolls.screen());
        assert!(!scrolls.screen().contains('▐') && !scrolls.screen().contains('▕'));
    }

    #[test]
    fn the_pointer_moves_the_one_highlight() {
        let mut h = path(7);
        h.click_text("…").advance(Duration::from_millis(300));
        h.press("home");
        assert_eq!(lit_rows(&h), ["level-1"], "the keyboard highlights the first row:\n{}", h.screen());
        let (x, y) = h.find("level-3").expect("third row");
        h.hover(x, y);
        assert_eq!(lit_rows(&h), ["level-3"], "only the hovered row is lit:\n{}", h.screen());
        h.press("down").press("enter");
        assert_eq!(h.app().opened, [4], "the keyboard continues from the hovered row");
    }

    #[test]
    fn a_pointer_resting_where_the_list_unfolds_does_not_take_the_highlight() {
        let mut h = path(7);
        h.click_text("…").advance(Duration::from_millis(300));
        let (x, y) = h.find("level-1").expect("first row");
        h.press("esc").hover(x, y).press("home").press("right").press("enter").advance(Duration::from_millis(300));
        assert!(h.screen().contains("level-1"), "reopened from the keyboard:\n{}", h.screen());
        h.press("home");
        assert_eq!(lit_rows(&h), ["level-1"], "{}", h.screen());
        h.press("end");
        assert_eq!(lit_rows(&h), ["src"], "the resting pointer waits:\n{}", h.screen());
        h.hover(x + 1, y);
        assert_eq!(lit_rows(&h), ["level-1"], "once it moves it carries the highlight:\n{}", h.screen());
    }

    #[test]
    fn a_press_on_the_opening_control_only_closes_and_a_press_elsewhere_still_acts() {
        let mut h = path(6);
        h.click_text("…").advance(Duration::from_millis(300));
        h.click_text("…").advance(Duration::from_millis(300));
        assert!(!h.screen().contains("level-1"), "closed and not reopened:\n{}", h.screen());
        h.click_text("…").advance(Duration::from_millis(300));
        h.click_text("workspace");
        assert!(!h.screen().contains("level-1"), "{}", h.screen());
        assert_eq!(h.app().opened, [0], "the same press opened the first segment");
    }

    #[test]
    fn the_scrollbar_can_be_dragged() {
        let mut h = path(14);
        h.click_text("…").advance(Duration::from_millis(300));
        let (_, y) = h.find("level-1").expect("first row");
        // The bar is the popup's last column: the rightmost cell of the row off the canvas.
        let canvas = h.env().theme().color("canvas");
        let row = u16::try_from(y).expect("row");
        let bar = (0..30u16).rev().find(|column| h.bg(*column, row) != canvas).map(i32::from).expect("popup");
        h.mouse(crate::event::MouseKind::Down(crate::event::MouseButton::Left), bar, y);
        h.mouse(crate::event::MouseKind::Drag(crate::event::MouseButton::Left), bar, y + 20);
        h.mouse(crate::event::MouseKind::Up(crate::event::MouseButton::Left), bar, y + 20);
        let screen = h.screen();
        assert!(screen.contains("level-11") && !screen.contains("level-1\n"), "{screen}");
        assert!(h.app().opened.is_empty(), "nothing was chosen");
    }

    /// A breadcrumb whose path the application can shorten while its popup is open.
    struct Shrinking(Path);

    impl App for Shrinking {
        type Msg = Option<usize>;
        fn update(&mut self, msg: Option<usize>) -> Command<Option<usize>> {
            match msg {
                Some(index) => self.0.opened.push(index),
                None => self.0.segments.drain(2..6).for_each(drop),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Option<usize>>) {
            ui.add(Breadcrumb::new(self.0.segments.clone()).on_select(Some)).width(Length::Cells(30));
        }
    }

    #[test]
    fn rows_removed_while_the_popup_is_open_are_never_chosen() {
        let path = path(10).app().segments.clone();
        let mut h = Harness::new(Shrinking(Path { segments: path, opened: Vec::new() }), 30, 16);
        h.set_reduced_motion(true).click_text("…").press("end");
        assert!(h.screen().contains("level-7"), "{}", h.screen());
        h.send(None).press("enter");
        assert_eq!(h.app().0.opened, [4], "the highlight moved back onto the last row left, `src`");
    }

    #[test]
    fn a_raised_row_is_cut_at_the_same_place_as_a_resting_one() {
        let segments =
            ["workspace", "a-very-long-directory-name-here", "b-very-long-directory-name-here", "src", "widgets"];
        let mut h = Harness::new(Path { segments: segments.map(str::to_owned).to_vec(), opened: Vec::new() }, 30, 16);
        h.set_reduced_motion(true);
        h.click_text("…");
        let row = |h: &Harness<Path>| {
            let screen = h.screen();
            screen.lines().find(|line| line.contains("a-very")).unwrap_or_default().replace('▌', " ").trim().to_owned()
        };
        h.press("end");
        let resting = row(&h);
        h.press("home");
        let raised = row(&h);
        assert!(resting.ends_with('…'), "{}", h.screen());
        assert_eq!(raised, resting, "{}", h.screen());
    }
}
