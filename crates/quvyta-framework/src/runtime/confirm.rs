//! "Are you sure?" dialogs the runtime shows for [`Command::confirm`](super::Command::confirm).

use std::marker::PhantomData;

use crate::event::{Event, MouseKind};
use crate::geometry::{Rect, Size};
use crate::widget::{EventCx, MeasureCx, Node, PaintCx, Widget, WidgetId};
use crate::widgets::{Button, Modal, Text};

/// A question for [`Command::confirm`](super::Command::confirm): a title, an optional message,
/// and the messages for the answers.
///
/// The dialog has two buttons, Cancel and the confirm button, labelled from
/// `quvyta.confirm.cancel` and `quvyta.confirm.confirm` unless given. Cancel has focus. The
/// question is dismissable by default: Esc and the close mark `×` at the top right cancel, always
/// together; [`dismissable(false)`](Confirm::dismissable) turns both off so only the buttons
/// answer. A click on the dimmed screen never answers, so a stray click is harmless.
pub struct Confirm<Msg> {
    title: String,
    message: Option<String>,
    confirm_label: Option<String>,
    cancel_label: Option<String>,
    danger: bool,
    dismissable: bool,
    on_confirm: Msg,
    on_cancel: Option<Msg>,
}

impl<Msg> Confirm<Msg> {
    /// Asks `title`; confirming sends `on_confirm`.
    #[must_use]
    pub fn new(title: impl Into<String>, on_confirm: Msg) -> Self {
        Self {
            title: title.into(),
            message: None,
            confirm_label: None,
            cancel_label: None,
            danger: false,
            dismissable: true,
            on_confirm,
            on_cancel: None,
        }
    }

    /// Explains the consequences below the title.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Marks the question as destructive: a danger pillar down the dialog's left edge and a
    /// danger confirm button.
    #[must_use]
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// Whether Esc and the close mark cancel the question; `true` by default. With `false`
    /// neither works, the mark is not drawn and only the two buttons answer.
    #[must_use]
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
        self
    }

    /// The confirm button's label, e.g. the action's own verb: "Remove".
    #[must_use]
    pub fn confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm_label = Some(label.into());
        self
    }

    /// The cancel button's label.
    #[must_use]
    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = Some(label.into());
        self
    }

    /// The message sent when the question is cancelled; without it cancelling only closes the
    /// dialog.
    #[must_use]
    pub fn on_cancel(mut self, message: Msg) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// The message for an answer.
    pub(crate) fn into_answer(self, confirmed: bool) -> Option<Msg> {
        if confirmed { Some(self.on_confirm) } else { self.on_cancel }
    }
}

/// The dialog's answers, the messages of its own buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Cancel,
    Confirm,
}

/// Which answer button took the pointer down, so the release answers only over that button.
#[derive(Debug, Default)]
struct ConfirmMemory {
    pressed: Option<WidgetId>,
}

/// The dialog of the topmost pending [`Confirm`]. It is added after the application's view and
/// draws a [`Modal`] of [`Answer`]s; the runtime turns the answer into the application's
/// message, which never has to be cloned.
pub(crate) struct ConfirmLayer<Msg> {
    title: String,
    message: Option<String>,
    confirm_label: Option<String>,
    cancel_label: Option<String>,
    danger: bool,
    dismissable: bool,
    marker: PhantomData<fn() -> Msg>,
}

impl<Msg> ConfirmLayer<Msg> {
    pub(crate) fn new(confirm: &Confirm<Msg>) -> Self {
        Self {
            title: confirm.title.clone(),
            message: confirm.message.clone(),
            confirm_label: confirm.confirm_label.clone(),
            cancel_label: confirm.cancel_label.clone(),
            danger: confirm.danger,
            dismissable: confirm.dismissable,
            marker: PhantomData,
        }
    }

    /// The dialog, with ids assigned under this layer so focus and hits land on its buttons.
    fn dialog(&self, env: &crate::env::Env, id: WidgetId) -> Modal<Answer> {
        let i18n = env.i18n();
        let cancel = self.cancel_label.clone().unwrap_or_else(|| i18n.translate("quvyta.confirm.cancel", &[]));
        let confirm = self.confirm_label.clone().unwrap_or_else(|| i18n.translate("quvyta.confirm.confirm", &[]));
        let mut confirm_button = Button::new(confirm).on_press(Answer::Confirm);
        confirm_button = confirm_button.variant(if self.danger { "danger" } else { "primary" });
        let mut dialog = Modal::new()
            .title(self.title.clone())
            .on_close(Answer::Cancel)
            .dismissable(self.dismissable)
            .action(Button::new(cancel).on_press(Answer::Cancel))
            .action(confirm_button);
        if self.danger {
            dialog = dialog.variant("danger");
        }
        if let Some(message) = &self.message {
            crate::widget::Container::set_children(&mut dialog, vec![Node::new(Text::new(message.clone()), 0)]);
        }
        for child in dialog.children_mut() {
            child.assign_ids(id);
        }
        dialog
    }
}

impl<Msg: 'static> Widget<Msg> for ConfirmLayer<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, _available: Size) -> Size {
        Size::default()
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.request_overlay(area);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let dialog = self.dialog(cx.env(), cx.id());
        Widget::<Answer>::paint_overlay(&dialog, cx, anchor);
        let rects = dialog.children()[1..]
            .iter()
            .filter_map(|button| cx.frame.rects.get(&button.id()).map(|rect| (button.id(), *rect)))
            .collect();
        cx.memory::<ButtonRects>().0 = rects;
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let dialog = self.dialog(cx.env, cx.id);
        // Esc and the close mark belong to the dialog surface: it reports them as its close
        // message, which here is the cancel answer, and only while the question is dismissable.
        let mut closed = Vec::new();
        let used = {
            let mut dialog_cx = EventCx {
                id: cx.id,
                rect: cx.rect,
                focus_rect: cx.focus_rect,
                env: cx.env,
                memory: &mut *cx.memory,
                interaction: cx.interaction,
                messages: &mut closed,
                effects: &mut *cx.effects,
                now: cx.now,
                persistent: cx.persistent,
            };
            Widget::<Answer>::event(&dialog, &mut dialog_cx, event)
        };
        if closed.contains(&Answer::Cancel) {
            cx.answer(false);
            return true;
        }
        if used {
            return true;
        }
        let buttons = &dialog.children()[1..];
        let target = match event {
            Event::Key(_) => buttons.iter().find(|button| cx.interaction.focused == Some(button.id())),
            Event::Mouse(mouse) => {
                let rects = cx.memory::<ButtonRects>().0.clone();
                let over = |button: &&Node<Answer>| {
                    rects.iter().any(|(id, rect)| *id == button.id() && rect.contains(mouse.x, mouse.y))
                };
                let pressed = cx.memory::<ConfirmMemory>().pressed;
                match mouse.kind {
                    MouseKind::Down(_) => buttons.iter().find(over),
                    _ => buttons.iter().find(|button| pressed == Some(button.id())),
                }
            }
            _ => None,
        };
        let Some(button) = target else {
            // Everything else on the dimmed screen is swallowed.
            return matches!(event, Event::Mouse(_));
        };
        if let Event::Mouse(mouse) = event {
            cx.memory::<ConfirmMemory>().pressed = matches!(mouse.kind, MouseKind::Down(_)).then_some(button.id());
        }
        let mut answers = Vec::new();
        let rect = cx.memory::<ButtonRects>().0.iter().find(|(id, _)| *id == button.id()).map(|(_, rect)| *rect);
        let handled = {
            let mut button_cx = EventCx {
                id: button.id(),
                rect: rect.unwrap_or_default(),
                focus_rect: cx.focus_rect,
                env: cx.env,
                memory: &mut *cx.memory,
                interaction: cx.interaction,
                messages: &mut answers,
                effects: &mut *cx.effects,
                now: cx.now,
                persistent: cx.persistent,
            };
            button.widget.event(&mut button_cx, event)
        };
        if let Some(answer) = answers.first() {
            cx.answer(*answer == Answer::Confirm);
        }
        handled || matches!(event, Event::Mouse(_))
    }
}

/// Where the answer buttons were painted, for routing pointer events to them.
#[derive(Debug, Default)]
struct ButtonRects(Vec<(WidgetId, Rect)>);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::runtime::{App, Command, Confirm, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    #[derive(Default)]
    struct Demo {
        log: Vec<&'static str>,
    }

    #[derive(Clone)]
    enum Msg {
        Ask,
        AskTwice,
        Remove,
        Kept,
        Prune,
        AskFirm,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Ask => {
                    return Command::confirm(
                        Confirm::new("Remove web?", Msg::Remove)
                            .message("Its volumes go too.")
                            .confirm_label("Remove")
                            .danger()
                            .on_cancel(Msg::Kept),
                    );
                }
                Msg::AskTwice => {
                    return Command::batch([
                        Command::confirm(Confirm::new("Prune images?", Msg::Prune)),
                        Command::confirm(Confirm::new("Remove web?", Msg::Remove)),
                    ]);
                }
                Msg::AskFirm => {
                    return Command::confirm(
                        Confirm::new("Rotate keys?", Msg::Remove).dismissable(false).on_cancel(Msg::Kept),
                    );
                }
                Msg::Remove => self.log.push("removed"),
                Msg::Kept => self.log.push("kept"),
                Msg::Prune => self.log.push("pruned"),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.add(Text::new("Containers"));
                ui.add(Button::new("Remove web").on_press(Msg::Ask)).id("ask");
            });
        }
    }

    fn asked() -> Harness<Demo> {
        let mut h = Harness::new(Demo::default(), 60, 14);
        h.press("tab").press("enter").advance(Duration::from_millis(200));
        h
    }

    fn cell(h: &Harness<Demo>, text: &str) -> (u16, u16) {
        let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` on screen:\n{}", h.screen()));
        (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0))
    }

    #[test]
    fn shows_the_question_with_cancel_focused() {
        let h = asked();
        let screen = h.screen();
        // A danger pillar down the left edge and the close mark in the top right corner, on the
        // row above the title.
        assert!(
            screen.lines().nth(4).is_some_and(|l| l.contains("▌  Remove web?") && !l.contains('×'))
                && screen.lines().nth(3).is_some_and(|l| l.ends_with('×')),
            "{screen}"
        );
        for row in 3..=9 {
            assert_eq!(h.fg(2, row), h.env().theme().color("danger"), "pillar on row {row}:\n{screen}");
        }
        assert!(screen.contains("Its volumes go too."));
        let (x, y) = cell(&h, "Cancel");
        assert_ne!(h.bg(x, y), h.env().theme().color("raised"), "Cancel has focus: {screen}");
        let (x, y) = cell(&h, "Remove  ");
        assert_ne!(h.bg(x, y), h.env().theme().color("raised"), "Remove is a danger button");
    }

    #[test]
    fn enter_on_the_safe_default_cancels_and_escape_cancels() {
        let mut h = asked();
        h.press("enter");
        assert_eq!(h.app().log, ["kept"]);
        assert!(!h.screen().contains("Remove web?"));
        assert!(h.is_focused("ask"), "focus returns to the button that asked");
        h.press("enter").advance(Duration::from_millis(200)).press("esc");
        assert_eq!(h.app().log, ["kept", "kept"]);
    }

    #[test]
    fn tab_then_enter_or_a_click_confirms() {
        let mut h = asked();
        h.press("tab").press("enter");
        assert_eq!(h.app().log, ["removed"]);
        h.press("enter").advance(Duration::from_millis(200));
        h.click_text("Containers");
        assert!(h.screen().contains("Remove web?"), "clicks on the dimmed screen do not answer");
        let (x, y) = cell(&h, "Remove  ");
        h.click(i32::from(x), i32::from(y));
        assert_eq!(h.app().log, ["removed", "removed"]);
    }

    #[test]
    fn the_close_mark_cancels_and_lights_three_cells() {
        let mut h = asked();
        let (x, y) = cell(&h, "×");
        assert_eq!(cell(&h, "Remove web?").1, y + 1, "the mark sits on the surface's first row");
        let resting = h.bg(x, y);
        h.hover(i32::from(x) + 1, i32::from(y));
        let lit = h.bg(x, y);
        assert_ne!(lit, resting);
        assert_eq!((h.bg(x - 1, y), h.bg(x + 1, y)), (lit, lit));
        h.click(i32::from(x), i32::from(y));
        assert_eq!(h.app().log, ["kept"], "the mark answers like Esc: cancel");
        assert!(!h.screen().contains("Remove web?"));
    }

    #[test]
    fn a_question_that_is_not_dismissable_answers_only_with_its_buttons() {
        let mut h = Harness::new(Demo::default(), 60, 14);
        h.send(Msg::AskFirm).advance(Duration::from_millis(200));
        let screen = h.screen();
        assert!(screen.contains("Rotate keys?") && !screen.contains('×') && !screen.contains("esc"), "{screen}");
        h.press("esc").click(57, 4).click(1, 12);
        assert!(h.app().log.is_empty() && h.screen().contains("Rotate keys?"), "{}", h.screen());
        h.press("enter");
        assert_eq!(h.app().log, ["kept"]);
    }

    #[test]
    fn questions_stack_and_the_newest_is_answered_first() {
        let mut h = Harness::new(Demo::default(), 60, 14);
        h.send(Msg::AskTwice).advance(Duration::from_millis(200));
        assert!(h.screen().contains("Remove web?"));
        h.press("tab").press("enter").advance(Duration::from_millis(200));
        assert!(h.screen().contains("Prune images?"), "{}", h.screen());
        h.press("tab").press("enter");
        assert_eq!(h.app().log, ["removed", "pruned"]);
    }
}
