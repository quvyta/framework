//! Pictures on a terminal that speaks sixel, read from the bytes a screen writes: painted after
//! the cells while the whole picture shows, half blocks while anything covers it, painted again
//! when it comes back or a cell under it is written, and nothing at all while nothing changes.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::Duration;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect as BufferRect;
use ratatui_core::terminal::{Terminal, TerminalOptions, Viewport};
use ratatui_crossterm::CrosstermBackend;

use super::present::Screen;
use crate::geometry::{Rect, Size};
use crate::graphics::Graphics;
use crate::runtime::{App, Command, Harness};
use crate::widget::{MeasureCx, PaintCx, View, Widget};
use crate::widgets::{Button, Fit, Image, ImageData, Modal, Popover, Text};

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

const WIDTH: u16 = 40;
const HEIGHT: u16 = 14;

/// A screen of the harness's size, and the wire it writes to.
fn screen() -> (Screen<Wire>, Wire) {
    let wire = Wire::default();
    let options = TerminalOptions { viewport: Viewport::Fixed(BufferRect::new(0, 0, WIDTH, HEIGHT)) };
    let terminal = Terminal::with_options(CrosstermBackend::new(wire.clone()), options).expect("a terminal");
    (Screen::new(terminal), wire)
}

/// What the screen wrote for the harness's frame, as text.
fn frame<A: App>(h: &mut Harness<A>, screen: &mut Screen<Wire>, wire: &Wire) -> String {
    wire.0.borrow_mut().clear();
    h.present_to(screen);
    String::from_utf8_lossy(&wire.0.borrow()).into_owned()
}

/// Every sixel image in `written`: the cursor move before it and its raster attributes.
fn sixels(written: &str) -> Vec<(String, String)> {
    written
        .match_indices("\x1bP0;1;0q")
        .map(|(at, start)| {
            let before = &written[..at];
            let cursor = before.rfind("\x1b[").map(|from| before[from..].to_owned()).unwrap_or_default();
            let raster: String = written[at + start.len()..].chars().take_while(|c| *c != '#').collect();
            (cursor, raster)
        })
        .collect()
}

/// The cells of `buffer` drawn with half blocks.
fn half_blocks(buffer: &Buffer) -> usize {
    buffer.content.iter().filter(|cell| matches!(cell.symbol(), "▀" | "▄")).count()
}

/// A picture of many colours, so a sixel of it is more than a few bytes.
fn photo() -> ImageData {
    let (width, height) = (48u32, 36u32);
    let rgb: Vec<u8> = (0..width * height)
        .flat_map(|at| {
            let (x, y) = (at % width, at / width);
            [u8::try_from(x * 5).expect("small"), u8::try_from(y * 7).expect("small"), 120]
        })
        .collect();
    ImageData::from_rgb(width, height, &rgb).expect("pixels")
}

/// A wallpaper under a row of buttons: one opens a dialog, one a menu, one hides the picture.
struct Desk {
    data: ImageData,
    dialog: bool,
    menu: bool,
    shown: bool,
}

#[derive(Clone)]
enum Msg {
    Dialog(bool),
    Menu(bool),
    Hide,
}

impl App for Desk {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Dialog(open) => self.dialog = open,
            Msg::Menu(open) => self.menu = open,
            Msg::Hide => self.shown = false,
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.column(|ui| {
            ui.row(|ui| {
                ui.add(Button::new("Open").on_press(Msg::Dialog(true)));
                Popover::new(self.menu)
                    .on_dismiss(Msg::Menu(false))
                    .anchor(|ui| {
                        ui.add(Button::new("Menu").on_press(Msg::Menu(true)));
                    })
                    .content(|ui| {
                        ui.add(Text::new("Rename"));
                    })
                    .show(ui);
                ui.add(Button::new("Hide").on_press(Msg::Hide));
            });
            if self.shown {
                ui.add(Image::new(&self.data).fit(Fit::Cover)).fill();
            }
            if self.dialog {
                ui.add_with(Modal::new().title("Filters").on_close(Msg::Dialog(false)), |ui| {
                    ui.add(Text::new("Europe only"));
                });
            }
        });
    }
}

fn desk(graphics: Graphics) -> Harness<Desk> {
    let mut h = Harness::new(Desk { data: photo(), dialog: false, menu: false, shown: true }, WIDTH, HEIGHT);
    h.set_graphics(graphics);
    h
}

/// Where the one sixel in `written` goes: its cursor move and its raster attributes.
fn only_sixel(written: &str) -> (String, String) {
    let found = sixels(written);
    assert_eq!(found.len(), 1, "one picture: {written:?}");
    found.into_iter().next().expect("one")
}

/// The row a cursor move `ESC [ row ; column H` goes to, counted from 1.
fn row_of(cursor: &str) -> u16 {
    cursor[2..].split(';').next().and_then(|row| row.parse().ok()).expect("a cursor move")
}

#[test]
fn a_wallpaper_is_a_sixel_until_a_dialog_covers_it_and_again_once_it_closes() {
    let mut h = desk(Graphics::Sixel);
    let (mut screen, wire) = screen();
    let first = frame(&mut h, &mut screen, &wire);
    let (cursor, raster) = only_sixel(&first);
    let top = row_of(&cursor);
    let rows = HEIGHT + 1 - top;
    assert_eq!(cursor, format!("\x1b[{top};1H"), "the cursor goes to the picture's first cell");
    let height = u32::from(rows) * 20 / 6 * 6;
    assert_eq!(raster, format!("\"1;1;{};{height}", u32::from(WIDTH) * 10), "shrunk to its cells, whole bands");
    let (begin, dcs, end) = (first.find("\x1b[?2026h"), first.find("\x1bP"), first.find("\x1b[?2026l"));
    assert!(begin < dcs && dcs < end, "inside the frame's synchronized update");
    assert!(first.find("Hide") < dcs, "after the cells");
    assert!(first.ends_with("\x1b\\\x1b[?2026l"), "the image is closed before the update ends");
    assert_eq!(half_blocks(h.buffer()), 0, "the picture's cells are plain ground");

    assert_eq!(frame(&mut h, &mut screen, &wire), "", "an idle screen with a sixel is not a byte");

    h.click_text("Open").advance(Duration::from_millis(400));
    let covered = frame(&mut h, &mut screen, &wire);
    assert!(h.screen().contains("Europe only"), "{}", h.screen());
    assert!(!covered.contains("\x1bP"), "no sixel while the dialog covers it: {covered:?}");
    assert!(half_blocks(h.buffer()) > 200, "the picture is drawn with half blocks under the backdrop");
    assert_eq!(frame(&mut h, &mut screen, &wire), "", "and stays silent while the dialog is open");

    h.press("esc").advance(Duration::from_millis(400));
    let uncovered = frame(&mut h, &mut screen, &wire);
    assert!(!h.app().dialog);
    assert_eq!(only_sixel(&uncovered), (cursor.clone(), raster.clone()), "the same picture, painted again");
    assert_eq!(half_blocks(h.buffer()), 0);
    assert_eq!(frame(&mut h, &mut screen, &wire), "");

    h.click_text("Hide");
    let removed = frame(&mut h, &mut screen, &wire);
    assert!(!removed.contains("\x1bP"), "{removed:?}");
    for row in top..=HEIGHT {
        assert!(removed.contains(&format!("\x1b[{row};1H")), "row {row} of the picture is written over: {removed:?}");
    }
    assert_eq!(frame(&mut h, &mut screen, &wire), "");
}

#[test]
fn a_menu_over_part_of_the_picture_turns_it_to_half_blocks_until_it_closes() {
    let mut h = desk(Graphics::Sixel);
    let (mut screen, wire) = screen();
    let placed = only_sixel(&frame(&mut h, &mut screen, &wire));
    h.click_text("Menu").advance(Duration::from_millis(400));
    let open = frame(&mut h, &mut screen, &wire);
    assert!(h.screen().contains("Rename"), "{}", h.screen());
    assert!(!open.contains("\x1bP"), "a sixel cannot be cut around the menu: {open:?}");
    assert!(half_blocks(h.buffer()) > 200, "the rest of the picture is half blocks");
    h.press("esc").advance(Duration::from_millis(400));
    assert_eq!(only_sixel(&frame(&mut h, &mut screen, &wire)), placed);
}

#[test]
fn a_new_ground_under_a_picture_paints_it_again() {
    let mut h = desk(Graphics::Sixel);
    let (mut screen, wire) = screen();
    let placed = only_sixel(&frame(&mut h, &mut screen, &wire));
    h.set_theme("nordic");
    let again = frame(&mut h, &mut screen, &wire);
    assert_eq!(only_sixel(&again), placed, "the new ground was written over the pixels, so they come back");
    assert!(again.find(" ") < again.find("\x1bP"), "after the cells");
    assert_eq!(frame(&mut h, &mut screen, &wire), "");
}

#[test]
fn after_a_handoff_the_picture_is_painted_again() {
    let mut h = desk(Graphics::Sixel);
    let (mut screen, wire) = screen();
    let placed = only_sixel(&frame(&mut h, &mut screen, &wire));
    screen.release_pictures().expect("a release");
    screen.redraw_all(BufferRect::new(0, 0, WIDTH, HEIGHT)).expect("a redraw");
    assert_eq!(only_sixel(&frame(&mut h, &mut screen, &wire)), placed, "after a program had the screen");
    assert_eq!(frame(&mut h, &mut screen, &wire), "");
}

#[test]
fn a_late_kitty_answer_wipes_the_sixel_and_places_the_picture_the_kitty_way() {
    let mut h = desk(Graphics::Sixel);
    let (mut screen, wire) = screen();
    let (cursor, _) = only_sixel(&frame(&mut h, &mut screen, &wire));
    let top = row_of(&cursor);
    h.set_graphics(Graphics::Kitty);
    let kitty = frame(&mut h, &mut screen, &wire);
    assert!(!kitty.contains("\x1bP"), "{kitty:?}");
    assert!(kitty.contains("\x1b_Ga=t,") && kitty.contains("\x1b_Ga=p,"), "sent and placed: {kitty:?}");
    for row in top..=HEIGHT {
        let at = kitty.find(&format!("\x1b[{row};1H\x1b[")).or_else(|| kitty.find(&format!("\x1b[{row};1H ")));
        assert!(at.is_some(), "row {row} is written over the sixel although its cells did not change: {kitty:?}");
        assert!(at < kitty.find("\x1b_G"), "before the kitty picture");
    }
    assert_eq!(frame(&mut h, &mut screen, &wire), "");
    h.set_graphics(Graphics::Sixel);
    let back = frame(&mut h, &mut screen, &wire);
    assert!(back.contains("\x1b_Ga=d,d=I,"), "the kitty picture is freed: {back:?}");
    assert_eq!(only_sixel(&back).0, cursor);
}

/// Pictures at fixed places, each painted within a clip.
#[derive(Clone, Default)]
struct Scene(Vec<(ImageData, Rect, Rect)>);

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

struct Stage(Scene);

impl Widget<Scene> for Stage {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }
    fn paint(&self, cx: &mut PaintCx<'_>, _area: Rect) {
        for (data, at, clip) in &self.0.0 {
            cx.with_clip(*clip, |cx| Widget::<Scene>::paint(&Image::new(data).fit(Fit::Cover), cx, *at));
        }
    }
}

/// Eight by eight pixels of one colour.
fn square() -> ImageData {
    ImageData::from_rgb(8, 8, &[200, 120, 40].repeat(64)).expect("pixels")
}

#[test]
fn a_picture_cut_by_its_clip_is_painted_on_the_part_that_shows() {
    let whole = Rect::new(2, 1, 8, 6);
    let clip = Rect::new(2, 3, 8, 4);
    let mut h = Harness::new(Scene(vec![(square(), whole, clip)]), WIDTH, HEIGHT);
    h.set_graphics(Graphics::Sixel);
    let (mut screen, wire) = screen();
    let written = frame(&mut h, &mut screen, &wire);
    assert_eq!(only_sixel(&written), ("\x1b[4;3H".to_owned(), "\"1;1;80;78".to_owned()), "{written:?}");
    assert!(written.contains("#"), "{written:?}");
}

#[test]
fn a_picture_under_a_later_one_is_half_blocks_and_the_one_on_top_a_sixel() {
    let screen_rect = Rect::new(0, 0, WIDTH, HEIGHT);
    let (under, over) = (photo(), square());
    let scene = Scene(vec![(under, Rect::new(0, 0, 20, 10), screen_rect), (over, Rect::new(10, 4, 8, 3), screen_rect)]);
    let mut h = Harness::new(scene, WIDTH, HEIGHT);
    h.set_graphics(Graphics::Sixel);
    let (mut screen, wire) = screen();
    let written = frame(&mut h, &mut screen, &wire);
    assert_eq!(only_sixel(&written), ("\x1b[5;11H".to_owned(), "\"1;1;80;60".to_owned()), "{written:?}");
    assert_eq!(half_blocks(h.buffer()), 20 * 10 - 8 * 3, "the one beneath, but for the one on top");
}
