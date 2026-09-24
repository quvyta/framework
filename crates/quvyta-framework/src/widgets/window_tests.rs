//! Windows on a small desktop: drawing, the pointer, narrow titles, glyph modes and colours.

use std::time::Duration;

use super::{Text, Window, WindowEdge, WindowEvent};
use crate::color::ColorDepth;
use crate::event::{Event, MouseButton, MouseEvent, MouseKind};
use crate::geometry::Rect;
use crate::icons::GlyphMode;
use crate::keymap::Modifiers;
use crate::runtime::{App, Command, Harness};
use crate::theme::State;
use crate::widget::{PointerShape, View};

/// One window on the desk.
struct Win {
    name: &'static str,
    subtitle: &'static str,
    rect: Rect,
    maximized: bool,
}

/// Windows bottom first; the last one is focused. Every event is recorded and applied.
struct Desk {
    windows: Vec<Win>,
    heard: Vec<(&'static str, WindowEvent)>,
    shadow: bool,
    interactive: bool,
}

impl Desk {
    fn new(windows: &[(&'static str, Rect)]) -> Self {
        let windows = windows
            .iter()
            .map(|(name, rect)| Win { name, subtitle: "~/projects", rect: *rect, maximized: false })
            .collect();
        Self { windows, heard: Vec::new(), shadow: false, interactive: true }
    }

    fn rect(&self, name: &str) -> Rect {
        self.windows.iter().find(|win| win.name == name).map(|win| win.rect).expect("window")
    }
}

impl App for Desk {
    type Msg = (&'static str, WindowEvent);

    fn update(&mut self, (name, event): Self::Msg) -> Command<Self::Msg> {
        self.heard.push((name, event));
        let Some(index) = self.windows.iter().position(|win| win.name == name) else {
            return Command::none();
        };
        match event {
            WindowEvent::Focus => {
                let win = self.windows.remove(index);
                self.windows.push(win);
            }
            WindowEvent::Move { dx, dy } => {
                let rect = &mut self.windows[index].rect;
                *rect = Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height);
            }
            WindowEvent::Resize { edge, dx, dy } => {
                let rect = &mut self.windows[index].rect;
                let (mut x, mut y, mut w, mut h) = (rect.x, rect.y, i32::from(rect.width), i32::from(rect.height));
                if edge.left() {
                    x += dx;
                    w -= dx;
                }
                if edge.right() {
                    w += dx;
                }
                if edge.top() {
                    y += dy;
                    h -= dy;
                }
                if edge.bottom() {
                    h += dy;
                }
                let cells = |v: i32| u16::try_from(v.max(1)).unwrap_or(1);
                *rect = Rect::new(x, y, cells(w), cells(h));
            }
            WindowEvent::Close => {
                self.windows.remove(index);
            }
            WindowEvent::ToggleMaximize => self.windows[index].maximized = !self.windows[index].maximized,
            WindowEvent::Minimize | WindowEvent::Dropped => {}
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Self::Msg>) {
        ui.stack(|ui| {
            let count = self.windows.len();
            for (index, win) in self.windows.iter().enumerate() {
                let name = win.name;
                let mut window = Window::new(name)
                    .subtitle(win.subtitle)
                    .icon("prompt")
                    .focused(index + 1 == count)
                    .maximized(win.maximized)
                    .shadow(self.shadow);
                if self.interactive {
                    window = window.on_event(move |event| (name, event));
                }
                ui.place(win.rect, |ui| {
                    ui.add_with(window, |ui| {
                        ui.add(Text::new(format!("{name} body")));
                    });
                })
                .id(name);
            }
        })
        .fill();
    }
}

/// Two overlapping windows: `back` at (2, 1) and `front`, focused, at (20, 4).
fn two() -> Harness<Desk> {
    Harness::new(Desk::new(&[("back", Rect::new(2, 1, 30, 8)), ("front", Rect::new(20, 4, 30, 8))]), 80, 24)
}

fn alt(kind: MouseKind, x: i32, y: i32) -> Event {
    Event::Mouse(MouseEvent { kind, x, y, mods: Modifiers { alt: true, ..Modifiers::default() } })
}

fn drag<A: App>(h: &mut Harness<A>, from: (i32, i32), to: &[(i32, i32)]) {
    h.mouse(MouseKind::Down(MouseButton::Left), from.0, from.1);
    for (x, y) in to {
        h.mouse(MouseKind::Drag(MouseButton::Left), *x, *y);
    }
    let last = to.last().copied().unwrap_or(from);
    h.mouse(MouseKind::Up(MouseButton::Left), last.0, last.1);
}

fn heard(h: &Harness<Desk>) -> Vec<(&'static str, WindowEvent)> {
    h.app().heard.clone()
}

#[test]
fn a_window_is_a_title_strip_and_a_body_with_marks_and_no_lines() {
    let h = two();
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert_eq!(lines[1], "   ❯ back  ~/projects  −  +  ×", "{screen}");
    assert_eq!(lines[2], "    back body", "{screen}");
    assert_eq!(lines[4].get(20..), Some("▌❯ front  ~/projec…  −  +  ×"), "{screen}");
    assert_eq!(lines[5].get(20..), Some("▌ front body"), "{screen}");
    for row in 4..12 {
        assert_eq!(h.buffer()[(20, row)].symbol(), "▌", "the pillar runs down the focused window, row {row}");
    }
    assert_eq!(h.buffer()[(2, 2)].symbol(), " ", "an unfocused window has no pillar");
    let theme = h.env().theme();
    let focused = |key: &str| theme.style(key, None, &[State::Focus]).paint("bg").map(|paint| paint.at(0.0));
    let resting = |key: &str| theme.style(key, None, &[]).paint("bg").map(|paint| paint.at(0.0));
    assert_eq!(h.bg(30, 4), focused("window-title"));
    assert_eq!(h.bg(30, 6), focused("window"));
    assert_eq!(h.bg(10, 1), resting("window-title"));
    assert_eq!(h.bg(10, 3), resting("window"));
    assert_ne!(focused("window"), resting("window"), "the focused window is raised one tone");
    assert!(h.is_bold(23, 4), "the focused name is bold");
    assert!(!h.is_bold(5, 1), "the other name is not");
}

#[test]
fn a_click_on_overlapping_windows_reaches_the_top_one() {
    let mut h = two();
    h.click(25, 6);
    assert!(heard(&h).is_empty(), "the focused window on top needs no focus: {:?}", heard(&h));
    h.click(10, 3);
    assert_eq!(heard(&h), [("back", WindowEvent::Focus)]);
    assert_eq!(h.app().windows.last().map(|win| win.name), Some("back"), "raised");
    h.click(25, 6);
    assert_eq!(heard(&h).len(), 1, "back is on top now, covers that cell and has focus already");
    h.click(40, 6);
    assert_eq!(heard(&h)[1], ("front", WindowEvent::Focus), "front shows beside it");
}

#[test]
fn a_window_past_the_screen_edge_is_cut_off() {
    let mut h = Harness::new(Desk::new(&[("wide", Rect::new(60, 20, 30, 8))]), 80, 24);
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert_eq!(lines[20].get(60..), Some("▌❯ wide  ~/projects"), "the marks are past the edge: {screen}");
    assert_eq!(lines.len(), 24);
    h.click(79, 23);
    assert!(heard(&h).is_empty(), "the visible part is the body");
}

#[test]
fn dragging_the_title_moves_the_window() {
    let mut h = two();
    drag(&mut h, (30, 4), &[(32, 5), (35, 5)]);
    assert_eq!(
        heard(&h),
        [
            ("front", WindowEvent::Move { dx: 2, dy: 1 }),
            ("front", WindowEvent::Move { dx: 3, dy: 0 }),
            ("front", WindowEvent::Dropped),
        ]
    );
    assert_eq!(h.app().rect("front"), Rect::new(25, 5, 30, 8));
}

#[test]
fn a_drag_stays_with_the_window_outside_the_screen() {
    let mut h = two();
    drag(&mut h, (30, 4), &[(-10, -3), (95, 30)]);
    assert_eq!(
        heard(&h),
        [
            ("front", WindowEvent::Move { dx: -40, dy: -7 }),
            ("front", WindowEvent::Move { dx: 105, dy: 33 }),
            ("front", WindowEvent::Dropped),
        ]
    );
}

#[test]
fn the_right_column_bottom_row_and_their_corner_resize() {
    let mut h = two();
    drag(&mut h, (49, 7), &[(52, 9)]);
    drag(&mut h, (30, 11), &[(33, 13)]);
    drag(&mut h, (52, 13), &[(50, 12)]);
    assert_eq!(
        heard(&h),
        [
            ("front", WindowEvent::Resize { edge: WindowEdge::Right, dx: 3, dy: 0 }),
            ("front", WindowEvent::Dropped),
            ("front", WindowEvent::Resize { edge: WindowEdge::Bottom, dx: 0, dy: 2 }),
            ("front", WindowEvent::Dropped),
            ("front", WindowEvent::Resize { edge: WindowEdge::BottomRight, dx: -2, dy: -1 }),
            ("front", WindowEvent::Dropped),
        ]
    );
    assert_eq!(h.app().rect("front"), Rect::new(20, 4, 31, 9));
}

#[test]
fn dragging_the_left_edge_widens_the_window_to_the_left_and_keeps_the_right_edge() {
    let mut h = two();
    let right = h.app().rect("front").right();
    drag(&mut h, (20, 7), &[(17, 7), (15, 8)]);
    assert_eq!(
        heard(&h),
        [
            ("front", WindowEvent::Resize { edge: WindowEdge::Left, dx: -3, dy: 0 }),
            ("front", WindowEvent::Resize { edge: WindowEdge::Left, dx: -2, dy: 0 }),
            ("front", WindowEvent::Dropped),
        ]
    );
    assert_eq!(h.app().rect("front"), Rect::new(15, 4, 35, 8));
    assert_eq!(h.app().rect("front").right(), right, "the right edge stays where it was");
    drag(&mut h, (15, 9), &[(19, 9)]);
    assert_eq!(h.app().rect("front"), Rect::new(19, 4, 31, 8), "and narrows it back from the left");
    assert_eq!(h.app().rect("front").right(), right);
}

#[test]
fn each_top_corner_resizes_and_the_bottom_left_corner_too() {
    let mut h = two();
    drag(&mut h, (20, 4), &[(18, 2)]);
    assert_eq!(h.app().rect("front"), Rect::new(18, 2, 32, 10), "the top left corner moves up and left");
    drag(&mut h, (49, 2), &[(52, 3)]);
    assert_eq!(h.app().rect("front"), Rect::new(18, 3, 35, 9), "the top right corner moves right and down");
    drag(&mut h, (18, 11), &[(16, 13)]);
    assert_eq!(h.app().rect("front"), Rect::new(16, 3, 37, 11), "the bottom left corner moves left and down");
    let edges: Vec<WindowEdge> = heard(&h)
        .into_iter()
        .filter_map(|(_, event)| match event {
            WindowEvent::Resize { edge, .. } => Some(edge),
            _ => None,
        })
        .collect();
    assert_eq!(edges, [WindowEdge::TopLeft, WindowEdge::TopRight, WindowEdge::BottomLeft]);
    assert_eq!(h.app().rect("front").bottom(), 14, "the top corners kept the bottom where it was until it moved");
}

#[test]
fn the_title_between_the_top_corners_still_moves_the_window() {
    let mut h = two();
    drag(&mut h, (21, 4), &[(23, 5)]);
    h.advance(Duration::from_secs(1));
    drag(&mut h, (41, 5), &[(40, 5)]);
    assert_eq!(
        heard(&h),
        [
            ("front", WindowEvent::Move { dx: 2, dy: 1 }),
            ("front", WindowEvent::Dropped),
            ("front", WindowEvent::Move { dx: -1, dy: 0 }),
            ("front", WindowEvent::Dropped),
        ],
        "the first cell after the corner and the last before the marks both move it"
    );
    assert_eq!(h.app().rect("front"), Rect::new(21, 5, 30, 8));
}

#[test]
fn the_left_edge_and_the_top_corners_light_under_the_pointer() {
    let mut h = two();
    let theme = h.env().theme();
    let hover = theme.style("split-handle", None, &[State::Hover]).paint("bg").map(|paint| paint.at(0.0));
    let (body, title) = (h.bg(25, 8), h.bg(30, 4));
    h.hover(20, 7);
    assert_eq!((h.bg(20, 5), h.bg(20, 10)), (hover, hover), "the whole left column lights");
    assert_eq!(h.buffer()[(20, 7)].symbol(), "▌", "under its light the pillar stays");
    assert_eq!((h.bg(25, 11), h.bg(49, 7)), (body, body), "the other sides stay");
    h.hover(49, 4);
    assert_eq!(
        (h.bg(49, 4), h.bg(49, 9), h.bg(20, 4)),
        (hover, hover, hover),
        "the corner lights its column and the top"
    );
    assert_eq!(h.bg(30, 4), title, "the title between the corners is not a handle");
    assert_eq!(h.bg(25, 11), body, "nor is the bottom row part of the top right corner");
    h.hover(25, 8);
    assert_eq!((h.bg(20, 7), h.bg(49, 7), h.bg(49, 4)), (body, body, title), "calm again");
}

#[test]
fn the_pointer_turns_into_a_resize_arrow_over_every_edge_and_corner() {
    let mut h = two();
    let mut shape = |x, y| h.hover(x, y).pointer_shape();
    assert_eq!(shape(20, 7), PointerShape::EwResize, "the left edge");
    assert_eq!(shape(49, 7), PointerShape::EwResize, "the right edge");
    assert_eq!(shape(30, 11), PointerShape::NsResize, "the bottom edge");
    assert_eq!(shape(20, 4), PointerShape::NwseResize, "the top left corner");
    assert_eq!(shape(49, 11), PointerShape::NwseResize, "the bottom right corner");
    assert_eq!(shape(49, 4), PointerShape::NeswResize, "the top right corner");
    assert_eq!(shape(20, 11), PointerShape::NeswResize, "the bottom left corner");
    assert_eq!(shape(30, 7), PointerShape::Default, "the body");
    assert_eq!(shape(30, 4), PointerShape::Default, "the title, which moves the window");
    assert_eq!(shape(47, 4), PointerShape::Default, "a mark");
    assert_eq!(shape(70, 20), PointerShape::Default, "the bare desktop");
}

#[test]
fn a_window_on_top_hides_the_arrows_beneath_it_and_a_resize_keeps_its_arrow() {
    let mut h = two();
    assert_eq!(h.hover(31, 2).pointer_shape(), PointerShape::EwResize, "the back window's right edge, where it shows");
    assert_eq!(h.hover(31, 6).pointer_shape(), PointerShape::Default, "and where the front window covers it");
    h.mouse(MouseKind::Down(MouseButton::Left), 49, 7).mouse(MouseKind::Drag(MouseButton::Left), 49, 20);
    assert_eq!(h.pointer_shape(), PointerShape::EwResize, "a resize keeps its arrow off the window");
    h.mouse(MouseKind::Up(MouseButton::Left), 49, 20);
    assert_eq!(h.pointer_shape(), PointerShape::Default, "and gives it up with the button");
    h.mouse(MouseKind::Down(MouseButton::Left), 30, 4).mouse(MouseKind::Drag(MouseButton::Left), 31, 11);
    assert_eq!(h.pointer_shape(), PointerShape::Default, "a move over another window's edge is still a move");
    h.mouse(MouseKind::Up(MouseButton::Left), 31, 11);
    let mut still = Desk::new(&[("still", Rect::new(2, 1, 30, 6))]);
    still.interactive = false;
    let mut h = Harness::new(still, 40, 8);
    assert_eq!(h.hover(2, 3).pointer_shape(), PointerShape::Default, "a window nobody resizes has no arrows");
}

#[test]
fn handles_light_under_the_pointer_and_take_the_accent_while_dragged() {
    let mut h = two();
    let theme = h.env().theme();
    let hover = theme.style("split-handle", None, &[State::Hover]).paint("bg").map(|paint| paint.at(0.0));
    let active = theme.style("split-handle", None, &[State::Active]).paint("bg").map(|paint| paint.at(0.0));
    let body = h.bg(49, 7);
    h.hover(49, 7);
    assert_eq!(h.bg(49, 9), hover, "the whole right column lights");
    assert_eq!(h.bg(30, 11), body, "the bottom row stays");
    h.hover(49, 11);
    assert_eq!((h.bg(49, 6), h.bg(30, 11)), (hover, hover), "the corner lights both");
    h.mouse(MouseKind::Down(MouseButton::Left), 49, 7).mouse(MouseKind::Drag(MouseButton::Left), 51, 7);
    assert_eq!(h.app().rect("front").right(), 52, "the right column is at 51 now");
    assert_eq!(h.bg(51, 8), active);
    h.mouse(MouseKind::Up(MouseButton::Left), 51, 7).hover(40, 2);
    assert_eq!(h.bg(51, 8), body, "calm again");
}

#[test]
fn the_marks_minimize_maximize_and_close_on_release() {
    let mut h = two();
    h.click(42, 4);
    h.click(45, 4);
    assert_eq!(heard(&h), [("front", WindowEvent::Minimize), ("front", WindowEvent::ToggleMaximize)]);
    assert!(h.screen().lines().nth(4).is_some_and(|line| line.ends_with("−  ◇  ×")), "{}", h.screen());
    h.mouse(MouseKind::Down(MouseButton::Left), 48, 4).mouse(MouseKind::Up(MouseButton::Left), 30, 4);
    assert_eq!(heard(&h).len(), 2, "released elsewhere, the close mark does nothing");
    h.click(47, 4);
    assert_eq!(heard(&h)[2], ("front", WindowEvent::Close));
    assert_eq!(h.app().windows.len(), 1);
}

#[test]
fn a_hovered_mark_lights_its_three_cells() {
    let mut h = two();
    let theme = h.env().theme();
    let lit = theme.style("close-mark", None, &[State::Active, State::Hover]).paint("bg").map(|paint| paint.at(0.0));
    h.hover(47, 4);
    assert_eq!([h.bg(46, 4), h.bg(47, 4), h.bg(48, 4)], [lit; 3]);
    assert_ne!(h.bg(45, 4), lit, "the next mark stays");
    assert_ne!(h.bg(49, 4), lit, "the corner after the marks is the right edge's");
}

#[test]
fn a_double_click_on_the_title_toggles_maximize() {
    let mut h = two();
    h.click(30, 4);
    h.click(31, 4);
    assert_eq!(heard(&h), [("front", WindowEvent::ToggleMaximize)]);
    h.advance(Duration::from_secs(1));
    h.click(30, 4);
    h.advance(Duration::from_secs(1));
    h.click(30, 4);
    assert_eq!(heard(&h).len(), 1, "slow clicks are two single clicks");
}

#[test]
fn a_click_on_an_unfocused_window_focuses_it_before_anything_else() {
    let mut h = two();
    h.click(10, 1);
    assert_eq!(heard(&h), [("back", WindowEvent::Focus)], "the title of the window beneath");
    h.click(40, 8);
    h.click(47, 4);
    assert_eq!(
        heard(&h)[1..],
        [("front", WindowEvent::Focus), ("front", WindowEvent::Close)],
        "its body raised it, so its close mark only closes"
    );
}

#[test]
fn alt_with_the_left_button_moves_from_anywhere_in_the_body() {
    let mut h = two();
    h.events(&[
        alt(MouseKind::Down(MouseButton::Left), 30, 8),
        alt(MouseKind::Drag(MouseButton::Left), 28, 9),
        alt(MouseKind::Up(MouseButton::Left), 28, 9),
    ]);
    assert_eq!(heard(&h), [("front", WindowEvent::Move { dx: -2, dy: 1 }), ("front", WindowEvent::Dropped)]);
}

#[test]
fn alt_with_the_right_button_resizes_from_the_nearest_edge_or_corner() {
    let mut h = two();
    let resize = |h: &mut Harness<Desk>, from: (i32, i32), to: (i32, i32)| {
        h.events(&[
            alt(MouseKind::Down(MouseButton::Right), from.0, from.1),
            alt(MouseKind::Drag(MouseButton::Right), to.0, to.1),
            alt(MouseKind::Up(MouseButton::Right), to.0, to.1),
        ]);
    };
    resize(&mut h, (22, 5), (20, 4));
    resize(&mut h, (20, 8), (18, 9));
    resize(&mut h, (35, 5), (36, 3));
    assert_eq!(
        heard(&h),
        [
            ("front", WindowEvent::Resize { edge: WindowEdge::TopLeft, dx: -2, dy: -1 }),
            ("front", WindowEvent::Dropped),
            ("front", WindowEvent::Resize { edge: WindowEdge::Left, dx: -2, dy: 0 }),
            ("front", WindowEvent::Dropped),
            ("front", WindowEvent::Resize { edge: WindowEdge::Top, dx: 0, dy: -2 }),
            ("front", WindowEvent::Dropped),
        ]
    );
    assert_eq!(h.app().rect("front"), Rect::new(16, 1, 34, 11));
}

#[test]
fn a_title_too_narrow_shortens_the_subtitle_then_the_name() {
    let title = |width: u16| {
        let desk = Desk::new(&[("terminal", Rect::new(0, 0, width, 5))]);
        let h = Harness::new(desk, 40, 5);
        h.screen().lines().next().unwrap_or_default().to_owned()
    };
    assert_eq!(title(35), "▌❯ terminal  ~/projects   −  +  ×", "everything fits");
    assert_eq!(title(31), "▌❯ terminal  ~/proj…  −  +  ×", "the subtitle shortens first");
    assert_eq!(title(27), "▌❯ terminal       −  +  ×", "then it is left out");
    assert_eq!(title(23), "▌❯ terminal   −  +  ×");
    assert_eq!(title(19), "▌❯ term…  −  +  ×", "then the name shortens");
    assert_eq!(title(13), "▌❯  −  +  ×", "the marks always stay");
}

#[test]
fn marks_follow_the_glyph_mode() {
    let marks = |mode: GlyphMode| {
        let mut h = two();
        h.set_glyph_mode(mode);
        h.screen().lines().nth(4).map(|line| line.chars().skip(40).collect::<String>()).unwrap_or_default()
    };
    assert_eq!(marks(GlyphMode::Ascii), " -  +  x");
    assert_eq!(marks(GlyphMode::Unicode), " −  +  ×");
    assert_eq!(marks(GlyphMode::Nerd), " \u{f2d1}  \u{f2d0}  \u{f00d}");
    let mut h = two();
    h.set_glyph_mode(GlyphMode::Ascii);
    let pillar = h.env().theme().style("window", None, &[State::Focus]).paint("pillar").map(|paint| paint.at(0.0));
    assert_eq!(h.buffer()[(20, 6)].symbol(), " ", "ASCII draws the pillar as a cell of colour");
    assert_eq!(h.bg(20, 6), pillar);
}

fn shadow_desk() -> Harness<Desk> {
    let mut desk = Desk::new(&[("lit", Rect::new(2, 1, 20, 6))]);
    desk.shadow = true;
    Harness::new(desk, 40, 10)
}

#[test]
fn the_shadow_darkens_one_column_right_and_one_row_below() {
    let h = shadow_desk();
    let canvas = h.env().theme().color("canvas");
    let ground = h.bg(30, 5);
    assert_eq!(ground, canvas);
    let right = h.bg(22, 3);
    let below = h.bg(10, 7);
    assert!(
        right.zip(canvas).is_some_and(|(shade, canvas)| shade.relative_luminance() < canvas.relative_luminance()),
        "{right:?}"
    );
    assert_eq!(right, below);
    assert_eq!(h.bg(22, 1), canvas, "the first row casts no shadow, as light comes from the top left");
    assert_eq!(h.bg(2, 7), canvas, "nor the first column");
}

#[test]
fn no_shadow_with_reduced_motion_or_in_sixteen_colours() {
    let mut h = shadow_desk();
    h.set_reduced_motion(true);
    let canvas = h.bg(30, 5);
    assert_eq!((h.bg(22, 3), h.bg(10, 7)), (canvas, canvas));
    let mut h = shadow_desk();
    h.set_depth(ColorDepth::Ansi16);
    let bg = |h: &Harness<Desk>, x: u16, y: u16| h.buffer()[(x, y)].bg;
    let canvas = bg(&h, 30, 5);
    assert_eq!((bg(&h, 22, 3), bg(&h, 10, 7)), (canvas, canvas));
    assert_ne!(bg(&h, 5, 1), canvas, "in 16 colours the title strip still stands off the canvas");
    assert_ne!(bg(&h, 5, 1), h.buffer()[(5, 1)].fg, "and its name reads on it");
    let mut desk = Desk::new(&[("back", Rect::new(0, 0, 20, 4)), ("front", Rect::new(0, 5, 20, 4))]);
    desk.shadow = true;
    let mut h = Harness::new(desk, 30, 10);
    h.set_depth(ColorDepth::Ansi16);
    assert_ne!(bg(&h, 5, 0), bg(&h, 5, 5), "the focused strip is told from the others");
    assert!(h.screen().contains("−  +  ×"));
}

#[test]
fn a_window_without_on_event_is_only_a_surface() {
    let mut desk = Desk::new(&[("still", Rect::new(2, 1, 30, 6))]);
    desk.interactive = false;
    let mut h = Harness::new(desk, 40, 8);
    assert_eq!(h.screen().lines().nth(1), Some("  ▌❯ still  ~/projects"), "{}", h.screen());
    drag(&mut h, (10, 1), &[(14, 3)]);
    h.click(31, 6);
    assert!(heard(&h).is_empty());
}

#[cfg(feature = "pty")]
mod with_a_terminal {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use vt100::MouseProtocolMode as Mode;

    use super::{alt, drag};
    use crate::event::{MouseButton, MouseKind};
    use crate::geometry::Rect;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Terminal, TerminalSession, Text, Window, WindowEvent};

    /// A terminal window, focused, beside another window.
    struct TermDesk {
        session: TerminalSession,
        heard: Vec<(&'static str, WindowEvent)>,
    }

    impl App for TermDesk {
        type Msg = (&'static str, WindowEvent);
        fn update(&mut self, heard: Self::Msg) -> Command<Self::Msg> {
            self.heard.push(heard);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Self::Msg>) {
            ui.stack(|ui| {
                ui.place(Rect::new(40, 2, 30, 8), |ui| {
                    ui.add_with(Window::new("other").on_event(|event| ("other", event)), |ui| {
                        ui.add(Text::new("other body"));
                    });
                })
                .id("other");
                ui.place(Rect::new(2, 2, 34, 12), |ui| {
                    let window = Window::new("term").focused(true).on_event(|event| ("term", event));
                    ui.add_with(window, |ui| {
                        ui.add(Terminal::new(&self.session)).fill();
                    });
                })
                .id("term");
            })
            .fill();
        }
    }

    /// Starts `script` under `sh` in raw mode, then echoes what it reads.
    fn start(script: &str) -> TerminalSession {
        let script = format!("stty raw -echo; {script}; cat -v");
        TerminalSession::spawn("/bin/sh".as_ref(), &["-c", &script], Path::new("/")).expect("pty")
    }

    /// Drives the session until `done` holds for its screen, for at most a minute: generous on a
    /// loaded machine, and finite. Only a wait still going ends the program; one that got its
    /// answer leaves it running for the next.
    fn wait(session: &TerminalSession, done: impl Fn(&vt100::Screen) -> bool) {
        const PATIENCE: Duration = Duration::from_secs(60);
        let watch = session.watch();
        let waiting = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (timer, still) = (session.clone(), std::sync::Arc::clone(&waiting));
        std::thread::spawn(move || {
            std::thread::sleep(PATIENCE);
            if still.load(std::sync::atomic::Ordering::SeqCst) {
                timer.kill();
            }
        });
        let started = Instant::now();
        while !done(session.parser().screen()) {
            let _ = watch.next();
            assert!(started.elapsed() < PATIENCE, "{}", session.parser().screen().contents());
        }
        waiting.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    #[test]
    fn the_title_and_alt_drags_stay_the_windows_while_the_program_reads_the_mouse() {
        let session = start("printf '\\033[?1002h\\033[?1006h'");
        wait(&session, |screen| screen.mouse_protocol_mode() == Mode::ButtonMotion);
        let mut h = Harness::new(TermDesk { session: session.clone(), heard: Vec::new() }, 80, 24);
        drag(&mut h, (12, 2), &[(14, 3)]);
        h.events(&[
            alt(MouseKind::Down(MouseButton::Left), 14, 8),
            alt(MouseKind::Drag(MouseButton::Left), 15, 8),
            alt(MouseKind::Up(MouseButton::Left), 15, 8),
        ]);
        assert_eq!(
            h.app().heard.clone(),
            [
                ("term", WindowEvent::Move { dx: 2, dy: 1 }),
                ("term", WindowEvent::Dropped),
                ("term", WindowEvent::Move { dx: 1, dy: 0 }),
                ("term", WindowEvent::Dropped),
            ]
        );
        h.click(20, 9);
        // A click is two sequences, the press and the release; waiting for the first alone would
        // count before the second arrived.
        wait(&session, |screen| screen.contents().matches("^[[<").count() >= 2);
        let contents = session.parser().screen().contents();
        assert_eq!(contents.matches("^[[<").count(), 2, "only the plain click reached the program: {contents}");
        assert_eq!(h.app().heard.clone().len(), 4, "the focused window's body click is the program's alone");
        session.kill();
    }
}

#[test]
fn a_press_that_moves_nothing_is_no_drop() {
    let mut h = two();
    h.mouse(MouseKind::Down(MouseButton::Left), 30, 4).mouse(MouseKind::Up(MouseButton::Left), 30, 4);
    assert!(heard(&h).is_empty(), "a click on the title of the focused window does nothing");
    // Far enough apart not to count as a double click.
    h.advance(Duration::from_secs(1));
    h.mouse(MouseKind::Down(MouseButton::Left), 30, 4).mouse(MouseKind::Drag(MouseButton::Left), 30, 4).mouse(
        MouseKind::Up(MouseButton::Left),
        30,
        4,
    );
    assert!(heard(&h).is_empty(), "a drag that stays on the same cell moves nothing");
    h.advance(Duration::from_secs(1));
    drag(&mut h, (30, 4), &[(31, 4)]);
    assert_eq!(
        heard(&h),
        [("front", WindowEvent::Move { dx: 1, dy: 0 }), ("front", WindowEvent::Dropped)],
        "the drop comes after the move it ends"
    );
}
