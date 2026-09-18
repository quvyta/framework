//! Settings lists: rows of a label on the left and a control anchored on the right.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{Axis, EventCx, Flex, MeasureCx, Node, NodeMut, PaintCx, View, Widget};

use super::cells;
use super::row::LEAD;

/// Cells between the label column and the control, and after the control.
const CONTROL_GAP: u16 = 2;

/// One setting of a [`SettingsList`]: a label, an optional description and the control added
/// with [`SettingsRows::row`].
pub struct SettingRow<Msg> {
    label: String,
    description: Option<String>,
    disabled: bool,
    on_activate: Option<Msg>,
}

impl<Msg> SettingRow<Msg> {
    /// A row labelled `label`.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), description: None, disabled: false, on_activate: None }
    }

    /// One faint line under the label.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Greys the row out; the keyboard skips it. Disable its control as well.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for Enter or Space on the row when its control does not use the key, or for a
    /// click on the label; for rows that open something, such as a detail page.
    #[must_use]
    pub fn on_activate(mut self, message: Msg) -> Self {
        self.on_activate = Some(message);
        self
    }

    fn height(&self) -> u16 {
        1 + u16::from(self.description.is_some())
    }
}

enum Entry<Msg> {
    Heading(String),
    /// A setting and the index of its control node.
    Row(SettingRow<Msg>, usize),
}

/// Adds headings and rows to a [`SettingsList`] inside [`SettingsList::show`].
pub struct SettingsRows<'a, Msg> {
    entries: Vec<Entry<Msg>>,
    controls: Vec<Node<Msg>>,
    env: &'a crate::env::Env,
    size: crate::geometry::Size,
    idle: &'a crate::widget::IdleScope<Msg>,
}

impl<Msg: 'static> SettingsRows<'_, Msg> {
    /// Adds a group heading.
    pub fn heading(&mut self, title: impl Into<String>) {
        self.entries.push(Entry::Heading(title.into()));
    }

    /// Adds `row` with the one control built by `control` (a switch, a select, a segmented control,
    /// a value text). Give the control no focus handling of its own: the list takes focus as
    /// one control and passes keys to the selected row.
    pub fn row(&mut self, row: SettingRow<Msg>, control: impl FnOnce(&mut View<'_, Msg>)) {
        let mut children = Vec::new();
        control(&mut View::new(&mut children, self.env, self.size, self.idle));
        let index = self.controls.len();
        self.controls.push(Node::new(Flex::new(Axis::Row, children), index));
        self.entries.push(Entry::Row(row, index));
    }
}

/// Settings, one per row: the label (and a faint description) on the left, its control
/// anchored on the right.
///
/// Rows are bare until touched. The row under the pointer raises its surface with a soft
/// pillar; the keyboard's row, while the list has focus, raises it further with a breathing
/// pillar. A focused list raises only one row: moving the pointer onto a row makes it the
/// keyboard's row. Only the label slides one cell right; the pillar and the control never move.
/// The label column keeps one spare cell for that and cuts long labels with `…`.
///
/// The list takes focus as one control. ↑/↓ (and Home/End) move between enabled rows; every
/// other key goes to the selected row's control, so Enter or Space toggles a switch or opens a
/// select and ←/→ change a segmented control. Keys the control does not use activate the row
/// when it has [`SettingRow::on_activate`]. The pointer works on controls directly; clicking a
/// label selects its row. The application owns every value; the keyboard's row lives in the
/// runtime.
///
/// Style keys: `setting-row` (`bg`, `pillar`) with `hover`, `selected`, `focus`, `disabled`;
/// `setting-label` (`fg`, `bold`) and `setting-description` (`fg`) with the same states;
/// `settings-heading` (`fg`, `bold`).
pub struct SettingsList<Msg> {
    entries: Vec<Entry<Msg>>,
    controls: Vec<Node<Msg>>,
}

#[derive(Debug, Default)]
struct SettingsMemory {
    selected: Option<usize>,
    /// Where each control was painted, by control index.
    controls: Vec<Rect>,
    /// Where each row was painted, by control index.
    rows: Vec<Rect>,
    /// Where the pointer was in the last frame; moving it carries the keyboard's row.
    pointer: Option<(i32, i32)>,
}

impl<Msg: Clone + 'static> SettingsList<Msg> {
    /// Adds a settings list with the headings and rows `build` adds to `ui`.
    pub fn show<'v>(ui: &'v mut View<'_, Msg>, build: impl FnOnce(&mut SettingsRows<'_, Msg>)) -> NodeMut<'v, Msg> {
        let (entries, controls) = {
            let mut rows = SettingsRows {
                entries: Vec::new(),
                controls: Vec::new(),
                env: ui.env(),
                size: ui.size(),
                idle: ui.idle_scope(),
            };
            build(&mut rows);
            (rows.entries, rows.controls)
        };
        ui.add(Self { entries, controls }).fill_width()
    }

    fn row(&self, index: usize) -> Option<&SettingRow<Msg>> {
        self.entries.iter().find_map(|entry| match entry {
            Entry::Row(row, i) if *i == index => Some(row),
            _ => None,
        })
    }

    fn enabled(&self) -> Vec<usize> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Row(row, index) if !row.disabled => Some(*index),
                _ => None,
            })
            .collect()
    }

    /// The keyboard's row: the remembered one when it is still enabled, else the first enabled.
    fn current(&self, remembered: Option<usize>) -> Option<usize> {
        let enabled = self.enabled();
        remembered.filter(|index| enabled.contains(index)).or_else(|| enabled.first().copied())
    }

    fn activate(&self, cx: &mut EventCx<'_, Msg>, index: usize) -> bool {
        match self.row(index).and_then(|row| row.on_activate.clone()) {
            Some(message) => {
                cx.flash();
                cx.emit(message);
                true
            }
            None => false,
        }
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for SettingsList<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let mut width = 0u16;
        let mut height = 0u16;
        for (position, entry) in self.entries.iter().enumerate() {
            match entry {
                Entry::Heading(title) => {
                    height = height.saturating_add(1 + u16::from(position > 0));
                    width = width.max(text::width(title).saturating_add(LEAD));
                }
                Entry::Row(row, index) => {
                    let control = cx.measure_child(&self.controls[*index], Size::new(available.width, 1)).width;
                    let label = text::width(&row.label).max(row.description.as_deref().map_or(0, text::width));
                    width = width.max(cells::sum([LEAD, label, 1, CONTROL_GAP * 2, control]));
                    height = height.saturating_add(row.height());
                }
            }
        }
        Size::new(width, height).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        let focused = cx.is_focused();
        let pointer = cx.pointer_within();
        let slide = cx.env().slide();
        let enabled = self.enabled();
        let selected = {
            let memory = cx.memory::<SettingsMemory>();
            // The pointer moves the one highlight: an enabled row it moves onto becomes the
            // keyboard's row, so a focused list never raises two rows at once.
            if pointer != memory.pointer {
                memory.pointer = pointer;
                let under = pointer.and_then(|(px, py)| memory.rows.iter().position(|rect| rect.contains(px, py)));
                if let Some(index) = under.filter(|index| enabled.contains(index)) {
                    memory.selected = Some(index);
                }
            }
            let current = self.current(memory.selected);
            memory.selected = current;
            current.filter(|_| focused)
        };
        let mut controls = vec![Rect::default(); self.controls.len()];
        let mut rows = vec![Rect::default(); self.controls.len()];
        let mut y = area.y;
        for (position, entry) in self.entries.iter().enumerate() {
            match entry {
                Entry::Heading(title) => {
                    if position > 0 {
                        y += 1;
                    }
                    let style = cx.style("settings-heading", None, &[]).text();
                    let budget = area.width.saturating_sub(LEAD + 1);
                    let shown = text::truncate(title, budget).into_owned();
                    cx.text(area.x + i32::from(LEAD), y, &shown, style, budget);
                    y += 1;
                }
                Entry::Row(row, index) => {
                    let rect = Rect::new(area.x, y, area.width, row.height());
                    y += i32::from(row.height());
                    rows[*index] = rect;
                    let mut states = Vec::new();
                    if row.disabled {
                        states.push(State::Disabled);
                    } else {
                        let pointed = pointer.is_some_and(|(px, py)| rect.contains(px, py));
                        if pointed && (!focused || selected == Some(*index)) {
                            states.push(State::Hover);
                        }
                        if selected == Some(*index) {
                            states.extend([State::Selected, State::Focus]);
                        }
                    }
                    let style = cx.style("setting-row", None, &states);
                    if let Some(bg) = style.text().bg {
                        cx.clear(rect, bg);
                    }
                    if let Some(color) = style.color("pillar") {
                        for row_y in rect.y..rect.bottom() {
                            cx.pillar(rect.x, row_y, color);
                        }
                    }

                    let node = &self.controls[*index];
                    let control_width = cx.measure_child(node, Size::new(area.width / 2, 1)).width;
                    let control_x = rect.right() - i32::from(CONTROL_GAP + control_width);
                    let control = Rect::new(control_x, rect.y, control_width, 1);
                    controls[*index] = control;
                    cx.paint_child_unfocusable(node, control);

                    let raised = states.contains(&State::Hover) || states.contains(&State::Selected);
                    let shift = u16::from(slide && raised);
                    let text_x = rect.x + i32::from(LEAD);
                    // The label column keeps one spare cell so the slide never reaches the control.
                    let budget = clamp_u16(control_x - i32::from(CONTROL_GAP) - text_x).saturating_sub(1);
                    let x = text_x + i32::from(shift);
                    let label_style = cx.style("setting-label", None, &states).text();
                    let label = text::truncate(&row.label, budget).into_owned();
                    cx.text(x, rect.y, &label, CellStyle { bg: None, ..label_style }, budget);
                    if let Some(description) = &row.description {
                        let style = cx.style("setting-description", None, &states).text();
                        let shown = text::truncate(description, budget).into_owned();
                        cx.text(x, rect.y + 1, &shown, CellStyle { bg: None, ..style }, budget);
                    }
                }
            }
        }
        let memory = cx.memory::<SettingsMemory>();
        memory.controls = controls;
        memory.rows = rows;
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let enabled = self.enabled();
        if enabled.is_empty() {
            return false;
        }
        let current = self.current(cx.memory::<SettingsMemory>().selected);
        match event {
            Event::Key(key) => {
                let position = current.and_then(|index| enabled.iter().position(|i| *i == index)).unwrap_or(0);
                let target = if key.is_plain(Key::Up) {
                    Some(position.saturating_sub(1))
                } else if key.is_plain(Key::Down) {
                    Some((position + 1).min(enabled.len() - 1))
                } else {
                    None
                };
                if let Some(target) = target {
                    cx.memory::<SettingsMemory>().selected = Some(enabled[target]);
                    return true;
                }
                let Some(index) = current else {
                    return false;
                };
                let rect = cx.memory::<SettingsMemory>().controls.get(index).copied().unwrap_or_default();
                // The row holds one control inside its layout node; that control gets the key.
                let used =
                    self.controls[index].widget.children().iter().any(|control| cx.forward(control, rect, event));
                if used {
                    return true;
                }
                if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                    return self.activate(cx, index);
                }
                if key.is_plain(Key::Home) || key.is_plain(Key::End) {
                    let target = if key.is_plain(Key::Home) { enabled[0] } else { enabled[enabled.len() - 1] };
                    cx.memory::<SettingsMemory>().selected = Some(target);
                    return true;
                }
                false
            }
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                let rows = cx.memory::<SettingsMemory>().rows.clone();
                let Some(index) = rows.iter().position(|rect| rect.contains(mouse.x, mouse.y)) else {
                    return false;
                };
                if !enabled.contains(&index) {
                    return true;
                }
                cx.memory::<SettingsMemory>().selected = Some(index);
                cx.request_focus();
                self.activate(cx, index);
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.enabled().is_empty()
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.controls
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.controls
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widgets::{Segmented, Switch};

    #[derive(Default)]
    struct Prefs {
        animations: bool,
        density: usize,
        opened: usize,
        telemetry_locked: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Animations(bool),
        Density(usize),
        Open,
    }

    impl App for Prefs {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Animations(on) => self.animations = on,
                Msg::Density(index) => self.density = index,
                Msg::Open => self.opened += 1,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            SettingsList::show(ui, |list| {
                list.heading("APPEARANCE");
                list.row(SettingRow::new("Animations").description("Motion in lists"), |ui| {
                    ui.add(Switch::new(self.animations).on_toggle(Msg::Animations));
                });
                list.row(SettingRow::new("Density"), |ui| {
                    ui.add(Segmented::new(["Cozy", "Compact"]).selected(self.density).on_select(Msg::Density));
                });
                list.heading("PRIVACY");
                list.row(SettingRow::new("Telemetry").disabled(self.telemetry_locked), |ui| {
                    ui.add(Switch::new(false).disabled(self.telemetry_locked));
                });
                list.row(SettingRow::new("Storage used by images and volumes").on_activate(Msg::Open), |ui| {
                    ui.add(Text::new("2.4 GB"));
                });
            })
            .id("settings");
        }
    }

    use crate::widgets::Text;

    #[test]
    fn labels_left_controls_anchored_right_with_headings() {
        let h = Harness::new(Prefs::default(), 40, 8);
        assert_eq!(
            h.screen(),
            "  APPEARANCE\n  Animations                      \n  Motion in lists\n  Density            Cozy    Compact\n\n  PRIVACY\n  Telemetry\n  Storage used by images and…   2.4 GB\n"
                .lines()
                .map(str::trim_end)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        );
    }

    #[test]
    fn keyboard_moves_rows_and_drives_the_selected_control() {
        let mut h = Harness::new(Prefs { telemetry_locked: true, ..Prefs::default() }, 40, 8);
        h.press("tab");
        let theme = h.env().theme();
        assert_eq!(h.bg(20, 1), theme.color("active"), "the first row is selected on focus");
        assert!(h.screen().lines().nth(1).is_some_and(|line| line.starts_with("▌  Animations")));
        h.press("space");
        assert!(h.app().animations);
        h.press("down").press("right");
        assert_eq!(h.app().density, 1);
        h.press("down").press("enter");
        assert_eq!(h.app().opened, 1, "the disabled row is skipped");
        h.press("up");
        assert!(h.screen().lines().nth(3).is_some_and(|line| line.starts_with("▌  Density")));
    }

    #[test]
    fn the_pointer_carries_the_keyboards_row() {
        let mut h = Harness::new(Prefs::default(), 40, 8);
        h.press("tab");
        assert!(h.screen().lines().nth(1).is_some_and(|line| line.starts_with("▌  Animations")));
        h.hover(6, 7);
        let screen = h.screen();
        let raised: Vec<&str> = screen.lines().filter(|line| line.starts_with('▌')).collect();
        assert_eq!(raised, ["▌  Storage used by images and…  2.4 GB"], "one raised row:\n{screen}");
        assert_eq!(h.bg(20, 7), h.env().theme().color("active"), "the pointer's row is the keyboard's row");
        assert_ne!(h.bg(20, 1), h.env().theme().color("active"));
        h.press("up");
        let screen = h.screen();
        let raised: Vec<&str> = screen.lines().filter(|line| line.starts_with('▌')).collect();
        assert_eq!(raised, ["▌  Telemetry"], "the keyboard continues from the pointer's row:\n{screen}");
    }

    #[test]
    fn hover_slides_the_label_but_not_the_control_and_clicks_reach_controls() {
        let mut h = Harness::new(Prefs::default(), 40, 8);
        let before = h.find("Cozy");
        h.hover(4, 3);
        assert!(h.screen().lines().nth(3).is_some_and(|line| line.starts_with("▌  Density")));
        assert_eq!(h.find("Cozy"), before);
        h.hover(before.map_or(0, |(x, _)| x), 3);
        assert!(
            h.screen().lines().nth(3).is_some_and(|line| line.starts_with("▌")),
            "the row stays lit over its control"
        );
        h.click_text("Compact");
        assert_eq!(h.app().density, 1);
        h.click_text("Storage");
        assert_eq!(h.app().opened, 1);
    }
}
