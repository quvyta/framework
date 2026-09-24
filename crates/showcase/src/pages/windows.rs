//! Windows: a small desktop of three windows that are dragged, resized, focused, minimized,
//! maximized and closed, with the window manager's policy kept in the page's own state.

use qframe::prelude::*;
use qframe::widgets::{EmptyState, Ghost, IconButton, Segmented, Window, WindowDrag, WindowEdge, WindowEvent};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "windows";

/// The desktop the demo draws its windows on, in columns and rows. The demo keeps its windows
/// inside it, as a window manager does with the screen.
const DESK: (u16, u16) = (58, 16);

/// The smallest window the demo allows, as in qdesk.
const MIN: (u16, u16) = (20, 5);

/// How far the arrow buttons move or resize the focused window.
const STEP: i32 = 2;

/// How much accent a dragged ghost and a snap preview mix into the ground.
const GHOST_MIX: f32 = 0.25;
const SNAP_MIX: f32 = 0.20;

/// One window of the demo.
#[derive(Debug, Clone)]
struct Win {
    name: &'static str,
    icon: &'static str,
    subtitle: &'static str,
    body: [&'static str; 2],
    rect: Rect,
    maximized: bool,
    minimized: bool,
    /// The size to go back to when the window is restored.
    restore: Rect,
}

/// The windows, bottom first; the last one shown is focused.
#[derive(Debug)]
pub struct State {
    windows: Vec<Win>,
    shadow: bool,
    /// Whether the arrow buttons move (0) or resize (1) the focused window.
    mode: usize,
    /// Whether a dragged window stays put and only a ghost moves, as over a slow connection.
    ghost_drag: bool,
    /// The ghost of the window being dragged, while ghost drag is on.
    drag: Option<(&'static str, Rect)>,
    /// The window being moved or resized and its rectangle when the button went down.
    start: Option<(&'static str, Rect)>,
    /// The area the dragged window would snap to when it is let go.
    snap: Option<Rect>,
}

fn desktop() -> Vec<Win> {
    let win = |name, icon, subtitle, body, rect| Win {
        name,
        icon,
        subtitle,
        body,
        rect,
        maximized: false,
        minimized: false,
        restore: rect,
    };
    vec![
        win("notes.md", "file", "edited", ["# Tomorrow", "- check the backups"], Rect::new(0, 0, 26, 7)),
        win("htop", "settings", "142 tasks", ["cpu  38%", "mem  1.2G/3.8G"], Rect::new(22, 2, 30, 8)),
        win("Terminal", "prompt", "~/projects", ["$ cargo build", "   Compiling qdesk"], Rect::new(8, 8, 34, 8)),
    ]
}

impl Default for State {
    fn default() -> Self {
        Self { windows: desktop(), shadow: true, mode: 0, ghost_drag: false, drag: None, start: None, snap: None }
    }
}

/// Demo messages.
#[derive(Debug, Clone, Copy)]
pub enum Msg {
    /// What the pointer did to the window of this name.
    Window(&'static str, WindowEvent),
    /// A step of a move or a resize of the window of this name, with the drag's totals.
    Drag(&'static str, WindowDrag),
    Shadow(bool),
    GhostDrag(bool),
    Mode(usize),
    /// The focused window one step left, right, up or down, or bigger and smaller.
    Nudge(WindowEdge),
    /// Focus the next window, with the keyboard.
    Cycle,
    /// Show a minimized window again.
    Restore(&'static str),
    /// Put the whole desktop back.
    Reopen,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::Windows(message))
}

/// The keymap action of the page: `ctrl+alt+w` focuses the next window without the pointer.
#[must_use]
pub fn action(name: &str) -> Option<AppMsg> {
    (name == "window-cycle").then(|| send(Msg::Cycle))
}

impl State {
    /// The window the keys and the arrow buttons act on: the last one shown.
    fn focused(&self) -> Option<&'static str> {
        self.windows.iter().rev().find(|win| !win.minimized).map(|win| win.name)
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.windows.iter().position(|win| win.name == name)
    }
}

// region: windows-policy
/// The policy the framework leaves to the application: a smallest size, staying on the desktop,
/// and which window is in front.
fn place(rect: Rect) -> Rect {
    let width = rect.width.clamp(MIN.0, DESK.0);
    let height = rect.height.clamp(MIN.1, DESK.1);
    let limit = |at: i32, size: u16, room: u16| at.clamp(0, i32::from(room.saturating_sub(size)));
    Rect::new(limit(rect.x, width, DESK.0), limit(rect.y, height, DESK.1), width, height)
}

/// The area a window dragged against an edge of the desktop would snap to: the left or right
/// half, or the whole desktop at the top edge. The framework only draws the preview; which edge
/// snaps where is the application's rule.
fn snap_target(rect: Rect) -> Option<Rect> {
    let half = DESK.0 / 2;
    if rect.y <= 0 {
        return Some(Rect::new(0, 0, DESK.0, DESK.1));
    }
    if rect.x <= 0 {
        return Some(Rect::new(0, 0, half, DESK.1));
    }
    (rect.right() >= i32::from(DESK.0)).then(|| Rect::new(i32::from(DESK.0 - half), 0, half, DESK.1))
}

/// Applies one movement to a window's rectangle.
fn moved(rect: Rect, event: WindowEvent) -> Rect {
    match event {
        WindowEvent::Move { dx, dy } => place(Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height)),
        WindowEvent::Resize { edge, dx, dy } => resized(rect, edge, dx, dy),
        _ => rect,
    }
}

/// Moves the sides `edge` names by `dx` and `dy`, within the desktop and never below the
/// smallest size. The sides that do not move stay where they are: pulling the left edge past the
/// smallest width stops it there instead of pushing the right edge along.
fn resized(rect: Rect, edge: WindowEdge, dx: i32, dy: i32) -> Rect {
    let (mut left, mut top, mut right, mut bottom) = (rect.x, rect.y, rect.right(), rect.bottom());
    let (min_width, min_height) = (i32::from(MIN.0), i32::from(MIN.1));
    if edge.left() {
        left = (left + dx).min(right - min_width).max(0);
    }
    if edge.right() {
        right = (right + dx).min(i32::from(DESK.0)).max(left + min_width);
    }
    if edge.top() {
        top = (top + dy).min(bottom - min_height).max(0);
    }
    if edge.bottom() {
        bottom = (bottom + dy).min(i32::from(DESK.1)).max(top + min_height);
    }
    let cells = |value: i32| u16::try_from(value).unwrap_or(0);
    Rect::new(left, top, cells(right - left), cells(bottom - top))
}
// endregion

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Window(name, event) => {
            let Some(index) = state.index(name) else {
                return Command::none();
            };
            let target = format!("Window#{name}");
            match event {
                // region: windows-update
                WindowEvent::Focus => {
                    // The clicked window comes to the front, which is where focus lives.
                    let win = state.windows.remove(index);
                    log.push(PAGE, target, "focused");
                    state.windows.push(win);
                }
                // The steps of a drag come through `on_drag` instead, as `Msg::Drag`.
                WindowEvent::Move { .. } | WindowEvent::Resize { .. } => {}
                WindowEvent::Dropped => {
                    // Snapping and the ghost's landing both happen when the button comes up.
                    let landing = state.snap.or(state.drag.map(|(_, rect)| rect));
                    if let Some(rect) = landing {
                        let win = &mut state.windows[index];
                        win.rect = rect;
                        win.maximized = rect.size() == Size::new(DESK.0, DESK.1);
                        log.push(PAGE, target, format!("dropped at {}, {}", rect.x, rect.y));
                    }
                    state.drag = None;
                    state.start = None;
                    state.snap = None;
                }
                WindowEvent::ToggleMaximize => {
                    let win = &mut state.windows[index];
                    win.maximized = !win.maximized;
                    if win.maximized {
                        win.restore = win.rect;
                        win.rect = Rect::new(0, 0, DESK.0, DESK.1);
                    } else {
                        win.rect = win.restore;
                    }
                    log.push(PAGE, target, if win.maximized { "maximized" } else { "restored" });
                }
                WindowEvent::Minimize => {
                    state.windows[index].minimized = true;
                    log.push(PAGE, target, "minimized");
                }
                WindowEvent::Close => {
                    state.windows.remove(index);
                    log.push(PAGE, target, "closed");
                } // endregion
            }
        }
        Msg::Drag(name, drag) => {
            let Some(index) = state.index(name) else {
                return Command::none();
            };
            // The totals count from where the button went down, so the window is placed from
            // where it was then. A side the desktop's edge or the smallest size holds back waits
            // there until the pointer comes back to it, instead of turning the moment the pointer
            // does, as a sum of steps would.
            let start = match state.start {
                Some((dragged, rect)) if dragged == name => rect,
                _ => state.windows[index].rect,
            };
            state.start = Some((name, start));
            let moving = matches!(drag.step, WindowEvent::Move { .. });
            let rect = match drag.step {
                WindowEvent::Resize { edge, .. } => resized(start, edge, drag.total_dx, drag.total_dy),
                _ => place(Rect::new(start.x + drag.total_dx, start.y + drag.total_dy, start.width, start.height)),
            };
            // With ghost drag on, the window stays where it is and only the ghost moves; it lands
            // in one frame when the button comes up.
            if state.ghost_drag && moving {
                state.drag = Some((name, rect));
            } else {
                let win = &mut state.windows[index];
                win.rect = rect;
                win.maximized = false;
            }
            if moving {
                state.snap = snap_target(rect);
            }
            let target = format!("Window#{name}");
            log.push(PAGE, target, format!("{} × {} at {}, {}", rect.width, rect.height, rect.x, rect.y));
        }
        Msg::GhostDrag(on) => {
            state.ghost_drag = on;
            state.drag = None;
            state.snap = None;
            log.push(PAGE, "Playground", format!("ghost drag = {on}"));
        }
        Msg::Shadow(on) => {
            state.shadow = on;
            log.push(PAGE, "Playground", format!("shadow = {on}"));
        }
        Msg::Mode(mode) => {
            state.mode = mode;
            log.push(PAGE, "Playground", format!("keys = {}", if mode == 0 { "move" } else { "resize" }));
        }
        Msg::Nudge(edge) => {
            let Some(focused) = state.focused().and_then(|name| state.index(name)) else {
                return Command::none();
            };
            let win = &mut state.windows[focused];
            let event = if state.mode == 0 {
                let (dx, dy) = (
                    STEP * i32::from(edge.right()) - STEP * i32::from(edge.left()),
                    STEP * i32::from(edge.bottom()) - STEP * i32::from(edge.top()),
                );
                WindowEvent::Move { dx, dy }
            } else {
                WindowEvent::Resize { edge, dx: STEP * i32::from(edge.right()), dy: STEP * i32::from(edge.bottom()) }
            };
            win.rect = moved(win.rect, event);
            let rect = win.rect;
            let name = win.name;
            log.push(
                PAGE,
                format!("Window#{name}"),
                format!("keys: {} × {} at {}, {}", rect.width, rect.height, rect.x, rect.y),
            );
        }
        Msg::Cycle => {
            if let Some(index) = state.windows.iter().position(|win| !win.minimized) {
                let win = state.windows.remove(index);
                let name = win.name;
                state.windows.push(win);
                log.push(PAGE, format!("Window#{name}"), "focused with ctrl+alt+w");
            }
        }
        Msg::Restore(name) => {
            if let Some(index) = state.index(name) {
                let mut win = state.windows.remove(index);
                win.minimized = false;
                state.windows.push(win);
                log.push(PAGE, format!("Window#{name}"), "shown again");
            }
        }
        Msg::Reopen => {
            *state = State { shadow: state.shadow, mode: state.mode, ..State::default() };
            log.push(PAGE, "Playground", "desktop reopened");
        }
    }
    Command::none()
}

/// The desktop: every window placed where the state says.
fn desk(state: &State, ui: &mut View<'_, AppMsg>) {
    let shown: Vec<&Win> = state.windows.iter().filter(|win| !win.minimized).collect();
    if shown.is_empty() {
        ui.add(
            EmptyState::new(t!("windows.empty"))
                .icon("inbox")
                .message(t!("windows.empty-hint"))
                .action(Button::new(t!("windows.reopen")).variant("primary").on_press(send(Msg::Reopen))),
        )
        .fill();
        return;
    }
    // region: windows
    ui.stack(|ui| {
        let top = shown.len() - 1;
        for (index, win) in shown.iter().enumerate() {
            let name = win.name;
            let window = Window::new(name)
                .subtitle(win.subtitle)
                .icon(win.icon)
                .focused(index == top)
                .maximized(win.maximized)
                .shadow(state.shadow)
                // Every movement of the pointer arrives as a message; the state decides.
                .on_event(move |event| send(Msg::Window(name, event)))
                // Moves and resizes with how far the pointer has gone since the press.
                .on_drag(move |drag| send(Msg::Drag(name, drag)));
            // Later children are drawn on top and take the pointer first, so the order of the
            // windows in the state is the stacking order.
            ui.place(win.rect, |ui| {
                ui.add_with(window, |ui| {
                    for line in win.body {
                        ui.add(Text::new(line).no_wrap());
                    }
                });
            })
            // Named, so state and a drag in progress follow a window that comes to the front.
            .id(name);
        }
        // The ghost goes over the windows: the snap target while an edge is touched, otherwise
        // the ghost of the window being dragged.
        if let Some(rect) = state.snap {
            ui.place(rect, |ui| {
                ui.add(Ghost::new().mix(SNAP_MIX));
            })
            .id("snap");
        } else if let Some((_, rect)) = state.drag {
            ui.place(rect, |ui| {
                ui.add(Ghost::new().mix(GHOST_MIX));
            })
            .id("ghost");
        }
    })
    .fill();
    // endregion
}

/// The live demo and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("windows.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        ui.column(|ui| desk(state, ui)).width(Length::Cells(DESK.0)).height(Length::Cells(DESK.1));
        let minimized: Vec<&Win> = state.windows.iter().filter(|win| win.minimized).collect();
        if !minimized.is_empty() {
            ui.spacer().height(Length::Cells(1));
            ui.row(|ui| {
                ui.add(Text::new(t!("windows.minimized")).role("faint").no_wrap());
                for win in minimized {
                    let name = win.name;
                    ui.add(Button::new(name).icon(win.icon).on_press(send(Msg::Restore(name)))).id(name);
                }
            })
            .gap(2);
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        setting(ui, t!("windows.shadow"), |ui| {
            ui.add(toggle(state.shadow, |on| send(Msg::Shadow(on)))).id("shadow");
        });
        setting(ui, t!("windows.ghost-drag"), |ui| {
            ui.add(toggle(state.ghost_drag, |on| send(Msg::GhostDrag(on)))).id("ghost-drag");
        });
        setting(ui, t!("windows.keys-do"), |ui| {
            let options = [t!("windows.move"), t!("windows.resize")];
            ui.add(Segmented::new(options).selected(state.mode).on_select(|index| send(Msg::Mode(index)))).id("mode");
        });
        setting(ui, t!("windows.arrows"), |ui| {
            ui.row(|ui| {
                let arrows = [
                    ("arrow-left", WindowEdge::Left, "nudge-left"),
                    ("arrow-right", WindowEdge::Right, "nudge-right"),
                    ("arrow-up", WindowEdge::Top, "nudge-up"),
                    ("arrow-down", WindowEdge::Bottom, "nudge-down"),
                ];
                for (icon, edge, id) in arrows {
                    let disabled = state.focused().is_none() || (state.mode == 1 && (edge.left() || edge.top()));
                    ui.add(IconButton::new(icon).on_press(send(Msg::Nudge(edge))).disabled(disabled)).id(id);
                }
            })
            .gap(1);
        });
        ui.add(Text::new(t!("windows.focused", name = state.focused().unwrap_or("—"))).role("secondary"));
        ui.add(Text::new(t!("windows.keys")).role("faint"));
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use qframe::event::{MouseButton, MouseKind};

    use super::*;
    use crate::tests::showcase_on;

    #[test]
    fn dragging_a_title_moves_a_window_and_a_click_brings_it_to_the_front() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("$ cargo build").expect("the terminal window's body");
        // The body starts two columns after the window's left edge and one row under the title.
        let title = (x - 2 + 4, y - 1);
        h.mouse(MouseKind::Down(MouseButton::Left), title.0, title.1);
        h.mouse(MouseKind::Drag(MouseButton::Left), title.0 + 3, title.1 - 1);
        h.mouse(MouseKind::Up(MouseButton::Left), title.0 + 3, title.1 - 1);
        let terminal = h.app().pages.windows.windows.last().expect("a window");
        assert_eq!((terminal.name, terminal.rect), ("Terminal", Rect::new(11, 7, 34, 8)));
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "34 × 8 at 11, 7");

        let (hx, hy) = h.find("cpu  38%").expect("htop's body");
        h.click(hx, hy);
        assert_eq!(h.app().pages.windows.focused(), Some("htop"), "the clicked window is in front");
        assert_eq!(h.app().log.recent(PAGE, 1)[0].message, "focused");
    }

    #[test]
    fn dragging_the_left_edge_widens_the_window_and_the_right_edge_stays() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("$ cargo build").expect("the terminal window's body");
        // The window's left column is two cells before its body.
        let edge = (x - 2, y + 2);
        let rect = |h: &crate::app::Showcase| h.pages.windows.windows.last().expect("a window").rect;
        assert_eq!(rect(h.app()), Rect::new(8, 8, 34, 8));
        h.mouse(MouseKind::Down(MouseButton::Left), edge.0, edge.1);
        h.mouse(MouseKind::Drag(MouseButton::Left), edge.0 - 5, edge.1);
        assert_eq!(rect(h.app()), Rect::new(3, 8, 39, 8), "five cells wider, to the left");
        h.mouse(MouseKind::Drag(MouseButton::Left), edge.0 - 12, edge.1);
        assert_eq!(rect(h.app()), Rect::new(0, 8, 42, 8), "the desktop's edge stops it, the right edge stays");
        h.mouse(MouseKind::Drag(MouseButton::Left), edge.0 - 10, edge.1);
        assert_eq!(rect(h.app()), Rect::new(0, 8, 42, 8), "the pointer turned, but is still past the desktop's edge");
        h.mouse(MouseKind::Drag(MouseButton::Left), edge.0 - 6, edge.1);
        assert_eq!(rect(h.app()), Rect::new(2, 8, 40, 8), "the edge is back under the pointer");
        h.mouse(MouseKind::Drag(MouseButton::Left), edge.0 - 12, edge.1);
        h.mouse(MouseKind::Up(MouseButton::Left), edge.0 - 12, edge.1);
        let edge = (edge.0 - 8, edge.1);
        h.mouse(MouseKind::Down(MouseButton::Left), edge.0, edge.1);
        h.mouse(MouseKind::Drag(MouseButton::Left), edge.0 + 30, edge.1);
        h.mouse(MouseKind::Up(MouseButton::Left), edge.0 + 30, edge.1);
        assert_eq!(rect(h.app()), Rect::new(22, 8, 20, 8), "the smallest width stops it, the right edge still stays");
    }

    #[test]
    fn the_keys_cycle_focus_and_the_arrows_move_or_resize_the_focused_window() {
        let mut h = showcase_on(PAGE);
        h.press("ctrl+alt+w");
        assert_eq!(h.app().pages.windows.focused(), Some("notes.md"));
        let rect = |h: &crate::app::Showcase| h.pages.windows.windows.last().expect("a window").rect;
        h.click_text("Resize");
        h.send(send(Msg::Nudge(WindowEdge::Right)));
        assert_eq!(rect(h.app()), Rect::new(0, 0, 28, 7), "the right edge moved out");
        h.click_text("Move");
        h.send(send(Msg::Nudge(WindowEdge::Left)));
        assert_eq!(rect(h.app()), Rect::new(0, 0, 28, 7), "already against the left edge");
        h.send(send(Msg::Nudge(WindowEdge::Bottom)));
        assert_eq!(rect(h.app()), Rect::new(0, 2, 28, 7));
    }

    #[test]
    fn the_marks_minimize_maximize_and_close_and_an_empty_desktop_reopens() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("$ cargo build").expect("the terminal window's body");
        let (left, row) = (x - 2, y - 1);
        // Where the desktop's own corner is on screen: the terminal window starts at 8, 8 on it.
        let desk = (left - 8, row - 8);
        let marks = left + 34 - 9;
        h.click(marks + 1, row);
        assert_eq!(h.app().pages.windows.focused(), Some("htop"), "the minimized window left the front");
        h.click_text("Terminal");
        assert_eq!(h.app().pages.windows.focused(), Some("Terminal"), "the button shows it again");
        for name in ["Terminal", "htop", "notes.md"] {
            let window = h.app().pages.windows.windows.last().expect("a window");
            assert_eq!(window.name, name);
            let rect = window.rect;
            let marks = desk.0 + rect.x + i32::from(rect.width) - 9;
            h.click(marks + 7, desk.1 + rect.y);
        }
        assert!(h.screen().contains("Every window is closed"), "{}", h.screen());
        h.click_text("Open them again");
        assert_eq!(h.app().pages.windows.windows.len(), 3);
    }

    #[test]
    fn ghost_drag_leaves_the_window_and_the_edge_shows_where_it_would_snap() {
        let mut h = showcase_on(PAGE);
        h.send(send(Msg::GhostDrag(true)));
        let (x, y) = h.find("$ cargo build").expect("the terminal window's body");
        let (title, desk) = ((x - 2 + 4, y - 1), (x - 2 - 8, y - 1 - 8));
        h.mouse(MouseKind::Down(MouseButton::Left), title.0, title.1);
        h.mouse(MouseKind::Drag(MouseButton::Left), title.0 + 6, title.1 - 2);
        assert_eq!(h.app().pages.windows.windows.last().expect("a window").rect, Rect::new(8, 8, 34, 8), "still there");
        assert_eq!(
            h.app().pages.windows.drag.map(|(_, rect)| rect),
            Some(Rect::new(14, 6, 34, 8)),
            "only the ghost moved"
        );
        let accent = h.env().theme().color("accent");
        let cell = |value: i32| u16::try_from(value).expect("a cell on screen");
        // Two cells of the bare desktop right of the other windows: one under the ghost, one not.
        let ground = h.bg(cell(desk.0 + 47), cell(desk.1 + 15));
        let ghost = ground.zip(accent).map(|(ground, accent)| ground.mix(accent, GHOST_MIX));
        assert_eq!(h.bg(cell(desk.0 + 47), cell(desk.1 + 13)), ghost, "the ghost's tone lies over the desktop");
        assert_ne!(ghost, ground);
        h.mouse(MouseKind::Up(MouseButton::Left), title.0 + 6, title.1 - 2);
        assert_eq!(
            h.app().pages.windows.windows.last().expect("a window").rect,
            Rect::new(14, 6, 34, 8),
            "the window lands where the ghost was"
        );
        assert!(h.app().pages.windows.drag.is_none());
    }

    #[test]
    fn a_window_dragged_against_the_left_edge_snaps_to_half_the_desktop() {
        let mut h = showcase_on(PAGE);
        let (x, y) = h.find("$ cargo build").expect("the terminal window's body");
        let title = (x - 2 + 4, y - 1);
        h.mouse(MouseKind::Down(MouseButton::Left), title.0, title.1);
        h.mouse(MouseKind::Drag(MouseButton::Left), title.0 - 20, title.1);
        assert_eq!(h.app().pages.windows.snap, Some(Rect::new(0, 0, 29, 16)), "the left half is previewed");
        h.mouse(MouseKind::Up(MouseButton::Left), title.0 - 20, title.1);
        assert_eq!(h.app().pages.windows.windows.last().expect("a window").rect, Rect::new(0, 0, 29, 16));
        assert!(h.app().pages.windows.snap.is_none(), "the preview goes with the drop");
    }
}
