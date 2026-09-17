//! Wizards: a multi-step flow with steps on top, one page per step and Back, Next and Finish.

use super::{Button, Steps};
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::widget::{Axis, EventCx, Flex, Length, MeasureCx, Node, NodeMut, PaintCx, View, Widget};

/// Rows between the steps, the page and the buttons.
const SECTION_GAP: u16 = 1;

/// A flow of pages the user goes through in order: [`Steps`] on top, the current step's page,
/// and a row of buttons.
///
/// The application owns the current step and every value. Next sends the next message; the
/// application validates the step there and either advances or returns
/// [`FormErrors::focus_first`](super::FormErrors::focus_first), so a broken step blocks the flow
/// with focus on the problem. On the last step Next reads Finish and sends the finish message.
/// Back is shown from the second step on.
///
/// Capabilities, each off until asked for: [`Wizard::on_cancel`] adds a Cancel button and makes
/// Esc inside the wizard cancel; [`Wizard::on_step`] lets people go back by choosing a finished
/// step; [`Wizard::busy`] shows Next working and ignores it; [`Wizard::page_height`] keeps the
/// buttons in place across pages of different heights.
///
/// The buttons are named `wizard-cancel`, `wizard-back` and `wizard-next`, so
/// `Command::focus("wizard-next")` reaches them. Their words are `quvyta.wizard.cancel`,
/// `back`, `next` and `finish`.
pub struct Wizard<Msg> {
    labels: Vec<String>,
    current: usize,
    on_back: Option<Msg>,
    on_next: Option<Msg>,
    on_finish: Option<Msg>,
    on_cancel: Option<Msg>,
    on_step: Option<Box<dyn Fn(usize) -> Msg>>,
    busy: bool,
    page_height: Option<u16>,
}

impl<Msg: Clone + 'static> Wizard<Msg> {
    /// A wizard with steps named `labels`, on the first step.
    #[must_use]
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            current: 0,
            on_back: None,
            on_next: None,
            on_finish: None,
            on_cancel: None,
            on_step: None,
            busy: false,
            page_height: None,
        }
    }

    /// The current step.
    #[must_use]
    pub fn current(mut self, index: usize) -> Self {
        self.current = index;
        self
    }

    /// Message for Back.
    #[must_use]
    pub fn on_back(mut self, message: Msg) -> Self {
        self.on_back = Some(message);
        self
    }

    /// Message for Next on every step but the last.
    #[must_use]
    pub fn on_next(mut self, message: Msg) -> Self {
        self.on_next = Some(message);
        self
    }

    /// Message for Finish on the last step.
    #[must_use]
    pub fn on_finish(mut self, message: Msg) -> Self {
        self.on_finish = Some(message);
        self
    }

    /// Adds a Cancel button; Esc inside the wizard sends the same message.
    #[must_use]
    pub fn on_cancel(mut self, message: Msg) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Lets finished steps be chosen to go back to them; the message carries the step.
    #[must_use]
    pub fn on_step(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_step = Some(Box::new(message));
        self
    }

    /// Shows Next (or Finish) working and ignores it, e.g. while the last step is applied.
    #[must_use]
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Gives every page exactly `rows` rows, so the buttons stay put between steps.
    #[must_use]
    pub fn page_height(mut self, rows: u16) -> Self {
        self.page_height = Some(rows);
        self
    }

    /// Adds the wizard to `ui` with the current step's page built by `page`.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>, page: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'v, Msg> {
        let env = ui.env();
        let mut children = Vec::new();
        {
            let ui = &mut View::new(&mut children, env);
            let mut steps = Steps::new(self.labels.clone()).current(self.current).running(self.busy);
            if let Some(message) = self.on_step {
                steps = steps.on_select(message);
            }
            ui.add(steps).id("wizard-steps");
            let body = ui.column(page).fill_width().id("wizard-page");
            if let Some(rows) = self.page_height {
                body.height(Length::Cells(rows));
            }
            let last = self.current + 1 >= self.labels.len();
            let (next_label, next) = if last {
                (crate::t!("quvyta.wizard.finish"), self.on_finish.clone())
            } else {
                (crate::t!("quvyta.wizard.next"), self.on_next.clone())
            };
            let busy = self.busy;
            let (on_back, on_cancel, current) = (self.on_back.clone(), self.on_cancel.clone(), self.current);
            ui.row(|ui| {
                if let Some(cancel) = on_cancel {
                    ui.add(Button::new(crate::t!("quvyta.wizard.cancel")).disabled(busy).on_press(cancel))
                        .id("wizard-cancel");
                }
                ui.spacer();
                if current > 0
                    && let Some(back) = on_back
                {
                    ui.add(Button::new(crate::t!("quvyta.wizard.back")).disabled(busy).on_press(back))
                        .id("wizard-back");
                }
                let mut button = Button::new(next_label).variant("primary").loading(busy);
                if let Some(message) = next {
                    button = button.on_press(message);
                }
                ui.add(button).id("wizard-next");
            })
            .gap(2)
            .fill_width();
        }
        let mut column = Node::new(Flex::new(Axis::Column, children), 0);
        column.layout.gap = SECTION_GAP;
        column.layout.width = Length::Fill(1);
        ui.add(WizardFrame { body: vec![column], on_cancel: self.on_cancel }).fill_width()
    }
}

/// The wizard's outer node: lays out its column and turns Esc into the cancel message.
struct WizardFrame<Msg> {
    body: Vec<Node<Msg>>,
    on_cancel: Option<Msg>,
}

impl<Msg: Clone + 'static> Widget<Msg> for WizardFrame<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.body.first().map_or(Size::default(), |body| cx.measure_child(body, available))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if let Some(body) = self.body.first() {
            cx.paint_child(body, area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        match (event, &self.on_cancel) {
            (Event::Key(key), Some(message)) if key.is_plain(Key::Esc) => {
                cx.emit(message.clone());
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
    use crate::widgets::{FormErrors, TextInput};

    #[derive(Default)]
    struct Setup {
        step: usize,
        name: String,
        errors: FormErrors,
        finished: bool,
        cancelled: bool,
        cancellable: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Name(String),
        Back,
        Next,
        Finish,
        Cancel,
    }

    impl App for Setup {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Name(name) => self.name = name,
                Msg::Back => self.step -= 1,
                Msg::Next => {
                    self.errors.check("name", !self.name.is_empty(), "Name the project");
                    if !self.errors.is_empty() {
                        return self.errors.focus_first();
                    }
                    self.step += 1;
                }
                Msg::Finish => self.finished = true,
                Msg::Cancel => self.cancelled = true,
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let mut wizard = Wizard::new(["Project", "Summary"])
                .current(self.step)
                .on_back(Msg::Back)
                .on_next(Msg::Next)
                .on_finish(Msg::Finish);
            if self.cancellable {
                wizard = wizard.on_cancel(Msg::Cancel);
            }
            wizard.page_height(2).show(ui, |ui| {
                if self.step == 0 {
                    ui.add(TextInput::new(&self.name).on_change(Msg::Name)).width(Length::Cells(20)).id("name");
                } else {
                    ui.add(crate::widgets::Text::new(format!("Create {}", self.name)));
                }
            });
        }
    }

    #[test]
    fn next_is_blocked_by_validation_then_advances_and_finishes() {
        let mut h = Harness::new(Setup::default(), 40, 8);
        assert_eq!(h.screen(), "●  Project   ○  Summary\n\n  ❯\n\n\n                                  Next\n\n\n");
        h.click_text("Next");
        assert_eq!(h.app().step, 0);
        assert!(h.is_focused("name"), "focus goes to the problem");
        h.type_text("web");
        h.click_text("Next");
        assert_eq!(h.app().step, 1);
        let screen = h.screen();
        assert!(screen.contains("✓  Project   ●  Summary"), "{screen}");
        // The pointer rests where Next was, so Finish shows the hover pillar.
        assert!(screen.contains("Back    ▌ Finish"), "{screen}");
        h.click_text("Finish");
        assert!(h.app().finished);
        h.click_text("Back");
        assert_eq!(h.app().step, 0);
    }

    #[test]
    fn cancel_is_a_button_and_esc_only_when_asked_for() {
        let mut h = Harness::new(Setup::default(), 40, 8);
        h.press("tab").press("esc");
        assert!(!h.app().cancelled);
        assert!(!h.screen().contains("Cancel"));
        let mut h = Harness::new(Setup { cancellable: true, ..Setup::default() }, 40, 8);
        h.press("tab").press("esc");
        assert!(h.app().cancelled);
        assert!(h.screen().contains("Cancel"));
    }
}
