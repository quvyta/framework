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

/// Cells a [nested](SettingRow::nested) row's text starts further in than its parent's.
const NEST: u16 = 2;

/// One setting of a [`SettingsList`]: a label, an optional description and the control added
/// with [`SettingsRows::row`].
pub struct SettingRow<Msg> {
    label: String,
    description: Option<String>,
    disabled: bool,
    nested: bool,
    on_activate: Option<Msg>,
}

impl<Msg> SettingRow<Msg> {
    /// A row labelled `label`.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), description: None, disabled: false, nested: false, on_activate: None }
    }

    /// A faint note under the label, wrapped over as many lines as it needs.
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

    /// Marks the row as part of the row above it, such as a choice that qualifies that setting:
    /// its label and description start two cells further in. The pillar and the control stay
    /// where every row has them, and the keys reach it as any other row.
    #[must_use]
    pub fn nested(mut self, nested: bool) -> Self {
        self.nested = nested;
        self
    }

    /// Cells the row's text starts further in than a top-level row's.
    fn indent(&self) -> u16 {
        if self.nested { NEST } else { 0 }
    }

    /// Message for Enter or Space on the row when its control does not use the key, or for a
    /// click on the label; for rows that open something, such as a detail page.
    #[must_use]
    pub fn on_activate(mut self, message: Msg) -> Self {
        self.on_activate = Some(message);
        self
    }

    /// Where the label, the description and the control go in a row `width` cells wide whose
    /// control is `control` cells wide.
    ///
    /// The label shares the first line with the control while it fits beside it. When it does
    /// not, it takes the whole width, wrapping if it must, and the control moves to the line
    /// under it. The description always wraps over the whole width, so a narrow screen shows all
    /// of it instead of cutting it.
    ///
    /// `squeezed` says the control would be wider than the `control` cells it has beside the
    /// label; it then goes under the label, where it has the whole row.
    fn lines(&self, control: u16, squeezed: bool, width: u16) -> RowLines {
        // The text column keeps one spare cell so the slide never reaches the control or the
        // right edge.
        let full = width.saturating_sub(LEAD + self.indent() + CONTROL_GAP + 1).max(1);
        let beside = full.saturating_sub(control.saturating_add(CONTROL_GAP));
        let (label, label_width, control_below) = if !squeezed && text::width(&self.label) <= beside {
            (vec![self.label.clone()], beside, false)
        } else {
            (text::wrap(&self.label, full), full, control > 0)
        };
        let description = self.description.as_deref().map_or_else(Vec::new, |text| text::wrap(text, full));
        RowLines { label, description, control_below, full, label_width }
    }
}

/// Cells a control on a line of its own may take in a row `width` cells wide: everything after
/// the pillar's lead and before the gap at the right edge.
fn control_room(width: u16) -> u16 {
    width.saturating_sub(LEAD + CONTROL_GAP)
}

/// The lines of one settings row, from [`SettingRow::lines`].
struct RowLines {
    label: Vec<String>,
    description: Vec<String>,
    /// Whether the control has its own line under the label.
    control_below: bool,
    /// Cells for text across the whole row.
    full: u16,
    /// Cells for each line of the label: beside the control, or the whole width.
    label_width: u16,
}

impl RowLines {
    fn label_rows(&self) -> u16 {
        clamp_u16(i32::try_from(self.label.len()).unwrap_or(i32::MAX)).max(1)
    }

    /// The line the control sits on, counted from the top of the row.
    fn control_row(&self) -> u16 {
        if self.control_below { self.label_rows() } else { 0 }
    }

    /// The line the description starts on.
    fn description_row(&self) -> u16 {
        self.label_rows() + u16::from(self.control_below)
    }

    fn height(&self) -> u16 {
        let description = clamp_u16(i32::try_from(self.description.len()).unwrap_or(i32::MAX));
        self.description_row().saturating_add(description)
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
    /// The environment the list is drawn in: the active theme, language and icons, for rows that
    /// show them.
    #[must_use]
    pub fn env(&self) -> &crate::env::Env {
        self.env
    }

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
/// The label column keeps one spare cell for that.
///
/// Nothing is cut on a narrow screen: a description wraps over the whole width of its row, and a
/// label too long to share its line with the control takes the whole width, wrapping if it must,
/// while the control moves to the line under it. So does a control that needs more than half the
/// row, which then has the whole row. A row reports the height it wraps to.
///
/// The list takes focus as one control. ↑/↓ (and Home/End) move between enabled rows; every
/// other key goes to the selected row's control, so Enter or Space toggles a switch or opens a
/// select and ←/→ change a segmented control. Keys the control does not use activate the row
/// when it has [`SettingRow::on_activate`]. The pointer works on controls directly; clicking a
/// label selects its row. The application owns every value; the keyboard's row lives in the
/// runtime.
///
/// One long list can hold a whole settings page inside a [`ScrollView`](crate::widgets::ScrollView):
/// moving with the keys scrolls just enough to show the new row, and a click never scrolls, so
/// the row stays under the pointer.
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
    /// The keys moved the selection since the last frame, so a scroll view around a list
    /// taller than itself shows the new row. A click or the pointer never scrolls: the row is
    /// already under it.
    reveal: bool,
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
                    let node = &self.controls[*index];
                    let control = cx.measure_child(node, Size::new(available.width, 1)).width;
                    let label = text::width(&row.label).max(row.description.as_deref().map_or(0, text::width));
                    width = width.max(cells::sum([LEAD, row.indent(), label, 1, CONTROL_GAP * 2, control]));
                    // Laid out as paint lays it out, so a narrow row reports the lines it wraps to.
                    let beside = cx.measure_child(node, Size::new(available.width / 2, 1)).width;
                    let whole = cx.measure_child(node, Size::new(control_room(available.width), 1)).width;
                    height = height.saturating_add(row.lines(beside, whole > beside, available.width).height());
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
                    let node = &self.controls[*index];
                    // Beside the label a control has half the row; one that wants more goes on a
                    // line of its own, where it has the whole row.
                    let beside = cx.measure_child(node, Size::new(area.width / 2, 1)).width;
                    let whole = cx.measure_child(node, Size::new(control_room(area.width), 1)).width;
                    let lines = row.lines(beside, whole > beside, area.width);
                    let control_width = if lines.control_below { whole } else { beside };
                    let rect = Rect::new(area.x, y, area.width, lines.height());
                    y += i32::from(lines.height());
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

                    let control_x = rect.right() - i32::from(CONTROL_GAP + control_width);
                    let control = Rect::new(control_x, rect.y + i32::from(lines.control_row()), control_width, 1);
                    controls[*index] = control;
                    // The keys of the list's own row go to its control, so the control is painted
                    // focused: a time or a duration keeps the part being typed only while focused.
                    let keyboard_row = focused && selected == Some(*index) && !row.disabled;
                    cx.paint_child_lending_focus(node, control, keyboard_row);

                    let raised = states.contains(&State::Hover) || states.contains(&State::Selected);
                    let shift = u16::from(slide && raised);
                    let x = rect.x + i32::from(LEAD + row.indent() + shift);
                    let label_budget = lines.label_width;
                    let label_style = cx.style("setting-label", None, &states).text();
                    for (line, label) in (rect.y..).zip(&lines.label) {
                        let label = text::truncate(label, label_budget).into_owned();
                        cx.text(x, line, &label, CellStyle { bg: None, ..label_style }, label_budget);
                    }
                    let style = cx.style("setting-description", None, &states).text();
                    let first = rect.y + i32::from(lines.description_row());
                    for (line, description) in (first..).zip(&lines.description) {
                        cx.text(x, line, description, CellStyle { bg: None, ..style }, lines.full);
                    }
                }
            }
        }
        let memory = cx.memory::<SettingsMemory>();
        let reveal = std::mem::take(&mut memory.reveal)
            .then(|| memory.selected.and_then(|index| rows.get(index)))
            .flatten()
            .copied();
        memory.controls = controls;
        memory.rows = rows;
        if let Some(row) = reveal {
            cx.reveal(row);
        }
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
                    let memory = cx.memory::<SettingsMemory>();
                    memory.selected = Some(enabled[target]);
                    memory.reveal = true;
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
                    let memory = cx.memory::<SettingsMemory>();
                    memory.selected = Some(target);
                    memory.reveal = true;
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

    /// A list whose one row holds a name field.
    #[derive(Default)]
    struct Named {
        name: String,
    }

    impl App for Named {
        type Msg = String;
        fn update(&mut self, name: String) -> Command<String> {
            self.name = name;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, String>) {
            SettingsList::show(ui, |list| {
                list.row(SettingRow::new("Name"), |ui| {
                    ui.add(crate::widgets::TextInput::new(&self.name).on_change(|name| name))
                        .width(crate::widget::Length::Cells(12));
                });
            });
        }
    }

    #[test]
    fn a_field_in_a_row_takes_every_space_of_a_burst() {
        // The list has the focus and lends it to the row's field; spaces that arrive together are
        // still typed, not taken for a held key.
        let mut h = Harness::new(Named::default(), 40, 4);
        h.press("tab");
        let burst: Vec<crate::event::Event> = ["a", "space", "space", "b"]
            .iter()
            .map(|chord| crate::event::Event::Key(crate::event::KeyEvent::press(chord)))
            .collect();
        h.events(&burst);
        assert_eq!(h.app().name, "a  b", "{}", h.screen());
    }

    /// Forty switches in one list, in a scroll view shorter than the list.
    #[derive(Default)]
    struct Long {
        on: Vec<usize>,
    }

    impl App for Long {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.on.push(index);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add_with(crate::widgets::ScrollView::new(), |ui| {
                SettingsList::show(ui, |list| {
                    for n in 1..=40 {
                        list.row(SettingRow::new(format!("Option {n}")), |ui| {
                            ui.add(Switch::new(self.on.contains(&n)).on_toggle(move |_| n));
                        });
                    }
                })
                .fill_width();
            })
            .fill();
        }
    }

    fn row_of(h: &Harness<Long>, n: usize) -> Option<usize> {
        let label = n.to_string();
        h.screen().lines().position(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            words.windows(2).any(|pair| pair[0] == "Option" && pair[1] == label)
        })
    }

    #[test]
    fn a_list_taller_than_its_scroll_view_keeps_the_clicked_row_in_place_and_follows_the_keys() {
        let mut h = Harness::new(Long::default(), 40, 20);
        h.set_reduced_motion(true);
        let third = row_of(&h, 3).unwrap_or_else(|| panic!("{}", h.screen()));
        let (x, y) = h.find("Option 3").unwrap_or_else(|| panic!("{}", h.screen()));
        h.click(x, y);
        assert_eq!(row_of(&h, 3), Some(third), "a click does not scroll: {}", h.screen());
        assert_eq!(row_of(&h, 1), Some(0), "the list stays at its top: {}", h.screen());
        for _ in 3..25 {
            h.press("down");
        }
        let screen = h.screen();
        let last = screen
            .lines()
            .collect::<Vec<_>>()
            .iter()
            .rposition(|line| line.contains("Option"))
            .unwrap_or_else(|| panic!("{screen}"));
        assert_eq!(row_of(&h, 25), Some(last), "the selected row is the last one shown: {screen}");
        h.press("up");
        assert_eq!(row_of(&h, 25), Some(last), "going back up inside the view does not scroll: {}", h.screen());
    }

    #[test]
    fn labels_left_controls_anchored_right_with_headings() {
        let h = Harness::new(Prefs::default(), 40, 10);
        assert_eq!(
            h.screen(),
            "  APPEARANCE\n  Animations                      \n  Motion in lists\n  Density            Cozy    Compact\n\n  PRIVACY\n  Telemetry\n  Storage used by images and volumes\n                                2.4 GB\n\n"
                .lines()
                .map(str::trim_end)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        );
    }

    /// Rows with long labels and descriptions, in English or German.
    struct Wordy {
        german: bool,
    }

    impl App for Wordy {
        type Msg = bool;
        fn update(&mut self, _: bool) -> Command<bool> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, bool>) {
            let (motion, calm, hour, hour_note) = if self.german {
                (
                    "Bewegung",
                    "Ebenen erscheinen sofort; nichts gleitet oder blendet über.",
                    "Sitzungen vor dieser Stunde zählen zum Vortag",
                    "Für Nachteulen, die nach Mitternacht arbeiten.",
                )
            } else {
                (
                    "Motion",
                    "Layers appear at once; nothing slides or fades.",
                    "Sessions before this hour count for the day before",
                    "For night owls who work past midnight.",
                )
            };
            SettingsList::show(ui, |list| {
                list.row(SettingRow::new(motion).description(calm), |ui| {
                    ui.add(Switch::new(true).on_toggle(|on| on));
                });
                list.row(SettingRow::new(hour).description(hour_note), |ui| {
                    ui.add(Segmented::new(["0", "3", "5"]).selected(1).on_select(|_| true));
                });
            });
        }
    }

    #[test]
    fn at_forty_columns_descriptions_wrap_and_a_long_label_puts_its_control_below() {
        for (german, code) in [(false, "en"), (true, "de")] {
            let mut h = Harness::new(Wordy { german }, 40, 14);
            h.set_locale(code);
            let screen = h.screen();
            assert!(!screen.contains('…'), "{code}: {screen}");
            let lines: Vec<&str> = screen.lines().collect();
            // The first label fits beside its switch; the description wraps under it.
            assert!(lines[1].starts_with("  ") && lines[2].starts_with("  "), "{code}: {screen}");
            let words: Vec<&str> = lines[1..3].iter().flat_map(|line| line.split_whitespace()).collect();
            assert!(words.contains(&"nothing") || words.contains(&"nichts"), "{code}: {screen}");
            // The long label takes its own lines and the control sits under it, at the right.
            let hour = lines.iter().position(|line| line.contains("Sessions") || line.contains("Sitzungen"));
            let hour = hour.unwrap_or_else(|| panic!("{code}: {screen}"));
            let control =
                lines.iter().position(|line| line.contains(" 0 ")).unwrap_or_else(|| panic!("{code}: {screen}"));
            assert!(control > hour, "{code}: {screen}");
            assert!(
                lines[control].trim_start().starts_with('0'),
                "the control has the line to itself: {code}: {screen}"
            );
            assert!(screen.contains("midnight") || screen.contains("Mitternacht"), "{code}: {screen}");
        }
    }

    #[test]
    fn below_forty_columns_every_row_degrades_to_label_then_control_then_description() {
        for width in [36, 30, 24, 20] {
            let h = Harness::new(Prefs::default(), width, 16);
            let screen = h.screen();
            let lines: Vec<&str> = screen.lines().collect();
            // At 20 columns the two options of the segmented control cannot fit even on a line
            // of their own; that cut is the control's, and every text of the list stays whole.
            let cut: Vec<&&str> = lines.iter().filter(|line| line.contains('…')).collect();
            assert!(cut.iter().all(|line| width == 20 && line.contains("Cozy")), "{width}: {screen}");
            let storage = lines.iter().position(|line| line.contains("Storage")).unwrap_or_else(|| panic!("{screen}"));
            let size = lines.iter().position(|line| line.contains("2.4 GB")).unwrap_or_else(|| panic!("{screen}"));
            assert!(size > storage, "the value sits under its label: {width}: {screen}");
            assert!(!lines[size].contains("Storage") && !lines[size].contains("volumes"), "{width}: {screen}");
            assert!(screen.contains("Motion in lists") || screen.contains("Motion in"), "{width}: {screen}");
            let density = lines.iter().position(|line| line.contains("Density")).unwrap_or_else(|| panic!("{screen}"));
            let cozy = lines.iter().position(|line| line.contains("Cozy")).unwrap_or_else(|| panic!("{screen}"));
            if cozy != density {
                assert_eq!(cozy, density + 1, "the control right under its label: {width}: {screen}");
                assert!(
                    width == 20 || lines[cozy].contains("Compact"),
                    "a control on its own line has the whole row: {width}: {screen}"
                );
            }
        }
        for width in [30, 24, 20] {
            for (german, code) in [(false, "en"), (true, "de")] {
                let mut h = Harness::new(Wordy { german }, width, 24);
                h.set_locale(code);
                let screen = h.screen();
                assert!(!screen.contains('…'), "{width} {code}: {screen}");
                assert!(screen.contains("midnight") || screen.contains("Mitternacht"), "{width} {code}: {screen}");
                assert!(screen.contains(" 0    3    5") || screen.contains("0    3    5"), "{width} {code}: {screen}");
            }
        }
    }

    #[test]
    fn a_row_reports_the_height_it_wraps_to() {
        let row = SettingRow::<()>::new("Sessions before this hour count for the day before")
            .description("For night owls who work past midnight.");
        let wide = row.lines(9, false, 100);
        assert_eq!((wide.height(), wide.control_row(), wide.description_row()), (2, 0, 1));
        let narrow = row.lines(9, false, 40);
        assert_eq!((narrow.label.len(), narrow.control_row(), narrow.description_row()), (2, 2, 3));
        assert_eq!(narrow.height(), 5, "two label lines, the control, two description lines");
        let squeezed = SettingRow::<()>::new("Density").lines(9, true, 40);
        assert_eq!((squeezed.control_row(), squeezed.height()), (1, 2), "a squeezed control goes under its label");
    }

    #[test]
    fn keyboard_moves_rows_and_drives_the_selected_control() {
        let mut h = Harness::new(Prefs { telemetry_locked: true, ..Prefs::default() }, 40, 10);
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
        let mut h = Harness::new(Prefs::default(), 40, 10);
        h.press("tab");
        assert!(h.screen().lines().nth(1).is_some_and(|line| line.starts_with("▌  Animations")));
        h.hover(6, 7);
        let screen = h.screen();
        let raised: Vec<&str> = screen.lines().filter(|line| line.starts_with('▌')).collect();
        assert_eq!(
            raised,
            ["▌  Storage used by images and volumes", "▌                               2.4 GB"],
            "one raised row, both its lines:\n{screen}"
        );
        assert_eq!(h.bg(20, 7), h.env().theme().color("active"), "the pointer's row is the keyboard's row");
        assert_ne!(h.bg(20, 1), h.env().theme().color("active"));
        h.press("up");
        let screen = h.screen();
        let raised: Vec<&str> = screen.lines().filter(|line| line.starts_with('▌')).collect();
        assert_eq!(raised, ["▌  Telemetry"], "the keyboard continues from the pointer's row:\n{screen}");
    }

    #[test]
    fn hover_slides_the_label_but_not_the_control_and_clicks_reach_controls() {
        let mut h = Harness::new(Prefs::default(), 40, 10);
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

    struct Nested;

    impl App for Nested {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            SettingsList::show(ui, |list| {
                list.row(SettingRow::new("Theme"), |ui| {
                    ui.add(Segmented::new(["Dark", "Light"]).selected(0));
                });
                list.row(SettingRow::new("Everywhere").description("In every application").nested(true), |ui| {
                    ui.add(Switch::new(true));
                });
            });
        }
    }

    #[test]
    fn a_nested_row_starts_two_cells_further_in_and_keeps_its_control_in_place() {
        let mut h = Harness::new(Nested, 40, 4);
        let (theme_x, _) = h.find("Theme").expect("parent");
        let (nested_x, _) = h.find("Everywhere").expect("nested");
        let (description_x, _) = h.find("In every").expect("description");
        assert_eq!((nested_x, description_x), (theme_x + 2, theme_x + 2), "{}", h.screen());
        h.resize(20, 6);
        let (nested_x, _) = h.find("Everywhere").expect("nested, narrow");
        assert_eq!(nested_x, theme_x + 2, "{}", h.screen());
    }

    /// A settings page with a time and a duration, as qfocus's settings have.
    struct Clock {
        turn: crate::date::TimeOfDay,
        away: std::time::Duration,
    }

    #[derive(Clone)]
    enum ClockMsg {
        Turn(crate::date::TimeOfDay),
        Away(std::time::Duration),
    }

    impl App for Clock {
        type Msg = ClockMsg;
        fn update(&mut self, msg: ClockMsg) -> Command<ClockMsg> {
            match msg {
                ClockMsg::Turn(time) => self.turn = time,
                ClockMsg::Away(duration) => self.away = duration,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ClockMsg>) {
            SettingsList::show(ui, |list| {
                list.row(SettingRow::new("Day turns at"), |ui| {
                    ui.add(crate::widgets::TimeInput::new(self.turn).on_change(ClockMsg::Turn));
                });
                list.row(SettingRow::new("Away after"), |ui| {
                    ui.add(crate::widgets::DurationInput::new(self.away).on_change(ClockMsg::Away));
                });
            });
        }
    }

    fn clock() -> Harness<Clock> {
        let app = Clock { turn: crate::date::TimeOfDay::new(4, 0, 0), away: std::time::Duration::from_secs(15 * 60) };
        let mut h = Harness::new(app, 60, 6);
        h.set_reduced_motion(true).render();
        h
    }

    #[test]
    fn two_digits_typed_into_a_time_in_a_settings_row_make_one_value() {
        let mut h = clock();
        let (x, y) = h.find("04").unwrap_or_else(|| panic!("the hour is on screen:\n{}", h.screen()));
        h.click(x, y);
        h.type_text("12");
        assert_eq!(
            h.app().turn,
            crate::date::TimeOfDay::new(12, 0, 0),
            "two digits make twelve, not two:\n{}",
            h.screen()
        );
        let (x, y) = h.find("00").unwrap_or_else(|| panic!("the minute is on screen:\n{}", h.screen()));
        h.click(x, y);
        h.type_text("05");
        assert_eq!(h.app().turn, crate::date::TimeOfDay::new(12, 5, 0), "the minute took the digits:\n{}", h.screen());
    }

    #[test]
    fn the_keys_reach_the_minute_of_a_time_in_a_settings_row() {
        let mut h = clock();
        h.press("tab");
        h.type_text("07");
        h.press("right");
        h.type_text("45");
        assert_eq!(h.app().turn, crate::date::TimeOfDay::new(7, 45, 0), "{}", h.screen());
    }

    #[test]
    fn two_digits_typed_into_a_duration_in_a_settings_row_go_to_the_part_clicked() {
        let mut h = clock();
        let (x, y) = h.find("15").unwrap_or_else(|| panic!("the minutes are on screen:\n{}", h.screen()));
        h.click(x, y);
        h.type_text("05");
        assert_eq!(h.app().away, std::time::Duration::from_secs(5 * 60), "the minutes, not the hours:\n{}", h.screen());
    }
}
