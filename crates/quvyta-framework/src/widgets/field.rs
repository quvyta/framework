//! Form fields: a label, a control, and a hint or an error.

use crate::geometry::{Rect, Size, clamp_u16};
use crate::text;
use crate::theme::State;
use crate::widget::{Axis, Container, Flex, Length, MeasureCx, Node, PaintCx, Widget};

use super::cells;

/// Cells between the label column and the control when labels sit beside controls.
const LABEL_GAP: u16 = 2;

/// The narrowest control a field keeps beside its label; below it the label moves above.
const MIN_CONTROL: u16 = 16;

/// A labelled control: the label, the control the application adds inside, and under it a
/// faint hint or, when there is one, the error in the danger colour with a marker.
///
/// The field only lays things out. The application owns the value and its error, and marks the
/// control itself as invalid (`TextInput::invalid`). The label brightens while the control has
/// focus. Required fields show a faint word after the label; nothing is starred.
///
/// Labels sit above the control by default. With [`Field::label_width`] they take a column of
/// that width beside the control whenever the field is wide enough, and move above it again on
/// narrow screens. The column is at least as wide as the required word of the active language.
///
/// Style keys: `field-label` (`fg`, `bold`) with states `focus` and `disabled`;
/// `field-required`, `field-hint` and `field-error` (`fg`). The required word is
/// `quvyta.form.required`; the error marker is the `error` icon.
pub struct Field<Msg> {
    label: String,
    hint: Option<String>,
    error: Option<String>,
    required: bool,
    disabled: bool,
    pub(super) label_width: Option<u16>,
    body: Vec<Node<Msg>>,
}

impl<Msg: 'static> Field<Msg> {
    /// A field labelled `label`. Add its control with
    /// [`View::add_with`](crate::widget::View::add_with) or [`FormFields::field`](super::FormFields::field).
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            hint: None,
            error: None,
            required: false,
            disabled: false,
            label_width: None,
            body: vec![Node::new(Flex::new(Axis::Column, Vec::new()), 0)],
        }
    }

    /// Faint help under the control, shown while there is no error.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The error under the control; `None` shows the hint instead. Pass
    /// [`FormErrors::get`](super::FormErrors::get) straight in.
    #[must_use]
    pub fn error<S: Into<String>>(mut self, error: Option<S>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Shows the faint "required" word after the label.
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Greys the label out; disable the control as well.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Puts the label in a column `cells` wide beside the control when the field is at least
    /// that wide plus room for a control; otherwise the label stays above.
    #[must_use]
    pub fn label_width(mut self, cells: u16) -> Self {
        self.label_width = Some(cells);
        self
    }

    /// The label column width when the label fits beside the control in `width` cells.
    fn beside(&self, width: u16) -> Option<u16> {
        self.label_width.filter(|label| width >= cells::sum([*label, LABEL_GAP, MIN_CONTROL]))
    }

    /// Whether the required word, too wide for the label column `column`, goes under the control
    /// instead, on its own line before the hint. Growing the column would take room from every
    /// control of the form, and cutting the word would lose it.
    fn required_under_control(&self, column: u16, word: &str) -> bool {
        self.required && text::width(word) > column
    }

    /// The hint or error lines at `width`, and whether they are an error.
    fn message_lines(&self, width: u16, marker_width: u16) -> (Vec<String>, bool) {
        match (&self.error, &self.hint) {
            (Some(error), _) => (text::wrap(error, width.saturating_sub(marker_width + 1)), true),
            (None, Some(hint)) => (text::wrap(hint, width), false),
            (None, None) => (Vec::new(), false),
        }
    }
}

impl<Msg: 'static> Container<Msg> for Field<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.width = Length::Fill(1);
        self.body = vec![column];
    }
}

/// Where the parts of a field go inside its area.
struct Parts {
    label: Rect,
    required: Option<Rect>,
    control: Rect,
    message_x: i32,
    message_width: u16,
}

fn required_word() -> String {
    crate::t!("quvyta.form.required")
}

impl<Msg: 'static> Widget<Msg> for Field<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let marker = text::width(&cx.env().icons().glyph("error"));
        let label_width = text::width(&self.label);
        let required = if self.required { text::width(&required_word()) } else { 0 };
        let Some(body) = self.body.first() else {
            return Size::default();
        };
        if let Some(column) = self.beside(available.width) {
            let control_width = available.width - column - LABEL_GAP;
            let control = cx.measure_child(body, Size::new(control_width, available.height));
            let (lines, _) = self.message_lines(control_width, marker);
            let below = self.required_under_control(column, &required_word());
            let label_rows = 1 + u16::from(self.required && !below);
            let messages = clamp_u16(i32::try_from(lines.len()).unwrap_or(i32::MAX)).saturating_add(u16::from(below));
            let right = control.height.saturating_add(messages);
            return Size::new(available.width, label_rows.max(right)).min(available);
        }
        let control = cx.measure_child(body, Size::new(available.width, available.height.saturating_sub(1)));
        let (lines, error) = self.message_lines(available.width, marker);
        let widest_line = lines.iter().map(|line| text::width(line)).max().unwrap_or(0).saturating_add(if error {
            marker.saturating_add(1)
        } else {
            0
        });
        let label_line = label_width.saturating_add(if self.required { required.saturating_add(2) } else { 0 });
        let height = cells::sum([1, control.height, clamp_u16(i32::try_from(lines.len()).unwrap_or(i32::MAX))]);
        Size::new(label_line.max(control.width).max(widest_line), height).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let Some(body) = self.body.first() else {
            return;
        };
        let marker = cx.env().icons().glyph("error").into_owned();
        let marker_width = text::width(&marker);
        let required = required_word();
        let parts = match self.beside(area.width) {
            Some(column) => {
                let control_x = area.x + i32::from(column + LABEL_GAP);
                let control_width = area.width - column - LABEL_GAP;
                let height = cx.measure_child(body, Size::new(control_width, area.height)).height;
                let required = if self.required_under_control(column, &required) {
                    Rect::new(control_x, area.y + i32::from(height), control_width, 1)
                } else {
                    Rect::new(area.x, area.y + 1, column, 1)
                };
                Parts {
                    label: Rect::new(area.x, area.y, column, 1),
                    required: self.required.then_some(required),
                    control: Rect::new(control_x, area.y, control_width, height),
                    message_x: control_x,
                    message_width: control_width,
                }
            }
            None => {
                let label_width = text::width(&self.label).min(area.width);
                let height = cx.measure_child(body, Size::new(area.width, area.height.saturating_sub(1))).height;
                let required_x = area.x + i32::from(label_width) + 2;
                Parts {
                    label: Rect::new(area.x, area.y, label_width, 1),
                    required: self
                        .required
                        .then(|| Rect::new(required_x, area.y, clamp_u16(area.right() - required_x), 1)),
                    control: Rect::new(area.x, area.y + 1, area.width, height),
                    message_x: area.x,
                    message_width: area.width,
                }
            }
        };
        // The control is painted first so the label can tell whether focus is inside it.
        cx.paint_child(body, parts.control);

        let mut states = Vec::new();
        if cx.has_focus_within() {
            states.push(State::Focus);
        }
        if self.disabled {
            states.push(State::Disabled);
        }
        let label_style = cx.style("field-label", None, &states).text();
        let label = text::truncate(&self.label, parts.label.width).into_owned();
        cx.text(parts.label.x, parts.label.y, &label, label_style, parts.label.width);
        if let Some(rect) = parts.required {
            let style = cx.style("field-required", None, &states).text();
            let word = text::truncate(&required, rect.width).into_owned();
            cx.text(rect.x, rect.y, &word, style, rect.width);
        }

        let (lines, error) = self.message_lines(parts.message_width, marker_width);
        // The hint starts under the required word when the word sits under the control.
        let top = match parts.required {
            Some(rect) if rect.x == parts.control.x && rect.y >= parts.control.bottom() => rect.bottom(),
            _ => parts.control.bottom(),
        };
        let indent = if error { i32::from(marker_width) + 1 } else { 0 };
        let style = cx.style(if error { "field-error" } else { "field-hint" }, None, &states).text();
        if error {
            cx.text(parts.message_x, top, &marker, style, marker_width);
        }
        let width = clamp_u16(i32::from(parts.message_width) - indent);
        for (y, line) in (top..).zip(lines) {
            cx.text(parts.message_x + indent, y, &line, style, width);
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.body
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.body
    }
}
