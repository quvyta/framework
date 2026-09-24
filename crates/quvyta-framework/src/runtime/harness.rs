//! Driving an application in tests: no terminal, a fake clock and inline background work.

use std::time::Duration;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect as BufferRect;
use ratatui_core::style::{Color, Modifier};

use super::app::App;
use super::detached::DetachedOutcome;
use super::engine::{Engine, TaskMode};
use super::handoff::{HandoffOutcome, HandoffRequest};
use super::open::{OpenOutcome, OpenRequest};
use super::termination::Termination;
use crate::color::{ColorDepth, Rgb};
use crate::env::Env;
use crate::event::{Event, KeyEvent, MouseButton, MouseEvent, MouseKind};
use crate::icons::GlyphMode;
use crate::keymap::Modifiers;
use crate::widget::PointerShape;

/// Time the fake clock moves before every simulated key press, so presses are never mistaken
/// for a held key.
const KEY_INTERVAL: Duration = Duration::from_millis(150);

/// Runs an [`App`] against an in-memory screen.
///
/// Every input method renders afterwards, like the real runtime does. Work of
/// [`Command::perform`](super::Command::perform) runs inline, one round per step like one pass of
/// the terminal loop: work that performs again runs at the next step, so a chain of performs
/// takes one [`Harness::render`] per link and an endless one never blocks a test.
pub struct Harness<A: App> {
    engine: Engine<A>,
    buffer: Buffer,
    now: Duration,
}

impl<A: App> Harness<A> {
    /// A harness with the built-in environment and a `width` × `height` screen, already rendered.
    ///
    /// The first frame starts the application as the terminal runtime does: the size reaches
    /// [`App::resized`], then [`App::init`] runs, so the focus it asks for is in place before the
    /// first simulated key.
    pub fn new(app: A, width: u16, height: u16) -> Self {
        Self::with_env(app, Env::builtin(), width, height)
    }

    /// A harness with a custom environment.
    pub fn with_env(app: A, env: Env, width: u16, height: u16) -> Self {
        let mut harness = Self {
            engine: Engine::new(app, env, TaskMode::Inline),
            buffer: Buffer::empty(BufferRect::new(0, 0, width, height)),
            now: Duration::ZERO,
        };
        harness.render();
        harness
    }

    /// Paints the current view.
    pub fn render(&mut self) -> &mut Self {
        self.settle_tasks();
        self.engine.render(&mut self.buffer, self.now);
        self.write_out();
        // Settling hover or scrolling to a focused widget can ask for one more frame at once.
        for _ in 0..3 {
            let due = self.engine.deadline().is_some_and(|deadline| deadline <= self.now);
            if !self.engine.dirty && !due {
                break;
            }
            self.engine.render(&mut self.buffer, self.now);
            self.write_out();
        }
        self
    }

    /// Paints the current view onto `screen` as the terminal runtime paints a frame, for the tests
    /// that read what a screen writes. Answers whether anything was written.
    #[cfg(all(test, feature = "image"))]
    pub(crate) fn present_to<W: std::io::Write>(&mut self, screen: &mut super::present::Screen<W>) -> bool {
        let now = self.now;
        let engine = &mut self.engine;
        screen
            .present(|buffer| {
                engine.render(buffer, now);
                engine.painted()
            })
            .expect("a frame")
    }

    /// Works out what the terminal would be sent for this frame over a cleared screen, as the
    /// terminal runtime does before writing, so a cell no terminal can take fails the test that
    /// drew it instead of the application that ships it.
    fn write_out(&self) {
        let blank = Buffer::empty(self.buffer.area);
        let _ = blank.diff(&self.buffer);
    }

    /// Runs the perform work queued so far, then lets background tasks run up to the fake clock:
    /// every task works until it sleeps past `now` or ends, and everything they sent is applied.
    /// Tasks started by those messages settle too. Perform work queued meanwhile runs at the next
    /// step, so work that performs again never keeps a step from ending.
    fn settle_tasks(&mut self) {
        self.engine.run_queued_work();
        loop {
            self.engine.task_clock.settle(self.now);
            if self.engine.poll_tasks() == 0 {
                break;
            }
        }
    }

    /// Delivers `message` to the application as if a widget had sent it, then renders.
    pub fn send(&mut self, message: A::Msg) -> &mut Self {
        self.engine.update(message);
        self.render()
    }

    /// Presses a key chord such as `"ctrl+s"`, `"tab"` or `"?"`.
    pub fn press(&mut self, chord: &str) -> &mut Self {
        self.now += KEY_INTERVAL;
        self.engine.handle(Event::Key(KeyEvent::press(chord)), self.now);
        self.render()
    }

    /// Types `text` one character at a time.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for c in text.chars() {
            let chord = match c {
                ' ' => "space".to_owned(),
                '+' => "+".to_owned(),
                c if c.is_uppercase() => format!("shift+{}", c.to_lowercase()),
                c => c.to_string(),
            };
            self.press(&chord);
        }
        self
    }

    /// Delivers `events` in order at the current clock time and renders once afterwards, the
    /// way the terminal loop handles every event waiting between two frames: several keys a fast
    /// typist, a terminal multiplexer or a paste without bracketed paste sent in one read. Each
    /// event meets what the ones before it did, as the view is rebuilt off screen between them,
    /// so four keys typed into a controlled [`TextInput`](crate::widgets::TextInput) at once all
    /// arrive.
    pub fn events(&mut self, events: &[Event]) -> &mut Self {
        for event in events {
            self.engine.handle(event.clone(), self.now);
        }
        self.render()
    }

    /// Delivers `event` at an exact clock time, without moving the clock.
    #[cfg(test)]
    pub(crate) fn inject(&mut self, event: Event, at: Duration) -> &mut Self {
        self.engine.handle(event, at);
        self.render()
    }

    /// Pastes `text`.
    pub fn paste(&mut self, text: &str) -> &mut Self {
        self.engine.handle(Event::Paste(text.to_owned()), self.now);
        self.render()
    }

    /// Clicks the left button on a cell.
    pub fn click(&mut self, x: i32, y: i32) -> &mut Self {
        self.mouse(MouseKind::Down(MouseButton::Left), x, y);
        self.mouse(MouseKind::Up(MouseButton::Left), x, y)
    }

    /// Clicks the first cell of the first occurrence of `text` on screen.
    ///
    /// # Panics
    ///
    /// Panics when `text` is not on screen.
    pub fn click_text(&mut self, text: &str) -> &mut Self {
        let (x, y) = self.find(text).unwrap_or_else(|| panic!("`{text}` is not on screen:\n{}", self.screen()));
        self.click(x, y)
    }

    /// Presses the left button on `from`, drags to `to` and releases there.
    pub fn drag(&mut self, from: (i32, i32), to: (i32, i32)) -> &mut Self {
        self.mouse(MouseKind::Down(MouseButton::Left), from.0, from.1);
        self.mouse(MouseKind::Drag(MouseButton::Left), to.0, to.1);
        self.mouse(MouseKind::Up(MouseButton::Left), to.0, to.1)
    }

    /// Moves the pointer to a cell.
    pub fn hover(&mut self, x: i32, y: i32) -> &mut Self {
        self.mouse(MouseKind::Moved, x, y)
    }

    /// Sends a mouse event.
    pub fn mouse(&mut self, kind: MouseKind, x: i32, y: i32) -> &mut Self {
        self.engine.handle(Event::Mouse(MouseEvent { kind, x, y, mods: Modifiers::default() }), self.now);
        self.render()
    }

    /// Moves the fake clock forward and renders. Idleness moves with it: what
    /// [`View::idle_for`](crate::widget::View::idle_for) reads grows by `duration`, and a
    /// [`View::on_idle`](crate::widget::View::on_idle) watch whose silence is reached is told.
    /// Every simulated input (a key, the mouse, a paste) starts the silence again; `send`,
    /// `resize` and theme or language changes do not.
    ///
    /// A termination whose [`Termination::grace`] is over by then quits, as it does in the
    /// runtime.
    pub fn advance(&mut self, duration: Duration) -> &mut Self {
        self.now += duration;
        self.engine.tick(self.now);
        self.engine.end_when_due(self.now);
        self.render()
    }

    /// Simulates the signal behind `cause`, the way the terminal runtime hears a `SIGTERM` or a
    /// `SIGHUP`, then renders. The application hears it through
    /// [`App::terminating`](super::App::terminating) exactly as it would in a terminal, so a test
    /// can check its answer:
    ///
    /// - An answer of `None` quits at once: [`Harness::quit_requested`] is true.
    /// - A message is applied; the application stays until it quits or until
    ///   [`Harness::advance`] moves the clock past [`Termination::grace`].
    /// - Calling this again with [`Termination::Terminate`] quits, as a second signal does. A
    ///   repeated [`Termination::Hangup`] changes nothing, and one during a pending terminate is
    ///   told to the application again.
    ///
    /// The harness keeps drawing after a hangup, so a test can still read the screen; the
    /// runtime stops drawing, since the terminal is gone.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::runtime::Termination;
    ///
    /// struct Editor;
    ///
    /// impl App for Editor {
    ///     type Msg = ();
    ///     fn update(&mut self, (): ()) -> Command<()> {
    ///         Command::none()
    ///     }
    ///     fn view(&self, ui: &mut View<'_, ()>) {
    ///         ui.add(Text::new("notes.md"));
    ///     }
    /// }
    ///
    /// // An application that implements nothing quits cleanly on either signal.
    /// let mut app = Harness::new(Editor, 20, 3);
    /// app.terminate(Termination::Terminate);
    /// assert!(app.quit_requested());
    /// ```
    pub fn terminate(&mut self, cause: Termination) -> &mut Self {
        self.engine.terminate(cause, self.now);
        self.render()
    }

    /// Delivers a key event exactly as given, without moving the clock: a
    /// [`KeyKind::Repeat`](crate::event::KeyKind::Repeat) or
    /// [`KeyKind::Release`](crate::event::KeyKind::Release) from a terminal with the kitty
    /// keyboard protocol, or a press repeated by a held key.
    pub fn key(&mut self, event: KeyEvent) -> &mut Self {
        self.engine.handle(Event::Key(event), self.now);
        self.render()
    }

    /// Switches theme, as `Command::set_theme` would.
    pub fn set_theme(&mut self, id: &str) -> &mut Self {
        self.engine.env.set_theme(id);
        self.render()
    }

    /// Switches language, as `Command::set_locale` would.
    pub fn set_locale(&mut self, code: &str) -> &mut Self {
        self.engine.env.set_locale(code);
        self.render()
    }

    /// Sets the region, as `Command::set_region` would.
    pub fn set_region(&mut self, region: Option<&str>) -> &mut Self {
        self.engine.env.set_region(region);
        self.render()
    }

    /// Turns reduced motion on or off.
    pub fn set_reduced_motion(&mut self, reduced: bool) -> &mut Self {
        self.engine.env.set_reduced_motion(reduced);
        self.render()
    }

    /// Draws as a terminal with `depth` colours would. Cells then carry palette indices instead of
    /// colours, which [`Harness::fg`] and [`Harness::bg`] cannot read; compare
    /// [`Harness::buffer`] cells for those.
    pub fn set_depth(&mut self, depth: ColorDepth) -> &mut Self {
        self.engine.env.set_depth(depth);
        self.render()
    }

    /// Answers the graphics probe as a terminal that shows pictures with `graphics` would. A
    /// harness asks no terminal, so until this is called it answers
    /// [`Graphics::HalfBlock`](crate::graphics::Graphics::HalfBlock). The rules of
    /// [`Env::graphics`](crate::env::Env::graphics) still apply: with [`Harness::set_depth`] at
    /// 16 colours or [`Harness::set_glyph_mode`] at ASCII no picture is drawn, whatever is set here.
    pub fn set_graphics(&mut self, graphics: crate::graphics::Graphics) -> &mut Self {
        self.engine.env.set_terminal_graphics(graphics);
        self.render()
    }

    /// Draws as a terminal at the other end of a remote connection would:
    /// [`Env::remote`](crate::env::Env::remote) answers `remote` in every view that follows. A
    /// harness is local until this is called, so a test draws the same wherever it runs, over
    /// SSH included.
    pub fn set_remote(&mut self, remote: bool) -> &mut Self {
        self.engine.env.set_remote(remote);
        self.render()
    }

    /// Switches the glyph column drawn.
    pub fn set_glyph_mode(&mut self, mode: GlyphMode) -> &mut Self {
        self.engine.env.set_glyph_mode(mode);
        self.render()
    }

    /// Resizes the screen to `width` × `height` and renders, as a terminal resize does in the
    /// runtime: the backend hands the engine a fresh, empty buffer of the new size and the next
    /// frame is drawn in full. A new size reaches [`App::resized`] before that frame is built.
    pub fn resize(&mut self, width: u16, height: u16) -> &mut Self {
        self.buffer = Buffer::empty(BufferRect::new(0, 0, width, height));
        self.engine.dirty = true;
        self.render()
    }

    /// The screen as text, one line per row, trailing spaces removed. A double-width character
    /// reads as itself, without the cell it covers, so `防火墙` is found as it is written.
    #[must_use]
    pub fn screen(&self) -> String {
        let mut out = String::new();
        for y in 0..self.buffer.area.height {
            out.push_str(self.row(y).0.trim_end());
            out.push('\n');
        }
        out
    }

    /// The screen as a self-contained HTML fragment with colours and weights, for looking at
    /// renders in a browser. Wrap fragments with [`html_page`] to get a document.
    #[must_use]
    pub fn html(&self, caption: &str) -> String {
        let area = self.buffer.area;
        let escape = |text: &str| text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let css = |color: Color| rgb(color).map_or_else(|| "inherit".to_owned(), |c| c.to_string());
        let mut out = format!("<figure><figcaption>{}</figcaption><div class=\"screen\">", escape(caption));
        for y in 0..area.height {
            out.push_str("<div class=\"row\">");
            for x in visible_columns(&self.buffer, y) {
                let cell = &self.buffer[(x, y)];
                let modifier = cell.modifier;
                let weight = if modifier.contains(Modifier::BOLD) { "font-weight:700;" } else { "" };
                let style = if modifier.contains(Modifier::ITALIC) { "font-style:italic;" } else { "" };
                let line = if modifier.contains(Modifier::UNDERLINED) { "text-decoration:underline;" } else { "" };
                out.push_str(&format!(
                    "<span style=\"color:{};background:{};width:{}ch;{weight}{style}{line}\">{}</span>",
                    css(cell.fg),
                    css(cell.bg),
                    crate::text::width(cell.symbol()).max(1),
                    escape(cell.symbol())
                ));
            }
            out.push_str("</div>");
        }
        out.push_str("</div></figure>");
        out
    }

    /// Screen position of the first occurrence of `text`, in cells; text after a double-width
    /// character is found at the column it is drawn in.
    #[must_use]
    pub fn find(&self, text: &str) -> Option<(i32, i32)> {
        (0..self.buffer.area.height).find_map(|y| {
            let (line, columns) = self.row(y);
            line.find(text).map(|byte| (i32::from(columns[byte]), i32::from(y)))
        })
    }

    /// Row `y` of the screen as text, with the column each byte of that text was drawn in.
    fn row(&self, y: u16) -> (String, Vec<u16>) {
        let mut line = String::new();
        let mut columns = Vec::new();
        for x in visible_columns(&self.buffer, y) {
            let symbol = self.buffer[(x, y)].symbol();
            columns.extend(std::iter::repeat_n(x, symbol.len()));
            line.push_str(symbol);
        }
        (line, columns)
    }

    /// Text colour of a cell.
    ///
    /// # Panics
    ///
    /// Panics when the cell is outside the screen.
    #[must_use]
    pub fn fg(&self, x: u16, y: u16) -> Option<Rgb> {
        rgb(self.buffer[(x, y)].fg)
    }

    /// Background colour of a cell.
    ///
    /// # Panics
    ///
    /// Panics when the cell is outside the screen.
    #[must_use]
    pub fn bg(&self, x: u16, y: u16) -> Option<Rgb> {
        rgb(self.buffer[(x, y)].bg)
    }

    /// Whether a cell is bold.
    ///
    /// # Panics
    ///
    /// Panics when the cell is outside the screen.
    #[must_use]
    pub fn is_bold(&self, x: u16, y: u16) -> bool {
        self.buffer[(x, y)].modifier.contains(Modifier::BOLD)
    }

    /// The rendered buffer.
    #[must_use]
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// The application.
    #[must_use]
    pub fn app(&self) -> &A {
        &self.engine.app
    }

    /// The environment.
    #[must_use]
    pub fn env(&self) -> &Env {
        &self.engine.env
    }

    /// The pointer shape the last frame asks for where the pointer is, as the terminal runtime
    /// would send it to a terminal that understands pointer shapes. The harness records the
    /// request whatever terminal a real run would meet; nothing is written anywhere.
    #[must_use]
    pub fn pointer_shape(&self) -> PointerShape {
        self.engine.pointer_shape()
    }

    /// Texts copied to the clipboard so far.
    #[must_use]
    pub fn copied(&self) -> &[String] {
        &self.engine.clipboard
    }

    /// The in-process clipboard: the text copied last, if any.
    #[must_use]
    pub fn clipboard(&self) -> Option<&str> {
        self.engine.clipboard_text.as_deref()
    }

    /// Stands in for the system clipboard that pasting reads first: `Some` text as if the user
    /// had copied it in another program, `None` for an empty clipboard (the start). A harness
    /// never reads the real clipboard or asks a terminal, so without this pasting uses the text
    /// the application copied last.
    pub fn set_system_clipboard(&mut self, text: Option<&str>) -> &mut Self {
        let system = super::clipboard::SystemClipboard::Fixed(text.map(str::to_owned));
        self.engine.clipboard_reader.set_system(system);
        self
    }

    /// The handoffs of [`Command::handoff`](super::Command::handoff) the application asked for,
    /// oldest first. A harness has no terminal to hand over, so it records the request and
    /// answers it with the outcome of [`Harness::set_handoff_outcome`] instead of running the
    /// program.
    #[must_use]
    pub fn handoffs(&self) -> &[HandoffRequest] {
        self.engine.handoff_requests()
    }

    /// The outcome every handoff from now on ends with; `Finished { code: Some(0) }` without
    /// this.
    pub fn set_handoff_outcome(&mut self, outcome: HandoffOutcome) -> &mut Self {
        self.engine.set_handoff_outcome(outcome);
        self
    }

    /// The handoffs of [`Command::handoff_detached`](super::Command::handoff_detached) the
    /// application asked for, oldest first. Like [`Harness::handoffs`] they are recorded, not
    /// run, and answered with the outcome of [`Harness::set_detached_outcome`].
    #[must_use]
    pub fn detached_handoffs(&self) -> &[HandoffRequest] {
        self.engine.detached_requests()
    }

    /// The outcome every detached handoff from now on ends with; `Finished { code: Some(0) }`
    /// without this. A [`DetachedOutcome::Detached`] with the child of
    /// [`LiveChild::for_tests`](super::LiveChild::for_tests) lets the test play the program: what
    /// the application writes is recorded on its [`TestChild`](super::TestChild), and the lines
    /// the test says there reach [`DetachedHandoff::on_line`](super::DetachedHandoff::on_line)
    /// at the next step.
    ///
    /// The harness keeps the outcome, and with it a clone of the child, until it is given
    /// another or dropped; the child's input closes then at the latest, as it does when a real
    /// run ends.
    pub fn set_detached_outcome(&mut self, outcome: DetachedOutcome) -> &mut Self {
        self.engine.set_detached_outcome(outcome);
        self
    }

    /// The openings of [`Command::open`](super::Command::open) and
    /// [`Command::open_with`](super::Command::open_with) the application asked for, oldest first.
    ///
    /// A harness reaches no desktop: the opening is recorded and answered with the outcome of
    /// [`Harness::set_open_outcome`] instead of starting anything. [`OpenRequest::target`] is
    /// what was asked to be opened, so a test reads the address without knowing which opener
    /// this system has.
    #[must_use]
    pub fn opens(&self) -> &[OpenRequest] {
        self.engine.open_requests()
    }

    /// The outcome every opening from now on ends with; [`OpenOutcome::Opened`] without this.
    pub fn set_open_outcome(&mut self, outcome: OpenOutcome) -> &mut Self {
        self.engine.set_open_outcome(outcome);
        self
    }

    /// The questions for a newer version of
    /// [`Command::check_for_update`](super::Command::check_for_update) the application asked,
    /// oldest first. A harness reaches no network and reads or writes none of the check's
    /// folders: the question is recorded and answered with the version of
    /// [`set_latest_version`](Self::set_latest_version), if any.
    #[cfg(feature = "updates")]
    #[must_use]
    pub fn update_checks(&self) -> &[super::UpdateCheckRequest] {
        self.engine.update_checks()
    }

    /// Answers the questions for a newer version as if the registry had named `latest` the newest,
    /// the ones asked already and every one from now on, then renders. With `None`, the default,
    /// a question gets no answer, as when the network is down. The application's message arrives
    /// only when the version is newer than the one it runs.
    #[cfg(feature = "updates")]
    pub fn set_latest_version(&mut self, latest: Option<&str>) -> &mut Self {
        self.engine.set_latest_version(latest.map(str::to_owned));
        self.render()
    }

    /// Whether the application asked to quit.
    #[must_use]
    pub fn quit_requested(&self) -> bool {
        self.engine.quit
    }

    /// Whether the widget named `name` has keyboard focus.
    #[must_use]
    pub fn is_focused(&self, name: &str) -> bool {
        self.engine.interaction.focused.is_some_and(|id| self.engine.frame.names.get(&id).is_some_and(|n| n == name))
    }
}

/// Wraps [`Harness::html`] fragments in an HTML document that lays screens out on a dark page.
#[must_use]
pub fn html_page(fragments: &[String]) -> String {
    format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Quvyta review</title><style>\
         body{{background:#050507;margin:24px;font-family:'JetBrainsMono Nerd Font Mono','JetBrains Mono',monospace}}\
         figure{{margin:0 0 28px}}figcaption{{color:#8a8f99;font:12px sans-serif;margin-bottom:6px}}\
         .screen{{display:inline-block;font-size:14px;line-height:19px;white-space:pre}}\
         .row{{display:flex;height:19px}}.row span{{display:inline-block;overflow:hidden}}</style>{}",
        fragments.concat()
    )
}

/// The columns of row `y` a terminal shows a symbol of: every one except those a wide
/// character before them covers. Such a cell holds nothing or, when ratatui or a painter unaware
/// of the character drew it, a space; reading it would split `防火墙` into `防 火 墙`.
fn visible_columns(buffer: &Buffer, y: u16) -> impl Iterator<Item = u16> + '_ {
    let mut covered = 0u16;
    (0..buffer.area.width).filter(move |&x| {
        if covered > 0 {
            covered -= 1;
            return false;
        }
        let symbol = buffer[(x, y)].symbol();
        covered = crate::text::width(symbol).saturating_sub(1);
        !symbol.is_empty()
    })
}

fn rgb(color: Color) -> Option<Rgb> {
    match color {
        Color::Rgb(r, g, b) => Some(Rgb::new(r, g, b)),
        _ => None,
    }
}

#[cfg(test)]
mod resize_tests {
    use super::Harness;
    use crate::runtime::{App, Command};
    use crate::widget::View;
    use crate::widgets::Text;

    struct Greeting;

    impl App for Greeting {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Text::new("container engines"));
        }
    }

    #[test]
    fn resize_redraws_the_whole_screen_at_the_new_size() {
        let mut harness = Harness::new(Greeting, 30, 2);
        assert_eq!(harness.screen(), "container engines\n\n");
        harness.resize(9, 1);
        assert_eq!(harness.screen(), "container\n");
        harness.resize(0, 0);
        assert_eq!(harness.screen(), "");
        harness.resize(40, 3);
        assert_eq!((harness.buffer().area.width, harness.buffer().area.height), (40, 3));
        assert_eq!(harness.screen(), "container engines\n\n\n");
    }

    struct Raw;

    impl App for Raw {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Text::new("bell\u{7} tab\t\u{1b}[1mé\r"));
        }
    }

    #[test]
    fn a_control_character_handed_to_any_widget_never_reaches_a_cell() {
        let harness = Harness::new(Raw, 30, 1);
        assert_eq!(harness.screen(), "bell  tab  [1mé\n", "each control character is a blank cell");
    }
}

#[cfg(test)]
mod handoff_tests {
    use std::ffi::OsString;

    use super::Harness;
    use crate::runtime::{App, Command, Handoff, HandoffOutcome};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    /// Asks for the authorization ticket and shows how the handoff ended.
    #[derive(Default)]
    struct Installer {
        outcomes: Vec<HandoffOutcome>,
    }

    #[derive(Clone)]
    enum Msg {
        Authorize,
        Done(HandoffOutcome),
    }

    impl App for Installer {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Authorize => Command::handoff(
                    Handoff::new("sudo", Msg::Done).arg("-v").notice("Authorizing the installation").pause(false),
                ),
                Msg::Done(outcome) => {
                    self.outcomes.push(outcome);
                    Command::none()
                }
            }
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.add(Button::new("Authorize").on_press(Msg::Authorize)).id("authorize");
            let text = match self.outcomes.last() {
                None => "not asked yet".to_owned(),
                Some(HandoffOutcome::Finished { code }) => format!("finished {code:?}"),
                Some(HandoffOutcome::Failed(reason)) => format!("failed {reason}"),
            };
            ui.add(Text::new(text));
        }
    }

    #[test]
    fn a_handoff_is_recorded_and_answered_with_the_outcome_the_test_set() {
        let mut harness = Harness::new(Installer::default(), 40, 3);
        assert!(harness.handoffs().is_empty(), "nothing was asked for yet");
        harness.send(Msg::Authorize);
        let asked = harness.handoffs();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].program, OsString::from("sudo"));
        assert_eq!(asked[0].args, vec![OsString::from("-v")]);
        assert_eq!(asked[0].notice.as_deref(), Some("Authorizing the installation"));
        assert!(!asked[0].pause);
        // No program ran: the outcome the harness holds answered the request.
        assert_eq!(harness.app().outcomes, [HandoffOutcome::Finished { code: Some(0) }]);
        assert!(harness.screen().contains("finished Some(0)"), "{}", harness.screen());
    }

    #[test]
    fn the_outcome_a_test_sets_reaches_the_application() {
        let mut harness = Harness::new(Installer::default(), 40, 3);
        harness.set_handoff_outcome(HandoffOutcome::Finished { code: Some(1) });
        harness.send(Msg::Authorize);
        assert_eq!(harness.app().outcomes, [HandoffOutcome::Finished { code: Some(1) }]);
        harness.set_handoff_outcome(HandoffOutcome::Failed("sudo is not installed".to_owned()));
        harness.send(Msg::Authorize);
        assert_eq!(harness.app().outcomes.len(), 2);
        assert!(harness.screen().contains("failed sudo is not installed"), "{}", harness.screen());
        assert_eq!(harness.handoffs().len(), 2, "both requests are kept, oldest first");
    }

    #[test]
    fn several_handoffs_are_answered_one_after_another() {
        let mut harness = Harness::new(Installer::default(), 40, 3);
        harness.send(Msg::Authorize).send(Msg::Authorize).send(Msg::Authorize);
        assert_eq!(harness.handoffs().len(), 3);
        assert_eq!(harness.app().outcomes.len(), 3);
    }
}

#[cfg(test)]
mod wide_text_tests {
    use super::Harness;
    use crate::runtime::{App, Command};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    /// A Chinese status line and a button with a Chinese label that counts its presses.
    #[derive(Default)]
    struct Firewall {
        presses: u32,
    }

    impl App for Firewall {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            self.presses += 1;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Text::new("状态 防火墙 on"));
            ui.add(Button::new("启用").on_press(()));
        }
    }

    /// The screen as ratatui leaves it when it draws wide text itself: the cell each wide
    /// character covers holds a space, as does a cell drawn by any painter that knows nothing of
    /// the character before it.
    fn with_covered_cells_as_spaces(harness: &mut Harness<Firewall>) {
        let area = harness.buffer.area;
        for y in 0..area.height {
            let mut covered = 0;
            for x in 0..area.width {
                let cell = &mut harness.buffer[(x, y)];
                if covered > 0 {
                    covered -= 1;
                    cell.reset();
                    continue;
                }
                covered = crate::text::width(cell.symbol()).saturating_sub(1);
            }
        }
    }

    fn firewall() -> Harness<Firewall> {
        let mut harness = Harness::new(Firewall::default(), 30, 3);
        with_covered_cells_as_spaces(&mut harness);
        harness
    }

    #[test]
    fn the_screen_reads_wide_text_without_gaps() {
        let harness = firewall();
        let screen = harness.screen();
        assert!(screen.starts_with("状态 防火墙 on\n"), "{screen}");
        assert!(screen.contains("防火墙"), "{screen}");
    }

    #[test]
    fn find_gives_the_column_a_wide_text_is_drawn_in() {
        let harness = firewall();
        assert_eq!(harness.find("防火墙"), Some((5, 0)));
        assert_eq!(harness.find("on"), Some((12, 0)), "text after wide characters keeps its column");
        let (x, y) = harness.find("启用").expect("the button label is on screen");
        assert_eq!(harness.buffer()[(u16::try_from(x).unwrap(), u16::try_from(y).unwrap())].symbol(), "启");
    }

    #[test]
    fn click_text_presses_a_wide_label() {
        let mut harness = firewall();
        harness.click_text("启用");
        assert_eq!(harness.app().presses, 1);
    }

    #[test]
    fn html_draws_a_wide_character_once() {
        let harness = firewall();
        let html = harness.html("wide");
        let first_row = html.split("<div class=\"row\">").nth(1).expect("a first row");
        assert_eq!(first_row.matches("<span").count(), 30 - 5, "five characters take two cells each: {first_row}");
        assert!(first_row.contains(">防</span><span"), "{first_row}");
    }
}

#[cfg(test)]
mod graphics_tests {
    use super::Harness;
    use crate::color::ColorDepth;
    use crate::graphics::Graphics;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command};
    use crate::widget::View;
    use crate::widgets::Text;

    /// Shows the graphics its view reads from the environment.
    struct Pictures;

    impl App for Pictures {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let graphics = ui.env().graphics();
            ui.add(Text::new(graphics.name()));
        }
    }

    #[test]
    fn a_harness_answers_half_blocks_until_told_otherwise() {
        let mut harness = Harness::new(Pictures, 20, 1);
        assert_eq!(harness.screen(), "halfblock\n", "no terminal is asked in a test");
        harness.set_graphics(Graphics::Kitty);
        assert_eq!(harness.screen(), "kitty\n");
        harness.set_graphics(Graphics::Sixel);
        assert_eq!(harness.screen(), "sixel\n");
    }

    #[test]
    fn the_rules_still_apply_to_what_a_harness_is_told() {
        let mut harness = Harness::new(Pictures, 20, 1);
        harness.set_graphics(Graphics::Kitty).set_depth(ColorDepth::Ansi16);
        assert_eq!(harness.screen(), "none\n", "16 colours show no picture");
        harness.set_depth(ColorDepth::TrueColor).set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(harness.screen(), "none\n", "ASCII glyphs show no picture");
        harness.set_glyph_mode(GlyphMode::Unicode);
        assert_eq!(harness.screen(), "kitty\n");
    }

    /// Says whether its view is drawn for a remote connection.
    struct Link;

    impl App for Link {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Text::new(if ui.env().remote() { "remote" } else { "local" }));
        }
    }

    #[test]
    fn a_harness_draws_a_remote_screen_when_told_so() {
        let mut harness = Harness::new(Link, 20, 1);
        assert_eq!(harness.screen(), "local\n", "a test draws the same wherever it runs");
        harness.set_remote(true);
        assert_eq!(harness.screen(), "remote\n");
        harness.set_remote(false);
        assert_eq!(harness.screen(), "local\n");
    }
}
