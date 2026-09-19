//! "Are you sure?" dialogs the runtime shows for [`Command::confirm`](super::Command::confirm).

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::event::{Event, MouseKind};
use crate::geometry::{Rect, Size};
use crate::i18n::Arg;
use crate::widget::{EventCx, Length, MeasureCx, Node, PaintCx, Widget, WidgetId};
use crate::widgets::{Button, Modal, Span, Text, TextInput};

/// Stands in for the word while the prompt is translated, so the prompt can be split around it
/// and the word drawn in its own tone wherever the language puts it. A private-use character
/// never appears in a translation.
const WORD_MARK: &str = "\u{E000}";

/// A question for [`Command::confirm`](super::Command::confirm): a title, an optional message,
/// and the messages for the answers.
///
/// The dialog has two buttons, Cancel and the confirm button, labelled from
/// `quvyta.confirm.cancel` and `quvyta.confirm.confirm` unless given. Cancel has focus. The
/// question is dismissable by default: Esc and the close mark `×` at the top right cancel, always
/// together; [`dismissable(false)`](Confirm::dismissable) turns both off so only the buttons
/// answer. A click on the dimmed screen never answers, so a stray click is harmless.
///
/// [`alternative`](Confirm::alternative) adds a third way between the two, such as "Save ·
/// Continue · Discard" for recovered work. The buttons then read Cancel, the alternative and the
/// confirm button from left to right, Tab and Shift+Tab visit them in that order, and Cancel
/// still has focus when the dialog opens; Esc and the close mark still cancel.
///
/// [`require_word`](Confirm::require_word) asks the user to type a word before the confirm button
/// works, for actions that cannot be undone.
pub struct Confirm<Msg> {
    title: String,
    message: Option<String>,
    confirm_label: Option<String>,
    cancel_label: Option<String>,
    danger: bool,
    dismissable: bool,
    on_confirm: Msg,
    on_cancel: Option<Msg>,
    alternative: Option<(String, Msg)>,
    /// The word the user types to unlock the confirm button.
    word: Option<String>,
    /// Set by the dialog when the alternative is chosen. The runtime reports an answer as
    /// confirmed or not; the alternative travels as a confirmation with this flag set, which
    /// [`into_answer`](Self::into_answer) reads.
    alternative_chosen: Arc<AtomicBool>,
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
            alternative: None,
            word: None,
            alternative_chosen: Arc::new(AtomicBool::new(false)),
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
    /// neither works, the mark is not drawn and only the buttons answer.
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

    /// A third answer between Cancel and the confirm button: a button labelled `label` that sends
    /// `message`, such as "Continue" beside "Discard" and "Save". Without it the dialog has its
    /// two buttons exactly as before.
    #[must_use]
    pub fn alternative(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.alternative = Some((label.into(), message));
        self
    }

    /// Asks the user to type `word` before confirming, for an action that cannot be undone, such
    /// as emptying the trash for good: the dialog gains a line "Type `word` to confirm"
    /// (`quvyta.confirm.type-word`, with the word in the `title` tone among `secondary` text)
    /// and a text field below the message.
    ///
    /// The field has focus when the dialog opens and each question starts with it empty. The
    /// confirm button is disabled, drawn in the disabled tone and passed over by Tab, until the
    /// typed text matches the word; then it takes its danger or primary tone and Enter in the
    /// field confirms too. Before that Enter does nothing and the dialog stays. Esc and the close
    /// mark still cancel, and Tab and Shift+Tab visit the field, Cancel, the
    /// [`alternative`](Self::alternative) and the confirm button in that order.
    ///
    /// Only the confirm button waits for the word: an alternative is a different, safe way out
    /// and works at once.
    ///
    /// Matching ignores spaces around the text and case, the Turkish way included: `İ`, `I`, `ı`
    /// and `i` are all the same letter, so "sil", "SİL", "SIL" and " Sil " all match "SİL", and
    /// "iptal" matches "İPTAL". Typing the word is meant as a deliberate act, not a secret, so a
    /// keyboard's idea of the dotted and dotless i never stands in the way. A blank word asks for
    /// nothing and leaves the dialog as it is without this option.
    ///
    /// Keep it for what cannot be undone; a question that always asks for typing teaches people
    /// to type without reading.
    #[must_use]
    pub fn require_word(mut self, word: impl Into<String>) -> Self {
        self.word = Some(word.into()).filter(|word| !word.trim().is_empty());
        self
    }

    /// The same question answered with `map(message)`.
    pub(crate) fn map<B>(self, map: impl Fn(Msg) -> B) -> Confirm<B> {
        Confirm {
            title: self.title,
            message: self.message,
            confirm_label: self.confirm_label,
            cancel_label: self.cancel_label,
            danger: self.danger,
            dismissable: self.dismissable,
            on_confirm: map(self.on_confirm),
            on_cancel: self.on_cancel.map(&map),
            alternative: self.alternative.map(|(label, message)| (label, map(message))),
            word: self.word,
            alternative_chosen: self.alternative_chosen,
        }
    }

    /// The message for an answer; a confirmation stands for the alternative when the dialog
    /// recorded that choice.
    pub(crate) fn into_answer(self, confirmed: bool) -> Option<Msg> {
        match self.alternative {
            Some((_, message)) if confirmed && self.alternative_chosen.load(Ordering::Relaxed) => Some(message),
            _ if confirmed => Some(self.on_confirm),
            _ => self.on_cancel,
        }
    }
}

/// `text` for comparing typed words: without surrounding spaces, in lower case, with `İ`, `I`,
/// `ı` and `i` all folded to `i`. A plain lower-casing would turn `İ` into `i` and a combining
/// dot, and keep `ı` apart from `i`.
fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    for c in text.trim().chars() {
        match c {
            'İ' | 'I' | 'ı' | 'i' => folded.push('i'),
            _ => folded.extend(c.to_lowercase()),
        }
    }
    folded
}

/// The dialog's answers, the messages of its own buttons and of the word's field.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Answer {
    Cancel,
    Alternative,
    Confirm,
    /// The field's text after an edit.
    Typed(String),
    /// Enter in the field.
    Submit,
}

/// Which part took the pointer down, so the release answers only over that button and a drag
/// that began in the field keeps selecting there.
#[derive(Debug, Default)]
struct ConfirmMemory {
    pressed: Option<WidgetId>,
}

/// The text typed into the word's field. It lives in the layer's memory, which is keyed by the
/// question's own id, so every question starts empty.
#[derive(Debug, Default)]
struct TypedWord(String);

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
    alternative_label: Option<String>,
    word: Option<String>,
    alternative_chosen: Arc<AtomicBool>,
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
            alternative_label: confirm.alternative.as_ref().map(|(label, _)| label.clone()),
            word: confirm.word.clone(),
            alternative_chosen: Arc::clone(&confirm.alternative_chosen),
            marker: PhantomData,
        }
    }

    /// Whether the confirm button works: always without a word, and once `typed` matches it.
    fn unlocked(&self, typed: &str) -> bool {
        self.word.as_deref().is_none_or(|word| fold(word) == fold(typed))
    }

    /// The dialog, with ids assigned under this layer so focus and hits land on its buttons and
    /// its field, which shows `typed`.
    fn dialog(&self, env: &crate::env::Env, id: WidgetId, typed: &str) -> Modal<Answer> {
        let i18n = env.i18n();
        let cancel = self.cancel_label.clone().unwrap_or_else(|| i18n.translate("quvyta.confirm.cancel", &[]));
        let confirm = self.confirm_label.clone().unwrap_or_else(|| i18n.translate("quvyta.confirm.confirm", &[]));
        let mut confirm_button = Button::new(confirm).on_press(Answer::Confirm).disabled(!self.unlocked(typed));
        confirm_button = confirm_button.variant(if self.danger { "danger" } else { "primary" });
        let mut dialog = Modal::new()
            .title(self.title.clone())
            .on_close(Answer::Cancel)
            .dismissable(self.dismissable)
            .action(Button::new(cancel).on_press(Answer::Cancel));
        // Read left to right: the safe answer, the third way, the confirming one; focus visits
        // them in the same order.
        if let Some(label) = &self.alternative_label {
            dialog = dialog.action(Button::new(label.clone()).on_press(Answer::Alternative));
        }
        dialog = dialog.action(confirm_button);
        if self.danger {
            dialog = dialog.variant("danger");
        }
        let mut body = Vec::new();
        if let Some(message) = &self.message {
            body.push(Node::new(Text::new(message.clone()), body.len()));
        }
        if let Some(word) = &self.word {
            let mut prompt = Node::new(prompt(env, word), body.len());
            if self.message.is_some() {
                prompt.layout.padding.top = 1;
            }
            body.push(prompt);
            // The field comes first in the dialog, so it has focus when the question opens.
            let field = TextInput::new(typed).on_change(Answer::Typed).on_submit(|_| Answer::Submit);
            let mut field = Node::new(field, body.len());
            field.layout.width = Length::Fill(1);
            body.push(field);
        }
        if !body.is_empty() {
            crate::widget::Container::set_children(&mut dialog, body);
        }
        for child in dialog.children_mut() {
            child.assign_ids(id);
        }
        dialog
    }

    /// Hands `event` to `part` of the dialog painted at `rect`, as if it had received it itself:
    /// it keeps its own memory, its requests (focus, captures, copies, the paste action) go out
    /// as the layer's, and its answers come back.
    fn forward(cx: &mut EventCx<'_, Msg>, part: &Node<Answer>, rect: Rect, event: &Event) -> (bool, Vec<Answer>) {
        let mut answers = Vec::new();
        let handled = {
            let mut part_cx = EventCx {
                id: part.id(),
                rect,
                focus_rect: cx.focus_rect,
                env: cx.env,
                memory: &mut *cx.memory,
                interaction: cx.interaction,
                messages: &mut answers,
                effects: &mut *cx.effects,
                now: cx.now,
                persistent: cx.persistent,
            };
            part.widget.event(&mut part_cx, event)
        };
        (handled, answers)
    }

    /// Acts on the answers a part gave.
    fn settle(&self, cx: &mut EventCx<'_, Msg>, answers: Vec<Answer>) {
        for answer in answers {
            match answer {
                Answer::Typed(text) => cx.memory::<TypedWord>().0 = text,
                Answer::Submit => {
                    if self.unlocked(&cx.memory::<TypedWord>().0) {
                        cx.answer(true);
                    }
                }
                Answer::Alternative => {
                    self.alternative_chosen.store(true, Ordering::Relaxed);
                    cx.answer(true);
                }
                Answer::Confirm => {
                    // The disabled button never answers; this only guards the gate twice.
                    if self.unlocked(&cx.memory::<TypedWord>().0) {
                        cx.answer(true);
                    }
                }
                Answer::Cancel => cx.answer(false),
            }
        }
    }
}

/// "Type `word` to confirm", with the word in the `title` tone among `secondary` text.
fn prompt(env: &crate::env::Env, word: &str) -> Text {
    let line = env.i18n().translate("quvyta.confirm.type-word", &[("word", Arg::from(WORD_MARK))]);
    match line.split_once(WORD_MARK) {
        Some((before, after)) => Text::rich([
            Span::new(before).role("secondary"),
            Span::new(word).role("title"),
            Span::new(after).role("secondary"),
        ]),
        None => Text::new(line).role("secondary"),
    }
}

/// The word's field of a dialog built by [`ConfirmLayer::dialog`] for a question with a word:
/// the last node of the body.
fn field(dialog: &Modal<Answer>) -> Option<&Node<Answer>> {
    dialog.children().first()?.widget.children().last()
}

/// The dialog's parts that take input: the word's field, when `word`, and the answer buttons.
fn parts(dialog: &Modal<Answer>, word: bool) -> Vec<&Node<Answer>> {
    let field = if word { field(dialog) } else { None };
    field.into_iter().chain(&dialog.children()[1..]).collect()
}

impl<Msg: 'static> Widget<Msg> for ConfirmLayer<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, _available: Size) -> Size {
        Size::default()
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.request_overlay(area);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let typed = cx.memory::<TypedWord>().0.clone();
        let dialog = self.dialog(cx.env(), cx.id(), &typed);
        Widget::<Answer>::paint_overlay(&dialog, cx, anchor);
        let parts = parts(&dialog, self.word.is_some());
        let rects =
            parts.iter().filter_map(|part| cx.frame.rects.get(&part.id()).map(|rect| (part.id(), *rect))).collect();
        cx.memory::<PartRects>().0 = rects;
        // The field's edit menu is an overlay of its own. The runtime paints the overlays of the
        // widgets in its tree, and this dialog is built by the layer, so the layer paints it.
        if self.word.is_some()
            && let Some(field) = field(&dialog)
            && let Some(rect) = cx.frame.rects.get(&field.id()).copied()
        {
            let saved = (cx.id, cx.layout);
            (cx.id, cx.layout) = (field.id(), field.layout());
            field.widget.paint_overlay(cx, rect);
            (cx.id, cx.layout) = saved;
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let typed = cx.memory::<TypedWord>().0.clone();
        let dialog = self.dialog(cx.env, cx.id, &typed);
        let field = if self.word.is_some() { field(&dialog) } else { None };
        let rect_of = |cx: &mut EventCx<'_, Msg>, id: WidgetId| {
            cx.memory::<PartRects>().0.iter().find(|(part, _)| *part == id).map(|(_, rect)| *rect).unwrap_or_default()
        };
        // An open edit menu of the field takes the keys, Esc included, and the presses first, as
        // it does anywhere else; a press outside it closes it and goes on.
        if let Some(field) = field
            && cx.interaction.key_capture == Some(field.id())
            && matches!(event, Event::Key(_) | Event::Mouse(_))
        {
            let rect = rect_of(cx, field.id());
            let (used, answers) = Self::forward(cx, field, rect, event);
            self.settle(cx, answers);
            if used {
                if let Event::Mouse(mouse) = event {
                    cx.memory::<ConfirmMemory>().pressed =
                        matches!(mouse.kind, MouseKind::Down(_)).then_some(field.id());
                }
                return true;
            }
        }
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
        let parts = parts(&dialog, self.word.is_some());
        let target = match event {
            Event::Key(_) | Event::Paste(_) => parts.iter().find(|part| cx.interaction.focused == Some(part.id())),
            Event::Mouse(mouse) => {
                let rects = cx.memory::<PartRects>().0.clone();
                let over = |part: &&&Node<Answer>| {
                    rects.iter().any(|(id, rect)| *id == part.id() && rect.contains(mouse.x, mouse.y))
                };
                let pressed = cx.memory::<ConfirmMemory>().pressed;
                match mouse.kind {
                    MouseKind::Down(_) => parts.iter().find(over),
                    _ => parts.iter().find(|part| pressed == Some(part.id())),
                }
            }
            Event::PointerOutside => None,
        };
        let Some(part) = target else {
            // Everything else on the dimmed screen is swallowed.
            return matches!(event, Event::Mouse(_));
        };
        if let Event::Mouse(mouse) = event {
            cx.memory::<ConfirmMemory>().pressed = matches!(mouse.kind, MouseKind::Down(_)).then_some(part.id());
        }
        let rect = rect_of(cx, part.id());
        let (handled, answers) = Self::forward(cx, part, rect, event);
        self.settle(cx, answers);
        handled || matches!(event, Event::Mouse(_))
    }
}

/// Where the word's field and the answer buttons were painted, for routing pointer events to
/// them.
#[derive(Debug, Default)]
struct PartRects(Vec<(WidgetId, Rect)>);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::event::{MouseButton, MouseKind};
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
        AskRecover,
        Saved,
        Resumed,
        Discarded,
        AskTyped(&'static str),
        AskTypedWithAlternative,
        /// A quit question with three long answers, in English or German.
        AskQuit(bool),
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
                Msg::AskRecover => {
                    return Command::confirm(
                        Confirm::new("Recover the session?", Msg::Saved)
                            .message("47 minutes were counted.")
                            .confirm_label("Save")
                            .cancel_label("Discard")
                            .on_cancel(Msg::Discarded)
                            .alternative("Continue", Msg::Resumed),
                    );
                }
                Msg::AskTyped(word) => {
                    return Command::confirm(
                        Confirm::new("Empty the trash?", Msg::Remove)
                            .message("12 items go for good.")
                            .confirm_label("Empty")
                            .danger()
                            .on_cancel(Msg::Kept)
                            .require_word(word),
                    );
                }
                Msg::AskTypedWithAlternative => {
                    return Command::confirm(
                        Confirm::new("Empty the trash?", Msg::Remove)
                            .confirm_label("Empty")
                            .danger()
                            .on_cancel(Msg::Kept)
                            .alternative("Archive", Msg::Resumed)
                            .require_word("SİL"),
                    );
                }
                Msg::AskQuit(german) => {
                    let (title, finish, leave) = if german {
                        ("Beenden?", "Beenden und schließen", "Weiterlaufen lassen")
                    } else {
                        ("Quit?", "Finish and quit", "Leave running")
                    };
                    return Command::confirm(
                        Confirm::new(title, Msg::Saved)
                            .confirm_label(finish)
                            .on_cancel(Msg::Discarded)
                            .alternative(leave, Msg::Resumed),
                    );
                }
                Msg::Saved => self.log.push("saved"),
                Msg::Resumed => self.log.push("resumed"),
                Msg::Discarded => self.log.push("discarded"),
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

    #[test]
    fn a_two_way_question_draws_exactly_as_before() {
        let h = asked();
        let screen = h.screen();
        let rows: Vec<&str> = screen.lines().collect();
        assert_eq!(rows[3].trim_end().chars().last(), Some('×'), "{screen}");
        assert_eq!(rows[4], "  ▌  Remove web?", "{screen}");
        assert_eq!(rows[6], "  ▌  Its volumes go too.", "{screen}");
        assert_eq!(rows[8], "  ▌  esc close   tab switch      ▌ Cancel      Remove", "{screen}");
        assert_eq!(rows.iter().filter(|row| row.contains('▌')).count(), 7, "one surface, two buttons:\n{screen}");
    }

    fn recovering() -> Harness<Demo> {
        let mut h = Harness::new(Demo::default(), 60, 14);
        h.send(Msg::AskRecover).advance(Duration::from_millis(200));
        h
    }

    #[test]
    fn a_third_way_sits_between_cancel_and_confirm_with_cancel_focused() {
        let mut h = recovering();
        let screen = h.screen();
        let row = screen.lines().find(|line| line.contains("Continue")).unwrap_or_else(|| panic!("{screen}"));
        let at = |label: &str| row.find(label).unwrap_or_else(|| panic!("`{label}` in {row}"));
        assert!(at("Discard") < at("Continue") && at("Continue") < at("Save"), "{row}");
        assert!(!screen.contains(['[', ']', '|']), "{screen}");
        let (dx, dy) = cell(&h, "Discard");
        let (cx, cy) = cell(&h, "Continue");
        let focused = h.bg(dx, dy);
        assert_ne!(focused, h.bg(cx, cy), "Cancel has the focus, the third way rests: {screen}");
        h.press("tab");
        assert_eq!(h.bg(cx, cy), focused, "tab lifts the third way like the focused Cancel was");
    }

    #[test]
    fn the_keyboard_reaches_each_way_in_reading_order() {
        let mut h = recovering();
        h.press("enter");
        assert_eq!(h.app().log, ["discarded"], "enter on the focused Cancel");
        let mut h = recovering();
        h.press("tab").press("enter");
        assert_eq!(h.app().log, ["resumed"], "tab reaches the third way first");
        let mut h = recovering();
        h.press("tab").press("tab").press("enter");
        assert_eq!(h.app().log, ["saved"]);
        let mut h = recovering();
        h.press("shift+tab").press("enter");
        assert_eq!(h.app().log, ["saved"], "shift tab goes round the other way");
        let mut h = recovering();
        h.press("esc");
        assert_eq!(h.app().log, ["discarded"], "esc still cancels");
        assert!(!h.screen().contains("Recover the session?"));
    }

    #[test]
    fn the_mouse_reaches_each_way() {
        for (label, answer) in [("Continue", "resumed"), ("Save", "saved"), ("Discard", "discarded")] {
            let mut h = recovering();
            let (x, y) = cell(&h, label);
            h.click(i32::from(x), i32::from(y));
            assert_eq!(h.app().log, [answer], "{label}");
            assert!(!h.screen().contains("Recover the session?"));
        }
        let mut h = recovering();
        let (x, y) = cell(&h, "Continue");
        h.mouse(MouseKind::Down(MouseButton::Left), i32::from(x), i32::from(y));
        let (x, y) = cell(&h, "Save");
        h.mouse(MouseKind::Up(MouseButton::Left), i32::from(x), i32::from(y));
        assert!(h.app().log.is_empty(), "a release over another button answers nothing");
    }

    #[test]
    fn a_three_way_question_survives_tiny_terminals_and_ascii() {
        let mut h = recovering();
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert!(!h.screen().contains(['[', ']', '|', '(', ')']), "{}", h.screen());
        assert!(h.screen().contains("Continue"), "{}", h.screen());
        for (width, height) in [(0, 0), (1, 1), (12, 4), (30, 8)] {
            let mut h = Harness::new(Demo::default(), width, height);
            h.send(Msg::AskRecover).advance(Duration::from_millis(200));
            h.press("tab").press("tab").press("enter");
            assert_eq!(h.app().log, ["saved"], "{width}×{height}");
        }
    }

    fn typing(word: &'static str) -> Harness<Demo> {
        let mut h = Harness::new(Demo::default(), 60, 16);
        h.send(Msg::AskTyped(word)).advance(Duration::from_millis(200));
        h
    }

    /// The backgrounds of a resting danger button, disabled and enabled, drawn by themselves.
    fn danger_tones() -> (Option<crate::color::Rgb>, Option<crate::color::Rgb>) {
        struct Tones;
        impl App for Tones {
            type Msg = ();
            fn update(&mut self, (): ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                ui.column(|ui| {
                    ui.add(Button::new("Off").variant("danger").on_press(()).disabled(true));
                    ui.add(Button::new("On").variant("danger").on_press(()));
                });
            }
        }
        let h = Harness::new(Tones, 20, 2);
        (h.bg(3, 0), h.bg(3, 1))
    }

    #[test]
    fn a_word_to_type_shows_a_prompt_and_a_field_that_has_focus() {
        let mut h = typing("SİL");
        let screen = h.screen();
        assert!(screen.contains("Type SİL to confirm"), "{screen}");
        let (x, y) = cell(&h, "SİL to");
        assert!(h.is_bold(x, y) && !h.is_bold(x - 2, y), "the word stands out by weight: {screen}");
        assert_ne!(h.fg(x, y), h.fg(x - 2, y), "and by tone");
        assert!(!screen.contains(['[', ']', '|', '(', ')', '"', '\'']), "{screen}");
        h.type_text("si");
        assert!(h.screen().contains("❯ si"), "typing goes straight into the field: {}", h.screen());
    }

    #[test]
    fn enter_confirms_only_once_the_word_matches() {
        let mut h = typing("SİL");
        h.type_text("sal").press("enter");
        assert!(h.app().log.is_empty() && h.screen().contains("Empty the trash?"), "{}", h.screen());
        h.press("backspace").press("backspace").type_text("il").press("enter");
        assert_eq!(h.app().log, ["removed"]);
        assert!(!h.screen().contains("Empty the trash?"));
    }

    #[test]
    fn matching_ignores_case_surrounding_spaces_and_the_turkish_i() {
        for (word, typed) in [
            ("SİL", "sil"),
            ("SİL", "SİL"),
            ("SİL", "SIL"),
            ("SİL", " Sil "),
            ("SİL", "sıl"),
            ("İPTAL", "iptal"),
            ("iptal", "IPTAL"),
            ("web-1", "WEB-1"),
        ] {
            // Typed as one piece: the test keyboard has no capital İ.
            let mut h = typing(word);
            h.paste(typed).press("enter");
            assert_eq!(h.app().log, ["removed"], "`{typed}` for `{word}`");
        }
        for (word, typed) in [("SİL", "sl"), ("SİL", "si l"), ("İPTAL", "ptal")] {
            let mut h = typing(word);
            h.type_text(typed).press("enter");
            assert!(h.app().log.is_empty(), "`{typed}` is not `{word}`");
        }
        assert_eq!(super::fold(" İIıi Ş "), "iiii ş");
    }

    #[test]
    fn escape_cancels_with_a_word_half_typed() {
        let mut h = typing("SİL");
        h.type_text("si").press("esc");
        assert_eq!(h.app().log, ["kept"]);
        assert!(!h.screen().contains("Empty the trash?"));
    }

    #[test]
    fn the_confirm_button_waits_in_the_disabled_tone_until_the_word_matches() {
        let (disabled, enabled) = danger_tones();
        assert_ne!(disabled, enabled);
        let mut h = typing("SİL");
        let (x, y) = cell(&h, "Empty  ");
        assert_eq!(h.bg(x, y), disabled, "{}", h.screen());
        h.click(i32::from(x), i32::from(y));
        assert!(h.app().log.is_empty() && h.screen().contains("Empty the trash?"), "a disabled button does nothing");
        h.type_text("sil").hover(0, 0);
        assert_eq!(h.bg(x, y), enabled, "{}", h.screen());
        h.click(i32::from(x), i32::from(y));
        assert_eq!(h.app().log, ["removed"]);
    }

    #[test]
    fn tab_passes_over_the_locked_button_and_reaches_it_once_open() {
        let mut h = typing("SİL");
        h.press("tab").press("tab").type_text("sil");
        assert!(h.screen().contains("❯ sil"), "tab went Cancel, then back to the field: {}", h.screen());
        h.press("tab").press("tab").press("enter");
        assert_eq!(h.app().log, ["removed"], "Cancel, then the confirm button");
        let mut h = typing("SİL");
        h.press("tab").press("enter");
        assert_eq!(h.app().log, ["kept"], "the field, then Cancel");
    }

    #[test]
    fn a_paste_fills_the_field() {
        let mut h = typing("SİL");
        h.paste("SİL").press("enter");
        assert_eq!(h.app().log, ["removed"]);
    }

    #[test]
    fn the_field_keeps_its_edit_menu() {
        let mut h = typing("SİL");
        h.set_system_clipboard(Some("SİL"));
        let (x, y) = cell(&h, "❯");
        h.mouse(MouseKind::Down(MouseButton::Right), i32::from(x) + 3, i32::from(y));
        h.mouse(MouseKind::Up(MouseButton::Right), i32::from(x) + 3, i32::from(y));
        h.advance(Duration::from_millis(200));
        assert!(h.screen().contains("Select all"), "a right click opens the menu: {}", h.screen());
        h.press("esc");
        assert!(!h.screen().contains("Select all"), "esc closes the menu first: {}", h.screen());
        assert!(h.app().log.is_empty() && h.screen().contains("Empty the trash?"), "and not the dialog");
        h.mouse(MouseKind::Down(MouseButton::Right), i32::from(x) + 3, i32::from(y));
        h.mouse(MouseKind::Up(MouseButton::Right), i32::from(x) + 3, i32::from(y));
        h.advance(Duration::from_millis(200)).click_text("Paste");
        assert!(h.screen().contains("❯ SİL"), "the menu pastes into the field: {}", h.screen());
        h.press("enter");
        assert_eq!(h.app().log, ["removed"]);
    }

    #[test]
    fn every_question_starts_with_an_empty_field() {
        let mut h = typing("SİL");
        h.type_text("sil").press("enter");
        h.send(Msg::AskTyped("SİL")).advance(Duration::from_millis(200));
        assert!(!h.screen().contains("❯ sil"), "{}", h.screen());
        h.press("enter");
        assert_eq!(h.app().log, ["removed"], "an empty field does not confirm");
        h.type_text("si").press("esc");
        h.send(Msg::AskTyped("SİL")).advance(Duration::from_millis(200));
        h.type_text("l").press("enter");
        assert_eq!(h.app().log, ["removed", "kept"], "nothing is left over from the cancelled question");
    }

    #[test]
    fn the_alternative_does_not_wait_for_the_word() {
        let mut h = Harness::new(Demo::default(), 60, 16);
        h.send(Msg::AskTypedWithAlternative).advance(Duration::from_millis(200));
        let screen = h.screen();
        let row = screen.lines().find(|line| line.contains("Archive")).unwrap_or_else(|| panic!("{screen}"));
        assert!(row.find("Archive") < row.find("Empty"), "{row}");
        h.press("tab").press("tab").press("enter");
        assert_eq!(h.app().log, ["resumed"], "field, Cancel, then the alternative");
        let mut h = Harness::new(Demo::default(), 60, 16);
        h.send(Msg::AskTypedWithAlternative).advance(Duration::from_millis(200));
        h.click_text("Archive");
        assert_eq!(h.app().log, ["resumed"]);
        let mut h = Harness::new(Demo::default(), 60, 16);
        h.send(Msg::AskTypedWithAlternative).advance(Duration::from_millis(200));
        h.type_text("sil").press("shift+tab").press("enter");
        assert_eq!(h.app().log, ["removed"], "shift tab from the field reaches the open confirm button");
    }

    #[test]
    fn a_word_to_type_survives_tiny_terminals_and_ascii() {
        let mut h = typing("SİL");
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        let screen = h.screen();
        assert!(!screen.contains(['[', ']', '|', '(', ')', '{', '}']), "{screen}");
        assert!(screen.contains("Type SİL to confirm"), "{screen}");
        for (width, height) in [(0, 0), (1, 1), (12, 4), (30, 8)] {
            let mut h = Harness::new(Demo::default(), width, height);
            h.send(Msg::AskTyped("SİL")).advance(Duration::from_millis(200));
            h.type_text("sil").press("enter");
            assert_eq!(h.app().log, ["removed"], "{width}×{height}");
            let mut h = Harness::new(Demo::default(), width, height);
            h.set_reduced_motion(true).send(Msg::AskTyped("SİL"));
            h.set_glyph_mode(crate::icons::GlyphMode::Ascii).press("esc");
            assert_eq!(h.app().log, ["kept"], "{width}×{height}");
        }
    }

    #[test]
    fn the_prompt_follows_the_language() {
        let mut h = typing("SİL");
        h.set_locale("tr");
        assert!(h.screen().contains("Onaylamak için SİL yaz"), "{}", h.screen());
    }

    /// The quit question at 40 columns, in English and German, with the labels of its three
    /// answers in Tab order and the language.
    fn quitting() -> Vec<(Harness<Demo>, [&'static str; 3])> {
        [
            (false, "en", ["Cancel", "Leave running", "Finish and quit"]),
            (true, "de", ["Abbrechen", "Weiterlaufen lassen", "Beenden und schließen"]),
        ]
        .into_iter()
        .map(|(german, code, labels)| {
            let mut h = Harness::new(Demo::default(), 40, 20);
            h.set_locale(code).send(Msg::AskQuit(german)).advance(Duration::from_millis(200));
            (h, labels)
        })
        .collect()
    }

    #[test]
    fn at_forty_columns_three_long_answers_stand_one_under_another_in_tab_order() {
        for (h, labels) in quitting() {
            let screen = h.screen();
            assert!(!screen.contains('…'), "{screen}");
            let rows: Vec<usize> = labels.iter().map(|label| usize::from(cell(&h, label).1)).collect();
            assert!(rows[0] < rows[1] && rows[1] < rows[2], "one per row, in Tab order: {screen}");
            let columns: Vec<u16> = labels.iter().map(|label| cell(&h, label).0).collect();
            assert!(columns.windows(2).all(|pair| pair[0] == pair[1]), "one column: {screen}");
            let lines: Vec<&str> = screen.lines().collect();
            assert!(
                lines[rows[0] + 1].trim_start_matches(' ').trim_start_matches('▌').trim().is_empty(),
                "a blank row between two buttons: {screen}"
            );
            assert!(!screen.contains(['[', ']', '|']), "{screen}");
        }
    }

    #[test]
    fn at_forty_columns_the_keyboard_and_the_mouse_reach_every_stacked_answer() {
        for (tabs, answer) in [(0, "discarded"), (1, "resumed"), (2, "saved")] {
            for (mut h, _) in quitting() {
                for _ in 0..tabs {
                    h.press("tab");
                }
                h.press("enter");
                assert_eq!(h.app().log, [answer], "{tabs} tabs");
            }
        }
        for (index, answer) in [(0, "discarded"), (1, "resumed"), (2, "saved")] {
            for (mut h, labels) in quitting() {
                let (x, y) = cell(&h, labels[index]);
                h.click(i32::from(x), i32::from(y));
                assert_eq!(h.app().log, [answer], "{}", labels[index]);
            }
        }
    }
}
