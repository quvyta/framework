//! Pictures decoded from files and drawn with half blocks, read the way a terminal shows them.

use std::path::PathBuf;

use ratatui_core::style::Color;

use super::*;
use crate::color::{ColorDepth, Rgb};
use crate::env::{AssetDirs, Env};
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;

const RED: Rgb = Rgb::new(255, 0, 0);
const GREEN: Rgb = Rgb::new(0, 255, 0);
const BLUE: Rgb = Rgb::new(0, 0, 255);
const WHITE: Rgb = Rgb::new(255, 255, 255);

/// A folder of its own under the system's temporary folder, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quvyta-image-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

/// Raw RGB bytes of `width` × `height` pixels, each coloured by `colour(x, y)`.
fn bytes(width: u32, height: u32, colour: impl Fn(u32, u32) -> Rgb) -> Vec<u8> {
    let mut out = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let c = colour(x, y);
            out.extend([c.r, c.g, c.b]);
        }
    }
    out
}

/// Writes a PNG of `width` × `height` pixels coloured by `colour` to `path`.
fn write_png(path: &std::path::Path, width: u32, height: u32, colour: impl Fn(u32, u32) -> Rgb) {
    let buffer = ::image::RgbImage::from_raw(width, height, bytes(width, height, colour)).expect("pixel buffer");
    buffer.save(path).expect("the PNG is written");
}

/// Four by four pixels in four quadrants: red top left, green top right, blue bottom left and
/// white bottom right.
fn quadrants() -> ImageData {
    let colour = |x: u32, y: u32| match (x < 2, y < 2) {
        (true, true) => RED,
        (false, true) => GREEN,
        (true, false) => BLUE,
        (false, false) => WHITE,
    };
    ImageData::from_rgb(4, 4, &bytes(4, 4, colour)).expect("sixteen pixels")
}

#[test]
fn a_png_decodes_to_its_pixels() {
    let dir = scratch("decode");
    let path = dir.join("dots.png");
    write_png(&path, 3, 2, |x, y| {
        [RED, GREEN, BLUE, WHITE, Rgb::new(10, 20, 30), Rgb::new(200, 100, 50)][(y * 3 + x) as usize]
    });
    let data = ImageData::decode_file(&path, (100, 100)).expect("the PNG decodes");
    assert_eq!((data.width(), data.height()), (3, 2));
    assert_eq!(data.pixel(0, 0), Some(RED));
    assert_eq!(data.pixel(2, 0), Some(BLUE));
    assert_eq!(data.pixel(1, 1), Some(Rgb::new(10, 20, 30)));
    assert_eq!(data.pixel(2, 1), Some(Rgb::new(200, 100, 50)));
    assert_eq!(data.pixel(3, 0), None);
    assert_eq!(data.name(), Some("dots.png"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_big_picture_is_scaled_down_to_at_most_the_size_asked_for() {
    let dir = scratch("big");
    let path = dir.join("wide.png");
    write_png(&path, 1200, 600, |x, _| if x < 600 { RED } else { BLUE });
    let data = ImageData::decode_file(&path, (300, 200)).expect("the PNG decodes");
    assert!(data.width() <= 300 && data.height() <= 200, "{} x {}", data.width(), data.height());
    assert_eq!((data.width(), data.height()), (300, 150), "the shape is kept");
    assert_eq!(data.original_size(), (1200, 600), "the file's own size is remembered");
    assert_eq!(data.pixel(0, 75), Some(RED));
    assert_eq!(data.pixel(299, 75), Some(BLUE));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn missing_unknown_and_broken_files_say_so_without_panicking() {
    let dir = scratch("errors");
    assert_eq!(ImageData::decode_file(&dir.join("nowhere.png"), (10, 10)).err(), Some(ImageError::Missing));

    let text = dir.join("notes.png");
    std::fs::write(&text, "these are words, not a picture").expect("text file");
    assert_eq!(ImageData::decode_file(&text, (10, 10)).err(), Some(ImageError::UnknownFormat));

    let whole = dir.join("whole.png");
    write_png(&whole, 40, 40, |x, y| Rgb::new((x * 6) as u8, (y * 6) as u8, 90));
    let mut cut = std::fs::read(&whole).expect("the PNG is read back");
    cut.truncate(cut.len() / 2);
    let broken = dir.join("broken.png");
    std::fs::write(&broken, cut).expect("half a PNG");
    assert_eq!(ImageData::decode_file(&broken, (10, 10)).err(), Some(ImageError::Broken));

    assert_eq!(ImageData::decode_file(&dir, (10, 10)).err(), Some(ImageError::Unreadable), "a folder is no picture");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn errors_read_as_sentences_from_the_language_files() {
    let mut i18n = crate::i18n::I18n::builtin();
    i18n.set_active("en");
    let english = crate::i18n::scope(std::sync::Arc::new(i18n.clone()), || ImageError::Missing.to_string());
    assert_eq!(english, "This picture does not exist");
    i18n.set_active("tr");
    let turkish = crate::i18n::scope(std::sync::Arc::new(i18n), || ImageError::Broken.to_string());
    assert_eq!(turkish, "Bu resim bozuk");
}

#[test]
fn raw_pixels_need_the_right_length() {
    assert!(ImageData::from_rgb(2, 2, &[0; 12]).is_some());
    assert!(ImageData::from_rgb(2, 2, &[0; 11]).is_none());
    assert!(ImageData::from_rgb(0, 2, &[]).is_none());
}

/// One picture filling the screen, drawn with `fit`.
struct Shown {
    data: ImageData,
    fit: Fit,
}

impl App for Shown {
    type Msg = ();
    fn update(&mut self, _: ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Image::new(&self.data).fit(self.fit)).fill().id("picture");
    }
}

fn shown(data: ImageData, fit: Fit, width: u16, height: u16) -> Harness<Shown> {
    let mut h = Harness::new(Shown { data, fit }, width, height);
    h.render();
    h
}

/// The colours a cell shows: the top pixel as the text of `▀`, the bottom one behind it.
fn halves(h: &Harness<Shown>, x: u16, y: u16) -> (Option<Rgb>, Option<Rgb>) {
    assert_eq!(h.buffer()[(x, y)].symbol(), "▀", "cell {x},{y} is a half block:\n{}", h.screen());
    (h.fg(x, y), h.bg(x, y))
}

#[test]
fn the_top_pixel_is_the_text_colour_and_the_bottom_pixel_the_background() {
    let data = ImageData::from_rgb(1, 2, &bytes(1, 2, |_, y| if y == 0 { RED } else { BLUE })).expect("two pixels");
    let h = shown(data, Fit::Center, 1, 1);
    assert_eq!(halves(&h, 0, 0), (Some(RED), Some(BLUE)));
}

#[test]
fn contain_fits_the_whole_picture_and_leaves_the_ground_beside_it() {
    // Four by two cells hold four by four pixels: the picture as it is.
    let h = shown(quadrants(), Fit::Contain, 4, 2);
    assert_eq!(halves(&h, 0, 0), (Some(RED), Some(RED)));
    assert_eq!(halves(&h, 3, 0), (Some(GREEN), Some(GREEN)));
    assert_eq!(halves(&h, 1, 1), (Some(BLUE), Some(BLUE)));
    assert_eq!(halves(&h, 2, 1), (Some(WHITE), Some(WHITE)));

    // Eight cells wide: the square picture keeps its shape in the middle four columns.
    let h = shown(quadrants(), Fit::Contain, 8, 2);
    let ground = h.bg(0, 0);
    assert_eq!(h.buffer()[(0, 0)].symbol(), " ", "the ground is left as it was");
    assert_eq!(h.bg(7, 1), ground);
    assert_eq!(halves(&h, 2, 0), (Some(RED), Some(RED)));
    assert_eq!(halves(&h, 5, 1), (Some(WHITE), Some(WHITE)));

    // Eight by four cells hold eight by eight pixels: every pixel becomes two by two.
    let h = shown(quadrants(), Fit::Contain, 8, 4);
    assert_eq!(halves(&h, 0, 0), (Some(RED), Some(RED)));
    assert_eq!(halves(&h, 7, 1), (Some(GREEN), Some(GREEN)));
    assert_eq!(halves(&h, 0, 3), (Some(BLUE), Some(BLUE)));
}

#[test]
fn contain_averages_the_pixels_a_cell_covers() {
    // One cell, two pixels tall: the square picture shrinks to its top pixel, the average of all
    // four colours, and the bottom half keeps the ground.
    let h = shown(quadrants(), Fit::Contain, 1, 1);
    let ground = h.env().theme().color("canvas");
    assert_eq!(h.buffer()[(0, 0)].symbol(), "▀");
    assert_eq!(h.fg(0, 0), Some(Rgb::new(128, 128, 128)), "the mean of red, green, blue and white");
    assert_eq!(h.bg(0, 0), ground);

    // Two by one cells: each pixel is the average of a quadrant of two by two pixels... of one
    // colour, so the quadrants come through clean.
    let h = shown(quadrants(), Fit::Contain, 2, 1);
    assert_eq!(halves(&h, 0, 0), (Some(RED), Some(BLUE)));
    assert_eq!(halves(&h, 1, 0), (Some(GREEN), Some(WHITE)));
}

#[test]
fn cover_fills_the_area_and_cuts_what_spills_over_from_the_middle() {
    // Eight by two cells are eight by four pixels: the picture doubles to eight by eight and
    // keeps its middle four rows, the lower red and green row and the upper blue and white one.
    let h = shown(quadrants(), Fit::Cover, 8, 2);
    for x in 0..8 {
        let (top, bottom) = if x < 4 { (RED, BLUE) } else { (GREEN, WHITE) };
        assert_eq!(halves(&h, x, 0), (Some(top), Some(top)), "column {x}");
        assert_eq!(halves(&h, x, 1), (Some(bottom), Some(bottom)), "column {x}");
    }

    // Two by four cells are two by eight pixels: the middle column of the picture, stretched.
    let h = shown(quadrants(), Fit::Cover, 2, 4);
    assert_eq!(halves(&h, 0, 0), (Some(RED), Some(RED)));
    assert_eq!(halves(&h, 1, 0), (Some(GREEN), Some(GREEN)));
    assert_eq!(halves(&h, 0, 3), (Some(BLUE), Some(BLUE)));
    assert_eq!(halves(&h, 1, 3), (Some(WHITE), Some(WHITE)));
}

#[test]
fn center_keeps_the_real_size_and_cuts_around_the_middle() {
    // Two by one cells are two by two pixels: the four pixels around the centre.
    let h = shown(quadrants(), Fit::Center, 2, 1);
    assert_eq!(halves(&h, 0, 0), (Some(RED), Some(BLUE)));
    assert_eq!(halves(&h, 1, 0), (Some(GREEN), Some(WHITE)));

    // Eight by four cells are eight by eight pixels: the picture at its own size in the middle.
    let h = shown(quadrants(), Fit::Center, 8, 4);
    assert_eq!(h.buffer()[(1, 0)].symbol(), " ", "the ground around it is left alone");
    assert_eq!(halves(&h, 2, 1), (Some(RED), Some(RED)));
    assert_eq!(halves(&h, 5, 1), (Some(GREEN), Some(GREEN)));
    assert_eq!(halves(&h, 2, 2), (Some(BLUE), Some(BLUE)));
    assert_eq!(halves(&h, 5, 2), (Some(WHITE), Some(WHITE)));
    assert_eq!(h.buffer()[(2, 3)].symbol(), " ");
}

#[test]
fn a_picture_with_an_odd_number_of_rows_ends_in_half_a_cell() {
    // One by three pixels in one by two cells, shown at their size: the last cell holds the
    // picture's bottom pixel above and keeps the ground below.
    let data = ImageData::from_rgb(1, 3, &bytes(1, 3, |_, y| [RED, GREEN, BLUE][y as usize])).expect("three pixels");
    let h = shown(data, Fit::Center, 1, 2);
    let ground = h.env().theme().color("canvas");
    assert_eq!(h.buffer()[(0, 0)].symbol(), "▀");
    assert_eq!((h.fg(0, 0), h.bg(0, 0)), (Some(RED), Some(GREEN)));
    assert_eq!(h.buffer()[(0, 1)].symbol(), "▀");
    assert_eq!((h.fg(0, 1), h.bg(0, 1)), (Some(BLUE), ground));
}

#[test]
fn the_cells_are_worked_out_once_and_again_only_when_something_changes() {
    let mut h = shown(quadrants(), Fit::Contain, 20, 6);
    let once = resamples();
    h.render().render().render();
    assert_eq!(resamples(), once, "frames where nothing changed reuse the cells");
    h.resize(30, 6);
    assert_eq!(resamples(), once + 1, "a new size works them out again");
    h.render();
    assert_eq!(resamples(), once + 1);
}

/// A picture whose fit a message changes.
struct Switching {
    data: ImageData,
    cover: bool,
}

impl App for Switching {
    type Msg = ImageData;
    fn update(&mut self, data: ImageData) -> Command<ImageData> {
        self.cover = !self.cover;
        self.data = data;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ImageData>) {
        let fit = if self.cover { Fit::Cover } else { Fit::Contain };
        ui.add(Image::new(&self.data).fit(fit)).fill().id("picture");
    }
}

#[test]
fn a_new_fit_or_a_new_picture_works_the_cells_out_again() {
    let mut h = Harness::new(Switching { data: quadrants(), cover: false }, 12, 4);
    h.render();
    let once = resamples();
    // The message switches the fit and hands over the same picture.
    let same = h.app().data.clone();
    h.send(same);
    assert_eq!(resamples(), once + 1, "the fit changed");
    h.send(quadrants());
    assert_eq!(resamples(), once + 2, "a picture decoded again is a new picture");
}

/// The screen's words on one line, so a sentence the empty state wraps reads whole.
fn words(screen: &str) -> String {
    screen.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn words_of(h: &Harness<Shown>) -> String {
    words(&h.screen())
}

#[test]
fn sixteen_colours_show_what_the_picture_is_instead_of_drawing_it() {
    let dir = scratch("sixteen");
    let path = dir.join("harbour.png");
    write_png(&path, 64, 48, |x, y| Rgb::new((x * 4) as u8, (y * 5) as u8, 120));
    let data = ImageData::decode_file(&path, (400, 400)).expect("the PNG decodes");
    let mut h = shown(data, Fit::Cover, 60, 12);
    assert!(h.screen().contains('▀'), "true colour draws it");
    h.set_depth(ColorDepth::Ansi16);
    let screen = h.screen();
    let words = words(&screen);
    assert!(!screen.contains('▀') && !screen.contains('▄'), "{screen}");
    assert!(words.contains("harbour.png"), "{screen}");
    assert!(words.contains("PNG, 64 x 48 pixels"), "{screen}");
    assert!(words.contains("This terminal cannot show pictures"), "{screen}");
    h.set_locale("tr");
    assert!(words_of(&h).contains("Bu terminal resim gösteremiyor"), "{}", h.screen());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn ascii_glyphs_show_what_the_picture_is_instead_of_drawing_it() {
    let mut h = shown(quadrants(), Fit::Contain, 60, 12);
    h.set_glyph_mode(GlyphMode::Ascii);
    let screen = h.screen();
    assert!(!screen.contains('▀') && !screen.contains('▄'), "{screen}");
    assert!(words(&screen).contains("4 x 4 pixels"), "{screen}");
    assert!(words(&screen).contains("This terminal cannot show pictures"), "{screen}");
    assert!(screen.is_ascii(), "every cell stays printable ASCII:\n{screen}");
}

#[test]
fn a_smooth_picture_in_256_colours_takes_the_nearest_entry_for_both_halves() {
    // Two close greys, one above the other: a half block is a fill, so its upper half must not be
    // pushed to a far entry to stand out from the lower half the way faint text would be.
    let (top, bottom) = (Rgb::new(120, 120, 120), Rgb::new(124, 124, 124));
    let data = ImageData::from_rgb(1, 2, &bytes(1, 2, |_, y| if y == 0 { top } else { bottom })).expect("two pixels");
    let mut h = shown(data, Fit::Center, 1, 1);
    h.set_depth(ColorDepth::Ansi256);
    let cell = &h.buffer()[(0, 0)];
    assert_eq!(cell.symbol(), "▀");
    assert_eq!(cell.fg, Color::Indexed(top.to_ansi256()));
    assert_eq!(cell.bg, Color::Indexed(bottom.to_ansi256()));
}

#[test]
fn cover_fills_a_large_screen_exactly() {
    let data = ImageData::from_rgb(90, 70, &bytes(90, 70, |x, y| Rgb::new((x * 2) as u8, (y * 3) as u8, 60)))
        .expect("a gradient");
    let h = shown(data, Fit::Cover, 120, 40);
    for y in 0..40 {
        for x in 0..120 {
            assert_eq!(h.buffer()[(x, y)].symbol(), "▀", "cell {x},{y}");
        }
    }
    assert_ne!(h.fg(0, 0), h.fg(119, 39), "the gradient runs across");
}

/// A picture that measures itself in a row, beside a word.
struct Measured(Fit);

impl App for Measured {
    type Msg = ();
    fn update(&mut self, _: ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.column(|ui| {
            ui.add(Image::new(&quadrants()).fit(self.0)).id("picture");
            ui.add(crate::widgets::Text::new("below"));
        });
    }
}

#[test]
fn a_picture_asks_for_the_cells_its_fit_needs() {
    // In a column forty cells wide and twenty tall, the square picture of four by four pixels
    // shown at its size takes four columns and two rows, so the word sits on the third row.
    let h = Harness::new(Measured(Fit::Center), 40, 20);
    assert_eq!(h.find("below"), Some((0, 2)), "{}", h.screen());
    // Contain grows it to the width: forty pixels square, twenty rows.
    let h = Harness::new(Measured(Fit::Contain), 40, 30);
    assert_eq!(h.find("below"), Some((0, 20)), "{}", h.screen());
}

#[test]
fn a_tiny_area_draws_nothing_outside_itself() {
    let h = shown(quadrants(), Fit::Cover, 1, 1);
    assert_eq!(h.buffer()[(0, 0)].symbol(), "▀");
    // One by two pixels: the middle two columns, their top half red and green, above their
    // bottom half blue and white.
    assert_eq!(h.fg(0, 0), Some(Rgb::new(128, 128, 0)), "red and green averaged");
    assert_eq!(h.bg(0, 0), Some(Rgb::new(128, 128, 255)), "blue and white averaged");
}

#[test]
fn a_kitty_terminal_gets_plain_ground_cells_to_draw_the_picture_on() {
    let mut h = Harness::new(Shown { data: quadrants(), fit: Fit::Cover }, 4, 2);
    let before = resamples();
    h.set_graphics(crate::graphics::Graphics::Kitty).render();
    let canvas = h.env().theme().color("canvas");
    for y in 0..2 {
        for x in 0..4 {
            assert_eq!(h.buffer()[(x, y)].symbol(), " ", "no half block at {x},{y}:\n{}", h.screen());
            assert_eq!(h.bg(x, y), canvas, "the ground at {x},{y}");
        }
    }
    assert_eq!(resamples(), before, "no half blocks are worked out for a picture the terminal draws");
    assert!(words_of(&h).is_empty(), "nothing a text selection or a screen reader would read");
}

/// Whether the picture's area shows pixels, of either kind, rather than what the picture is.
fn draws_pixels(h: &Harness<Shown>) -> bool {
    !words_of(h).contains("This terminal cannot show pictures")
}

/// A harness whose environment reads `QUVYTA_GRAPHICS=<forced>` on a 256-colour terminal.
fn forced(graphics: Graphics) -> Harness<Shown> {
    let lookup = move |name: &str| match name {
        "LANG" => Some("en_US.UTF-8".to_owned()),
        "TERM" => Some("xterm-256color".to_owned()),
        "QUVYTA_ICONS" => Some("unicode".to_owned()),
        "QUVYTA_GRAPHICS" => Some(graphics.name().to_owned()),
        _ => None,
    };
    let env = Env::load_with(&AssetDirs::default(), lookup).expect("the built-in files load");
    Harness::with_env(Shown { data: quadrants(), fit: Fit::Contain }, env, 60, 12)
}

#[test]
fn can_draw_answers_what_the_image_itself_does() {
    let mut h = shown(quadrants(), Fit::Contain, 60, 12);
    let agrees = |h: &Harness<Shown>, case: &str| {
        let graphics = h.env().graphics();
        assert_eq!(graphics.can_draw(), draws_pixels(h), "{case}: {graphics:?}\n{}", h.screen());
    };
    agrees(&h, "half blocks");
    assert!(h.env().graphics().can_draw(), "half blocks draw");
    h.set_graphics(Graphics::Kitty);
    agrees(&h, "kitty");
    h.set_graphics(Graphics::HalfBlock).set_depth(ColorDepth::Ansi16);
    agrees(&h, "16 colours");
    assert!(!h.env().graphics().can_draw(), "16 colours draw nothing");
    h.set_depth(ColorDepth::TrueColor).set_glyph_mode(GlyphMode::Ascii);
    agrees(&h, "ASCII");
    assert!(!h.env().graphics().can_draw(), "ASCII draws nothing");
    // The variable wins over the terminal: a picture refused on a terminal that could draw it
    // says so, and the check says the same.
    let none = forced(Graphics::None);
    agrees(&none, "QUVYTA_GRAPHICS=none");
    assert!(!none.env().graphics().can_draw());
    let mut half = forced(Graphics::HalfBlock);
    half.set_depth(ColorDepth::Ansi16);
    agrees(&half, "QUVYTA_GRAPHICS=halfblock at 16 colours");
    assert!(half.env().graphics().can_draw());
}

/// A PNG of `width` × `height` pixels coloured by `colour`, made in memory.
fn png_bytes(width: u32, height: u32, colour: impl Fn(u32, u32) -> Rgb) -> Vec<u8> {
    let buffer = ::image::RgbImage::from_raw(width, height, bytes(width, height, colour)).expect("pixel buffer");
    let mut out = std::io::Cursor::new(Vec::new());
    buffer.write_to(&mut out, ::image::ImageFormat::Png).expect("the PNG is encoded");
    out.into_inner()
}

#[test]
fn a_picture_in_memory_decodes_and_shrinks_as_a_file_does() {
    let png = png_bytes(3, 2, |x, y| {
        [RED, GREEN, BLUE, WHITE, Rgb::new(10, 20, 30), Rgb::new(200, 100, 50)][(y * 3 + x) as usize]
    });
    let data = ImageData::decode_bytes(&png, (100, 100)).expect("the PNG decodes");
    assert_eq!((data.width(), data.height()), (3, 2));
    assert_eq!(data.pixel(0, 0), Some(RED));
    assert_eq!(data.pixel(2, 1), Some(Rgb::new(200, 100, 50)));
    assert_eq!(data.name(), None, "no file, no name");

    let wide = png_bytes(1200, 600, |x, _| if x < 600 { RED } else { BLUE });
    let small = ImageData::decode_bytes(&wide, (300, 200)).expect("the PNG decodes");
    assert_eq!((small.width(), small.height()), (300, 150));
    assert_eq!(small.original_size(), (1200, 600));
    let dir = scratch("from-memory");
    let path = dir.join("wide.png");
    std::fs::write(&path, &wide).expect("the same PNG as a file");
    let from_file = ImageData::decode_file(&path, (300, 200)).expect("the PNG decodes");
    assert_eq!(from_file.pixels(), small.pixels(), "the same bytes shrink to the same pixels");
    assert_eq!(from_file.name(), Some("wide.png"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unknown_and_broken_bytes_say_so_without_panicking() {
    assert_eq!(
        ImageData::decode_bytes(b"these are words, not a picture", (10, 10)).err(),
        Some(ImageError::UnknownFormat)
    );
    assert_eq!(ImageData::decode_bytes(&[], (10, 10)).err(), Some(ImageError::UnknownFormat), "nothing at all");
    let mut cut = png_bytes(40, 40, |x, y| Rgb::new((x * 6) as u8, (y * 6) as u8, 90));
    cut.truncate(cut.len() / 2);
    assert_eq!(ImageData::decode_bytes(&cut, (10, 10)).err(), Some(ImageError::Broken));
}

#[test]
fn the_extensions_are_exactly_the_formats_the_decoder_reads() {
    use std::collections::BTreeSet;
    // Every format the image crate was built to read, with every extension it knows for it: a
    // feature turned on or off there changes this side, and the list must follow.
    let compiled: BTreeSet<&str> = ::image::ImageFormat::all()
        .filter(::image::ImageFormat::reading_enabled)
        .inspect(|format| assert!(data::Kind::from_format(*format).is_some(), "{format:?} reads but has no kind"))
        .flat_map(|format| format.extensions_str().iter().copied())
        .collect();
    let listed: BTreeSet<&str> = ImageData::EXTENSIONS.iter().copied().collect();
    assert_eq!(listed, compiled);
    assert_eq!(listed.len(), ImageData::EXTENSIONS.len(), "no extension twice");
    for extension in ImageData::EXTENSIONS {
        assert_eq!(extension.to_lowercase(), *extension, "lower case, as FileBrowser::extensions keeps them");
    }
    assert!(ImageData::reads(std::path::Path::new("/pictures/Harbour.JPG")));
    assert!(ImageData::reads(std::path::Path::new("dusk.webp")));
    assert!(!ImageData::reads(std::path::Path::new("notes.txt")));
    assert!(!ImageData::reads(std::path::Path::new("png")), "a name without an extension");
}
