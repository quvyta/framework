//! The engine: builds, paints and routes input, independent of any real terminal.
//!
//! This module builds and paints frames, applies messages and delivers events to widgets.
//! Keys, the pointer, focus and layers, and the clipboard each have a module of their own.

mod clipboard;
mod ending;
mod focus;
mod frames;
mod idle;
mod keys;
mod pointer;

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ratatui_core::buffer::Buffer;

use self::clipboard::ClipboardRead;
use self::frames::Pacing;
use self::idle::Idle;
use self::pointer::PointerRepeat;

use super::app::App;
use super::clipboard::ClipboardReader;
use super::command::{Action, Command};
use super::confirm::{Confirm, ConfirmLayer};
use super::debug;
use super::detached::{DetachedHandoff, DetachedOutcome};
use super::handoff::{Handoff, HandoffOutcome, HandoffRequest};
use super::present::Painted;
use super::selection::{Press, Selection};
use super::selection_menu::{self, SelectionMenu};
use super::task::{self, Delivery, TaskClock};
use super::termination::Ending;
use crate::env::Env;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::i18n;
use crate::keymap::Key;
use crate::style;
use crate::widget::{
    Axis, Effects, EventCx, Flex, Frame, IdleScope, Interaction, Key as NodeKey, Length, Memory, Node, PaintCx, View,
    WidgetId,
};
use crate::widgets::ToastStack;

/// Both ends of the channel background work reports through.
type Channel<Msg> = (Sender<Delivery<Msg>>, Receiver<Delivery<Msg>>);

/// The work of a `Command::perform`.
type Work<Msg> = Box<dyn FnOnce() -> Msg + Send>;

/// A handoff waiting for the loop that owns the terminal.
pub(crate) enum HandOver<Msg> {
    /// Until the program ends.
    Wait(Handoff<Msg>),
    /// Until the program's first line.
    Detach(DetachedHandoff<Msg>),
}

impl<Msg: Send + 'static> HandOver<Msg> {
    /// What a test sees of the handoff.
    #[cfg(test)]
    pub(crate) fn request(&self) -> HandoffRequest {
        match self {
            Self::Wait(handoff) => handoff.request(),
            Self::Detach(handoff) => handoff.request(),
        }
    }
}

/// Whether background work runs on threads or inline (tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskMode {
    Threads,
    Inline,
}

/// Measurements shown by the debug layer.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Stats {
    pub(crate) frames: u64,
    pub(crate) paint_time: Duration,
}

/// Everything a running application needs besides a terminal: its view tree, input routing,
/// layers, clipboard and background work. The terminal [`Runtime`](super::Runtime) and the test
/// [`Harness`](super::Harness) drive it.
pub(crate) struct Engine<A: App> {
    pub(crate) app: A,
    pub(crate) env: Env,
    memory: Memory,
    pub(crate) frame: Frame,
    /// The frame before `frame`, emptied and painted into next, so its memory is reused.
    spare_frame: Frame,
    /// Half blocks of pictures the terminal cannot draw whole this frame; see
    /// [`resolve`](crate::widgets::image::resolve).
    #[cfg(feature = "image")]
    picture_halves: crate::widgets::image::Halves,
    pub(crate) interaction: Interaction,
    tree: Option<Node<A::Msg>>,
    pointer: Option<(i32, i32)>,
    last_activation_key: Option<(Key, Duration)>,
    /// A press closed a layer instead of reaching its target; its release goes nowhere either.
    swallow_release: bool,
    /// The widget that used the last pointer press, until a key is pressed: a layer appearing
    /// right after it was opened by that widget.
    press_target: Option<WidgetId>,
    /// Non-modal layers painted in the last frame, each with the widget whose press opened it.
    openers: Vec<(WidgetId, Option<WidgetId>)>,
    toasts: ToastStack<A::Msg>,
    scope_focus: HashMap<WidgetId, WidgetId>,
    /// A `Command::focus` name that was not on screen yet; tried once more after the next frame.
    pending_focus: Option<String>,
    pressed_button: Option<MouseButton>,
    pointer_repeat: Option<PointerRepeat>,
    /// Pending questions of `Command::confirm` with their serial numbers, oldest first.
    confirms: Vec<(u64, Confirm<A::Msg>)>,
    asked: u64,
    tasks: Channel<A::Msg>,
    pub(crate) task_clock: Arc<TaskClock>,
    task_mode: TaskMode,
    pub(crate) pending_tasks: usize,
    /// Perform work to run on this thread from the loop, oldest first: all of it in inline mode,
    /// and work whose thread could not start. Each round runs only what was queued before it,
    /// so work that performs again waits for the next round instead of recursing.
    queued_work: Vec<Work<A::Msg>>,
    /// Handoffs waiting for the loop that owns the terminal, oldest first. The engine never
    /// touches the terminal itself, so it only queues them.
    handoffs: Vec<HandOver<A::Msg>>,
    /// Handoffs a harness recorded instead of running, oldest first.
    handoff_requests: Vec<HandoffRequest>,
    /// The outcome a harness gives every handoff.
    handoff_outcome: HandoffOutcome,
    /// Detached handoffs a harness recorded instead of running, oldest first.
    detached_requests: Vec<HandoffRequest>,
    /// The outcome a harness gives every detached handoff.
    detached_outcome: DetachedOutcome,
    /// Openings a harness recorded instead of starting, oldest first.
    open_requests: Vec<crate::runtime::OpenRequest>,
    /// The outcome a harness answers every opening with.
    open_outcome: crate::runtime::OpenOutcome,
    /// Questions for a newer version a harness recorded instead of asking.
    #[cfg(feature = "updates")]
    update_checks: Vec<crate::runtime::UpdateCheckRequest>,
    /// The newest version a harness answers every such question with; none by default.
    #[cfg(feature = "updates")]
    latest_version: Option<String>,
    /// Questions a harness has not answered yet, because it was not told a version.
    #[cfg(feature = "updates")]
    unanswered_checks: Vec<crate::runtime::UpdateCheck<A::Msg>>,
    /// Starts the threads of performs and tasks; tests swap in one that fails.
    pub(crate) spawner: task::Spawner,
    pub(crate) clipboard: Vec<String>,
    /// The text copied last inside the application, for pasting without OSC 52.
    pub(crate) clipboard_text: Option<String>,
    selection: Option<Selection>,
    last_press: Option<Press>,
    /// The runtime node of the selection's Copy and Raw copy menu, while text is selected.
    selection_menu: Option<WidgetId>,
    pub(crate) clipboard_reader: ClipboardReader,
    /// Clipboard reads waiting for the reader, oldest first.
    clipboard_reads: Vec<ClipboardRead<A::Msg>>,
    /// Set when the reader wants the terminal asked with OSC 52; the terminal runtime sends the
    /// query and answers with [`Engine::terminal_clipboard`].
    pub(crate) terminal_query: bool,
    /// The time of the latest input, frame or tick, for work that finishes between them.
    clock: Duration,
    /// When the user last did something, and the silences the view watches.
    idle: Idle<A::Msg>,
    /// When the latest frame was drawn, and whether the next one answers input that never waits.
    pacing: Pacing,
    /// Whether [`App::init`] ran; it runs at the start of the first frame.
    started: bool,
    /// The screen size last reported through [`App::resized`].
    screen: Option<Size>,
    /// The graphics last reported through [`App::graphics`].
    told_graphics: Option<crate::graphics::Graphics>,
    pub(crate) dirty: bool,
    pub(crate) quit: bool,
    /// A termination the application was told about, until the run ends.
    ending: Option<Ending>,
    pub(crate) debug: bool,
    pub(crate) stats: Stats,
}

impl<A: App> Engine<A> {
    pub(crate) fn new(app: A, env: Env, task_mode: TaskMode) -> Self {
        Self {
            app,
            env,
            memory: Memory::default(),
            frame: Frame::default(),
            spare_frame: Frame::default(),
            #[cfg(feature = "image")]
            picture_halves: crate::widgets::image::Halves::default(),
            interaction: Interaction::default(),
            tree: None,
            pointer: None,
            last_activation_key: None,
            swallow_release: false,
            press_target: None,
            openers: Vec::new(),
            toasts: ToastStack::default(),
            scope_focus: HashMap::new(),
            pending_focus: None,
            pressed_button: None,
            pointer_repeat: None,
            confirms: Vec::new(),
            asked: 0,
            tasks: mpsc::channel(),
            task_clock: TaskClock::new(task_mode == TaskMode::Inline),
            task_mode,
            pending_tasks: 0,
            queued_work: Vec::new(),
            handoffs: Vec::new(),
            handoff_requests: Vec::new(),
            handoff_outcome: HandoffOutcome::Finished { code: Some(0) },
            detached_requests: Vec::new(),
            detached_outcome: DetachedOutcome::Finished { code: Some(0) },
            open_requests: Vec::new(),
            open_outcome: crate::runtime::OpenOutcome::Opened,
            #[cfg(feature = "updates")]
            update_checks: Vec::new(),
            #[cfg(feature = "updates")]
            latest_version: None,
            #[cfg(feature = "updates")]
            unanswered_checks: Vec::new(),
            spawner: task::spawn_thread,
            clipboard: Vec::new(),
            clipboard_text: None,
            selection: None,
            last_press: None,
            selection_menu: None,
            clipboard_reader: match task_mode {
                TaskMode::Threads => ClipboardReader::runtime(),
                TaskMode::Inline => ClipboardReader::fixed(),
            },
            clipboard_reads: Vec::new(),
            terminal_query: false,
            clock: Duration::ZERO,
            pacing: Pacing::default(),
            idle: Idle::default(),
            started: false,
            screen: None,
            told_graphics: None,
            dirty: true,
            quit: false,
            ending: None,
            debug: false,
            stats: Stats::default(),
        }
    }

    /// Builds the view and paints it into `buf`.
    pub(crate) fn render(&mut self, buf: &mut Buffer, now: Duration) {
        let started = Instant::now();
        let size = Size::new(buf.area.width, buf.area.height);
        self.clock = now;
        self.begin_frame(size);
        // A silence the view watches may have been reached: its message changes what is built.
        self.wake_idle(now);
        self.memory.begin_frame();
        let mut selection_copy = None;
        let idle = self.idle.scope(now);
        let root = self.build_tree(size, &idle);
        let silent = idle.silent;
        self.idle.settle(idle, now);
        self.selection_menu =
            self.selection.is_some().then(|| root.id.child(&NodeKey::Named(selection_menu::NAME.to_owned()), ""));

        let screen = Rect::new(i32::from(buf.area.x), i32::from(buf.area.y), buf.area.width, buf.area.height);
        let mut frame = std::mem::take(&mut self.spare_frame);
        frame.clear();
        let canvas;
        {
            let mut cx = PaintCx {
                buf,
                env: &self.env,
                frame: &mut frame,
                memory: &mut self.memory,
                interaction: &self.interaction,
                now,
                clip: screen,
                id: WidgetId::ROOT,
                layout: root.layout,
                scope: None,
                idle: silent,
                focus_lent: false,
            };
            canvas = cx.color("canvas");
            cx.clear(screen, canvas);
            i18n::scope(self.env.i18n_arc(), || {
                cx.paint_child(&root, screen);
                let mut index = 0;
                while let Some((id, anchor)) = cx.frame.overlays.get(index).copied() {
                    index += 1;
                    let Some(node) = root.find(id) else {
                        continue;
                    };
                    cx.id = id;
                    cx.layout = node.layout();
                    cx.scope = cx.frame.scopes.get(&id).copied();
                    cx.clip = screen;
                    node.paint_overlay(&mut cx, anchor);
                }
                cx.id = WidgetId::ROOT;
                cx.clip = screen;
                cx.scope = None;
                if self.selection.as_mut().is_some_and(|selection| !selection.follow(cx.frame)) {
                    self.selection = None;
                }
                if let Some(selection) = &mut self.selection {
                    if let Some(kind) = selection.copy_pending {
                        selection_copy = Some(selection.text(cx.buf, &cx.frame.decorations, kind));
                        selection.copied(now);
                    }
                    selection.paint(&mut cx);
                }
                // The selection's menu opens over the highlight it acts on.
                if let Some(node) = self.selection_menu.and_then(|id| root.find(id)) {
                    cx.id = node.id();
                    cx.layout = node.layout();
                    node.paint_overlay(&mut cx, screen);
                    cx.id = WidgetId::ROOT;
                }
                self.toasts.paint(&mut cx);
                if self.debug {
                    debug::paint(&mut cx, &self.stats);
                }
            });
        }
        // Before colours are reduced, while the marks pictures left are still as painted.
        #[cfg(feature = "image")]
        {
            use crate::widgets::image::{Placing, resolve};
            let graphics = self.env.graphics();
            // A sixel cut into pieces is sent again piece by piece as windows move over it:
            // nothing on a local terminal, more than half blocks over a remote connection.
            let whole = graphics == crate::graphics::Graphics::Sixel && self.env.remote();
            let placing = if whole { Placing::Whole } else { Placing::Split };
            frame.placements =
                resolve(buf, &frame.pictures, &frame.dims, &mut self.picture_halves, graphics.can_draw(), placing);
        }
        style::reduce(buf, self.env.depth(), canvas);
        self.memory.end_frame();
        self.tree = Some(root);
        self.spare_frame = std::mem::replace(&mut self.frame, frame);
        // Settling what this frame showed can change what the next one looks like, such as the
        // focus a command asked for landing on a widget that just appeared; each such change
        // asks for that frame.
        self.dirty = false;
        self.settle_layers(now);
        self.settle_openers();
        self.settle_interaction();
        self.apply_focus_request();
        self.frame_drawn(now);
        self.stats.frames += 1;
        self.stats.paint_time = started.elapsed();
        if let Some(text) = selection_copy.filter(|text| !text.is_empty()) {
            self.copy_from_ui(text);
        }
    }

    /// What the last painted frame asks of the terminal beyond its cells: the pointer's shape
    /// where the pointer is now, and the pictures it draws itself.
    pub(crate) fn painted(&self) -> Painted {
        Painted {
            shape: self.pointer_shape(),
            #[cfg(feature = "image")]
            pictures: self.frame.placements.clone(),
            #[cfg(feature = "image")]
            painted: self.frame.pictures.iter().map(crate::widgets::image::Picture::image).collect(),
            #[cfg(feature = "image")]
            sixel: self.env.graphics() == crate::graphics::Graphics::Sixel,
        }
    }

    /// The lifecycle hooks due before a frame of `size` is built: the size when it is new, the
    /// graphics when they are new, then, on the first frame only, [`App::init`]. All come before
    /// the view, so the frame already shows what they changed, and before any input is read, so a
    /// focus `init` asks for is in place for the first key.
    fn begin_frame(&mut self, size: Size) {
        if self.screen != Some(size) {
            self.screen = Some(size);
            let message = {
                let app = &self.app;
                i18n::scope(self.env.i18n_arc(), || app.resized(size))
            };
            if let Some(message) = message {
                self.update(message);
            }
        }
        let graphics = self.env.graphics();
        if self.told_graphics != Some(graphics) {
            self.told_graphics = Some(graphics);
            let message = {
                let app = &self.app;
                i18n::scope(self.env.i18n_arc(), || app.graphics(graphics))
            };
            if let Some(message) = message {
                self.update(message);
            }
        }
        if !self.started {
            self.started = true;
            let command = {
                let app = &mut self.app;
                i18n::scope(self.env.i18n_arc(), || app.init())
            };
            self.run(command);
        }
    }

    /// Quits on the user's behalf, unless the application answers [`App::before_quit`] with a
    /// message; that message is applied instead and the application stays. Every quit the
    /// runtime starts for the user goes through here; a quit the system asks for with a signal
    /// goes through `Engine::terminate`, and [`Command::quit`] is the application's own decision
    /// and goes through neither.
    pub(crate) fn ask_to_quit(&mut self) {
        if self.quit {
            return;
        }
        let message = {
            let app = &self.app;
            i18n::scope(self.env.i18n_arc(), || app.before_quit())
        };
        match message {
            Some(message) => self.update(message),
            None => self.quit = true,
        }
    }

    /// The tree of this frame with ids assigned: the application's view filling the `screen`,
    /// then the layers the runtime adds itself. The view reads idleness from `idle` and declares
    /// its watches there.
    fn build_tree(&self, screen: Size, idle: &IdleScope<A::Msg>) -> Node<A::Msg> {
        let mut nodes = Vec::new();
        let env = &self.env;
        let app = &self.app;
        i18n::scope(env.i18n_arc(), || app.view(&mut View::new(&mut nodes, env, screen, idle)));
        // A pending confirmation is a runtime-owned layer after the application's view; every
        // question gets its own id, so the next one opens fresh with Cancel focused.
        if let Some((serial, confirm)) = self.confirms.last() {
            let mut node = Node::new(ConfirmLayer::new(confirm), nodes.len());
            node.key = NodeKey::Named(format!("quvyta-confirm-{serial}"));
            nodes.push(node);
        }
        // While text is selected, the node of its Copy and Raw copy menu comes last.
        if self.selection.is_some() {
            let mut node = Node::new(SelectionMenu::new(), nodes.len());
            node.key = NodeKey::Named(selection_menu::NAME.to_owned());
            nodes.push(node);
        }
        let mut root = Node::new(Flex::new(Axis::Column, nodes), 0);
        root.layout.width = Length::Fill(1);
        root.layout.height = Length::Fill(1);
        root.assign_ids(WidgetId::ROOT);
        root
    }

    /// Handles one input event at `now`.
    pub(crate) fn handle(&mut self, event: Event, now: Duration) {
        self.catch_up(now);
        self.clock = now;
        self.note_input(&event, now);
        // A key, a paste, a press or a release is answered on screen at once, whatever the frame
        // limit is; the pointer moving and the wheel wait for it like the application's own work.
        if frames::answers_at_once(&event) {
            self.frame_is_urgent();
        }
        match &event {
            Event::Key(_) => self.interaction.focus_by_pointer = false,
            Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::Down(_)) => {
                self.interaction.focus_by_pointer = true;
            }
            _ => {}
        }
        match event {
            Event::Key(key) => self.handle_key(key, now),
            Event::Mouse(mouse) => self.handle_mouse(mouse, now),
            Event::Paste(text) => self.paste(text, now),
            Event::PointerOutside => {}
        }
    }

    /// Offers `event` to each widget in `targets` until one uses it. Returns that widget.
    fn dispatch(&mut self, targets: &[WidgetId], event: &Event, now: Duration) -> Option<WidgetId> {
        self.dispatch_as(targets, event, now, false)
    }

    /// Like [`Engine::dispatch`], telling the widgets whether the event is a preview of a press,
    /// see [`PaintCx::preview_presses`](crate::widget::PaintCx::preview_presses).
    fn dispatch_as(&mut self, targets: &[WidgetId], event: &Event, now: Duration, preview: bool) -> Option<WidgetId> {
        for id in targets {
            let mut effects = Effects::default();
            let mut messages = Vec::new();
            let handled = {
                let Some(node) = self.tree.as_ref().and_then(|tree| tree.find(*id)) else {
                    continue;
                };
                let mut cx = EventCx {
                    id: *id,
                    rect: self.frame.rects.get(id).copied().unwrap_or_default(),
                    focus_rect: self.interaction.focused.and_then(|focused| self.frame.rects.get(&focused).copied()),
                    env: &self.env,
                    memory: &mut self.memory,
                    interaction: &self.interaction,
                    messages: &mut messages,
                    effects: &mut effects,
                    now,
                    persistent: self.frame.scopes.contains_key(id),
                    preview,
                    holds_pointer: self.interaction.pointer_capture == Some(*id),
                };
                node.event(&mut cx, event)
            };
            let (run_action, answer) = (effects.run_action.clone(), effects.answer);
            self.apply_effects(*id, effects, handled, now);
            for message in messages {
                self.update(message);
            }
            if let Some(confirmed) = answer {
                self.answer_confirm(confirmed);
            }
            if let Some((scope, action)) = run_action {
                self.run_action(scope, &action, true, now);
            }
            if handled {
                self.dirty = true;
                return Some(*id);
            }
        }
        None
    }

    fn apply_effects(&mut self, id: WidgetId, effects: Effects, handled: bool, now: Duration) {
        if let Some(focus) = effects.focus {
            self.interaction.focused = Some(focus);
        }
        if let Some(capture) = effects.key_capture {
            self.interaction.key_capture = capture;
        }
        if effects.pointer_capture && handled {
            self.interaction.pointer_capture = Some(id);
        }
        if effects.stop_pointer_repeat && handled && self.pointer_repeat.is_some_and(|repeat| repeat.owner == id) {
            self.pointer_repeat = None;
        }
        if let (Some(interval), true) = (effects.pointer_repeat, handled) {
            let button = self.pressed_button.unwrap_or(MouseButton::Left);
            self.pointer_repeat = Some(PointerRepeat { owner: id, button, interval, next: now + interval });
        }
        if let Some(flash) = effects.flash {
            self.interaction.pressed = Some((flash, now));
        }
        for text in effects.copy {
            self.copy_from_ui(text);
        }
        if let (Some(kind), Some(selection)) = (effects.copy_selection, &mut self.selection) {
            selection.copy_pending = Some(kind);
        }
        if effects.probe_clipboard {
            self.read_clipboard(ClipboardRead::Probe, now);
        }
        if let Some(step) = effects.focus_step {
            self.move_focus(step);
        }
    }

    /// Closes the topmost confirmation dialog and delivers the message of the chosen answer.
    fn answer_confirm(&mut self, confirmed: bool) {
        let Some((_, confirm)) = self.confirms.pop() else {
            return;
        };
        self.dirty = true;
        if let Some(message) = confirm.into_answer(confirmed) {
            self.update(message);
        }
    }

    /// Applies a message and runs the command it returns.
    pub(crate) fn update(&mut self, message: A::Msg) {
        let command: Command<A::Msg> = {
            let app = &mut self.app;
            i18n::scope(self.env.i18n_arc(), || app.update(message))
        };
        self.run(command);
    }

    /// Runs the work of a command the application returned.
    fn run(&mut self, command: Command<A::Msg>) {
        self.dirty = true;
        for action in command.actions {
            match action {
                Action::Quit => self.quit = true,
                Action::Focus(name) => match self.named_focusable(&name) {
                    Some(target) => self.interaction.focused = Some(target),
                    // The widget may appear with this very update, e.g. the first field of the
                    // next wizard step.
                    None => self.pending_focus = Some(name),
                },
                Action::SetTheme(id) => self.env.set_theme(&id),
                Action::SetLocale(code) => self.env.set_locale(&code),
                Action::SetRegion(region) => self.env.set_region(region.as_deref()),
                Action::SetIconMode(mode) => self.env.set_icon_mode(mode),
                Action::SetReducedMotion(reduced) => self.env.set_reduced_motion(reduced),
                Action::SetPillar(style) => self.env.set_pillar_style(style),
                Action::SetSlide(slide) => self.env.set_slide(slide),
                Action::Copy(text) => self.store_copy(text),
                Action::Confirm(confirm) => {
                    self.asked += 1;
                    self.confirms.push((self.asked, confirm));
                }
                Action::ReadClipboard(message) => self.read_clipboard(ClipboardRead::Message(message), self.clock),
                Action::Toast(toast) => self.toasts.push(toast),
                Action::DismissToast(key) => self.toasts.dismiss(&key),
                Action::ToastCorner(corner) => self.toasts.set_corner(corner),
                Action::Perform(work) => match self.task_mode {
                    TaskMode::Inline => self.queued_work.push(work),
                    TaskMode::Threads => self.perform_on_thread(work),
                },
                Action::Task(work) => {
                    self.pending_tasks += 1;
                    if let Some(started) = task::spawn(work, &self.task_clock, &self.tasks.0, self.spawner) {
                        self.update(started);
                    }
                }
                Action::CancelTask(id) => self.task_clock.cancel(id),
                Action::Handoff(handoff) => match self.task_mode {
                    // No terminal to hand over: the request is recorded and the outcome the test
                    // set answers it, from the loop like perform work so the message arrives in
                    // a later update.
                    TaskMode::Inline => {
                        self.handoff_requests.push(handoff.request());
                        let outcome = self.handoff_outcome.clone();
                        self.queued_work.push(Box::new(move || handoff.finish(outcome)));
                    }
                    TaskMode::Threads => self.handoffs.push(HandOver::Wait(handoff)),
                },
                Action::HandoffDetached(handoff) => match self.task_mode {
                    // As for a handoff; a detached child of the test's outcome sends its lines
                    // through the channel of background work, like a real one.
                    TaskMode::Inline => {
                        self.detached_requests.push(handoff.request());
                        let outcome = self.detached_outcome.clone();
                        let deliveries = self.tasks.0.clone();
                        self.queued_work.push(Box::new(move || handoff.finish(outcome, deliveries)));
                    }
                    TaskMode::Threads => self.handoffs.push(HandOver::Detach(handoff)),
                },
                Action::Open(open) => match self.task_mode {
                    // Nothing of the desktop is reached from a test: the opening is recorded and
                    // the outcome the test set answers it, from the loop like perform work.
                    TaskMode::Inline => {
                        self.open_requests.push(open.request());
                        let outcome = self.open_outcome.clone();
                        if let Some(message) = open.finish(outcome) {
                            self.queued_work.push(Box::new(move || message));
                        }
                    }
                    TaskMode::Threads => self.open_on_thread(open),
                },
                #[cfg(feature = "updates")]
                Action::CheckForUpdate(check) => match self.task_mode {
                    // A test never reaches the network, and never the folders of the check: the
                    // question is recorded and answered with the version the test named.
                    TaskMode::Inline => {
                        self.update_checks.push(check.request());
                        self.unanswered_checks.push(check);
                        self.answer_update_checks();
                    }
                    TaskMode::Threads => self.check_on_thread(check),
                },
            }
        }
    }

    /// Runs perform work on a thread of its own. When no thread can start, the work is queued
    /// and runs from the loop instead, so its message still arrives.
    fn perform_on_thread(&mut self, work: Work<A::Msg>) {
        let sender = self.tasks.0.clone();
        // The work moves into the thread only once it exists, so a failed start leaves it here.
        let slot = Arc::new(Mutex::new(Some(work)));
        let shared = Arc::clone(&slot);
        let run = Box::new(move || {
            let Some(work) = shared.lock().ok().and_then(|mut slot| slot.take()) else {
                let _ = sender.send(Delivery::Ended);
                return;
            };
            // Work that panics delivers nothing but still ends, so nothing waits for it.
            if let Ok(message) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
                let _ = sender.send(Delivery::Message(message));
            }
            let _ = sender.send(Delivery::Ended);
        });
        if (self.spawner)("quvyta-perform".to_owned(), run).is_ok() {
            self.pending_tasks += 1;
        } else if let Some(work) = slot.lock().ok().and_then(|mut slot| slot.take()) {
            self.queued_work.push(work);
        }
    }

    /// Starts an opening on a thread of its own: the program is spawned there and the message
    /// sent from there, so the loop never waits for a process to start.
    ///
    /// The loop is told the work is over as soon as the message is on its way, and only then is
    /// the child waited for: an opener may live as long as the window it opened, and nothing
    /// should wait for that. When no thread can start, the program is started from the loop
    /// instead, so the message still arrives.
    fn open_on_thread(&mut self, open: crate::runtime::Open<A::Msg>) {
        let sender = self.tasks.0.clone();
        let slot = Arc::new(Mutex::new(Some(open)));
        let shared = Arc::clone(&slot);
        let run = Box::new(move || {
            let Some(open) = shared.lock().ok().and_then(|mut slot| slot.take()) else {
                let _ = sender.send(Delivery::Ended);
                return;
            };
            let (message, child) = open.start();
            if let Some(message) = message {
                let _ = sender.send(Delivery::Message(message));
            }
            let _ = sender.send(Delivery::Ended);
            if let Some(mut child) = child {
                let _ = child.wait();
            }
        });
        if (self.spawner)("quvyta-open".to_owned(), run).is_ok() {
            self.pending_tasks += 1;
        } else if let Some(open) = slot.lock().ok().and_then(|mut slot| slot.take()) {
            let (message, _child) = open.start();
            if let Some(message) = message {
                self.queued_work.push(Box::new(move || message));
            }
        }
    }

    /// The handoff waiting longest, taken out of the queue. The loop that owns the terminal
    /// calls this until it returns `None`, so several handoffs run one after another.
    pub(crate) fn take_handoff(&mut self) -> Option<HandOver<A::Msg>> {
        let handoff = (!self.handoffs.is_empty()).then(|| self.handoffs.remove(0))?;
        // The user works in the program that gets the terminal, so its end counts as input.
        self.idle.handing_off();
        Some(handoff)
    }

    /// Handoffs a harness recorded, oldest first.
    pub(crate) fn handoff_requests(&self) -> &[HandoffRequest] {
        &self.handoff_requests
    }

    /// Sets the outcome a harness answers every handoff with.
    pub(crate) fn set_handoff_outcome(&mut self, outcome: HandoffOutcome) {
        self.handoff_outcome = outcome;
    }

    /// Detached handoffs a harness recorded, oldest first.
    pub(crate) fn detached_requests(&self) -> &[HandoffRequest] {
        &self.detached_requests
    }

    /// Sets the outcome a harness answers every detached handoff with.
    pub(crate) fn set_detached_outcome(&mut self, outcome: DetachedOutcome) {
        self.detached_outcome = outcome;
    }

    /// Asks for a newer version on a thread of its own, so the application never waits for the
    /// network. When no thread can start the question is not asked: asking from the loop would
    /// hold the screen for as long as the registry takes, and the next start asks again.
    #[cfg(feature = "updates")]
    fn check_on_thread(&mut self, check: crate::runtime::UpdateCheck<A::Msg>) {
        let sender = self.tasks.0.clone();
        let run = Box::new(move || {
            if let Some(message) = check.ask(std::time::SystemTime::now()) {
                let _ = sender.send(Delivery::Message(message));
            }
            let _ = sender.send(Delivery::Ended);
        });
        if (self.spawner)("quvyta-update-check".to_owned(), run).is_ok() {
            self.pending_tasks += 1;
        }
    }

    /// Questions for a newer version a harness recorded, oldest first.
    #[cfg(feature = "updates")]
    pub(crate) fn update_checks(&self) -> &[crate::runtime::UpdateCheckRequest] {
        &self.update_checks
    }

    /// Sets the newest version a harness answers every question for one with, those still
    /// waiting included.
    #[cfg(feature = "updates")]
    pub(crate) fn set_latest_version(&mut self, latest: Option<String>) {
        self.latest_version = latest;
        self.answer_update_checks();
    }

    /// Answers the waiting questions with the version a harness was told, from the loop like
    /// perform work; without one they keep waiting, as a question does without a network.
    #[cfg(feature = "updates")]
    fn answer_update_checks(&mut self) {
        let Some(latest) = self.latest_version.clone() else { return };
        for check in std::mem::take(&mut self.unanswered_checks) {
            if let Some(message) = check.answer(&latest) {
                self.queued_work.push(Box::new(move || message));
            }
        }
    }

    /// Openings a harness recorded, oldest first.
    pub(crate) fn open_requests(&self) -> &[crate::runtime::OpenRequest] {
        &self.open_requests
    }

    /// Sets the outcome a harness answers every opening with.
    pub(crate) fn set_open_outcome(&mut self, outcome: crate::runtime::OpenOutcome) {
        self.open_outcome = outcome;
    }

    /// Where background work hands the loop its messages, for a detached child's lines.
    pub(crate) fn deliveries(&self) -> Sender<Delivery<A::Msg>> {
        self.tasks.0.clone()
    }

    /// Whether perform work waits for [`Engine::run_queued_work`].
    pub(crate) fn has_queued_work(&self) -> bool {
        !self.queued_work.is_empty()
    }

    /// Runs the perform work queued so far and applies each message. Work queued while this runs
    /// waits for the next call. Returns how much work ran.
    pub(crate) fn run_queued_work(&mut self) -> usize {
        let queued = std::mem::take(&mut self.queued_work);
        let count = queued.len();
        for work in queued {
            let message = work();
            self.update(message);
        }
        count
    }

    /// Delivers messages from background work. Returns how many deliveries there were.
    ///
    /// Only what arrived before the call is delivered: a message sent while these are applied
    /// waits for the next call, so work that keeps sending never holds the loop here.
    pub(crate) fn poll_tasks(&mut self) -> usize {
        let deliveries: Vec<Delivery<A::Msg>> = self.tasks.1.try_iter().collect();
        let count = deliveries.len();
        for delivery in deliveries {
            match delivery {
                Delivery::Message(message) => self.update(message),
                Delivery::Ended => self.pending_tasks = self.pending_tasks.saturating_sub(1),
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, TextInput};

    /// Shows a field only after `Reveal`, and focuses it in the same update.
    #[derive(Default)]
    struct Hidden {
        shown: bool,
    }

    #[derive(Clone)]
    enum Msg {
        Reveal,
        Typed,
    }

    impl App for Hidden {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Reveal => {
                    self.shown = true;
                    Command::focus("name")
                }
                Msg::Typed => Command::none(),
            }
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add(Button::new("Reveal").on_press(Msg::Reveal)).id("reveal");
            if self.shown {
                ui.add(TextInput::new("").on_change(|_| Msg::Typed)).id("name");
            }
        }
    }

    /// Starts background work that panics.
    struct Failing;

    impl App for Failing {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::perform(|| panic!("the work failed"))
        }
        fn view(&self, _ui: &mut View<'_, ()>) {}
    }

    /// Counts down with work that schedules the next step of itself.
    #[derive(Default)]
    struct Countdown {
        left: u32,
        keys: u32,
    }

    #[derive(Clone)]
    enum Tick {
        Start(u32),
        Next,
        Key,
    }

    impl App for Countdown {
        type Msg = Tick;
        fn update(&mut self, msg: Tick) -> Command<Tick> {
            match msg {
                Tick::Start(steps) => self.left = steps,
                Tick::Next => self.left = self.left.saturating_sub(1),
                Tick::Key => {
                    self.keys += 1;
                    return Command::none();
                }
            }
            if self.left == 0 { Command::none() } else { Command::perform(|| Tick::Next) }
        }
        fn view(&self, ui: &mut View<'_, Tick>) {
            ui.add(Button::new(format!("{} left", self.left)).on_press(Tick::Key)).id("count");
        }
    }

    #[test]
    fn a_perform_that_schedules_itself_runs_one_step_per_frame() {
        let mut h = Harness::new(Countdown::default(), 20, 1);
        h.send(Tick::Start(10_000));
        let left = h.app().left;
        assert!(left > 0 && left < 10_000, "each step finishes before the next starts: {left} left");
        h.press("tab").press("enter");
        assert_eq!(h.app().keys, 1, "the harness still takes input while the work goes on");
        let mut frames = 0;
        while h.app().left > 0 && frames < 20_000 {
            h.render();
            frames += 1;
        }
        assert_eq!(h.app().left, 0);
        assert!(h.screen().contains("0 left"), "{}", h.screen());
    }

    fn no_thread(_: String, _: Box<dyn FnOnce() + Send>) -> std::io::Result<()> {
        Err(std::io::Error::other("no threads left"))
    }

    #[test]
    fn perform_work_without_a_thread_runs_from_the_loop_one_step_at_a_time() {
        let mut engine = super::Engine::new(Countdown::default(), crate::env::Env::builtin(), super::TaskMode::Threads);
        engine.spawner = no_thread;
        engine.update(Tick::Start(10_000));
        assert_eq!(engine.app.left, 10_000, "the work waits for the loop");
        let mut rounds = 0;
        while engine.has_queued_work() {
            assert_eq!(engine.run_queued_work(), 1);
            engine.poll_tasks();
            rounds += 1;
        }
        assert_eq!((engine.app.left, rounds, engine.pending_tasks), (0, 10_000, 0));
    }

    /// Streams two messages from a task: the second is sent while the first is being applied.
    struct Relay {
        applied: Vec<&'static str>,
        /// Tells the task the first message is being applied, and waits until it sent the second.
        handshake: Option<(std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>)>,
    }

    impl App for Relay {
        type Msg = &'static str;
        fn update(&mut self, msg: &'static str) -> Command<&'static str> {
            self.applied.push(msg);
            match msg {
                "start" => {
                    let (to_task, from_app) = std::sync::mpsc::channel();
                    let (to_app, from_task) = std::sync::mpsc::channel();
                    self.handshake = Some((to_task, from_task));
                    Command::task(crate::runtime::Task::new("relay", move |cx| {
                        cx.send("first");
                        let _ = from_app.recv();
                        cx.send("second");
                        let _ = to_app.send(());
                        Ok("done")
                    }))
                }
                "first" => {
                    if let Some((to_task, from_task)) = &self.handshake {
                        let _ = to_task.send(());
                        let _ = from_task.recv();
                    }
                    Command::none()
                }
                _ => Command::none(),
            }
        }
        fn view(&self, _ui: &mut View<'_, &'static str>) {}
    }

    #[test]
    fn a_message_sent_while_deliveries_apply_waits_for_the_next_poll() {
        let relay = Relay { applied: Vec::new(), handshake: None };
        let mut engine = super::Engine::new(relay, crate::env::Env::builtin(), super::TaskMode::Threads);
        engine.update("start");
        let started = std::time::Instant::now();
        while engine.app.applied.len() < 2 && started.elapsed() < std::time::Duration::from_secs(5) {
            engine.poll_tasks();
            std::thread::yield_now();
        }
        assert_eq!(engine.app.applied, ["start", "first"], "the poll that applied `first` returned before `second`");
        while engine.pending_tasks > 0 && started.elapsed() < std::time::Duration::from_secs(5) {
            engine.poll_tasks();
            std::thread::yield_now();
        }
        assert_eq!(engine.app.applied, ["start", "first", "second", "done"]);
    }

    #[test]
    fn work_that_panics_still_ends() {
        let mut engine = super::Engine::new(Failing, crate::env::Env::builtin(), super::TaskMode::Threads);
        engine.update(());
        assert_eq!(engine.pending_tasks, 1);
        let started = std::time::Instant::now();
        while engine.pending_tasks > 0 && started.elapsed() < std::time::Duration::from_secs(5) {
            engine.poll_tasks();
            std::thread::yield_now();
        }
        assert_eq!(engine.pending_tasks, 0, "the loop would otherwise keep waking up for it");
    }

    /// Asks for a handoff per message and remembers what came back.
    #[derive(Default)]
    struct Steps {
        asked: Vec<&'static str>,
        ended: Vec<crate::runtime::HandoffOutcome>,
    }

    #[derive(Clone)]
    enum Step {
        Hand(&'static str),
        Back(crate::runtime::HandoffOutcome),
    }

    impl App for Steps {
        type Msg = Step;
        fn update(&mut self, msg: Step) -> Command<Step> {
            match msg {
                Step::Hand(program) => {
                    self.asked.push(program);
                    Command::handoff(crate::runtime::Handoff::new(program, Step::Back))
                }
                Step::Back(outcome) => {
                    self.ended.push(outcome);
                    Command::none()
                }
            }
        }
        fn view(&self, _ui: &mut View<'_, Step>) {}
    }

    /// The engine owns no terminal, so a handoff waits in its queue for the loop that does, and
    /// the loop runs the queued handoffs one after another.
    #[test]
    fn handoffs_wait_for_the_loop_and_run_in_the_order_they_were_asked_for() {
        use std::io;

        use crate::runtime::HandoffOutcome;

        let mut engine = super::Engine::new(Steps::default(), crate::env::Env::builtin(), super::TaskMode::Threads);
        engine.update(Step::Hand("true"));
        engine.update(Step::Hand("false"));
        assert_eq!(engine.app.ended, [], "nothing ran while the engine held them");
        engine.dirty = false;
        let mut programs = Vec::new();
        while let Some(handoff) = engine.take_handoff() {
            programs.push(handoff.request().program);
            // The loop's screen, without a terminal behind it.
            let mut release = |_: Option<&str>| -> io::Result<()> { Ok(()) };
            let mut take = || -> io::Result<()> { Ok(()) };
            let mut wait_for_key = || -> io::Result<()> { Ok(()) };
            let screen = &mut crate::runtime::handoff::HandoffScreen {
                release: &mut release,
                take: &mut take,
                wait_for_key: &mut wait_for_key,
            };
            let super::HandOver::Wait(handoff) = handoff else {
                panic!("only handoffs that wait were asked for");
            };
            let message = crate::runtime::handoff::run(handoff, screen);
            engine.dirty = true;
            engine.update(message);
        }
        assert_eq!(programs, ["true", "false"].map(std::ffi::OsString::from));
        assert_eq!(
            engine.app.ended,
            [HandoffOutcome::Finished { code: Some(0) }, HandoffOutcome::Finished { code: Some(1) }],
            "each program's own exit code came back, in the order they were asked for"
        );
        assert!(engine.dirty, "the screen the program wrote over is drawn again in full");
    }

    #[test]
    fn focus_reaches_a_widget_shown_by_the_same_update() {
        let mut h = Harness::new(Hidden::default(), 30, 3);
        h.press("tab").press("enter");
        assert!(h.is_focused("name"));
        h.press("tab");
        h.send(Msg::Typed);
        assert!(h.is_focused("reveal"), "a resolved focus request does not linger");
    }
}
