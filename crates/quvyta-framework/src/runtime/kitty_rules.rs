//! Pictures on a terminal that speaks the kitty graphics protocol, read from the bytes a screen
//! writes: sent once, placed by number, cut around what is painted over them, freed when gone,
//! and nothing at all when nothing changed.

use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::time::Duration;

use ratatui_core::layout::Rect as BufferRect;
use ratatui_core::terminal::{Terminal, TerminalOptions, Viewport};
use ratatui_crossterm::CrosstermBackend;

use super::engine::{Engine, TaskMode};
use super::kitty::number;
use super::present::Screen;
use crate::color::Rgb;
use crate::env::Env;
use crate::geometry::{Rect, Size};
use crate::graphics::Graphics;
use crate::runtime::{App, Command};
use crate::widget::{MeasureCx, PaintCx, View, Widget};
use crate::widgets::{Fit, Image, ImageData};

/// What a screen wrote, kept outside it so a test can read it.
#[derive(Clone, Default)]
struct Wire(Rc<RefCell<Vec<u8>>>);

impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Pictures at fixed places, and a layer painted over the screen at `cover`.
#[derive(Clone, Default)]
struct Scene {
    pictures: Vec<(ImageData, Rect)>,
    cover: Option<Rect>,
    /// A dialog's dimmed backdrop over the whole screen.
    backdrop: bool,
    /// Flat tiles painted right over the pictures, as icons on a desktop.
    icons: Vec<Rect>,
    /// A surface with grey words on a grey ground painted over the pictures, as a window's
    /// description lines: `(95, 95, 105)` on `(29, 29, 35)`, spaces between the words included.
    grey: Option<Rect>,
    /// A shadow darkening this rectangle, as a window casts one.
    shadow: Option<Rect>,
}

impl App for Scene {
    type Msg = Scene;
    fn update(&mut self, scene: Scene) -> Command<Scene> {
        *self = scene;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Scene>) {
        ui.add(Stage(self.clone())).fill();
    }
}

/// Paints a [`Scene`]: its pictures where it says, then a layer over them.
struct Stage(Scene);

impl Widget<Scene> for Stage {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        for (data, at) in &self.0.pictures {
            Widget::<Scene>::paint(&Image::new(data).fit(Fit::Cover), cx, *at);
        }
        for icon in &self.0.icons {
            cx.clear(*icon, Rgb::new(60, 60, 70));
        }
        if let Some(grey) = self.0.grey {
            cx.clear(grey, Rgb::new(29, 29, 35));
            let words = crate::style::CellStyle { fg: Some(Rgb::new(95, 95, 105)), ..Default::default() };
            for y in grey.y..grey.bottom() {
                let line = "Every Quvyta application  reads   it ".repeat(4);
                cx.text(grey.x, y, &line, words, grey.width);
            }
        }
        if let Some(shadow) = self.0.shadow {
            cx.tint(shadow, Rgb::new(0, 0, 0), 0.3);
        }
        if self.0.cover.is_some() || self.0.backdrop {
            cx.request_overlay(area);
        }
    }
    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        cx.open_layer();
        if self.0.backdrop {
            cx.tint(anchor, Rgb::new(0, 0, 0), 0.5);
        }
        if let Some(cover) = self.0.cover {
            cx.clear(cover, Rgb::new(60, 60, 70));
            cx.text(cover.x, cover.y, "Open", crate::style::CellStyle::default(), cover.width);
        }
    }
}

/// A square picture of `side` × `side` pixels whose every pixel differs from its neighbours, so
/// compressing it saves little.
fn noise(side: u32) -> ImageData {
    let mut state = 0x2545_f491_u32;
    let rgb: Vec<u8> = (0..side * side * 3)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_be_bytes()[0]
        })
        .collect();
    ImageData::from_rgb(side, side, &rgb).expect("pixels")
}

/// Eight by eight pixels of one colour: four columns by two rows when it covers them.
fn square() -> ImageData {
    ImageData::from_rgb(8, 8, &[200, 120, 40].repeat(64)).expect("pixels")
}

/// An engine for `scene` on a 12 × 6 screen whose terminal answered `graphics`, and the wire
/// its screen writes to.
fn running(scene: Scene, graphics: Graphics) -> (Engine<Scene>, Screen<Wire>, Wire) {
    running_on(scene, graphics, (12, 6))
}

/// An engine for `scene` on a screen of `size` whose terminal answered `graphics`, and the wire
/// its screen writes to.
fn running_on(scene: Scene, graphics: Graphics, size: (u16, u16)) -> (Engine<Scene>, Screen<Wire>, Wire) {
    let mut env = Env::builtin();
    env.set_terminal_graphics(graphics);
    let engine = Engine::new(scene, env, TaskMode::Inline);
    let wire = Wire::default();
    let options = TerminalOptions { viewport: Viewport::Fixed(BufferRect::new(0, 0, size.0, size.1)) };
    let terminal = Terminal::with_options(CrosstermBackend::new(wire.clone()), options).expect("a terminal");
    (engine, Screen::new(terminal), wire)
}

/// What one frame wrote, as text.
fn frame(engine: &mut Engine<Scene>, screen: &mut Screen<Wire>, wire: &Wire) -> String {
    wire.0.borrow_mut().clear();
    screen
        .present(|buffer| {
            engine.render(buffer, Duration::from_secs(5));
            engine.painted()
        })
        .expect("a frame");
    String::from_utf8_lossy(&wire.0.borrow()).into_owned()
}

/// The kitty commands in `written`, each as its keys and its payload.
fn commands(written: &str) -> Vec<(String, String)> {
    written
        .split("\x1b_G")
        .skip(1)
        .map(|rest| {
            let body = rest.split("\x1b\\").next().expect("a command ends");
            let (keys, payload) = body.split_once(';').unwrap_or((body, ""));
            (keys.to_owned(), payload.to_owned())
        })
        .collect()
}

/// Standard base64 back into bytes.
fn unbase64(text: &str) -> Vec<u8> {
    const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let values: Vec<u32> = text
        .bytes()
        .filter(|byte| *byte != b'=')
        .map(|byte| u32::try_from(ALPHABET.find(char::from(byte)).expect("base64")).expect("small"))
        .collect();
    let mut out = Vec::new();
    for group in values.chunks(4) {
        let joined = group.iter().enumerate().fold(0u32, |all, (index, value)| all | value << (18 - 6 * index));
        let bytes = joined.to_be_bytes();
        out.extend_from_slice(&bytes[1..group.len()]);
    }
    out
}

/// The pixels a transmission carries: its chunks joined, base64 decoded, zlib inflated.
fn pixels_sent(chunks: &[(String, String)]) -> Vec<u8> {
    let joined: String = chunks.iter().map(|(_, payload)| payload.as_str()).collect();
    let mut rgb = Vec::new();
    flate2::read::ZlibDecoder::new(unbase64(&joined).as_slice()).read_to_end(&mut rgb).expect("zlib");
    rgb
}

fn rgb_of(data: &ImageData) -> Vec<u8> {
    data.pixels().iter().flat_map(|pixel| [pixel.r, pixel.g, pixel.b]).collect()
}

fn with(pictures: &[(ImageData, Rect)]) -> Scene {
    Scene { pictures: pictures.to_vec(), ..Scene::default() }
}

#[test]
fn the_first_frame_sends_the_picture_then_places_it_after_the_cells() {
    let data = square();
    let id = number(&data);
    let (mut engine, mut screen, wire) = running(with(&[(data.clone(), Rect::new(2, 1, 4, 2))]), Graphics::Kitty);
    let written = frame(&mut engine, &mut screen, &wire);
    let sent = commands(&written);
    assert_eq!(sent.len(), 2, "one transmission in one chunk, one place: {written:?}");
    assert_eq!(sent[0].0, format!("a=t,f=24,o=z,s=8,v=8,i={id},q=2,m=0"));
    assert_eq!(pixels_sent(&sent[..1]), rgb_of(&data), "the pixels as kept, at their own size");
    let place = format!("\x1b[2;3H\x1b_Ga=p,i={id},p=1,x=0,y=0,w=8,h=8,c=4,r=2,z=-1,C=1,q=2\x1b\\");
    let (begin, at, end) = (written.find("\x1b[?2026h"), written.find(&place), written.find("\x1b[?2026l"));
    assert!(at.is_some(), "the cursor goes to the picture's first cell, then it is placed: {written:?}");
    assert!(begin < written.find("\x1b_Ga=t") && at < end, "inside the frame's synchronized update: {written:?}");
    assert!(written.find("\x1b_Ga=t") < at, "sent before it is placed");
    assert!(written.rfind(' ') < written.find("\x1b_G"), "the cells come first: {written:?}");
}

#[test]
fn a_large_picture_is_sent_in_chunks_of_at_most_4096_bytes() {
    let data = noise(96);
    let id = number(&data);
    let (mut engine, mut screen, wire) = running(with(&[(data.clone(), Rect::new(0, 0, 12, 6))]), Graphics::Kitty);
    let written = frame(&mut engine, &mut screen, &wire);
    let sent = commands(&written);
    let chunks: Vec<_> = sent.iter().filter(|(keys, _)| !keys.starts_with("a=p")).cloned().collect();
    assert!(chunks.len() > 2, "{} chunks", chunks.len());
    assert_eq!(chunks[0].0, format!("a=t,f=24,o=z,s=96,v=96,i={id},q=2,m=1"));
    for (keys, _) in &chunks[1..chunks.len() - 1] {
        assert_eq!(keys, "m=1,q=2", "a middle chunk carries only m and q");
    }
    assert_eq!(chunks[chunks.len() - 1].0, "m=0,q=2", "the last chunk says it is the last");
    assert!(chunks.iter().all(|(_, payload)| payload.len() <= 4096 && !payload.is_empty()));
    assert!(chunks[..chunks.len() - 1].iter().all(|(_, payload)| payload.len() == 4096), "only the last is short");
    assert_eq!(pixels_sent(&chunks), rgb_of(&data));
}

#[test]
fn the_same_frame_again_is_not_a_byte() {
    let (mut engine, mut screen, wire) = running(with(&[(square(), Rect::new(2, 1, 4, 2))]), Graphics::Kitty);
    assert!(!frame(&mut engine, &mut screen, &wire).is_empty());
    assert_eq!(frame(&mut engine, &mut screen, &wire), "", "an idle screen with a picture stays silent");
    assert_eq!(frame(&mut engine, &mut screen, &wire), "");
}

#[test]
fn a_picture_that_moves_or_grows_is_placed_again_and_not_sent_again() {
    let data = square();
    let id = number(&data);
    let (mut engine, mut screen, wire) = running(with(&[(data.clone(), Rect::new(2, 1, 4, 2))]), Graphics::Kitty);
    frame(&mut engine, &mut screen, &wire);
    engine.update(with(&[(data.clone(), Rect::new(4, 2, 4, 2))]));
    let moved = frame(&mut engine, &mut screen, &wire);
    assert_eq!(commands(&moved), vec![(format!("a=p,i={id},p=1,x=0,y=0,w=8,h=8,c=4,r=2,z=-1,C=1,q=2"), String::new())]);
    assert!(moved.contains("\x1b[3;5H\x1b_Ga=p"), "{moved:?}");
    engine.update(with(&[(data, Rect::new(4, 2, 8, 4))]));
    let grown = frame(&mut engine, &mut screen, &wire);
    assert_eq!(commands(&grown), vec![(format!("a=p,i={id},p=1,x=0,y=0,w=8,h=8,c=8,r=4,z=-1,C=1,q=2"), String::new())]);
}

#[test]
fn a_place_no_longer_used_is_deleted() {
    let data = square();
    let id = number(&data);
    let twice = with(&[(data.clone(), Rect::new(0, 0, 4, 2)), (data.clone(), Rect::new(6, 2, 4, 2))]);
    let (mut engine, mut screen, wire) = running(twice, Graphics::Kitty);
    let first = commands(&frame(&mut engine, &mut screen, &wire));
    assert_eq!(first.iter().filter(|(keys, _)| keys.starts_with("a=t")).count(), 1, "sent once, shown twice");
    assert!(first.iter().any(|(keys, _)| keys.starts_with(&format!("a=p,i={id},p=2,"))), "{first:?}");
    engine.update(with(&[(data, Rect::new(0, 0, 4, 2))]));
    let written = commands(&frame(&mut engine, &mut screen, &wire));
    assert_eq!(written, vec![(format!("a=d,d=i,i={id},p=2,q=2"), String::new())]);
}

#[test]
fn a_picture_shown_nowhere_is_freed() {
    let (kept, gone) = (square(), noise(8));
    let (mut engine, mut screen, wire) =
        running(with(&[(kept.clone(), Rect::new(0, 0, 4, 2)), (gone.clone(), Rect::new(6, 2, 4, 2))]), Graphics::Kitty);
    frame(&mut engine, &mut screen, &wire);
    engine.update(with(&[(kept, Rect::new(0, 0, 4, 2))]));
    let written = commands(&frame(&mut engine, &mut screen, &wire));
    assert_eq!(written, vec![(format!("a=d,d=I,i={},q=2", number(&gone)), String::new())]);
    engine.update(with(&[(gone.clone(), Rect::new(6, 2, 4, 2))]));
    let back = commands(&frame(&mut engine, &mut screen, &wire));
    assert!(
        back.iter().any(|(keys, _)| keys.starts_with(&format!("a=t,f=24,o=z,s=8,v=8,i={}", number(&gone)))),
        "{back:?}"
    );
}

#[test]
fn after_a_handoff_every_picture_is_sent_and_placed_again() {
    let data = square();
    let id = number(&data);
    let (mut engine, mut screen, wire) = running(with(&[(data, Rect::new(2, 1, 4, 2))]), Graphics::Kitty);
    frame(&mut engine, &mut screen, &wire);
    screen.redraw_all(BufferRect::new(0, 0, 12, 6)).expect("a resize");
    let written = commands(&frame(&mut engine, &mut screen, &wire));
    assert_eq!(written.len(), 2, "{written:?}");
    assert!(written[0].0.starts_with(&format!("a=t,f=24,o=z,s=8,v=8,i={id},")));
    assert!(written[1].0.starts_with(&format!("a=p,i={id},p=1,")));
    assert_eq!(frame(&mut engine, &mut screen, &wire), "");
}

#[test]
fn releasing_frees_every_picture_and_the_next_frame_sends_it_again() {
    let data = square();
    let id = number(&data);
    let (mut engine, mut screen, wire) = running(with(&[(data, Rect::new(2, 1, 4, 2))]), Graphics::Kitty);
    frame(&mut engine, &mut screen, &wire);
    wire.0.borrow_mut().clear();
    screen.release_pictures().expect("a release");
    assert_eq!(String::from_utf8_lossy(&wire.0.borrow()), format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\"));
    screen.redraw_all(BufferRect::new(0, 0, 12, 6)).expect("a resize");
    let again = commands(&frame(&mut engine, &mut screen, &wire));
    assert!(again[0].0.starts_with("a=t,"), "{again:?}");
}

#[test]
fn a_layer_along_one_side_cuts_the_picture_to_the_part_left_showing() {
    let data = square();
    let id = number(&data);
    let right = Scene { cover: Some(Rect::new(4, 0, 8, 6)), ..with(&[(data.clone(), Rect::new(2, 1, 4, 2))]) };
    let (mut engine, mut screen, wire) = running(right, Graphics::Kitty);
    let written = frame(&mut engine, &mut screen, &wire);
    assert!(
        written.contains(&format!("\x1b[2;3H\x1b_Ga=p,i={id},p=1,x=0,y=0,w=4,h=8,c=2,r=2,z=-1,C=1,q=2\x1b\\")),
        "the left half of the picture in the two columns left of the layer: {written:?}"
    );
    engine.update(Scene { cover: Some(Rect::new(0, 2, 12, 4)), ..with(&[(data, Rect::new(2, 1, 4, 2))]) });
    let written = frame(&mut engine, &mut screen, &wire);
    assert_eq!(
        commands(&written),
        vec![(format!("a=p,i={id},p=1,x=0,y=0,w=8,h=4,c=4,r=1,z=-1,C=1,q=2"), String::new())],
        "the top half above a layer over its bottom row"
    );
}

/// The places `written` asks for, as their keys.
fn places(written: &str) -> Vec<String> {
    commands(written).into_iter().map(|(keys, _)| keys).filter(|keys| keys.starts_with("a=p")).collect()
}

/// The cells of `buffer` drawn with half blocks.
fn half_blocks(buffer: &ratatui_core::buffer::Buffer) -> Vec<(u16, u16)> {
    let area = buffer.area;
    (area.y..area.bottom())
        .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
        .filter(|&(x, y)| matches!(buffer[(x, y)].symbol(), "▀" | "▄"))
        .collect()
}

#[test]
fn an_icon_in_the_middle_splits_the_picture_into_places_around_it() {
    let data = noise(24);
    let id = number(&data);
    let icon = Scene { icons: vec![Rect::new(5, 2, 2, 2)], ..with(&[(data, Rect::new(0, 0, 12, 6))]) };
    let (mut engine, mut screen, wire) = running(icon, Graphics::Kitty);
    let written = frame(&mut engine, &mut screen, &wire);
    let placed = places(&written);
    let cells: Vec<String> = placed
        .iter()
        .map(|keys| {
            let keep = |key: &str| keys.split(',').find(|part| part.starts_with(key)).expect("a key").to_owned();
            [keep("p="), keep("c="), keep("r=")].join(",")
        })
        .collect();
    assert_eq!(
        cells,
        ["p=1,c=12,r=2", "p=2,c=5,r=2", "p=3,c=5,r=2", "p=4,c=12,r=2"],
        "above, left of, right of and below the icon: {written:?}"
    );
    assert!(placed.iter().all(|keys| keys.starts_with(&format!("a=p,i={id},"))));
    for (place, at) in [(1, "\x1b[1;1H"), (2, "\x1b[3;1H"), (3, "\x1b[3;8H"), (4, "\x1b[5;1H")] {
        assert!(written.contains(&format!("{at}\x1b_Ga=p,i={id},p={place},")), "place {place} at {at:?}");
    }
    assert_eq!(half_blocks(&engine_buffer(&mut engine)), [], "no half blocks at all");
}

#[test]
fn moving_a_window_over_a_picture_only_places_it_again() {
    let data = noise(24);
    let picture = with(&[(data, Rect::new(0, 0, 12, 6))]);
    let (mut engine, mut screen, wire) =
        running(Scene { icons: vec![Rect::new(2, 2, 3, 2)], ..picture.clone() }, Graphics::Kitty);
    frame(&mut engine, &mut screen, &wire);
    engine.update(Scene { icons: vec![Rect::new(6, 1, 3, 2)], ..picture });
    let moved = frame(&mut engine, &mut screen, &wire);
    let sent = commands(&moved);
    assert!(!sent.is_empty(), "the places change");
    for (keys, payload) in &sent {
        assert!(keys.starts_with("a=p,") || keys.starts_with("a=d,d=i,"), "only places: {keys}");
        assert!(payload.is_empty(), "no pixels: {keys}");
    }
    assert!(moved.len() < 1000, "a few commands, {} bytes", moved.len());
}

#[test]
fn an_idle_frame_with_a_split_picture_is_not_a_byte() {
    let icon = Scene { icons: vec![Rect::new(5, 2, 2, 2)], ..with(&[(noise(24), Rect::new(0, 0, 12, 6))]) };
    let (mut engine, mut screen, wire) = running(icon, Graphics::Kitty);
    assert!(places(&frame(&mut engine, &mut screen, &wire)).len() > 1);
    assert_eq!(frame(&mut engine, &mut screen, &wire), "", "an idle screen stays silent");
    assert_eq!(frame(&mut engine, &mut screen, &wire), "");
}

#[test]
fn a_picture_cut_into_too_many_pieces_is_drawn_with_half_blocks_and_keeps_its_pixels() {
    let data = noise(24);
    let id = number(&data);
    let open = with(&[(data, Rect::new(0, 0, 20, 10))]);
    let (mut engine, mut screen, wire) = running_on(open.clone(), Graphics::Kitty, (20, 10));
    frame(&mut engine, &mut screen, &wire);
    // Every other cell covered: a hundred free cells, none touching another.
    let board: Vec<Rect> =
        (0..10).flat_map(|y| (0..20).filter(move |x| (x + y) % 2 == 0).map(move |x| Rect::new(x, y, 1, 1))).collect();
    engine.update(Scene { icons: board, ..open.clone() });
    let covered = commands(&frame(&mut engine, &mut screen, &wire));
    assert_eq!(covered, vec![(format!("a=d,d=i,i={id},p=1,q=2"), String::new())], "only its place goes");
    let mut buffer = ratatui_core::buffer::Buffer::empty(BufferRect::new(0, 0, 20, 10));
    engine.render(&mut buffer, Duration::from_secs(5));
    assert_eq!(half_blocks(&buffer).len(), 100, "every free cell in half blocks");
    engine.update(open);
    let back = commands(&frame(&mut engine, &mut screen, &wire));
    assert_eq!(back.len(), 1, "placed again, not sent again: {back:?}");
    assert!(back[0].0.starts_with(&format!("a=p,i={id},p=1,")), "{back:?}");
}

#[test]
fn grey_words_on_a_grey_surface_cover_the_picture_as_half_blocks_do() {
    let surface = Rect::new(2, 1, 8, 3);
    let scene = Scene { grey: Some(surface), ..with(&[(noise(24), Rect::new(0, 0, 12, 6))]) };
    let (mut halves, _, _) = running(scene.clone(), Graphics::HalfBlock);
    let under_halves = half_blocks(&engine_buffer(&mut halves));
    let (mut engine, mut screen, wire) = running(scene, Graphics::Kitty);
    let written = frame(&mut engine, &mut screen, &wire);
    let under_kitty = half_blocks(&engine_buffer(&mut engine));
    let inside = |&(x, y): &(u16, u16)| surface.contains(i32::from(x), i32::from(y));
    assert!(!under_halves.iter().any(inside), "the surface covers the half blocks");
    assert_eq!(under_kitty.iter().filter(|cell| inside(cell)).count(), 0, "nor does the picture leak into it");
    assert!(!places(&written).is_empty(), "the rest of the picture is pixels: {written:?}");
    let buffer = engine_buffer(&mut engine);
    let placed = engine.painted().pictures;
    for placement in placed {
        let (x, y, w, h) = placement.cells;
        let rect = Rect::new(i32::from(x), i32::from(y), w, h);
        assert!(rect.intersect(surface).is_empty(), "no place under the surface: {rect:?}");
    }
    assert_eq!(buffer[(3, 1)].symbol(), "v", "the surface's words are on top");
}

#[test]
fn a_shadow_dims_its_cells_in_half_blocks_while_the_rest_stays_pixels() {
    let data = square();
    let shadow = Rect::new(2, 3, 6, 1);
    let scene = Scene { shadow: Some(shadow), ..with(&[(data, Rect::new(2, 1, 6, 3))]) };
    let (mut engine, mut screen, wire) = running(scene, Graphics::Kitty);
    let written = frame(&mut engine, &mut screen, &wire);
    assert_eq!(places(&written).len(), 1, "the rows above the shadow are pixels: {written:?}");
    let buffer = engine_buffer(&mut engine);
    let dimmed: Vec<(u16, u16)> = half_blocks(&buffer);
    assert_eq!(dimmed, (2..8).map(|x| (x, 3)).collect::<Vec<_>>(), "only the shadow's row in half blocks");
    let ratatui_core::style::Color::Rgb(r, g, b) = buffer[(3, 3)].fg else { panic!("a colour") };
    assert_eq!((r, g, b), (140, 84, 28), "darkened as the shadow darkens: 200 120 40 at 70 %");
}

#[test]
fn under_a_dialog_backdrop_the_picture_shows_dimmed_in_half_blocks() {
    let data = square();
    let dimmed = Scene { backdrop: true, ..with(&[(data, Rect::new(2, 1, 4, 2))]) };
    let (mut engine, mut screen, wire) = running(dimmed, Graphics::Kitty);
    let written = frame(&mut engine, &mut screen, &wire);
    assert!(!written.contains("\x1b_G"), "{written:?}");
    let buffer = engine_buffer(&mut engine);
    assert_eq!(buffer[(3, 1)].symbol(), "▀");
    let ratatui_core::style::Color::Rgb(r, g, b) = buffer[(3, 1)].fg else { panic!("a colour") };
    let near = |value: u8, expected: u8| value.abs_diff(expected) <= 1;
    assert!(near(r, 100) && near(g, 60) && near(b, 20), "halfway to black, as the backdrop: {r} {g} {b}");
}

/// The cells the engine paints for its scene now.
fn engine_buffer(engine: &mut Engine<Scene>) -> ratatui_core::buffer::Buffer {
    let mut buffer = ratatui_core::buffer::Buffer::empty(BufferRect::new(0, 0, 12, 6));
    engine.render(&mut buffer, Duration::from_secs(5));
    buffer
}

#[test]
fn only_a_kitty_terminal_is_sent_picture_commands() {
    for graphics in [Graphics::Sixel, Graphics::HalfBlock, Graphics::None] {
        let (mut engine, mut screen, wire) = running(with(&[(square(), Rect::new(2, 1, 4, 2))]), graphics);
        let first = frame(&mut engine, &mut screen, &wire);
        screen.redraw_all(BufferRect::new(0, 0, 12, 6)).expect("a resize");
        let again = frame(&mut engine, &mut screen, &wire);
        assert!(!first.contains("\x1b_G") && !again.contains("\x1b_G"), "{graphics:?}: {first:?}");
    }
}

#[test]
fn every_command_asks_the_terminal_not_to_answer() {
    let (a, b) = (square(), noise(96));
    let (mut engine, mut screen, wire) =
        running(with(&[(a.clone(), Rect::new(0, 0, 4, 2)), (b, Rect::new(4, 2, 8, 4))]), Graphics::Kitty);
    let mut written = frame(&mut engine, &mut screen, &wire);
    engine.update(with(&[(a.clone(), Rect::new(0, 0, 4, 2)), (a.clone(), Rect::new(6, 0, 4, 2))]));
    written += &frame(&mut engine, &mut screen, &wire);
    engine.update(Scene { cover: Some(Rect::new(0, 1, 12, 1)), ..with(&[(a, Rect::new(0, 0, 4, 2))]) });
    written += &frame(&mut engine, &mut screen, &wire);
    wire.0.borrow_mut().clear();
    screen.release_pictures().expect("a release");
    written += &String::from_utf8_lossy(&wire.0.borrow());
    let sent = commands(&written);
    for kind in ["a=t", "m=1", "m=0", "a=p", "a=d,d=i", "a=d,d=I"] {
        assert!(sent.iter().any(|(keys, _)| keys.starts_with(kind)), "the run wrote {kind}: {sent:?}");
    }
    for (keys, _) in &sent {
        let quiet: Vec<&str> = keys.split(',').filter(|key| key.starts_with("q=")).collect();
        assert_eq!(quiet, ["q=2"], "{keys}");
    }
}
