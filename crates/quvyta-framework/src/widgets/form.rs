//! Forms: fields laid out together, an error summary and Enter moving to the next field.

use super::cells;
use super::{Field, FormErrors};
use crate::event::Event;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::text;
use crate::widget::{Axis, EventCx, Flex, Length, MeasureCx, Node, NodeMut, PaintCx, View, Widget};

/// Rows between the error summary and the first field.
const SUMMARY_GAP: u16 = 1;

/// A column of [`Field`]s that share one label layout, with an optional summary of problems
/// above them.
///
/// Enter in a field whose control does not use it (a text input without `on_submit`) moves
/// focus to the next field, and from the last field on to whatever follows the form, usually
/// its submit button. The form does not validate: the application checks its values into
/// [`FormErrors`], passes each message to its field and, on submit, returns
/// [`FormErrors::focus_first`].
///
/// Style keys: `form-summary` (`bg`, `padding`), `form-summary-title` (`fg`, `bold`),
/// `form-summary-marker` and `form-summary-item` (`fg`). The summary title is
/// `quvyta.form.summary` with `n` problems; its marker is the `error` icon.
pub struct Form<Msg> {
    label_width: Option<u16>,
    gap: u16,
    summary: Vec<String>,
    body: Vec<Node<Msg>>,
}

/// Adds fields to a [`Form`] inside [`Form::show`].
pub struct FormFields<'v, 'a, Msg> {
    ui: &'v mut View<'a, Msg>,
    label_width: Option<u16>,
}

impl<'a, Msg: 'static> FormFields<'_, 'a, Msg> {
    /// Adds `field` with the control built by `control`. The field takes the form's label
    /// width unless it has its own.
    pub fn field(&mut self, mut field: Field<Msg>, control: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        if field.label_width.is_none() {
            field.label_width = self.label_width;
        }
        self.ui.add_with(field, control).fill_width()
    }

    /// The form's column, for content between fields such as a sub-heading.
    pub fn ui(&mut self) -> &mut View<'a, Msg> {
        self.ui
    }
}

impl<Msg: 'static> Form<Msg> {
    /// A form with labels above controls and a row between fields.
    #[must_use]
    pub fn new() -> Self {
        Self { label_width: None, gap: 1, summary: Vec::new(), body: Vec::new() }
    }

    /// Puts every field's label in a column `cells` wide beside its control while the form is
    /// wide enough; on narrow screens labels move above controls.
    #[must_use]
    pub fn label_width(mut self, cells: u16) -> Self {
        self.label_width = Some(cells);
        self
    }

    /// Rows between fields; 1 by default.
    #[must_use]
    pub fn gap(mut self, rows: u16) -> Self {
        self.gap = rows;
        self
    }

    /// Lists every problem of `errors` above the fields. Nothing is shown while there are none.
    #[must_use]
    pub fn summary(mut self, errors: &FormErrors) -> Self {
        self.summary = errors.iter().map(|(_, message)| message.to_owned()).collect();
        self
    }

    /// Adds the form to `ui` with the fields `build` adds.
    pub fn show<'v>(
        mut self,
        ui: &'v mut View<'_, Msg>,
        build: impl FnOnce(&mut FormFields<'_, '_, Msg>),
    ) -> NodeMut<'v, Msg> {
        let env = ui.env();
        let mut children = Vec::new();
        build(&mut FormFields { ui: &mut View::new(&mut children, env), label_width: self.label_width });
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.gap = self.gap;
        column.layout.width = Length::Fill(1);
        self.body = vec![column];
        ui.add(self).fill_width()
    }

    /// Rows the error summary takes with `vertical_padding` above and below its lines, and the
    /// gap after it; nothing while there are no problems.
    fn summary_height(&self, vertical_padding: u16) -> u16 {
        if self.summary.is_empty() {
            return 0;
        }
        let rows = clamp_u16(i32::try_from(self.summary.len()).unwrap_or(i32::MAX)).saturating_add(1);
        cells::sum([rows, vertical_padding.saturating_mul(2), SUMMARY_GAP])
    }
}

impl<Msg: 'static> Default for Form<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: 'static> Widget<Msg> for Form<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let padding = cx.env().theme().style("form-summary", None, &[]).pair("padding").unwrap_or((1, 2));
        let summary = self.summary_height(padding.0);
        let body = self.body.first().map_or(Size::default(), |body| {
            cx.measure_child(body, Size::new(available.width, available.height.saturating_sub(summary)))
        });
        let widest = self
            .summary
            .iter()
            .map(|line| cells::sum([text::width(line), padding.1.saturating_mul(2), 3]))
            .max()
            .unwrap_or(0);
        Size::new(body.width.max(widest), body.height.saturating_add(summary)).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let style = cx.style("form-summary", None, &[]);
        let padding = style.padding();
        let summary_height = self.summary_height(padding.top);
        if !self.summary.is_empty() {
            let block = Rect::new(area.x, area.y, area.width, summary_height - SUMMARY_GAP);
            cx.clear(block, style.text().bg.unwrap_or_else(|| cx.color("surface")));
            let inner = block.inset(padding);
            let marker = cx.env().icons().glyph("error").into_owned();
            let indent = text::width(&marker).saturating_add(2);
            let marker_style = cx.style("form-summary-marker", None, &[]).text();
            cx.text(inner.x, inner.y, &marker, marker_style, inner.width);
            let title = crate::t!("quvyta.form.summary", n = self.summary.len());
            let budget = inner.width.saturating_sub(indent);
            let title_style = cx.style("form-summary-title", None, &[]).text();
            let shown = text::truncate(&title, budget).into_owned();
            cx.text(inner.x + i32::from(indent), inner.y, &shown, title_style, budget);
            let item_style = cx.style("form-summary-item", None, &[]).text();
            for (row, message) in self.summary.iter().enumerate() {
                let y = inner.y + 1 + i32::try_from(row).unwrap_or(i32::MAX);
                let shown = text::truncate(message, budget).into_owned();
                cx.text(inner.x + i32::from(indent), y, &shown, item_style, budget);
            }
        }
        if let Some(body) = self.body.first() {
            let rest = Rect::new(
                area.x,
                area.y + i32::from(summary_height),
                area.width,
                area.height.saturating_sub(summary_height),
            );
            cx.paint_child(body, rest);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        match event {
            Event::Key(key) if key.is_plain(Key::Enter) => {
                cx.focus_next();
                true
            }
            _ => false,
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.body
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widgets::{Button, TextInput};

    #[derive(Default)]
    struct Signup {
        name: String,
        image: String,
        errors: FormErrors,
        submitted: bool,
        label_width: Option<u16>,
        summary: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Name(String),
        Image(String),
        Submit,
    }

    impl Signup {
        fn validate(&mut self) {
            self.errors.check("name", self.name.chars().count() >= 3, "Use at least 3 characters");
            self.errors.check("image", !self.image.is_empty(), "Choose an image");
        }
    }

    impl App for Signup {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Name(name) => self.name = name,
                Msg::Image(image) => self.image = image,
                Msg::Submit => {
                    self.validate();
                    self.submitted = self.errors.is_empty();
                    return self.errors.focus_first();
                }
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let mut form = Form::new();
            if let Some(width) = self.label_width {
                form = form.label_width(width);
            }
            if self.summary {
                form = form.summary(&self.errors);
            }
            form.show(ui, |form| {
                form.field(Field::new("Name").required(true).hint("Lowercase").error(self.errors.get("name")), |ui| {
                    ui.add(TextInput::new(&self.name).invalid(self.errors.has("name")).on_change(Msg::Name))
                        .width(Length::Cells(20))
                        .id("name");
                });
                form.field(Field::new("Image").error(self.errors.get("image")), |ui| {
                    ui.add(TextInput::new(&self.image).on_change(Msg::Image)).width(Length::Cells(20)).id("image");
                });
            });
            ui.add(Button::new("Create").on_press(Msg::Submit)).id("create");
        }
    }

    #[test]
    fn labels_above_with_required_word_hint_and_error() {
        let mut h = Harness::new(Signup::default(), 40, 8);
        assert_eq!(h.screen(), "Name  required\n  ❯\nLowercase\n\nImage\n  ❯\n  Create\n\n");
        h.press("tab").type_text("ab");
        h.send(Msg::Submit);
        assert_eq!(h.screen().lines().nth(2), Some("✕ Use at least 3 characters"));
        assert_eq!(h.fg(2, 2), Some(h.env().theme().color("danger").expect("danger")));
        assert!(h.is_focused("name"), "submit focuses the first problem");
    }

    #[test]
    fn label_brightens_while_its_control_has_focus() {
        let mut h = Harness::new(Signup::default(), 40, 8);
        let idle = h.fg(0, 0);
        h.press("tab");
        assert_ne!(h.fg(0, 0), idle);
        assert!(h.is_bold(0, 0));
    }

    #[test]
    fn enter_moves_to_the_next_field_and_then_to_the_button() {
        let mut h = Harness::new(Signup::default(), 40, 8);
        h.press("tab").press("enter");
        assert!(h.is_focused("image"));
        h.press("enter");
        assert!(h.is_focused("create"));
        h.press("enter");
        assert!(!h.app().submitted);
        assert!(h.is_focused("name"));
    }

    #[test]
    fn labels_beside_controls_fall_back_above_when_narrow() {
        let app = Signup { label_width: Some(10), ..Signup::default() };
        let mut h = Harness::new(app, 40, 6);
        assert_eq!(h.screen(), "Name          ❯\nrequired    Lowercase\n\nImage         ❯\n  Create\n\n");
        let app = Signup { label_width: Some(10), ..Signup::default() };
        h = Harness::new(app, 24, 7);
        assert!(h.screen().starts_with("Name  required\n  ❯\n"), "{}", h.screen());
    }

    #[test]
    fn summary_lists_problems_above_the_fields() {
        let app = Signup { summary: true, ..Signup::default() };
        let mut h = Harness::new(app, 40, 14);
        assert!(h.screen().starts_with("Name"));
        h.send(Msg::Submit);
        let screen = h.screen();
        assert!(screen.contains("✕  2 fields need attention"), "{screen}");
        assert!(screen.contains("Use at least 3 characters\n"), "{screen}");
    }
}
