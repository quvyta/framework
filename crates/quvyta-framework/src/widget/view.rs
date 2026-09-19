//! Building the view tree.

use std::time::Duration;

use super::flex::{Axis, Flex};
use super::idle::{IdleScope, IdleWatch};
use super::mapped::Mapped;
use super::place::Placed;
use super::{Align, Container, FocusAction, Key, Length, Node, Widget};
use crate::env::Env;
use crate::geometry::{Padding, Rect, Size};
use crate::keymap::Scope;

/// Collects the nodes of one container while an application's `view` runs.
pub struct View<'a, Msg> {
    nodes: &'a mut Vec<Node<Msg>>,
    env: &'a Env,
    size: Size,
    idle: &'a IdleScope<Msg>,
}

impl<'a, Msg: 'static> View<'a, Msg> {
    pub(crate) fn new(nodes: &'a mut Vec<Node<Msg>>, env: &'a Env, size: Size, idle: &'a IdleScope<Msg>) -> Self {
        Self { nodes, env, size, idle }
    }

    /// A builder for the children of a container inside this one: the same environment and size.
    /// The idleness this view reads and declares watches in, for builders that make views of
    /// their own.
    pub(crate) fn idle_scope(&self) -> &'a IdleScope<Msg> {
        self.idle
    }

    pub(crate) fn nested<'b>(&self, nodes: &'b mut Vec<Node<Msg>>) -> View<'b, Msg>
    where
        'a: 'b,
    {
        View::new(nodes, self.env, self.size, self.idle)
    }

    /// The environment: theme, icons, language and keymap.
    #[must_use]
    pub fn env(&self) -> &Env {
        self.env
    }

    /// The room the application is drawing into: the whole terminal, in columns and rows.
    ///
    /// This is the value for an application's own layout decision, such as "below 48 columns,
    /// fold the three columns into one": `if ui.size().width < 48 { .. } else { .. }` in `view`.
    ///
    /// The application's view fills the screen, so at the top of `view` this is exactly the
    /// area it lays out. Every nested builder reports the same value: the children of `column`,
    /// `row`, `stack`, `page` and `add_with`, the parts of an `AppShell`, `SidePanel`,
    /// `Splitter` or `Popover`, the content of a `Modal` or other layer. The view is built
    /// before layout divides the screen, so a container's own share is not known yet while its
    /// children are being built; the number never pretends to be that share. A widget that
    /// adapts to its own rectangle (a column that shortens its labels) does so in `measure` and
    /// `paint`, which receive it.
    ///
    /// Reading it performs no I/O: it is the size of the frame the framework is about to draw,
    /// which it already holds. After a terminal resize the next frame reports the new size, and
    /// [`Harness::resize`](crate::runtime::Harness::resize) does the same in tests.
    #[must_use]
    pub fn size(&self) -> Size {
        self.size
    }

    /// How long no input has reached this terminal: the time since the last key, mouse event or
    /// paste the runtime received, or since the application started when none came yet.
    ///
    /// Everything the user does in this terminal counts: a key going down, repeating or coming
    /// up, a mouse button, the wheel, the pointer moving over the window, a paste, and the end of
    /// a [`Handoff`](crate::runtime::Handoff), because the program that had the terminal was
    /// being used meanwhile. A terminal resize does not count: a window manager or a monitor
    /// change resizes a window nobody is sitting at. Messages, background work and timers do not
    /// count either; they are the application, not the user. Other programs and other terminals
    /// are out of reach: this is idleness *here*, not idleness of the machine.
    ///
    /// Reading the value keeps it current on screen: while `view` reads it, the runtime draws
    /// again each time it passes a whole second, and stops once `view` no longer reads it. A
    /// view that shows minutes therefore redraws once a second while it shows them; one that
    /// only needs to act after a silence uses [`View::on_idle`], which wakes the application
    /// once, at that moment, without drawing in between.
    ///
    /// [`Harness::advance`](crate::runtime::Harness::advance) moves it forward in tests, and
    /// every simulated input starts it again from zero.
    #[must_use]
    pub fn idle_for(&self) -> Duration {
        self.idle.read.set(true);
        self.idle.silent
    }

    /// Tells the application when no input has arrived for `after`, and when input comes back.
    ///
    /// `message(true)` is delivered once, at the moment the silence reaches `after`: the runtime
    /// wakes for it even when nothing else happens, and does not draw in between. The first
    /// input afterwards delivers `message(false)`, before that input reaches any widget, and
    /// starts the next wait. What counts as input is listed at [`View::idle_for`].
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use qframe::prelude::*;
    ///
    /// #[derive(Default)]
    /// struct Focus {
    ///     away: bool,
    /// }
    ///
    /// impl App for Focus {
    ///     type Msg = bool;
    ///     fn update(&mut self, away: bool) -> Command<bool> {
    ///         self.away = away;
    ///         Command::none()
    ///     }
    ///     fn view(&self, ui: &mut View<'_, bool>) {
    ///         ui.on_idle(Duration::from_secs(300), |away| away);
    ///         ui.add(Text::new(if self.away { "away" } else { "working" }));
    ///     }
    /// }
    ///
    /// let mut app = Harness::new(Focus::default(), 20, 1);
    /// app.advance(Duration::from_secs(299));
    /// assert!(app.screen().contains("working"));
    /// app.advance(Duration::from_secs(1));
    /// assert!(app.screen().contains("away"));
    /// app.press("x");
    /// assert!(app.screen().contains("working"));
    /// ```
    ///
    /// Declare the watch in every frame it should stay active, like a widget: the runtime
    /// answers the watches of the latest frame. One that is no longer declared is not told the
    /// silence ended. A watch declared when the silence has already lasted `after` is told at
    /// once. Watches with different `after` are independent, so an application can dim the
    /// screen after one minute and pause a timer after five.
    pub fn on_idle(&mut self, after: Duration, message: impl Fn(bool) -> Msg + 'static) {
        self.idle.watches.borrow_mut().push(IdleWatch { after, message: Box::new(message) });
    }

    /// Adds a widget.
    pub fn add<W: Widget<Msg>>(&mut self, widget: W) -> NodeMut<'_, Msg> {
        let index = self.nodes.len();
        self.nodes.push(Node::new(widget, index));
        NodeMut { node: self.nodes.last_mut().expect("a node was just pushed") }
    }

    /// Adds a widget that contains other widgets, built by `build`.
    pub fn add_with<W: Container<Msg>>(
        &mut self,
        mut widget: W,
        build: impl FnOnce(&mut View<'_, Msg>),
    ) -> NodeMut<'_, Msg> {
        let mut children = Vec::new();
        build(&mut self.nested(&mut children));
        widget.set_children(children);
        self.add(widget)
    }

    /// Adds a column whose children are built by `build`.
    pub fn column(&mut self, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        self.container(Axis::Column, build)
    }

    /// Adds a row whose children are built by `build`.
    pub fn row(&mut self, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        self.container(Axis::Row, build)
    }

    /// Adds a stack: children are drawn on top of each other in the same area, later ones on top.
    pub fn stack(&mut self, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        self.container(Axis::Stack, build)
    }

    /// Adds children at `rect`, for a stack whose children sit where the application says, such
    /// as windows on a desktop.
    ///
    /// Inside a [`stack`](Self::stack), `rect` counts from the stack's top left corner, whatever
    /// the stack's alignment: the children fill it, drawn on top of each other. A rectangle may
    /// reach past the stack on any side, also to negative coordinates; what lies outside is not
    /// drawn and takes no pointer. Children added later are drawn on top and get the pointer
    /// first where they overlap, so the order of the calls is the stacking order. A placed child
    /// may draw one cell past its right and bottom edges, where a window drops its shadow; that
    /// cell never takes the pointer.
    /// Outside a stack only the size of `rect` counts.
    ///
    /// Name every placed child whose position in the stack can change, as when a clicked window
    /// comes to the front: `ui.place(rect, ..).id("htop")`. Its state, and a drag it is in the
    /// middle of, follow the name.
    ///
    /// ```
    /// use qframe::prelude::*;
    ///
    /// struct Desk;
    ///
    /// impl App for Desk {
    ///     type Msg = ();
    ///     fn update(&mut self, (): ()) -> Command<()> {
    ///         Command::none()
    ///     }
    ///     fn view(&self, ui: &mut View<'_, ()>) {
    ///         ui.stack(|ui| {
    ///             ui.place(Rect::new(2, 1, 6, 1), |ui| {
    ///                 ui.add(Text::new("below"));
    ///             })
    ///             .id("first");
    ///             ui.place(Rect::new(6, 1, 5, 1), |ui| {
    ///                 ui.add(Text::new("above"));
    ///             })
    ///             .id("second");
    ///         })
    ///         .fill();
    ///     }
    /// }
    ///
    /// let app = Harness::new(Desk, 12, 2);
    /// assert_eq!(app.screen(), "\n  beloabove\n");
    /// ```
    pub fn place(&mut self, rect: Rect, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        let mut children = Vec::new();
        build(&mut self.nested(&mut children));
        self.add(Placed::new(rect, children)).width(Length::Cells(rect.width)).height(Length::Cells(rect.height))
    }

    /// Adds a column that remembers its widgets' state (focus, scroll, cursors) while it is not
    /// shown. Give every page of a router its own `key`.
    pub fn page(&mut self, key: impl Into<String>, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        let node = self.container(Axis::Column, build);
        node.node.persistent = true;
        node.node.key = Key::Named(key.into());
        node.fill()
    }

    /// Adds a column whose children are built by `build` with messages of their own type
    /// `Inner`, each converted by `map` on its way to the application. A screen with its own
    /// messages writes its view for them, and the application places it in one line:
    ///
    /// ```
    /// use qframe::prelude::*;
    ///
    /// mod search {
    ///     use qframe::prelude::*;
    ///
    ///     #[derive(Clone)]
    ///     pub enum Msg {
    ///         Run,
    ///     }
    ///
    ///     pub fn view(ui: &mut View<'_, Msg>) {
    ///         ui.add(Button::new("Search").on_press(Msg::Run));
    ///     }
    /// }
    ///
    /// enum Msg {
    ///     Search(search::Msg),
    /// }
    ///
    /// fn view(ui: &mut View<'_, Msg>) {
    ///     ui.map(Msg::Search, search::view).fill();
    /// }
    /// ```
    ///
    /// Everything the screen does inside arrives converted: the messages of its widgets and
    /// handlers, the children of [`add_with`](Self::add_with) and nested containers, layers such
    /// as a `Modal` and the widgets in them, overlays such as an open dropdown. Focus, memory and
    /// ids work as for any column; [`Command::map`](crate::runtime::Command::map) converts the
    /// commands the screen's `update` returns the same way.
    pub fn map<Inner: 'static>(
        &mut self,
        map: impl Fn(Inner) -> Msg + 'static,
        build: impl FnOnce(&mut View<'_, Inner>),
    ) -> NodeMut<'_, Msg> {
        let map = std::rc::Rc::new(map);
        let mut children = Vec::new();
        // The screen reads and watches the same silence as the application; what it read and the
        // watches it declared are handed up, their messages converted like any other.
        let idle = IdleScope::new(self.idle.silent);
        build(&mut View::new(&mut children, self.env, self.size, &idle));
        if idle.read.get() {
            self.idle.read.set(true);
        }
        for watch in idle.watches.into_inner() {
            let map = std::rc::Rc::clone(&map);
            let message = watch.message;
            self.idle
                .watches
                .borrow_mut()
                .push(IdleWatch { after: watch.after, message: Box::new(move |away| map(message(away))) });
        }
        self.add(Mapped::new(children, move |inner| map(inner)))
    }

    /// Adds empty space that takes the room left in a row or column.
    pub fn spacer(&mut self) -> NodeMut<'_, Msg> {
        self.container(Axis::Stack, |_| {}).fill()
    }

    fn container(&mut self, axis: Axis, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        let mut children = Vec::new();
        build(&mut self.nested(&mut children));
        self.add(Flex::new(axis, children))
    }
}

/// Adjusts the node just added. Every method changes the node in place, so the result can be
/// ignored or chained.
pub struct NodeMut<'a, Msg> {
    node: &'a mut Node<Msg>,
}

impl<'a, Msg> NodeMut<'a, Msg> {
    /// Names the node. Name widgets whose position among their siblings can change (list rows,
    /// optional widgets) so their state and focus follow them.
    pub fn id(self, name: impl Into<String>) -> Self {
        self.node.key = Key::Named(name.into());
        self
    }

    /// Sets the width.
    pub fn width(self, width: Length) -> Self {
        self.node.layout.width = width;
        self
    }

    /// Sets the height.
    pub fn height(self, height: Length) -> Self {
        self.node.layout.height = height;
        self
    }

    /// Takes all space left in both directions.
    pub fn fill(self) -> Self {
        self.width(Length::Fill(1)).height(Length::Fill(1))
    }

    /// Takes all width left.
    pub fn fill_width(self) -> Self {
        self.width(Length::Fill(1))
    }

    /// Takes all height left.
    pub fn fill_height(self) -> Self {
        self.height(Length::Fill(1))
    }

    /// Keeps `padding` free inside the node.
    pub fn padding(self, padding: Padding) -> Self {
        self.node.layout.padding = padding;
        self
    }

    /// Leaves `cells` between the children of a row or column.
    pub fn gap(self, cells: u16) -> Self {
        self.node.layout.gap = cells;
        self
    }

    /// Places children along the main axis of a row or column (both axes of a stack).
    pub fn justify(self, align: Align) -> Self {
        self.node.layout.justify = align;
        self
    }

    /// Whether a mouse drag may select text in this node. Nothing is selectable unless asked:
    /// `true` makes the node a selection region, so a drag that starts inside it selects text
    /// within the node only (widgets such as `CodeView` and `Markdown` are regions by
    /// themselves). `false` keeps selection out of the node and everything inside it, also out
    /// of regions within it, e.g. for a secret shown inside a selectable log.
    pub fn selectable(self, selectable: bool) -> Self {
        self.node.selectable = Some(selectable);
        self
    }

    /// Places children across the main axis of a row or column.
    pub fn align(self, align: Align) -> Self {
        self.node.layout.align = align;
        self
    }
}

impl<Msg: Clone + 'static> NodeMut<'_, Msg> {
    /// While keyboard focus is on this node or inside it, a key bound to the keymap action
    /// `action` of `scope` sends `message` instead of reaching [`App::action`](crate::runtime::App::action).
    ///
    /// This is how an application tells where a shortcut was pressed. With focus elsewhere the
    /// same key reaches `App::action` as usual, so one key can mean two things: leave a
    /// terminal while inside it, go back into it from outside. The focus in force when the key
    /// arrives decides, however it got there (`tab`, a click, [`Command::focus`](crate::runtime::Command::focus)),
    /// so nothing has to be tracked in application state.
    ///
    /// The innermost node that answers the action wins. The key must first get past the focused
    /// widgets: a widget that uses it (a text field typing a character) keeps it, and a
    /// [`Terminal`](crate::widgets::Terminal) lets it out only for actions named with its
    /// `pass_through`. The actions the runtime owns (`quit`, `focus-next`, `focus-prev`,
    /// `debug`, `copy`, `paste`, `toggle-panel`) are not answered here. Call once per action.
    ///
    /// ```
    /// use qframe::env::Env;
    /// use qframe::prelude::*;
    /// use qframe::widgets::TextInput;
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// enum Msg {
    ///     Leave,
    ///     Enter,
    /// }
    ///
    /// struct Editor;
    ///
    /// impl App for Editor {
    ///     type Msg = Msg;
    ///
    ///     fn update(&mut self, msg: Msg) -> Command<Msg> {
    ///         match msg {
    ///             Msg::Leave => Command::focus("files"),
    ///             Msg::Enter => Command::focus("note"),
    ///         }
    ///     }
    ///
    ///     fn view(&self, ui: &mut View<'_, Msg>) {
    ///         ui.add(List::new(["notes.md", "todo.md"].map(ListItem::new))).id("files");
    ///         ui.add(TextInput::new("")).id("note").on_action(Scope::App, "switch", Msg::Leave);
    ///     }
    ///
    ///     // Reached only while focus is outside the note.
    ///     fn action(&self, name: &str) -> Option<Msg> {
    ///         (name == "switch").then_some(Msg::Enter)
    ///     }
    /// }
    ///
    /// let mut env = Env::builtin();
    /// env.keymap_mut().bind(Scope::App, "switch", &["alt+s".parse().unwrap()]);
    /// let mut app = Harness::with_env(Editor, env, 30, 3);
    /// app.press("alt+s");
    /// assert!(app.is_focused("note"));
    /// app.press("alt+s");
    /// assert!(app.is_focused("files"));
    /// ```
    pub fn on_action(self, scope: Scope, action: impl Into<String>, message: Msg) -> Self {
        self.node.actions.push(FocusAction {
            scope,
            action: action.into(),
            message: Box::new(move || message.clone()),
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::geometry::Size;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};
    use crate::widgets::{Modal, Text};

    /// Records the size every builder of its view saw: the root, a nested column and a row with
    /// a fixed width inside it, and the content of a modal layer.
    #[derive(Default)]
    struct Probe {
        seen: RefCell<Vec<Size>>,
    }

    impl Probe {
        fn take(&self) -> Vec<Size> {
            std::mem::take(&mut *self.seen.borrow_mut())
        }
    }

    impl App for Probe {
        type Msg = ();

        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }

        fn view(&self, ui: &mut View<'_, ()>) {
            self.seen.borrow_mut().push(ui.size());
            ui.column(|ui| {
                self.seen.borrow_mut().push(ui.size());
                ui.row(|ui| {
                    self.seen.borrow_mut().push(ui.size());
                    ui.add(Text::new("probe"));
                })
                .width(Length::Cells(10));
            });
            ui.add_with(Modal::new(), |ui| {
                self.seen.borrow_mut().push(ui.size());
                ui.add(Text::new("layer"));
            });
        }
    }

    #[test]
    fn every_builder_of_the_view_sees_the_terminal_size() {
        let mut harness = Harness::new(Probe::default(), 83, 27);
        let seen = harness.app().take();
        assert!(seen.len() >= 4, "{seen:?}");
        assert!(seen.iter().all(|size| *size == Size::new(83, 27)), "{seen:?}");

        harness.resize(31, 9);
        let seen = harness.app().take();
        assert!(seen.len() >= 4, "{seen:?}");
        assert!(seen.iter().all(|size| *size == Size::new(31, 9)), "{seen:?}");

        harness.resize(0, 0);
        let seen = harness.app().take();
        assert!(seen.len() >= 4, "{seen:?}");
        assert!(seen.iter().all(|size| *size == Size::new(0, 0)), "{seen:?}");
    }

    /// Three columns side by side from 48 columns up; below that the groups fold into a strip
    /// above the list and the detail is left out.
    struct Folding;

    impl App for Folding {
        type Msg = ();

        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }

        fn view(&self, ui: &mut View<'_, ()>) {
            if ui.size().width < 48 {
                ui.column(|ui| {
                    ui.add(Text::new("groups"));
                    ui.add(Text::new("items"));
                })
                .fill();
            } else {
                ui.row(|ui| {
                    ui.add(Text::new("groups"));
                    ui.add(Text::new("items"));
                    ui.add(Text::new("detail"));
                })
                .gap(2)
                .fill();
            }
        }
    }

    #[test]
    fn an_application_folds_its_layout_below_a_width() {
        let mut harness = Harness::new(Folding, 120, 10);
        assert_eq!(harness.screen().lines().next(), Some("groups  items  detail"), "{}", harness.screen());

        harness.resize(40, 10);
        let screen = harness.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(lines.get(..2), Some(&["groups", "items"][..]), "{screen}");
        assert!(!screen.contains("detail"), "{screen}");

        harness.resize(120, 10);
        assert_eq!(harness.screen().lines().next(), Some("groups  items  detail"), "{}", harness.screen());
    }
}
