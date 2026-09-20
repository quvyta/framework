//! The card a repository shows when its link is shared: a screenshot, a name and one sentence on
//! an opaque ground of the theme's own `canvas`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io;
use std::path::Path;

use qframe::color::Rgb;

use crate::font::{self, GlyphKey};
use crate::geometry::{CELL_H, CELL_W, num};
use crate::png::SCALE;
use crate::{Shot, svg, with_extension};

/// The default card: the size GitHub, Mastodon and the chat applications show a shared link at.
pub(crate) const DEFAULT_SIZE: (u32, u32) = (1280, 640);

/// Free ground left and right of everything, as a share of the card's width.
const PAD_X: f32 = 0.075;
/// Free ground above and below everything, as a share of the card's height.
const PAD_Y: f32 = 0.125;
/// Width of the column the name and the sentence live in, as a share of the card's width.
const TEXT_W: f32 = 0.34;
/// Free ground between that column and the screenshot, as a share of the card's width.
const GAP: f32 = 0.03;
/// Height of the name's line, as a share of the card's height.
const NAME_H: f32 = 0.16;
/// Height of one line of the sentence, as a share of the card's height.
const PROMISE_H: f32 = 0.0625;
/// Free ground between the name and the sentence, as a share of the card's height.
const TEXT_GAP: f32 = 0.055;
/// Space from one line of the sentence to the next, as a share of the line's own height.
const LINE_STEP: f32 = 1.2;

/// A repository's card: one screenshot, the application's name and its one-sentence promise on a
/// canvas of a fixed size.
///
/// Everything is drawn from the embedded font, and the ground is the theme's `canvas` with no
/// transparency anywhere, so the card is the same file on every machine and shows the same
/// colours in every viewer:
///
/// ```rust,ignore
/// qshots::Card::new(shot)
///     .name("qtools")
///     .promise("Every tool you keep reaching for, in one window.")
///     .save("docs/screenshots/social")?;
/// ```
///
/// The screenshot keeps its shape and is scaled to fit its share of the card, so nothing of it is
/// cut; a compact scene, around 80 columns, reads best. Text that does not fit is never cut
/// either: the sentence wraps, and text with no room left is an error naming what was too long.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    shot: Shot,
    name: String,
    promise: String,
    size: (u32, u32),
}

impl Card {
    /// A card showing `shot`, 1280 x 640, with no name and no sentence yet.
    ///
    /// The screenshot is drawn with square corners, since the card's ground is opaque.
    #[must_use]
    pub fn new(shot: Shot) -> Self {
        Self { shot: shot.square(), name: String::new(), promise: String::new(), size: DEFAULT_SIZE }
    }

    /// The application's name, drawn large in the theme's accent colour. It stays on one line: a
    /// name too long for the column is an error.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// One sentence saying what the application is for, drawn under the name in the theme's text
    /// colour and wrapped to the column.
    #[must_use]
    pub fn promise(mut self, promise: impl Into<String>) -> Self {
        self.promise = promise.into();
        self
    }

    /// The size of the PNG in pixels. Both numbers are even, since the card is drawn at half the
    /// size and rasterised at twice it, as the rest of this crate is.
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Characters the embedded fonts have no glyph for, in the screenshot, the name or the
    /// sentence. They are left out of the picture, so a test asserts this is empty before
    /// publishing a card.
    #[must_use]
    pub fn missing(&self) -> Vec<char> {
        let mut missing: BTreeSet<char> = self.shot.missing().into_iter().collect();
        for (text, bold) in [(&self.name, true), (&self.promise, false)] {
            missing.extend(text.chars().filter(|c| !c.is_whitespace() && font::glyph(*c, bold).is_none()));
        }
        missing.into_iter().collect()
    }

    /// The card as SVG, holding glyph outlines instead of text and no external resource.
    ///
    /// # Errors
    ///
    /// Fails when the size is not one a card can be drawn at, when the name is too long for its
    /// column, or when the sentence has more lines than there is room for. The message names what
    /// did not fit; nothing is ever cut to make it fit.
    pub fn to_svg(&self) -> io::Result<String> {
        draw(self)
    }

    /// The card as PNG, at exactly the size [`Card::size`] asked for.
    ///
    /// # Errors
    ///
    /// The errors of [`Card::to_svg`], and those of the rasteriser.
    pub fn to_png(&self) -> io::Result<Vec<u8>> {
        crate::png::render(&self.to_svg()?)
    }

    /// Writes `<path>.svg` and `<path>.png`, creating the folder. `path` names the picture
    /// without an extension, e.g. `docs/screenshots/social`.
    ///
    /// # Errors
    ///
    /// The errors of [`Card::to_svg`], and those of creating the folder or writing either file.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let svg = self.to_svg()?;
        let png = crate::png::render(&svg)?;
        std::fs::write(with_extension(path, "svg"), svg)?;
        std::fs::write(with_extension(path, "png"), png)
    }
}

/// Glyph outlines of the card's own text, defined once each and placed by a transform, so a line
/// of any size costs one outline per character.
#[derive(Default)]
struct Text {
    defs: Vec<String>,
    ids: std::collections::HashMap<GlyphKey, Option<usize>>,
    body: String,
}

impl Text {
    /// Draws `line` with its top left corner at (`x`, `y`), every cell `scale` times the size of
    /// a terminal cell. Characters without a glyph are left out; [`Card::missing`] reports them.
    fn line(&mut self, line: &str, x: f32, y: f32, scale: f32, bold: bool, color: Rgb) {
        let mut uses = String::new();
        let mut pen = 0.0;
        for c in line.chars() {
            let span = f32::from(qframe::text::width(&c.to_string())) * CELL_W;
            if !c.is_whitespace()
                && let Some((key, advance)) = font::glyph(c, bold)
            {
                let defs = &mut self.defs;
                let id = *self.ids.entry(key).or_insert_with(|| {
                    let outline = font::outline(key)?;
                    defs.push(outline);
                    Some(defs.len() - 1)
                });
                if let Some(id) = id {
                    let at = pen + (span - advance) / 2.0;
                    let _ = write!(uses, "<use href=\"#t{id}\" x=\"{}\" y=\"0\"/>", num(at));
                }
            }
            pen += span;
        }
        if !uses.is_empty() {
            let _ = writeln!(
                self.body,
                "<g fill=\"{color}\" transform=\"translate({} {}) scale({})\">{uses}</g>",
                num(x),
                num(y),
                num(scale),
            );
        }
    }
}

/// The width `text` takes in terminal cells.
fn cells(text: &str) -> f32 {
    text.chars().map(|c| f32::from(qframe::text::width(&c.to_string()))).sum()
}

/// `text` broken at spaces into lines of at most `columns` cells.
///
/// # Errors
///
/// A word longer than the whole column: it can be neither wrapped nor cut, so the caller is told
/// which word it was.
fn wrap(text: &str, columns: f32) -> io::Result<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let width = cells(word);
        if width > columns {
            return Err(io::Error::other(format!(
                "`{word}` is {width:.0} cells wide and the card's text column is {columns:.0}: \
                 shorten the sentence or widen the card"
            )));
        }
        match lines.last_mut() {
            Some(line) if cells(line) + 1.0 + width <= columns => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    Ok(lines)
}

/// Draws `card`: the ground, the name and the wrapped sentence in their column, and the
/// screenshot scaled to fit the rest.
fn draw(card: &Card) -> io::Result<String> {
    let (px_w, px_h) = card.size;
    if px_w % 2 != 0 || px_h % 2 != 0 {
        return Err(io::Error::other(format!(
            "a card is drawn at half its size and rasterised at twice it, so {px_w} x {px_h} has \
             to be even in both directions"
        )));
    }
    #[expect(clippy::cast_precision_loss, reason = "card sizes are small whole numbers of pixels")]
    let (width, height) = (px_w as f32 / SCALE, px_h as f32 / SCALE);
    let (pad_x, pad_y) = (width * PAD_X, height * PAD_Y);
    let text_w = width * TEXT_W;
    let shot_x = pad_x + text_w + width * GAP;
    let (shot_w, shot_h) = (width - pad_x - shot_x, height - 2.0 * pad_y);
    if text_w < CELL_W || shot_w < CELL_W || shot_h < CELL_H {
        return Err(io::Error::other(format!("{px_w} x {px_h} is too small for a name, a sentence and a screenshot")));
    }

    let (name_scale, promise_scale) = (height * NAME_H / CELL_H, height * PROMISE_H / CELL_H);
    let name_width = cells(&card.name) * CELL_W * name_scale;
    if name_width > text_w {
        return Err(io::Error::other(format!(
            "the name `{}` is {name_width:.0} units wide and its column is {text_w:.0}: \
             shorten the name or widen the card",
            card.name,
        )));
    }
    let promise_lines = wrap(&card.promise, (text_w / (CELL_W * promise_scale)).floor())?;
    let name_h = if card.name.is_empty() { 0.0 } else { height * NAME_H };
    let line_step = height * PROMISE_H * LINE_STEP;
    #[expect(clippy::cast_precision_loss, reason = "a card holds a handful of lines")]
    let promise_h = match promise_lines.len() {
        0 => 0.0,
        n => (n - 1) as f32 * line_step + height * PROMISE_H,
    };
    let gap = if name_h > 0.0 && promise_h > 0.0 { height * TEXT_GAP } else { 0.0 };
    let text_h = name_h + gap + promise_h;
    if text_h > height - 2.0 * pad_y {
        return Err(io::Error::other(format!(
            "the name and the sentence need {text_h:.0} units and the card has {:.0}: \
             shorten the sentence or make the card taller",
            height - 2.0 * pad_y,
        )));
    }

    let body = svg::body(&card.shot);
    // Cut down to the two decimals the picture is written with, so the size drawn is the size
    // placed: rounding up instead would push the screenshot a pixel past its own margin.
    let scale = ((shot_w / body.width).min(shot_h / body.height) * 100.0).floor() / 100.0;
    if scale <= 0.0 {
        return Err(io::Error::other(format!(
            "a screen of {} x {} cells is more than a hundred times the room a {px_w} x {px_h} \
             card has for it: use a smaller scene or a larger card",
            card.shot.screen.width, card.shot.screen.height,
        )));
    }
    let (drawn_w, drawn_h) = (body.width * scale, body.height * scale);

    let mut text = Text::default();
    let mut y = (height - text_h) / 2.0;
    if !card.name.is_empty() {
        text.line(&card.name, pad_x, y, name_scale, true, card.shot.screen.palette.accent);
        y += name_h + gap;
    }
    for line in &promise_lines {
        text.line(line, pad_x, y, promise_scale, false, card.shot.screen.palette.text);
        y += line_step;
    }

    let (w, h) = (num(width), num(height));
    let mut svg = String::new();
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"
    );
    if !text.defs.is_empty() {
        svg.push_str("<defs>\n");
        for (id, d) in text.defs.iter().enumerate() {
            let _ = writeln!(svg, "<path id=\"t{id}\" d=\"{d}\"/>");
        }
        svg.push_str("</defs>\n");
    }
    // The whole ground first: the card is opaque everywhere, so a viewer without transparency
    // shows the theme's canvas and never black.
    let _ = writeln!(svg, "<rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>", card.shot.screen.palette.canvas);
    // The screenshot is held against the card's right edge: whatever its shape, the ground left
    // of it is free, and its margin matches the name's on the other side.
    let _ = writeln!(
        svg,
        "<g transform=\"translate({} {}) scale({})\">",
        num(width - pad_x - drawn_w),
        num(pad_y + (shot_h - drawn_h) / 2.0),
        num(scale),
    );
    svg.push_str(&body.content);
    svg.push_str("</g>\n");
    svg.push_str(&text.body);
    svg.push_str("</svg>\n");
    Ok(svg)
}
