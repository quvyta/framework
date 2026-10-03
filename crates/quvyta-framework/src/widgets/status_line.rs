//! Status lines: one line of the application's own, saying how things stand.

use super::Button;
use super::cells;
use super::toast::ToastKind;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, Node, PaintCx, Widget};

/// Cells between the sentence and the button when both fit on one row.
const ACTION_GAP: u16 = 2;

/// The cells from the area's left edge to the sentence: the sign, then a space.
fn indent_cells(sign_width: u16) -> u16 {
    cells::sum([sign_width, 1])
}

/// How a status line fits into `width` cells.
struct Plan {
    /// The cells the line wants, which is the whole sentence on one row while it fits.
    width: u16,
    /// The sentence's lines.
    lines: Vec<String>,
    /// Whether the button takes a row of its own because the line does not fit on one.
    below: bool,
}

/// Lays `text` out in `width` cells, beside a button `action` cells wide, with `indent` cells of
/// sign in front of it.
fn plan(text: &str, width: u16, indent: u16, action: u16) -> Plan {
    // The gap belongs between the sentence and the button, so only a button needs room for it.
    let gap = if action == 0 { 0 } else { ACTION_GAP };
    let wanted = cells::sum([indent, text::width(text), gap, action]);
    if wanted <= width {
        return Plan { width: wanted, lines: vec![text.to_owned()], below: false };
    }
    // Every line but the first starts under the sentence and never under the sign, so the sign keeps
    // standing beside the words it belongs to.
    Plan { width, lines: text::wrap(text, width.saturating_sub(indent).max(1)), below: true }
}

/// A line that says how things stand: the sign of a [`ToastKind`], the sentence in that kind's
/// colour, and at most one button at the end.
///
/// One row while there is room for it: sign, a space, the sentence, and the button after a gap. In
/// a narrow area the sentence wraps under itself and the button moves to a row of its own, so
/// nothing is cut and the sign never ends up below a line it does not belong to.
///
/// A status colour never comes without its sign, and both come from the kind: the sign is the glyph
/// a [`Toast`](super::Toast) of that kind shows, in the same colour, so a sentence and a toast
/// that say the same thing look like one family.
///
/// Style keys: `status-line` (`fg`, `bold`) with the variants `success`, `warning`, `danger` and
/// `info`; the sentence takes the kind's colour when a theme names no variant, and the sign always
/// does.
pub struct StatusLine<Msg> {
    text: String,
    tone: ToastKind,
    action: Vec<Node<Msg>>,
}

impl<Msg: Clone + 'static> StatusLine<Msg> {
    /// A status line reading `text` in the neutral tone.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), tone: ToastKind::default(), action: Vec::new() }
    }

    /// The status the sentence reports, which gives it its sign and its colour.
    #[must_use]
    pub fn tone(mut self, tone: ToastKind) -> Self {
        self.tone = tone;
        self
    }

    /// A button at the end of the line, e.g. the way out of what the sentence reports.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::widgets::{Button, StatusLine, ToastKind};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Save,
    /// }
    ///
    /// let unsaved: StatusLine<Msg> = StatusLine::new("3 changes are not saved")
    ///     .tone(ToastKind::Warning)
    ///     .action(Button::new("Save").on_press(Msg::Save));
    /// ```
    #[must_use]
    pub fn action(mut self, button: Button<Msg>) -> Self {
        self.action = vec![Node::new(button, 0)];
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for StatusLine<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let sign = text::width(&cx.env().icons().glyph(self.tone.icon()));
        // The button is asked in the room there is, the way paint asks it again in the room the
        // line ends up with, so the two agree on whether it fits beside the sentence.
        let action = self.action.first().map_or(0, |node| cx.measure_child(node, Size::new(available.width, 1)).width);
        let plan = plan(&self.text, available.width, indent_cells(sign), action);
        let lines = clamp_u16(i32::try_from(plan.lines.len()).unwrap_or(i32::MAX));
        Size::new(plan.width, cells::sum([lines, u16::from(plan.below)])).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let sign = cx.env().icons().glyph(self.tone.icon()).into_owned();
        let sign_width = text::width(&sign);
        let indent = indent_cells(sign_width);
        let action = self.action.first().map(|node| cx.measure_child(node, Size::new(area.width, 1)));
        let plan = plan(&self.text, area.width, indent, action.map_or(0, |size| size.width));

        let tone = cx.color(self.tone.name());
        let mut style = cx.style("status-line", Some(self.tone.name()), &[]).text();
        style.bg = None;
        style.fg = style.fg.or(Some(tone));
        cx.text(area.x, area.y, &sign, CellStyle { fg: Some(tone), ..CellStyle::default() }, sign_width);

        let sentence_x = area.x.saturating_add(i32::from(indent));
        let room = clamp_u16(area.right().saturating_sub(sentence_x));
        for (row, line) in plan.lines.iter().enumerate() {
            let y = area.y.saturating_add(i32::try_from(row).unwrap_or(i32::MAX));
            cx.text(sentence_x, y, line, style, room);
        }

        let (Some(node), Some(size)) = (self.action.first(), action) else { return };
        let (x, y) = if plan.below {
            (area.x, area.y.saturating_add(i32::try_from(plan.lines.len()).unwrap_or(i32::MAX)))
        } else {
            (
                sentence_x.saturating_add(i32::from(text::width(&self.text))).saturating_add(i32::from(ACTION_GAP)),
                area.y,
            )
        };
        let rect = Rect::new(x, y, size.width.min(clamp_u16(area.right().saturating_sub(x))), 1);
        if area.contains(rect.x, rect.y) {
            cx.paint_child(node, rect);
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.action
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    /// What each kind says, so a demo can show all four signs.
    const REPORTED: [(ToastKind, &str); 4] = [
        (ToastKind::Success, "Deployed api-gateway"),
        (ToastKind::Warning, "Disk is almost full"),
        (ToastKind::Danger, "Build failed on web-frontend"),
        (ToastKind::Info, "Release 2.14.0 is live"),
    ];

    /// One line per status kind.
    struct Kinds;

    /// A danger line with the way out beside it, and a plain warning line above it.
    struct Reported {
        saved: u32,
    }

    #[derive(Clone)]
    enum Msg {
        Save,
    }

    impl App for Kinds {
        type Msg = Msg;
        fn update(&mut self, _: Msg) -> Command<Msg> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                for (kind, sentence) in REPORTED {
                    ui.add(StatusLine::new(sentence).tone(kind));
                }
            });
        }
    }

    impl App for Reported {
        type Msg = Msg;
        fn update(&mut self, _: Msg) -> Command<Msg> {
            self.saved += 1;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.add(StatusLine::new("3 changes are not saved").tone(ToastKind::Warning));
                ui.add(
                    StatusLine::new("The build of web-frontend failed")
                        .tone(ToastKind::Danger)
                        .action(Button::new("Retry").on_press(Msg::Save)),
                );
            });
        }
    }

    /// The row `text` is on and the column it starts in, in cells.
    fn at<A: App>(h: &Harness<A>, text: &str) -> (i32, usize) {
        let (x, y) = h.find(text).unwrap_or_else(|| panic!("`{text}` is not on screen:\n{}", h.screen()));
        (y, x.max(0) as usize)
    }

    /// The column `text` starts in on the row `row` of the screen, in cells.
    fn column<A: App>(h: &Harness<A>, row: i32, text: &str) -> usize {
        let line = h.screen().lines().nth(usize::try_from(row).unwrap_or(0)).unwrap_or_default().to_owned();
        line.find(text)
            .map(|at| line[..at].chars().count())
            .unwrap_or_else(|| panic!("`{text}` is not on row {row}:\n{line}"))
    }

    #[test]
    fn the_sign_stands_in_every_glyph_mode_and_the_sentence_takes_the_tone_colour() {
        for mode in [GlyphMode::Unicode, GlyphMode::Ascii, GlyphMode::Nerd] {
            let mut h = Harness::new(Kinds, 44, 4);
            h.set_glyph_mode(mode);
            let theme = h.env().theme();
            for (row, (kind, sentence)) in REPORTED.into_iter().enumerate() {
                let y = u16::try_from(row).unwrap_or(0);
                let sign = h.screen().lines().nth(row).unwrap_or_default().chars().next().unwrap_or(' ');
                assert_eq!(sign.to_string(), h.env().icons().glyph(kind.icon()).to_string(), "{mode:?} row {row}");
                assert!(!sign.is_whitespace(), "{mode:?} row {row}: the sign is never a blank cell");
                let tone = theme.color(kind.name()).expect("every theme has the four status colours");
                assert_eq!(h.fg(0, y), Some(tone), "{mode:?} row {row}: the sign carries the tone");
                let (at, column) = at(&h, sentence);
                assert_eq!(at, i32::try_from(row).unwrap_or(0), "{mode:?} row {row}: the sentence is on its own row");
                assert!(column > 1, "{mode:?} row {row}: a space stands between the sign and the sentence");
                assert_eq!(
                    h.fg(u16::try_from(column).unwrap_or(0), y),
                    Some(tone),
                    "{mode:?} row {row}: and so does the sentence"
                );
            }
        }
    }

    #[test]
    fn the_button_at_the_end_of_a_line_is_a_button() {
        let mut h = Harness::new(Reported { saved: 0 }, 50, 2);
        let (sentence, button) = (h.find("The build of"), h.find("Retry"));
        let (sentence, button) = (sentence.expect("the sentence"), button.expect("the button"));
        assert_eq!(sentence.1, button.1, "one row while there is room: {}", h.screen());
        assert!(button.0 > sentence.0, "the button stands after the sentence: {}", h.screen());
        h.click(sentence.0, sentence.1).click_text("Retry");
        assert_eq!(h.app().saved, 1, "the sentence is not a target, the button is");
        h.press("tab").press("enter");
        assert_eq!(h.app().saved, 2, "and it takes focus like any other button");
    }

    #[test]
    fn a_narrow_area_wraps_the_sentence_under_itself_and_moves_the_button_below() {
        let h = Harness::new(Reported { saved: 0 }, 20, 7);
        let screen = h.screen();
        for word in "The build of web-frontend failed".split(' ') {
            assert!(screen.contains(word), "`{word}` is nowhere: {screen}");
        }
        let (first, sentence_column) = at(&h, "The build of");
        let (rest, wrapped) = at(&h, "web-frontend");
        let (button, _) = at(&h, "Retry");
        assert!(rest > first, "the sentence wraps under itself: {screen}");
        assert_eq!(sentence_column, wrapped, "and keeps its column: {screen}");
        assert!(
            column(&h, first, &h.env().icons().glyph(ToastKind::Danger.icon())) < sentence_column,
            "never under the sign: {screen}"
        );
        assert!(button > rest, "the button moves below: {screen}");
    }
}
