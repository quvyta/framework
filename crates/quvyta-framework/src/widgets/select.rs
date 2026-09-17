//! Dropdown selection.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, Modifiers};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::IndexMessage;
use super::cells;
use super::placement::{self, Placement};
use super::popup_menu::{OptionList, OptionStyles, type_ahead};

/// A field showing the chosen option that opens a list of options as a layer.
///
/// Closed: Enter, Space, ↓ or a click opens it. Open: ↑/↓, Home/End and PgUp/PgDn move,
/// typing a letter jumps to the next option starting with it, Enter or Space chooses, Esc or a
/// click elsewhere closes. A click elsewhere still reaches what it landed on, so one click on
/// another dropdown opens that one; a click on this field while open only closes it. The pointer
/// moves the one highlight once it moves. Style keys: `select` with `hover`, `focus`, `active` (open),
/// `disabled`; `select-placeholder`, `select-chevron`, `select-menu` (`bg`) and
/// `select-option` with `hover`, `selected`, `checked`.
pub struct Select<Msg> {
    options: Vec<String>,
    selected: Option<usize>,
    placeholder: String,
    disabled: bool,
    max_visible: usize,
    on_select: Option<IndexMessage<Msg>>,
}

#[derive(Debug, Default)]
struct SelectMemory {
    open: bool,
    opened_at: std::time::Duration,
    popup: Rect,
    list: OptionList,
}

/// The option list's style keys.
const OPTION_STYLES: OptionStyles = OptionStyles { menu: "select-menu", item: "select-option", check: "select-check" };

impl<Msg: 'static> Select<Msg> {
    /// A dropdown of `options`.
    #[must_use]
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: None,
            placeholder: String::new(),
            disabled: false,
            max_visible: 8,
            on_select: None,
        }
    }

    /// The chosen option.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Faint text shown while nothing is chosen.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Greys the field out; it cannot be opened.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Rows shown before the option list scrolls; 8 by default.
    #[must_use]
    pub fn max_visible(mut self, rows: usize) -> Self {
        self.max_visible = rows.max(1);
        self
    }

    /// Message for choosing option `index`.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    fn visible_rows(&self) -> usize {
        self.options.len().min(self.max_visible)
    }

    fn open(&self, cx: &mut EventCx<'_, Msg>) {
        let highlight = self.selected.unwrap_or(0).min(self.options.len().saturating_sub(1));
        let list = OptionList::open(highlight, self.visible_rows(), cx.interaction.pointer);
        let now = cx.now();
        let memory = cx.memory::<SelectMemory>();
        memory.open = true;
        memory.opened_at = now;
        memory.list = list;
        cx.capture_keys(true);
    }

    fn close(cx: &mut EventCx<'_, Msg>) {
        cx.memory::<SelectMemory>().open = false;
        cx.capture_keys(false);
    }

    fn choose(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        Self::close(cx);
        cx.flash();
        if Some(index) != self.selected
            && let Some(message) = &self.on_select
        {
            cx.emit(message(index));
        }
    }

    fn move_highlight(&self, cx: &mut EventCx<'_, Msg>, target: usize) {
        let (len, visible) = (self.options.len(), self.visible_rows());
        cx.memory::<SelectMemory>().list.move_to(target, len, visible);
    }

    fn popup_rect(&self, cx: &PaintCx<'_>, anchor: Rect) -> (Rect, Placement) {
        let screen = cx.clip();
        let longest = self.options.iter().map(|option| text::width(option)).max().unwrap_or(0);
        let width = anchor.width.max(longest.saturating_add(6));
        let height = clamp_u16(i32::try_from(self.visible_rows()).unwrap_or(i32::MAX));
        placement::place(anchor, Size::new(width, height), screen, Placement::Below)
    }
}

/// Paints a closed dropdown field in `states`: the `select` surface, the pillar in its left padding
/// while hovered, focused from the keyboard or open, and the chevron at the right. The field is
/// not a list, so its label never slides; the options of the open list do. `label` is the chosen
/// text; without one the placeholder shows. Shared by every dropdown field, such as [`Select`] and
/// the date picker.
pub(crate) fn paint_field(cx: &mut PaintCx<'_>, area: Rect, states: &[State], label: Option<&str>, placeholder: &str) {
    let style = cx.style("select", None, states);
    let surface = style.text();
    cx.clear(area, surface.bg.unwrap_or_else(|| cx.color("raised")));
    let padding = style.padding();
    let inner = area.inset(padding);
    let pillar = style.color("pillar").filter(|_| padding.left >= 1);
    if let Some(color) = pillar {
        cx.pillar(area.x, inner.y, color);
    }
    let chevron = cx.env().icons().glyph("chevron-down").into_owned();
    let chevron_style = cx.style("select-chevron", None, states).text();
    let chevron_width = text::width(&chevron);
    cx.text(inner.right() - i32::from(chevron_width), inner.y, &chevron, chevron_style, chevron_width);
    let budget = inner.width.saturating_sub(chevron_width + 2);
    let (shown, text_style) = match label {
        Some(label) => (label, CellStyle { bg: None, ..surface }),
        None => (placeholder, cx.style("select-placeholder", None, states).text()),
    };
    let shown = text::truncate(shown, budget).into_owned();
    cx.text(inner.x, inner.y, &shown, text_style, budget);
}

impl<Msg: 'static> Widget<Msg> for Select<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let style = cx.env().theme().style("select", None, &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((0, 1));
        let longest =
            self.options.iter().map(|o| text::width(o)).chain([text::width(&self.placeholder)]).max().unwrap_or(0);
        Size::new(cells::sum([longest, 3, horizontal.saturating_mul(2)]), vertical.saturating_mul(2).saturating_add(1))
            .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let open = cx.memory::<SelectMemory>().open;
        let mut states = if self.disabled { vec![State::Disabled] } else { cx.pressable_states() };
        if open {
            states.push(State::Active);
        }
        let label = self.selected.and_then(|i| self.options.get(i)).map(String::as_str);
        paint_field(cx, area, &states, label, &self.placeholder);
        if !self.disabled {
            cx.register_hit(area);
        }
        if open && !self.disabled {
            cx.request_overlay(area);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let (full, side) = self.popup_rect(cx, anchor);
        // The list unfolds from the field over the theme's `motion.enter`.
        let opened_at = cx.memory::<SelectMemory>().opened_at;
        let enter = cx.env().theme().motion().enter;
        let progress = cx.progress_since(opened_at, enter, crate::motion::Easing::EaseOut);
        let popup = placement::unfold(full, side, progress);
        let mut list = cx.memory::<SelectMemory>().list;
        // The application may have removed options while the list was open.
        list.clamp(self.options.len(), usize::from(full.height));
        list.paint(cx, popup, full.height, &self.options, self.selected, OPTION_STYLES);
        let memory = cx.memory::<SelectMemory>();
        memory.popup = popup;
        memory.list = list;
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.disabled || self.options.is_empty() {
            return false;
        }
        let open = cx.memory::<SelectMemory>().open;
        match event {
            Event::PointerOutside => {
                Self::close(cx);
                true
            }
            Event::Key(key) if !open => {
                let opens = key.is_plain(Key::Enter) || key.is_plain(Key::Space) || key.is_plain(Key::Down);
                if opens {
                    self.open(cx);
                }
                opens
            }
            Event::Key(key) => {
                let page = self.visible_rows();
                let last = self.options.len() - 1;
                // Events can arrive before the next frame clamps a highlight left by removed options.
                let highlight = cx.memory::<SelectMemory>().list.highlight.min(last);
                if key.is_plain(Key::Esc) {
                    Self::close(cx);
                } else if key.is_plain(Key::Tab) {
                    Self::close(cx);
                    return false;
                } else if key.is_plain(Key::Up) {
                    self.move_highlight(cx, highlight.saturating_sub(1));
                } else if key.is_plain(Key::Down) {
                    self.move_highlight(cx, (highlight + 1).min(last));
                } else if key.is_plain(Key::Home) {
                    self.move_highlight(cx, 0);
                } else if key.is_plain(Key::End) {
                    self.move_highlight(cx, last);
                } else if key.is_plain(Key::PageUp) {
                    self.move_highlight(cx, highlight.saturating_sub(page));
                } else if key.is_plain(Key::PageDown) {
                    self.move_highlight(cx, (highlight + page).min(last));
                } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                    self.choose(cx, highlight);
                } else if let (Some(typed), false) = (key.text, key.chord.mods.ctrl || key.chord.mods.alt) {
                    if let Some(next) = type_ahead(&self.options, highlight, typed) {
                        self.move_highlight(cx, next);
                    }
                } else if key.chord.mods != Modifiers::default() {
                    return false;
                }
                true
            }
            Event::Mouse(mouse) => {
                let (len, visible) = (self.options.len(), self.visible_rows());
                let (popup, mut list) = {
                    let memory = cx.memory::<SelectMemory>();
                    (memory.popup, memory.list)
                };
                if open {
                    let dragged = list.bar_event(cx, mouse, len, visible);
                    cx.memory::<SelectMemory>().list = list;
                    if dragged {
                        return true;
                    }
                }
                let in_popup = open && popup.contains(mouse.x, mouse.y);
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) if in_popup => {
                        let index = list.offset + usize::try_from(mouse.y - popup.y).unwrap_or(0);
                        if index < self.options.len() {
                            self.choose(cx, index);
                        }
                        true
                    }
                    MouseKind::Down(MouseButton::Left) => {
                        if open {
                            Self::close(cx);
                        } else {
                            self.open(cx);
                        }
                        true
                    }
                    MouseKind::ScrollUp | MouseKind::ScrollDown if in_popup => {
                        cx.memory::<SelectMemory>().list.scroll(mouse.kind == MouseKind::ScrollUp, len, visible);
                        true
                    }
                    _ => false,
                }
            }
            Event::Paste(_) => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.disabled && !self.options.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};
    use crate::widgets::Text;

    struct Demo {
        theme: Option<usize>,
    }

    impl App for Demo {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.theme = Some(index);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.column(|ui| {
                ui.add(
                    Select::new(["Monochrome", "Iris", "Nordic", "Amber"])
                        .selected(self.theme)
                        .placeholder("Theme")
                        .max_visible(3)
                        .on_select(|i| i),
                )
                .width(Length::Cells(20))
                .id("theme");
                ui.add(Text::new("below"));
            });
        }
    }

    #[test]
    fn opens_as_layer_and_chooses_by_keyboard() {
        let mut h = Harness::new(Demo { theme: None }, 30, 6);
        assert!(h.screen().starts_with("  Theme"), "{}", h.screen());
        h.press("tab").press("enter");
        let unfolding = h.screen();
        assert!(!unfolding.contains("Nordic"), "the list unfolds over motion.enter: {unfolding}");
        h.advance(std::time::Duration::from_millis(200));
        let screen = h.screen();
        assert!(screen.contains("Monochrome") && screen.contains("Nordic"), "{screen}");
        assert!(!screen.contains("below"), "the layer covers content: {screen}");
        h.press("down").press("down").press("enter");
        assert_eq!(h.app().theme, Some(2));
        assert!(h.screen().contains("below"));
    }

    #[test]
    fn typing_jumps_and_list_scrolls() {
        let mut h = Harness::new(Demo { theme: None }, 30, 6);
        h.press("tab").press("space").press("a").press("enter");
        assert_eq!(h.app().theme, Some(3));
    }

    #[test]
    fn clicks_choose_and_outside_click_closes() {
        let mut h = Harness::new(Demo { theme: Some(0) }, 30, 6);
        h.click_text("Monochrome").advance(std::time::Duration::from_millis(200));
        h.click_text("Iris");
        assert_eq!(h.app().theme, Some(1));
        h.click_text("Iris").advance(std::time::Duration::from_millis(200));
        assert!(h.screen().contains("Nordic"));
        h.click(28, 5);
        assert!(!h.screen().contains("Nordic"));
        assert_eq!(h.app().theme, Some(1));
    }

    /// A select whose four options fit when `max_visible` allows it.
    struct Sized(usize);

    impl App for Sized {
        type Msg = usize;
        fn update(&mut self, _: usize) -> Command<usize> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(
                Select::new(["Monochrome", "Iris", "Nordic", "Amber"])
                    .placeholder("Theme")
                    .max_visible(self.0)
                    .on_select(|i| i),
            )
            .width(Length::Cells(20));
        }
    }

    #[test]
    fn hovering_the_field_raises_the_pillar_and_slides_the_label_but_not_the_chevron() {
        let mut h = Harness::new(Sized(8), 30, 12);
        let resting = h.screen().lines().next().unwrap_or_default().to_owned();
        h.hover(4, 0);
        let hovered = h.screen().lines().next().unwrap_or_default().to_owned();
        assert_eq!(resting, "  Theme          ▾");
        assert_eq!(hovered, "▌ Theme          ▾", "label slides, chevron stays");
    }

    #[test]
    fn the_pointer_moves_the_one_highlight() {
        let mut h = Harness::new(Sized(8), 30, 12);
        h.click_text("Theme");
        h.advance(std::time::Duration::from_millis(300));
        let (x, y) = h.find("Nordic").expect("open list");
        h.hover(x + 3, y);
        let rows: String = h.screen().lines().skip(1).collect::<Vec<_>>().join("\n");
        assert_eq!(rows.matches('▌').count(), 1, "one raised row under the open field:\n{}", h.screen());
        assert!(h.screen().contains("▌  Nordic"), "{}", h.screen());
    }

    #[test]
    fn keys_move_on_from_a_resting_pointer_and_a_pointer_resting_at_opening_waits() {
        let mut h = Harness::new(Sized(8), 30, 12);
        h.click_text("Theme").advance(std::time::Duration::from_millis(300));
        let (x, y) = h.find("Iris").expect("open list");
        h.hover(x, y).press("down");
        let lit =
            |h: &Harness<Sized>| h.screen().lines().skip(1).filter(|l| l.contains('▌')).collect::<Vec<_>>().join("|");
        assert!(lit(&h).contains("Nordic"), "the key moves on from the hovered row: {}", h.screen());
        h.press("esc").press("enter").advance(std::time::Duration::from_millis(300));
        assert!(lit(&h).contains("Monochrome"), "the pointer resting on Iris does not take it: {}", h.screen());
        h.hover(x + 1, y);
        assert!(lit(&h).contains("Iris"), "{}", h.screen());
    }

    #[test]
    fn scrollbar_is_decided_by_the_unfolded_height() {
        let scrollbar = |h: &Harness<Sized>| super::super::scrollbar::column(h, 19).contains('#');
        let mut fits = Harness::new(Sized(8), 30, 12);
        fits.click_text("Theme");
        let mut scrolls = Harness::new(Sized(2), 30, 12);
        scrolls.click_text("Theme");
        for _ in 0..8 {
            assert!(!scrollbar(&fits), "a list that fits never shows one:\n{}", fits.screen());
            assert!(scrollbar(&scrolls), "a list that scrolls shows one from the start:\n{}", scrolls.screen());
            fits.advance(std::time::Duration::from_millis(20));
            scrolls.advance(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn reduced_motion_opens_at_once() {
        let mut h = Harness::new(Demo { theme: None }, 30, 6);
        h.set_reduced_motion(true).press("tab").press("enter");
        assert!(h.screen().contains("Nordic"));
    }

    #[test]
    fn escape_closes_and_tab_moves_on() {
        let mut h = Harness::new(Demo { theme: None }, 30, 6);
        h.press("tab").press("enter").press("esc");
        assert!(!h.screen().contains("Iris"));
        h.press("enter").press("tab");
        assert!(!h.screen().contains("Iris"));
    }

    #[test]
    fn the_scrollbar_of_the_open_list_can_be_dragged() {
        let mut h = Harness::new(Sized(2), 30, 12);
        h.click_text("Theme").advance(std::time::Duration::from_millis(300));
        let (_, y) = h.find("Monochrome").expect("open list");
        h.mouse(MouseKind::Down(MouseButton::Left), 19, y);
        h.mouse(MouseKind::Drag(MouseButton::Left), 19, y + 5);
        h.mouse(MouseKind::Up(MouseButton::Left), 19, y + 5);
        let screen = h.screen();
        assert!(screen.contains("Amber") && !screen.contains("Monochrome"), "{screen}");
        assert!(screen.contains("Nordic"), "still open, nothing chosen: {screen}");
    }

    /// A select whose options the application can shorten while the list is open.
    struct Shrinking {
        options: Vec<&'static str>,
        chosen: Vec<usize>,
    }

    impl App for Shrinking {
        type Msg = Option<usize>;
        fn update(&mut self, msg: Option<usize>) -> Command<Option<usize>> {
            match msg {
                Some(index) => self.chosen.push(index),
                None => self.options.truncate(1),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Option<usize>>) {
            ui.add(Select::new(self.options.clone()).on_select(Some)).width(Length::Cells(20));
        }
    }

    #[test]
    fn options_removed_while_the_list_is_open_are_never_chosen() {
        let mut h = Harness::new(Shrinking { options: vec!["web", "db", "cache"], chosen: Vec::new() }, 30, 8);
        h.set_reduced_motion(true).press("tab").press("enter").press("end");
        h.send(None).press("enter");
        assert_eq!(h.app().chosen, [0], "the highlight moved back onto the one option left");
    }

    /// Long option names in a narrow list.
    struct Long;

    impl App for Long {
        type Msg = usize;
        fn update(&mut self, _: usize) -> Command<usize> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(Select::new(["eu-central-1 Frankfurt", "us-east-1 North Virginia"]).on_select(|i| i))
                .width(Length::Cells(20));
        }
    }

    #[test]
    fn a_raised_option_is_cut_at_the_same_place_as_a_resting_one() {
        let mut h = Harness::new(Long, 22, 6);
        h.set_reduced_motion(true).press("tab").press("enter");
        let label = |h: &Harness<Long>, row: usize| {
            h.screen().lines().nth(row).unwrap_or_default().replace('▌', " ").trim().to_owned()
        };
        let resting = label(&h, 2);
        h.press("down");
        let raised = label(&h, 2);
        assert!(resting.ends_with('…'), "{}", h.screen());
        assert_eq!(raised, resting, "the slide moves the label, it does not cut it shorter");
        let line = h.screen().lines().nth(2).unwrap_or_default().to_owned();
        assert!(line.starts_with("▌  us-east"), "the raised label slid one cell: {line}");
    }
}
