//! Form fields: a label, a control, and a hint or an error.

use crate::env::Env;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::text;
use crate::theme::State;
use crate::widget::{Axis, Container, Flex, Length, MeasureCx, Node, PaintCx, Widget};

use super::cells;
use super::toast::ToastKind;

/// Cells between the label column and the control when labels sit beside controls.
const LABEL_GAP: u16 = 2;

/// The narrowest control a field keeps beside its label; below it the label moves above.
const MIN_CONTROL: u16 = 16;

/// The width a control is measured in to learn how wide it wants to be. One that answers with all
/// of it fills whatever it is given.
const UNBOUNDED: u16 = 4096;

/// A labelled control: the label, the control the application adds inside, and under it a
/// faint hint or, when there is one, a warning or an error with its sign in the tone of its kind.
///
/// The field only lays things out. The application owns the value and its error, and marks the
/// control itself as invalid (`TextInput::invalid`). A warning is the application's own sentence,
/// drawn where the error is drawn and kept out of [`FormErrors`](super::FormErrors), so a form
/// whose only problem is a warning still submits. The label brightens while the control has
/// focus. Required fields show a faint word after the label; nothing is starred.
///
/// Labels sit above the control by default. With [`Field::label_width`] they take a column of
/// that width beside the control whenever the field is wide enough, and move above it again on
/// narrow screens. A control that asks for more than the room beside the label (an input with a
/// long placeholder) puts its label above as well and takes the whole row instead of being cut.
/// The column is at least as wide as the required word of the active language.
///
/// Style keys: `field-label` (`fg`, `bold`) with states `focus` and `disabled`, and variant
/// `value` for a [value](Field::value) field;
/// `field-required`, `field-hint`, `field-warning` and `field-error` (`fg`). The required word is
/// `quvyta.form.required`; the error marker is the `error` icon and the warning marker the
/// `warning` icon, the two signs the toasts of those kinds carry.
pub struct Field<Msg> {
    label: String,
    hint: Option<String>,
    warning: Option<String>,
    error: Option<String>,
    required: bool,
    disabled: bool,
    value: bool,
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
            warning: None,
            error: None,
            required: false,
            disabled: false,
            value: false,
            label_width: None,
            body: vec![Node::new(Flex::new(Axis::Column, Vec::new()), 0)],
        }
    }

    /// Shows `text` where the control would be: a value to read, not to change, such as the
    /// folder a program installs into, lined up with the fields around it. The label is faint,
    /// since there is nothing to fill in, and anything added inside is replaced by the value.
    #[must_use]
    pub fn value(mut self, text: impl Into<String>) -> Self {
        self.value = true;
        self.body = vec![Node::new(Flex::new(Axis::Column, vec![Node::new(super::Text::new(text), 0)]), 0)];
        self
    }

    /// Faint help under the control, shown while there is neither a warning nor an error.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The error under the control; `None` leaves the place to the warning or the hint. Pass
    /// [`FormErrors::get`](super::FormErrors::get) straight in.
    #[must_use]
    pub fn error<S: Into<String>>(mut self, error: Option<S>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// The warning under the control, in the warning tone with its sign, where the error would be
    /// drawn; an error beside it wins the place.
    ///
    /// A warning says what the value is about to cost, not what is wrong with it, so it never
    /// enters [`FormErrors`](super::FormErrors) and a form whose fields carry only warnings still
    /// submits.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::widgets::{Field, Form, TextInput};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Image(String),
    /// }
    ///
    /// struct Image {
    ///     value: String,
    /// }
    ///
    /// impl App for Image {
    ///     type Msg = Msg;
    ///     fn update(&mut self, msg: Msg) -> Command<Msg> {
    ///         match msg {
    ///             Msg::Image(value) => self.value = value,
    ///         }
    ///         Command::none()
    ///     }
    ///     fn view(&self, ui: &mut View<'_, Msg>) {
    ///         // A warning is the application's own sentence, so it is built from the value beside it.
    ///         let warning = (!self.value.ends_with(":latest")).then(|| "Not pinned to a version");
    ///         Form::new().show(ui, |form| {
    ///             form.field(Field::new("Image").warning(warning), |ui| {
    ///                 ui.add(TextInput::new(&self.value).on_change(Msg::Image)).id("image");
    ///             });
    ///         });
    ///     }
    /// }
    ///
    /// let mut app = Harness::new(Image { value: String::from("nginx:1.27") }, 40, 4);
    /// assert!(app.screen().contains("Not pinned to a version"));
    /// ```
    #[must_use]
    pub fn warning<S: Into<String>>(mut self, warning: Option<S>) -> Self {
        self.warning = warning.map(Into::into);
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
    ///
    /// `natural` is the width the control asks for with no limit. A control wider than the room
    /// beside the label would be cut there, so its label goes above and it gets the whole row; a
    /// control that takes whatever it is given stays beside.
    fn beside(&self, width: u16, natural: u16) -> Option<u16> {
        self.label_width.filter(|label| {
            let room = width.saturating_sub(cells::sum([*label, LABEL_GAP]));
            room >= MIN_CONTROL && (natural <= room || natural >= UNBOUNDED)
        })
    }

    /// Whether the required word, too wide for the label column `column`, goes under the control
    /// instead, on its own line before the hint. Growing the column would take room from every
    /// control of the form, and cutting the word would lose it.
    fn required_under_control(&self, column: u16, word: &str) -> bool {
        self.required && text::width(word) > column
    }

    /// The message under the control, and the status it reports. The error takes the place from
    /// the warning and the warning from the hint, so a control never says two things where one
    /// fits.
    fn message(&self) -> Option<Message<'_>> {
        if let Some(error) = &self.error {
            return Some(Message { text: error, kind: Some(ToastKind::Danger), style: "field-error" });
        }
        if let Some(warning) = &self.warning {
            return Some(Message { text: warning, kind: Some(ToastKind::Warning), style: "field-warning" });
        }
        self.hint.as_deref().map(|hint| Message { text: hint, kind: None, style: "field-hint" })
    }
}

/// The one sentence a field shows under its control.
#[derive(Clone, Copy)]
struct Message<'a> {
    /// The sentence as the application wrote it.
    text: &'a str,
    /// The status the sentence reports, which gives it its sign and its colour; `None` for the
    /// hint, which is neither a status nor a colour.
    kind: Option<ToastKind>,
    /// The style key the sign and the sentence take together.
    style: &'static str,
}

impl Message<'_> {
    /// The glyph that signs the sentence in this glyph mode; nothing for the hint.
    fn sign(&self, env: &Env) -> Option<String> {
        self.kind.map(|kind| env.icons().glyph(kind.icon()).into_owned())
    }

    /// The cells the sign and the gap after it take in front of the sentence, which is where the
    /// hint starts when it has no sign.
    fn indent(&self, sign_width: u16) -> u16 {
        self.kind.map_or(0, |_| sign_width.saturating_add(1))
    }

    /// The lines the sentence takes in `width` cells, once the sign has had its part.
    fn lines(&self, width: u16, sign_width: u16) -> Vec<String> {
        text::wrap(self.text, width.saturating_sub(self.indent(sign_width)))
    }
}

/// The width of the sign of `message` in the active glyph mode; nothing for the hint.
fn sign_width(env: &Env, message: Option<&Message<'_>>) -> u16 {
    message.map_or(0, |message| text::width(&message.sign(env).unwrap_or_default()))
}

impl<Msg: 'static> Container<Msg> for Field<Msg> {
    fn set_children(&mut self, children: Vec<Node<Msg>>) {
        // A value field shows its value; a control added to it has no place.
        if self.value {
            return;
        }
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
        let message = self.message();
        let sign_width = sign_width(cx.env(), message.as_ref());
        let label_width = text::width(&self.label);
        let required = if self.required { text::width(&required_word()) } else { 0 };
        let Some(body) = self.body.first() else {
            return Size::default();
        };
        let natural = cx.measure_child(body, Size::new(UNBOUNDED, available.height)).width;
        if let Some(column) = self.beside(available.width, natural) {
            let control_width = available.width - column - LABEL_GAP;
            let control = cx.measure_child(body, Size::new(control_width, available.height));
            let lines = message.map(|message| message.lines(control_width, sign_width)).unwrap_or_default();
            let below = self.required_under_control(column, &required_word());
            let label_rows = 1 + u16::from(self.required && !below);
            let messages = clamp_u16(i32::try_from(lines.len()).unwrap_or(i32::MAX)).saturating_add(u16::from(below));
            let right = control.height.saturating_add(messages);
            return Size::new(available.width, label_rows.max(right)).min(available);
        }
        let control = cx.measure_child(body, Size::new(available.width, available.height.saturating_sub(1)));
        let lines = message.map(|message| message.lines(available.width, sign_width)).unwrap_or_default();
        let widest_line = lines
            .iter()
            .map(|line| text::width(line))
            .max()
            .unwrap_or(0)
            .saturating_add(message.map_or(0, |message| message.indent(sign_width)));
        let label_line = label_width.saturating_add(if self.required { required.saturating_add(2) } else { 0 });
        let height = cells::sum([1, control.height, clamp_u16(i32::try_from(lines.len()).unwrap_or(i32::MAX))]);
        Size::new(label_line.max(control.width).max(widest_line), height).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let Some(body) = self.body.first() else {
            return;
        };
        let message = self.message();
        let sign = message.as_ref().and_then(|message| message.sign(cx.env()));
        let sign_width = sign_width(cx.env(), message.as_ref());
        let required = required_word();
        let natural = cx.measure_child(body, Size::new(UNBOUNDED, area.height)).width;
        let parts = match self.beside(area.width, natural) {
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
        let label_style = cx.style("field-label", self.value.then_some("value"), &states).text();
        let label = text::truncate(&self.label, parts.label.width).into_owned();
        cx.text(parts.label.x, parts.label.y, &label, label_style, parts.label.width);
        if let Some(rect) = parts.required {
            let style = cx.style("field-required", None, &states).text();
            let word = text::truncate(&required, rect.width).into_owned();
            cx.text(rect.x, rect.y, &word, style, rect.width);
        }

        // The message starts under the required word when the word sits under the control.
        let top = match parts.required {
            Some(rect) if rect.x == parts.control.x && rect.y >= parts.control.bottom() => rect.bottom(),
            _ => parts.control.bottom(),
        };
        let Some(message) = message else { return };
        let indent = i32::from(message.indent(sign_width));
        let mut style = cx.style(message.style, None, &states).text();
        // A theme that gives the message no colour of its own still gets the tone of the status it
        // reports, so a warning never reads as the hint it stands in for.
        style.fg = style.fg.or(message.kind.map(|kind| cx.color(kind.name())));
        if let Some(sign) = &sign {
            cx.text(parts.message_x, top, sign, style, sign_width);
        }
        let width = clamp_u16(i32::from(parts.message_width) - indent);
        for (y, line) in (top..).zip(message.lines(parts.message_width, sign_width)) {
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
