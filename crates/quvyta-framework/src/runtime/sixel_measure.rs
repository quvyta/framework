//! What a desktop with a wallpaper costs on the wire, drawn with half blocks, with sixel and with
//! the kitty protocol: the first frame, selecting an icon, opening a window and dragging it.
//! Ignored by default; `cargo test --release -p quvyta-framework --features image --lib sixel_measure
//! -- --ignored --nocapture` prints the table the design note keeps.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use ratatui_core::layout::Rect as BufferRect;
use ratatui_core::terminal::{Terminal, TerminalOptions, Viewport};
use ratatui_crossterm::CrosstermBackend;

use super::present::Screen;
use crate::color::Rgb;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::graphics::Graphics;
use crate::runtime::{App, Command, Harness};
use crate::style::CellStyle;
use crate::widget::{EventCx, MeasureCx, PaintCx, View, Widget};
use crate::widgets::{Fit, Image, ImageData};

/// What a screen wrote, kept outside it so a test can read it.
#[derive(Clone, Default)]
pub(super) struct Wire(pub(super) Rc<RefCell<Vec<u8>>>);

impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A desktop: a wallpaper over the whole screen, eight icon tiles in two columns, one of them
/// selected, and a window somewhere or nowhere. A click on a tile selects it and a click on the
/// floor selects none; the window's top row drags it and its top right cell closes it.
#[derive(Clone)]
pub(super) struct Desktop {
    pub(super) wallpaper: ImageData,
    pub(super) selected: Option<usize>,
    pub(super) window: Option<(i32, i32)>,
    /// Where the window was taken, from its corner, while it is dragged.
    grab: Option<(i32, i32)>,
}

impl Desktop {
    pub(super) fn new(wallpaper: &ImageData, window: Option<(i32, i32)>) -> Self {
        Self { wallpaper: wallpaper.clone(), selected: None, window, grab: None }
    }
}

#[derive(Clone)]
pub(super) enum Msg {
    Select(Option<usize>),
    Grab(i32, i32),
    Move(i32, i32),
    Drop,
    Close,
}

/// The size of the window.
pub(super) const WINDOW: (u16, u16) = (44, 14);

/// Where icon `index` stands: its tile, with the free column for the selection bar to its left
/// and a free row under it, as qdesk lays icons out.
pub(super) fn tile(index: usize) -> Rect {
    let (column, row) = (i32::try_from(index % 2).unwrap_or(0), i32::try_from(index / 2).unwrap_or(0));
    Rect::new(3 + column * 17, 2 + row * 4, 14, 2)
}

impl App for Desktop {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(selected) => self.selected = selected,
            Msg::Grab(x, y) => self.grab = Some((x, y)),
            Msg::Move(x, y) => self.window = Some((x, y)),
            Msg::Drop => self.grab = None,
            Msg::Close => self.window = None,
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Floor(self.clone())).fill();
    }
}

struct Floor(Desktop);

impl Widget<Msg> for Floor {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        Widget::<Msg>::paint(&Image::new(&self.0.wallpaper).fit(Fit::Cover), cx, area);
        for index in 0..8 {
            let at = tile(index);
            let selected = self.0.selected == Some(index);
            let ground = if selected { Rgb::new(70, 90, 140) } else { Rgb::new(40, 40, 48) };
            cx.clear(at, ground);
            let words = CellStyle { fg: Some(Rgb::new(220, 220, 225)), ..Default::default() };
            cx.text(at.x + 1, at.y, "Documents", words, at.width - 1);
            cx.text(at.x + 1, at.y + 1, "folder", words, at.width - 1);
            if selected {
                let bar = CellStyle { fg: Some(Rgb::new(120, 160, 255)), ..Default::default() };
                cx.text(at.x - 1, at.y, "▌", bar, 1);
                cx.text(at.x - 1, at.y + 1, "▌", bar, 1);
            }
        }
        if let Some((x, y)) = self.0.window {
            let window = Rect::new(x, y, WINDOW.0, WINDOW.1);
            cx.clear(window, Rgb::new(29, 29, 35));
            let words = CellStyle { fg: Some(Rgb::new(200, 200, 205)), ..Default::default() };
            for (offset, line) in (0..WINDOW.1).zip(["Files", "", "Pictures", "Music", "Notes.txt"].iter().cycle()) {
                cx.text(x + 2, y + i32::from(offset), line, words, WINDOW.0 - 4);
            }
        }
    }
    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let Event::Mouse(mouse) = event else { return false };
        let window = self.0.window.map(|(x, y)| Rect::new(x, y, WINDOW.0, WINDOW.1));
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                cx.capture_pointer();
                if let Some(window) = window.filter(|window| window.contains(mouse.x, mouse.y)) {
                    if mouse.y == window.y && mouse.x == window.right() - 1 {
                        cx.emit(Msg::Close);
                    } else if mouse.y == window.y {
                        cx.emit(Msg::Grab(mouse.x - window.x, mouse.y - window.y));
                    }
                } else {
                    let hit = (0..8).find(|index| {
                        let at = tile(*index);
                        Rect::new(at.x - 1, at.y, at.width + 1, at.height).contains(mouse.x, mouse.y)
                    });
                    cx.emit(Msg::Select(hit));
                }
            }
            MouseKind::Drag(MouseButton::Left) => {
                if let Some((dx, dy)) = self.0.grab {
                    cx.emit(Msg::Move(mouse.x - dx, mouse.y - dy));
                }
            }
            MouseKind::Up(MouseButton::Left) => cx.emit(Msg::Drop),
            _ => return false,
        }
        true
    }
}

/// A photo-like picture of `width` × `height` pixels: soft gradients under grain every pixel of
/// which differs from its neighbours, so neither palette runs nor zlib save much.
pub(super) fn noisy(width: u32, height: u32) -> ImageData {
    let mut state = 0x2545_f491_u32;
    let rgb: Vec<u8> = (0..width * height)
        .flat_map(|at| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let grain = i32::from(state.to_be_bytes()[0] % 48) - 24;
            let (x, y) = (at % width, at / width);
            let base = [x * 200 / width + 30, y * 180 / height + 40, 150 - y * 100 / height];
            base.map(|channel| u8::try_from((i32::try_from(channel).unwrap_or(0) + grain).clamp(0, 255)).unwrap_or(0))
        })
        .collect();
    ImageData::from_rgb(width, height, &rgb).expect("pixels")
}

/// A smooth picture of `width` × `height` pixels: a sky fading into a sea, and a sun.
pub(super) fn smooth(width: u32, height: u32) -> ImageData {
    let rgb: Vec<u8> = (0..width * height)
        .flat_map(|at| {
            let (x, y) = (at % width, at / width);
            let (dx, dy) = (i64::from(x) - i64::from(width) * 7 / 10, i64::from(y) - i64::from(height) / 3);
            let sun = dx * dx + dy * dy < i64::from(height / 8).pow(2);
            let sea = y > height * 3 / 5;
            let pixel = if sun {
                [250, 210, 120]
            } else if sea {
                [20, 60 + y * 60 / height, 110 + y * 60 / height]
            } else {
                [90 + y * 100 / height, 140 + y * 60 / height, 220]
            };
            pixel.map(|channel| u8::try_from(channel.min(255)).unwrap_or(255))
        })
        .collect();
    ImageData::from_rgb(width, height, &rgb).expect("pixels")
}

/// A screen of `size` and the wire it writes to.
pub(super) fn screen(size: (u16, u16)) -> (Screen<Wire>, Wire) {
    let wire = Wire::default();
    let options = TerminalOptions { viewport: Viewport::Fixed(BufferRect::new(0, 0, size.0, size.1)) };
    let terminal = Terminal::with_options(CrosstermBackend::new(wire.clone()), options).expect("a terminal");
    (Screen::new(terminal), wire)
}

/// The bytes the screen writes for the harness's frame.
fn bytes<A: App>(h: &mut Harness<A>, screen: &mut Screen<Wire>, wire: &Wire) -> usize {
    wire.0.borrow_mut().clear();
    h.present_to(screen);
    wire.0.borrow().len()
}

/// What each moment costs.
struct Costs {
    first: usize,
    idle: usize,
    select: usize,
    deselect: usize,
    open: usize,
    step: usize,
    close: usize,
}

/// Clicks the middle of icon `index`.
pub(super) fn click_icon<A: App>(h: &mut Harness<A>, index: usize) {
    let at = tile(index);
    h.click(at.x + 3, at.y);
}

fn measure(graphics: Graphics, remote: bool, size: (u16, u16), wallpaper: &ImageData) -> Costs {
    let mut h = Harness::new(Desktop::new(wallpaper, None), size.0, size.1);
    h.set_graphics(graphics).set_remote(remote);
    let (mut screen, wire) = screen(size);
    let first = bytes(&mut h, &mut screen, &wire);
    let idle = bytes(&mut h, &mut screen, &wire);
    click_icon(&mut h, 3);
    let select = bytes(&mut h, &mut screen, &wire);
    click_icon(&mut h, 4);
    let deselect = bytes(&mut h, &mut screen, &wire);
    let start = (i32::from(size.0) / 2 - 10, 6);
    h.send(Msg::Move(start.0, start.1));
    let open = bytes(&mut h, &mut screen, &wire);
    let steps = 10;
    let mut moved = 0;
    h.mouse(MouseKind::Down(MouseButton::Left), start.0 + 5, start.1);
    for step in 1..=steps {
        h.mouse(MouseKind::Drag(MouseButton::Left), start.0 + 5 - step * 2, start.1 + step % 3);
        moved += bytes(&mut h, &mut screen, &wire);
    }
    h.mouse(MouseKind::Up(MouseButton::Left), start.0 + 5 - steps * 2, start.1 + steps % 3);
    bytes(&mut h, &mut screen, &wire);
    let window = h.app().window.expect("a window");
    h.click(window.0 + i32::from(WINDOW.0) - 1, window.1);
    assert!(h.app().window.is_none(), "closed");
    let close = bytes(&mut h, &mut screen, &wire);
    Costs { first, idle, select, deselect, open, step: moved / usize::try_from(steps).unwrap_or(1), close }
}

#[test]
#[ignore = "a measurement, printed for the design note"]
fn sixel_measure() {
    println!(
        "| screen | picture | decoded | way | first | idle | select | select another | open window | drag step | close window |"
    );
    for size in [(100u16, 30u16), (200, 50)] {
        // qdesk decodes a wallpaper at ten by twenty pixels a cell for a local terminal and two by
        // four over a remote connection. Over one, split sixel is measured as a local terminal
        // would draw that smaller picture: the same bytes.
        for (decoded, per_cell, ways) in [
            (
                "local",
                (10, 20),
                [
                    ("half blocks", Graphics::HalfBlock, false),
                    ("sixel, split", Graphics::Sixel, false),
                    ("kitty", Graphics::Kitty, false),
                ]
                .as_slice(),
            ),
            (
                "remote",
                (2, 4),
                [
                    ("half blocks", Graphics::HalfBlock, true),
                    ("sixel, whole", Graphics::Sixel, true),
                    ("sixel, split", Graphics::Sixel, false),
                    ("kitty", Graphics::Kitty, true),
                ]
                .as_slice(),
            ),
        ] {
            let pixels = (u32::from(size.0) * per_cell.0, u32::from(size.1) * per_cell.1);
            for (name, picture) in [("noisy", noisy(pixels.0, pixels.1)), ("smooth", smooth(pixels.0, pixels.1))] {
                for (way, graphics, remote) in ways {
                    let c = measure(*graphics, *remote, size, &picture);
                    println!(
                        "| {}×{} | {name} | {decoded} | {way} | {} | {} | {} | {} | {} | {} | {} |",
                        size.0, size.1, c.first, c.idle, c.select, c.deselect, c.open, c.step, c.close
                    );
                }
            }
        }
    }
}
