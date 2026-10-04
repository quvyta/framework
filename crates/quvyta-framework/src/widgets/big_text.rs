//! Big text: digits, Latin letters and the signs of a clock drawn several rows tall, for counters,
//! titles and headings in any Latin alphabet.

use crate::color::{ColorDepth, Rgb};
use crate::env::Env;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::GlyphMode;
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Pixel rows of every glyph.
const PIXEL_ROWS: usize = 5;

/// A space between two words: as wide as a letter and lit nowhere.
const SPACE: [&str; PIXEL_ROWS] = ["..", "..", "..", "..", ".."];

/// The pixel rows of one glyph, one string per row, every row as long as the glyph is wide.
type Form = [&'static str; PIXEL_ROWS];

/// Whether `active`, the code [`I18n::active`](crate::i18n::I18n::active) gives, is a language that
/// writes `i` with a dot: `tr` and `az`, whatever region follows them.
fn dotted_i(active: &str) -> bool {
    matches!(active.split('-').next(), Some("tr" | "az"))
}

/// `c` as the key its glyph is looked up under: its Unicode uppercase, first character, with the
/// Turkish rule for the two i's, so `ı` is always `I` and `i` is `İ` in the languages that write it
/// that way. `ß` keeps its own form, because uppercasing it would spell a different word.
fn uppercase(c: char, dotted_i: bool) -> char {
    match c {
        'ı' => 'I',
        'i' if dotted_i => 'İ',
        'ß' => 'ß',
        _ => c.to_uppercase().next().unwrap_or(c),
    }
}

/// The bitmap of `c`, one string per pixel row, `#` for a lit pixel, or `None` for a character
/// this font has no form for.
///
/// Lowercase letters use the uppercase forms. Five rows leave no room above a capital, so an
/// accent is a mark of its own in the top row and the letter is drawn in the rows under it; a
/// cedilla takes the bottom row instead. `Ü` and `Ÿ` use their top row for the two dots and stand a
/// row lower. `Ä` and `Ö` are a pixel wider than the plain letters they come from, because squeezed
/// into three columns they would read as `H` and `V`.
fn glyph(c: char, dotted_i: bool) -> Option<Form> {
    Some(match uppercase(c, dotted_i) {
        '0' => ["###", "#.#", "#.#", "#.#", "###"],
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["###", "..#", "###", "#..", "###"],
        '3' => ["###", "..#", ".##", "..#", "###"],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' => ["###", "#..", "###", "..#", "###"],
        '6' => ["###", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", "..#", "..#", "..#"],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "###"],
        ':' => [".", "#", ".", "#", "."],
        '.' => [".", ".", ".", ".", "#"],
        '%' => ["#.#", "..#", ".#.", "#..", "#.#"],
        '-' => ["...", "...", "###", "...", "..."],
        '\'' => [".", "#", ".", ".", "."],
        '!' => [".#.", ".#.", ".#.", "...", ".#."],
        '?' => ["###", "..#", ".#.", "...", ".#."],
        '&' => [".##..", "#..#.", "#.#..", "#..#.", ".##.#"],
        '(' => [".#.", "##.", "#..", "##.", ".#."],
        ')' => [".#.", ".##", "..#", ".##", ".#."],
        ',' => ["..", "..", "..", ".#", "#."],
        '/' => ["..#", "..#", ".#.", "#..", "#.."],
        'A' => [".#.", "#.#", "###", "#.#", "#.#"],
        'À' => ["#..", "#.#", "###", "#.#", "#.#"],
        'Á' => ["..#", "#.#", "###", "#.#", "#.#"],
        'Â' => ["##.", "#.#", "###", "#.#", "#.#"],
        'Ä' => ["#..#", ".##.", "####", "#..#", "#..#"],
        'Å' => ["###", "#.#", "###", "#.#", "#.#"],
        'Æ' => [".####", "#.#..", "#####", "#.#..", "#.###"],
        'B' => ["##.", "#.#", "##.", "#.#", "##."],
        'C' => [".##", "#..", "#..", "#..", ".##"],
        'Ç' => [".##", "#..", "#..", ".##", ".#."],
        'D' => ["##.", "#.#", "#.#", "#.#", "##."],
        'E' => ["###", "#..", "##.", "#..", "###"],
        'È' => ["#..", "#..", "##.", "#..", "###"],
        'É' => ["..#", "#..", "##.", "#..", "###"],
        'Ê' => ["##.", "#..", "##.", "#..", "###"],
        'Ë' => ["#.#", "#..", "##.", "#..", "###"],
        'F' => ["###", "#..", "##.", "#..", "#.."],
        'G' => [".##", "#..", "#.#", "#.#", ".##"],
        'Ğ' => ["#.#", "#..", "#.#", "#.#", ".##"],
        'H' => ["#.#", "#.#", "###", "#.#", "#.#"],
        'I' => ["###", ".#.", ".#.", ".#.", "###"],
        'İ' => [".#.", ".#.", ".#.", ".#.", "###"],
        'Ì' => ["#..", ".#.", ".#.", ".#.", "###"],
        'Í' => ["..#", ".#.", ".#.", ".#.", "###"],
        'Î' => ["##.", ".#.", ".#.", ".#.", "###"],
        'Ï' => ["#.#", ".#.", ".#.", ".#.", "###"],
        'J' => ["..#", "..#", "..#", "#.#", ".#."],
        'K' => ["#.#", "#.#", "##.", "#.#", "#.#"],
        'L' => ["#..", "#..", "#..", "#..", "###"],
        'M' => ["#...#", "##.##", "#.#.#", "#...#", "#...#"],
        'N' => ["#..#", "##.#", "#.##", "#..#", "#..#"],
        'Ñ' => ["#.#.", "##.#", "#.##", "#..#", "#..#"],
        'O' => [".#.", "#.#", "#.#", "#.#", ".#."],
        'Ò' => ["#..", "#.#", "#.#", "#.#", ".#."],
        'Ó' => ["..#", "#.#", "#.#", "#.#", ".#."],
        'Ô' => ["##.", "#.#", "#.#", "#.#", ".#."],
        'Ö' => ["#..#", ".##.", "#..#", "#..#", ".##."],
        'Ø' => [".#.", "#.#", "###", "#.#", ".#."],
        'Œ' => [".####", "#.#..", "#####", "#.#..", ".####"],
        'P' => ["##.", "#.#", "##.", "#..", "#.."],
        'Q' => [".#.", "#.#", "#.#", "##.", ".##"],
        'R' => ["##.", "#.#", "##.", "#.#", "#.#"],
        'S' => [".##", "#..", ".#.", "..#", "##."],
        'Ş' => [".##", "#..", ".#.", "##.", "..#"],
        'T' => ["###", ".#.", ".#.", ".#.", ".#."],
        'U' => ["#.#", "#.#", "#.#", "#.#", "###"],
        'Ù' => ["#..", "#.#", "#.#", "#.#", "###"],
        'Ú' => ["..#", "#.#", "#.#", "#.#", "###"],
        'Û' => ["##.", "#.#", "#.#", "#.#", "###"],
        'Ü' => ["#.#", "...", "#.#", "#.#", "###"],
        'V' => ["#.#", "#.#", "#.#", "#.#", ".#."],
        'W' => ["#...#", "#...#", "#.#.#", "##.##", "#...#"],
        'X' => ["#.#", "#.#", ".#.", "#.#", "#.#"],
        'Y' => ["#.#", "#.#", ".#.", ".#.", ".#."],
        'Ý' => ["..#", "#.#", ".#.", ".#.", ".#."],
        'Ÿ' => ["#.#", "...", "#.#", ".#.", ".#."],
        'Z' => ["###", "..#", ".#.", "#..", "###"],
        'ß' => ["##.", "#.#", "##.", "#.#", ".##"],
        ' ' => SPACE,
        _ => return None,
    })
}

/// The width of `forms`: every glyph as wide as its rows are long, with a column between them.
fn big_width(forms: &[Form]) -> u16 {
    // Saturating: a long text is wider than any screen and falls back to plain text anyway.
    let glyphs =
        forms.iter().map(|pixels| clamp_u16(i32::try_from(pixels[0].len()).unwrap_or(0))).fold(0, u16::saturating_add);
    let gaps = clamp_u16(i32::try_from(forms.len()).unwrap_or(i32::MAX) - 1);
    glyphs.saturating_add(gaps)
}

/// Which way a [`BigText`] gradient runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gradient {
    /// From left to right: every column of cells takes one step of the blend.
    Columns,
    /// From top to bottom: every row of cells takes one step of the blend.
    Rows,
}

/// Text drawn large from block elements: the digits, the letters A to Z with their Turkish
/// (`Ç Ğ İ Ö Ş Ü`) and Western European (`À Á Â Ä Å È É Ê Ë Ì Í Î Ï Ñ Ò Ó Ô Ö Ø Ù Ú Û Ü Ý Ÿ ß Æ Œ`)
/// forms, the signs `: . % -`, the punctuation `' ! ? & ( ) , /` and a space.
///
/// Each glyph is five pixels tall. With Unicode and Nerd Font glyphs two pixels share a cell
/// through half blocks, so the text is three rows tall; ASCII mode draws one pixel per cell in
/// the background colour, five rows tall. Glyphs are separated by one column. When the area is too
/// small, the text is drawn at normal size in bold instead of being cut.
///
/// Lowercase letters are drawn with the uppercase form of their character, following the Turkish
/// rule: `ı` is always `I`, `i` is `İ` while the active language is Turkish or Azerbaijani and
/// `I` in every other. A text with a character this font has no form for, Cyrillic or an emoji
/// among them, is drawn whole as plain text in bold rather than with a hole where the character
/// is; [`BigText::fits`] says which of the two a text gets.
///
/// The letters take one flat colour unless [`BigText::gradient`] blends them into a second theme
/// colour; the blend is painted, not animated ([`ShimmerText`](super::ShimmerText) is the moving
/// one). It steps per cell, so the three glyph modes differ only in how many steps they have: a
/// blend down the rows has three steps with Unicode and Nerd Font glyphs and five in ASCII mode.
///
/// Style keys: `big-text` and `big-text.<variant>` (`fg`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigText {
    text: String,
    variant: Option<String>,
    gradient: Option<(String, Gradient)>,
}

impl BigText {
    /// Big `text`, e.g. `"14:32"`, `"98%"` or `"AŞK İÇİNDE"`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), variant: None, gradient: None }
    }

    /// Whether every character of `text` has a big form, so `BigText::new(text)` draws it as
    /// glyphs; a space counts, it draws as a blank between two words. A text that does not fit,
    /// Cyrillic or Japanese or an emoji, is drawn whole as plain text in bold instead, the same
    /// way a narrow area is, so it never loses a character to a hole. Ask when the choice matters:
    /// which of the two an application shows, or how much room it leaves.
    ///
    /// ```
    /// use qframe::widgets::BigText;
    ///
    /// assert!(BigText::fits("Çağrı 12:30"));
    /// assert!(!BigText::fits("Привет"));
    /// ```
    ///
    /// The language in force while painting decides how `i` is drawn, as [`BigText`] describes; it
    /// changes the form, not whether there is one.
    #[must_use]
    pub fn fits(text: &str) -> bool {
        let dotted_i = dotted_i(&crate::i18n::active_code());
        text.chars().all(|c| glyph(c, dotted_i).is_some())
    }

    /// Theme variant, e.g. `"accent"`.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Blends the letters from their own colour into the theme colour `to`, running `direction`.
    ///
    /// `to` is a theme colour token such as `"info"` or `"accent-2"`, never a colour of its own, so
    /// the blend changes with the theme. It falls back to the flat colour, which every glyph mode
    /// and colour depth can draw, when the terminal has only the sixteen standard colours or when
    /// the theme does not know `to`.
    #[must_use]
    pub fn gradient(mut self, to: impl Into<String>, direction: Gradient) -> Self {
        self.gradient = Some((to.into(), direction));
        self
    }

    /// The glyph of every character of the text, uppercased for the language `env` is in, or
    /// `None` when one of them has no form: such a text is plain text, whole, not glyphs with a
    /// hole in them.
    fn forms(&self, env: &Env) -> Option<Vec<Form>> {
        let dotted_i = dotted_i(env.i18n().active());
        self.text.chars().map(|c| glyph(c, dotted_i)).collect()
    }

    /// The whole text at normal size in bold, cut with `…` where the area ends: what a text with
    /// no form, and an area too small for one, are drawn as.
    fn plain(&self, cx: &mut PaintCx<'_>, area: Rect, color: Rgb) {
        let shown = text::truncate(&self.text, area.width).into_owned();
        cx.text(area.x, area.y, &shown, CellStyle::fg(color).with_bold(true), area.width);
    }

    /// The far end of the gradient and its direction, when one is asked for and both the terminal
    /// and the theme can give it.
    ///
    /// The sixteen standard colours cannot hold a blend: every cell would round to its own palette
    /// entry on its own and the letters would speckle instead of shading, so there the text keeps
    /// its flat colour. A token the theme does not know falls back the same way, rather than
    /// blending into a colour nobody chose.
    fn blend(&self, cx: &PaintCx<'_>) -> Option<(Rgb, Gradient)> {
        let (token, direction) = self.gradient.as_ref()?;
        if cx.env().depth() == ColorDepth::Ansi16 {
            return None;
        }
        Some((cx.env().theme().color(token)?, *direction))
    }
}

/// Rows the big form takes in `mode`.
fn big_rows(mode: GlyphMode) -> u16 {
    if mode == GlyphMode::Ascii { 5 } else { 3 }
}

impl<Msg: 'static> Widget<Msg> for BigText {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let rows = big_rows(cx.env().glyph_mode());
        let plain = Size::new(text::width(&self.text), 1);
        // Measured as it is painted: a text with no form takes one row, as the narrow area does.
        let Some(forms) = self.forms(cx.env()) else {
            return plain.min(available);
        };
        let big = Size::new(big_width(&forms), rows);
        if big.width <= available.width && big.height <= available.height { big } else { plain.min(available) }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let mut style = cx.style("big-text", self.variant.as_deref(), &[]).text();
        style.bg = None;
        let color = style.fg.unwrap_or_else(|| cx.color("text"));
        let mode = cx.env().glyph_mode();
        let rows = big_rows(mode);
        let Some(forms) = self.forms(cx.env()) else {
            self.plain(cx, area, color);
            return;
        };
        let width = big_width(&forms);
        if width > area.width || rows > area.height {
            self.plain(cx, area, color);
            return;
        }
        let blend = self.blend(cx);
        // The blend runs over the whole text, so the gaps between glyphs count as steps too.
        let tone = |cell_x: i32, cell_row: u16| match blend {
            None => color,
            Some((end, Gradient::Columns)) => color.mix(end, share(clamp_u16(cell_x - area.x), width)),
            Some((end, Gradient::Rows)) => color.mix(end, share(cell_row, rows)),
        };
        let mut x = area.x;
        for pixels in &forms {
            let columns = clamp_u16(i32::try_from(pixels[0].len()).unwrap_or(0));
            for column in 0..usize::from(columns) {
                let lit = |row: usize| pixels.get(row).is_some_and(|line| line.as_bytes()[column] == b'#');
                let cell_x = x + i32::try_from(column).unwrap_or(0);
                if mode == GlyphMode::Ascii {
                    for row in 0..PIXEL_ROWS {
                        if lit(row) {
                            let row = clamp_u16(i32::try_from(row).unwrap_or(0));
                            cx.clear(Rect::new(cell_x, area.y + i32::from(row), 1, 1), tone(cell_x, row));
                        }
                    }
                    continue;
                }
                for cell_row in 0..3 {
                    let symbol = match (lit(cell_row * 2), lit(cell_row * 2 + 1)) {
                        (true, true) => "█",
                        (true, false) => "▀",
                        (false, true) => "▄",
                        (false, false) => continue,
                    };
                    let row = clamp_u16(i32::try_from(cell_row).unwrap_or(0));
                    let y = area.y + i32::from(row);
                    cx.text(cell_x, y, symbol, CellStyle::fg(tone(cell_x, row)), 1);
                }
            }
            x += i32::from(columns) + 1;
        }
    }
}

/// Where step `step` of `steps` stands, from 0 to 1; a single step is at the start.
fn share(step: u16, steps: u16) -> f32 {
    match steps {
        0 | 1 => 0.0,
        // Cell counts are small, so the division is exact enough for a colour blend.
        steps => f32::from(step.min(steps - 1)) / f32::from(steps - 1),
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::style::Color;

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo(&'static str);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(BigText::new(self.0).variant("accent"));
        }
    }

    /// Big text with a gradient towards a theme colour.
    struct Blended {
        text: &'static str,
        to: &'static str,
        direction: Gradient,
    }

    impl App for Blended {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(BigText::new(self.text).variant("accent").gradient(self.to, self.direction));
        }
    }

    fn blended(text: &'static str, direction: Gradient) -> Harness<Blended> {
        Harness::new(Blended { text, to: "info", direction }, 20, 5)
    }

    /// Two big texts one under the other, so where the second one stands is where the first one
    /// measured itself to end.
    struct Stacked(&'static str);

    impl App for Stacked {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(BigText::new(self.0));
            ui.add(BigText::new("7"));
        }
    }

    /// What one character draws on screen, so two of them can be told apart.
    fn drawn(c: char) -> String {
        Harness::new(Demo(c.to_string().leak()), 8, 3).screen()
    }

    /// The columns each character of `text` covers in the big form, in the order the text names
    /// them: its glyph, then the gap after it.
    fn spans(text: &str) -> Vec<(usize, usize)> {
        let mut x = 0;
        text.chars()
            .map(|c| {
                let form = glyph(c, false).expect("a word drawn in big form");
                let columns = form[0].len();
                let span = (x, x + columns);
                x += columns + 1;
                span
            })
            .collect()
    }

    /// Every character the table grew a form for, with the plain letter it must not look like.
    /// `None` where there is none to look like: `Æ` and `Œ` are ligatures of two letters and
    /// stand on their own, and the signs are signs.
    const NEW_FORMS: [(char, Option<char>); 40] = [
        ('Ç', Some('C')),
        ('Ğ', Some('G')),
        ('İ', Some('I')),
        ('Ö', Some('O')),
        ('Ş', Some('S')),
        ('Ü', Some('U')),
        ('À', Some('A')),
        ('Á', Some('A')),
        ('Â', Some('A')),
        ('Ä', Some('A')),
        ('Å', Some('A')),
        ('Æ', None),
        ('È', Some('E')),
        ('É', Some('E')),
        ('Ê', Some('E')),
        ('Ë', Some('E')),
        ('Ì', Some('I')),
        ('Í', Some('I')),
        ('Î', Some('I')),
        ('Ï', Some('I')),
        ('Ñ', Some('N')),
        ('Ò', Some('O')),
        ('Ó', Some('O')),
        ('Ô', Some('O')),
        ('Ø', Some('O')),
        ('Œ', None),
        ('Ù', Some('U')),
        ('Ú', Some('U')),
        ('Û', Some('U')),
        ('Ý', Some('Y')),
        ('Ÿ', Some('Y')),
        ('ß', Some('S')),
        ('\'', None),
        ('!', None),
        ('?', None),
        ('&', None),
        ('(', None),
        (')', None),
        (',', None),
        ('/', None),
    ];

    #[test]
    fn every_new_character_draws_a_form_of_its_own() {
        for (c, base) in NEW_FORMS {
            let form = drawn(c);
            assert!(form.contains(['█', '▀', '▄']), "{c:?} has no form of its own:\n{form}");
            if let Some(base) = base {
                assert_ne!(form, drawn(base), "{c:?} draws as the plain {base:?}");
            }
        }
    }

    #[test]
    fn a_turkish_title_leaves_no_hole_where_a_letter_is() {
        let h = Harness::new(Demo("AŞK İÇİNDE"), 40, 3);
        let screen = h.screen();
        let rows: Vec<Vec<char>> = screen.lines().map(|line| line.chars().collect()).collect();
        let lit = |column: usize| {
            rows.iter().any(|row| row.get(column).copied().is_some_and(|cell| matches!(cell, '█' | '▀' | '▄')))
        };
        for (c, (from, to)) in "AŞK İÇİNDE".chars().zip(spans("AŞK İÇİNDE")) {
            if c == ' ' {
                continue;
            }
            for column in from..to {
                assert!(lit(column), "{c:?} is a hole at column {column}:\n{screen}");
            }
        }
    }

    #[test]
    fn a_lowercase_word_is_drawn_with_the_forms_of_its_letters() {
        // "çağrı" reads as Ç A Ğ R I: the accented forms of the table, each letter uppercased.
        let h = Harness::new(Demo("çağrı"), 20, 3);
        assert_eq!(h.screen(), "▄▀▀ ▄▀▄ █ ▀ █▀▄ ▀█▀\n▀▄▄ █▀█ █ █ █▀▄  █\n ▀  ▀ ▀  ▀▀ ▀ ▀ ▀▀▀\n");
    }

    #[test]
    fn the_two_i_s_follow_the_language_of_the_screen() {
        // The top row of "▀█▀" is the bar of an I over its stem, of " █" a dot over that stem.
        let mut h = Harness::new(Demo("i"), 6, 3);
        assert_eq!(h.screen(), "▀█▀\n █\n▀▀▀\n", "English i is the I\n{}", h.screen());
        h.set_locale("tr");
        assert_eq!(h.screen(), " █\n █\n▀▀▀\n", "Turkish i is the İ\n{}", h.screen());
        let mut dotless = Harness::new(Demo("ı"), 6, 3);
        dotless.set_locale("tr");
        assert_eq!(dotless.screen(), "▀█▀\n █\n▀▀▀\n", "and ı is the I in every language\n{}", dotless.screen());
    }

    #[test]
    fn a_text_without_a_form_is_drawn_whole_as_plain_text() {
        assert!(!BigText::fits("Привет"), "Cyrillic has no form in this font");
        assert!(BigText::fits("Çağrı 12:30"), "a Turkish title has every one of its own");
        let h = Harness::new(Demo("Привет"), 20, 3);
        assert_eq!(h.screen(), "Привет\n\n\n", "no hole where a letter is:\n{}", h.screen());
        assert!(h.is_bold(0, 0));
    }

    #[test]
    fn a_text_without_a_form_leaves_the_one_row_it_measured() {
        let h = Harness::new(Stacked("Привет"), 20, 5);
        assert_eq!(h.screen(), "Привет\n▀▀█\n  █\n  ▀\n\n", "the second text starts where the first one ended");
    }

    #[test]
    fn ascii_cells_carry_the_new_forms_too() {
        let mut h = Harness::new(Demo("Ç"), 6, 5);
        h.set_glyph_mode(GlyphMode::Ascii);
        let accent = h.env().theme().color("accent");
        assert_eq!(h.bg(1, 0), accent, "the top bar of the C");
        assert_eq!(h.bg(1, 3), accent, "the bowl closes a row higher to leave room for the cedilla");
        assert_eq!(h.bg(1, 4), accent, "and the cedilla is a lit cell of its own under it");
        assert_ne!(h.bg(2, 4), accent, "one cell wide, not the bar of the C");
    }

    #[test]
    fn ascii_cells_tell_the_two_i_s_apart_too() {
        let mut h = Harness::new(Demo("i"), 6, 5);
        h.set_glyph_mode(GlyphMode::Ascii);
        let accent = h.env().theme().color("accent");
        assert_eq!(h.bg(0, 0), accent, "English: the bar of the I");
        h.set_locale("tr");
        assert_ne!(h.bg(0, 0), accent, "Turkish: only the dot over the stem");
        assert_eq!(h.bg(1, 0), accent);
    }

    #[test]
    fn ascii_cells_fall_back_to_plain_text_the_same_way() {
        let mut h = Harness::new(Demo("Привет"), 20, 5);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "Привет\n\n\n\n\n");
        assert!(h.is_bold(0, 0));
    }

    #[test]
    fn digits_and_colon_in_half_blocks() {
        let h = Harness::new(Demo("12:05"), 20, 3);
        assert_eq!(h.screen(), "▄█  ▀▀█ ▄ █▀█ █▀▀\n █  █▀▀ ▄ █ █ ▀▀█\n▀▀▀ ▀▀▀   ▀▀▀ ▀▀▀\n");
        assert_eq!(h.fg(0, 0), h.env().theme().color("accent"));
    }

    #[test]
    fn letters_and_percent() {
        let h = Harness::new(Demo("OK 9%"), 20, 3);
        assert_eq!(h.screen(), "▄▀▄ █ █    █▀█ ▀ █\n█ █ █▀▄    ▀▀█ ▄▀\n ▀  ▀ ▀    ▀▀▀ ▀ ▀\n");
    }

    #[test]
    fn ascii_uses_coloured_cells_five_rows_tall() {
        let mut h = Harness::new(Demo("7"), 6, 5);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "\n\n\n\n\n");
        let accent = h.env().theme().color("accent");
        assert_eq!(h.bg(0, 0), accent);
        assert_eq!(h.bg(2, 4), accent);
        assert_ne!(h.bg(0, 4), accent);
    }

    #[test]
    fn falls_back_to_plain_bold_text_when_small() {
        let h = Harness::new(Demo("12:05"), 20, 2);
        assert_eq!(h.screen(), "12:05\n\n");
        assert!(h.is_bold(0, 0));
    }

    #[test]
    fn text_wider_than_any_screen_falls_back_without_overflowing() {
        let h = Harness::new(Demo("12:05".repeat(4_000).leak()), 8, 3);
        assert_eq!(h.screen(), "12:0512…\n\n\n");
    }

    /// The two ends of the blend in the built-in theme, and where a step of it stands.
    fn ends<A: App>(h: &Harness<A>) -> (Rgb, Rgb) {
        let theme = h.env().theme();
        (theme.color("accent").expect("token"), theme.color("info").expect("token"))
    }

    #[test]
    fn the_flat_colour_is_the_default() {
        let h = Harness::new(Demo("OK"), 20, 3);
        let (accent, info) = ends(&h);
        // "OK" is seven columns wide: three for each letter and the gap between them.
        for x in [0, 4, 6] {
            assert_eq!(h.fg(x, 0), Some(accent), "column {x} keeps the flat colour");
        }
        assert_ne!(accent, info, "the test would say nothing if the ends were the same colour");
    }

    #[test]
    fn a_column_gradient_blends_from_left_to_right() {
        let h = blended("OK", Gradient::Columns);
        let (accent, info) = ends(&h);
        assert_eq!(h.screen(), "▄▀▄ █ █\n█ █ █▀▄\n ▀  ▀ ▀\n\n\n");
        assert_eq!(h.fg(0, 0), Some(accent), "the first column is the near end");
        assert_eq!(h.fg(4, 0), Some(accent.mix(info, 4.0 / 6.0)), "and every column a step further");
        assert_eq!(h.fg(6, 0), Some(info), "the last column is the far end");
        assert_eq!(h.fg(0, 1), h.fg(0, 0), "a column is one tone from top to bottom");
    }

    #[test]
    fn a_row_gradient_blends_from_top_to_bottom() {
        let h = blended("OK", Gradient::Rows);
        let (accent, info) = ends(&h);
        // The stem of the K is lit in all three rows.
        assert_eq!(h.fg(4, 0), Some(accent), "the first row is the near end");
        assert_eq!(h.fg(4, 1), Some(accent.mix(info, 0.5)), "the middle row is halfway");
        assert_eq!(h.fg(4, 2), Some(info), "the last row is the far end");
        assert_eq!(h.fg(6, 0), h.fg(4, 0), "a row is one tone from left to right");
    }

    #[test]
    fn ascii_blends_over_its_five_rows_of_cells() {
        let mut h = blended("OK", Gradient::Rows);
        h.set_glyph_mode(GlyphMode::Ascii);
        let (accent, info) = ends(&h);
        assert_eq!(h.bg(4, 0), Some(accent));
        assert_eq!(h.bg(4, 2), Some(accent.mix(info, 0.5)), "five rows of cells, so five steps");
        assert_eq!(h.bg(4, 4), Some(info));

        let mut columns = blended("OK", Gradient::Columns);
        columns.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(columns.bg(0, 1), Some(accent));
        assert_eq!(columns.bg(6, 0), Some(info), "and the same ends across the columns");
    }

    #[test]
    fn nerd_font_glyphs_blend_like_unicode_ones() {
        let mut h = blended("OK", Gradient::Columns);
        let unicode: Vec<Option<Rgb>> = (0..7).map(|x| h.fg(x, 0)).collect();
        h.set_glyph_mode(GlyphMode::Nerd);
        let nerd: Vec<Option<Rgb>> = (0..7).map(|x| h.fg(x, 0)).collect();
        assert_eq!(unicode, nerd);
    }

    #[test]
    fn sixteen_colours_fall_back_to_the_flat_colour() {
        let mut h = blended("OK", Gradient::Columns);
        h.set_depth(ColorDepth::Ansi16);
        let (accent, _) = ends(&h);
        let flat = Color::Indexed(accent.to_ansi16());
        for (x, y) in [(0, 0), (4, 0), (6, 0), (4, 1), (4, 2)] {
            assert_eq!(h.buffer()[(x, y)].fg, flat, "cell {x},{y} is the flat colour");
        }
        let mut deeper = blended("OK", Gradient::Columns);
        deeper.set_depth(ColorDepth::Ansi256);
        assert_ne!(
            deeper.buffer()[(0, 0)].fg,
            deeper.buffer()[(6, 0)].fg,
            "the 256-colour palette still shows the blend"
        );
    }

    #[test]
    fn a_colour_the_theme_does_not_know_stays_flat() {
        let h = Harness::new(Blended { text: "OK", to: "sunset", direction: Gradient::Columns }, 20, 5);
        let (accent, _) = ends(&h);
        assert_eq!(h.fg(0, 0), Some(accent));
        assert_eq!(h.fg(6, 0), Some(accent));
    }

    #[test]
    fn the_plain_fallback_keeps_the_flat_colour() {
        let h = Harness::new(Blended { text: "OK", to: "info", direction: Gradient::Columns }, 20, 2);
        let (accent, _) = ends(&h);
        assert_eq!(h.screen(), "OK\n\n");
        assert!(h.is_bold(0, 0));
        assert_eq!(h.fg(0, 0), Some(accent));
        assert_eq!(h.fg(1, 0), Some(accent), "a blend over two cells would read as a mistake");
    }

    #[test]
    fn one_glyph_wide_text_and_every_theme_keep_both_ends() {
        for theme in ["monochrome", "nordic", "amber", "iris"] {
            let mut h = blended("OK", Gradient::Columns);
            h.set_theme(theme);
            let (accent, info) = ends(&h);
            assert_eq!(h.fg(0, 0), Some(accent), "{theme}");
            assert_eq!(h.fg(6, 0), Some(info), "{theme}");

            let mut single = blended("7", Gradient::Columns);
            single.set_theme(theme);
            assert_eq!(single.fg(0, 0), Some(accent), "{theme}: three columns still blend");
            assert_eq!(single.fg(2, 0), Some(info), "{theme}");
        }
    }

    #[test]
    fn share_of_a_single_step_is_the_near_end() {
        assert_eq!(share(0, 0), 0.0);
        assert_eq!(share(0, 1), 0.0);
        assert_eq!(share(3, 3), 1.0, "a step past the end stays at the far end");
        assert_eq!(share(1, 3), 0.5);
    }
}
