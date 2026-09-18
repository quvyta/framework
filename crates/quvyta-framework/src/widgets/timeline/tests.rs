use super::*;
use crate::color::{ColorDepth, Rgb};
use crate::event::MouseKind;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::{Length, View};

/// Every built-in theme, so a timeline is checked in all of them.
const THEMES: [&str; 4] = ["monochrome", "nordic", "amber", "iris"];

fn t(hour: u8, minute: u8) -> TimeOfDay {
    TimeOfDay::new(hour, minute, 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Msg {
    Select(usize),
    Zoom(TimeOfDay, TimeOfDay),
}

/// A timeline built fresh every frame from what the application keeps, the way a real
/// application builds it.
#[derive(Default)]
struct Demo {
    blocks: Vec<TimeBlock>,
    day_start: Option<TimeOfDay>,
    range: Option<(TimeOfDay, TimeOfDay)>,
    axis: bool,
    readout: bool,
    selectable: bool,
    zoomable: bool,
    disabled: bool,
    height: Option<u16>,
    selected: Option<usize>,
    picks: Vec<usize>,
    zooms: Vec<(TimeOfDay, TimeOfDay)>,
}

impl Demo {
    fn new(blocks: impl IntoIterator<Item = TimeBlock>) -> Self {
        Self { blocks: blocks.into_iter().collect(), ..Self::default() }
    }

    fn interactive(mut self) -> Self {
        self.selectable = true;
        self
    }
}

impl App for Demo {
    type Msg = Msg;

    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Select(index) => {
                self.selected = Some(index);
                self.picks.push(index);
            }
            Msg::Zoom(from, to) => {
                self.range = Some((from, to));
                self.zooms.push((from, to));
            }
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut timeline = Timeline::new(self.blocks.clone()).selected(self.selected).disabled(self.disabled);
        if let Some(start) = self.day_start {
            timeline = timeline.day_starts_at(start);
        }
        if let Some((from, to)) = self.range {
            timeline = timeline.range(from, to);
        }
        if self.axis {
            timeline = timeline.axis();
        }
        if self.readout {
            timeline = timeline.readout();
        }
        if self.selectable {
            timeline = timeline.on_select(Msg::Select);
        }
        if self.zoomable {
            timeline = timeline.on_zoom(Msg::Zoom);
        }
        let height = self.height.map_or(Length::Auto, Length::Cells);
        ui.add(timeline).width(Length::Fill(1)).height(height).id("timeline");
    }
}

/// A working morning: work, a break, more work, lunch.
fn morning() -> Vec<TimeBlock> {
    vec![
        TimeBlock::new("Work", t(9, 0), t(11, 10)),
        TimeBlock::new("Mail", t(11, 30), t(12, 0)),
        TimeBlock::new("Lunch", t(12, 0), t(13, 0)).tone(2),
    ]
}

fn track(h: &Harness<Demo>) -> Rgb {
    h.env().theme().color("raised").expect("token")
}

fn accent(h: &Harness<Demo>) -> Rgb {
    h.env().theme().color("accent").expect("token")
}

fn line(h: &Harness<Demo>, row: usize) -> String {
    h.screen().lines().nth(row).unwrap_or_default().to_owned()
}

// ---- Placement -------------------------------------------------------------------------------

#[test]
fn a_block_covers_the_cells_its_time_falls_in() {
    // 96 cells for a day: a quarter of an hour a cell. 09:00 is cell 36, 11:10 falls in cell 44.
    let h = Harness::new(Demo::new([TimeBlock::new("Work", t(9, 0), t(11, 10))]), 96, 1);
    assert_eq!(h.bg(35, 0), Some(track(&h)), "before the block is the empty day");
    for x in 36..44 {
        assert_eq!(h.bg(x, 0), Some(accent(&h)), "cell {x} is inside the block");
    }
    assert_eq!(h.bg(44, 0), Some(track(&h)), "the cell 11:10 falls in belongs to the gap after it");
    assert_eq!(h.bg(0, 0), Some(track(&h)));
    assert_eq!(h.bg(95, 0), Some(track(&h)));
    assert_eq!(line(&h, 0), format!("{}Work", " ".repeat(37)), "the name sits inside, after a cell of air");
}

#[test]
fn an_empty_day_is_the_track_and_says_so() {
    let mut demo = Demo::new([]);
    demo.readout = true;
    demo.axis = true;
    let h = Harness::new(demo, 48, 3);
    assert!((0..48).all(|x| h.bg(x, 0) == Some(track(&h))), "the whole day is the empty track");
    assert_eq!(line(&h, 0), "", "and draws no characters");
    assert!(line(&h, 1).starts_with("00:00"), "the axis still gives the day its hours: {}", line(&h, 1));
    assert_eq!(line(&h, 2), "Nothing on this day");
    assert_eq!(h.fg(0, 2), h.env().theme().color("muted"), "in the faint tone");
}

#[test]
fn a_block_shorter_than_a_cell_still_takes_one() {
    // 24 cells for a day: an hour a cell. Two minutes would be a thirtieth of a cell.
    let h = Harness::new(Demo::new([TimeBlock::new("Call", t(9, 0), t(9, 2))]), 24, 1);
    assert_eq!(h.bg(9, 0), Some(accent(&h)), "the short block is seen");
    assert_eq!(h.bg(8, 0), Some(track(&h)));
    assert_eq!(h.bg(10, 0), Some(track(&h)));
    assert_eq!(line(&h, 0), "", "a one-cell block has no room for its name");
    let moment = Harness::new(Demo::new([TimeBlock::new("Ping", t(15, 0), t(15, 0))]), 24, 1);
    assert_eq!(moment.bg(15, 0), Some(accent(&moment)), "a moment is a cell too");
}

#[test]
fn a_block_takes_its_pinned_series_tone() {
    let h = Harness::new(Demo::new(morning()), 48, 1);
    assert_eq!(h.bg(24, 0), Some(h.env().theme().series_color(2)), "lunch keeps tone 2");
    assert_eq!(h.bg(18, 0), Some(accent(&h)), "a block without a tone takes the accent");
}

#[test]
fn touching_blocks_of_one_tone_are_parted_by_a_seam_of_tone() {
    let blocks = [TimeBlock::new("A", t(9, 0), t(10, 0)), TimeBlock::new("B", t(10, 0), t(11, 0))];
    let h = Harness::new(Demo::new(blocks), 48, 1);
    let accent = accent(&h);
    assert_eq!(h.bg(18, 0), Some(accent));
    assert_eq!(h.bg(19, 0), Some(accent), "the first block ends in its own tone");
    let seam = h.bg(20, 0).expect("drawn");
    assert_ne!(seam, accent, "the second block starts a step quieter");
    assert_ne!(seam, track(&h), "and the seam is not a gap");
    assert_eq!(h.bg(21, 0), Some(accent), "then goes on in its own tone");
    assert!(!h.screen().contains('|'), "no line is drawn between them");

    let apart = Harness::new(Demo::new(morning()), 48, 1);
    assert_eq!(apart.bg(24, 0), Some(apart.env().theme().series_color(2)), "different tones need no seam");
}

// ---- Overlap, midnight, range ----------------------------------------------------------------

#[test]
fn overlapping_blocks_stand_in_lanes() {
    let blocks = [
        TimeBlock::new("Work", t(9, 0), t(12, 0)),
        TimeBlock::new("Call", t(10, 0), t(11, 0)).tone(2),
        TimeBlock::new("Lunch", t(12, 0), t(13, 0)),
    ];
    let h = Harness::new(Demo::new(blocks.clone()), 48, 4);
    let series = h.env().theme().series_color(2);
    assert_eq!(h.bg(20, 0), Some(accent(&h)), "work keeps the top lane");
    assert_eq!(h.bg(20, 1), Some(series), "the call runs at the same time, so it takes the next lane");
    assert_eq!(h.bg(18, 1), Some(track(&h)), "the second lane is empty outside the call");
    assert_eq!(h.bg(25, 0), Some(accent(&h)), "lunch starts as work ends, so it shares the top lane");
    assert_eq!(h.bg(0, 2), h.env().theme().color("canvas"), "two lanes measure two rows");

    let mut short = Demo::new(blocks);
    short.height = Some(1);
    let squeezed = Harness::new(short, 48, 1);
    assert_eq!(squeezed.bg(20, 0), Some(series), "one row draws the later block over the earlier");
    let mut short = Demo::new(squeezed.app().blocks.clone());
    short.height = Some(1);
    short.selected = Some(0);
    let picked = Harness::new(short, 48, 1);
    assert_ne!(picked.bg(20, 0), Some(series), "the selected block is always drawn on top");
}

#[test]
fn a_block_across_midnight_is_cut_at_the_end_of_a_midnight_day() {
    let night = [TimeBlock::new("Sleep", t(23, 0), t(1, 30))];
    let h = Harness::new(Demo::new(night.clone()), 24, 1);
    assert_eq!(h.bg(23, 0), Some(accent(&h)), "the part before midnight is drawn to the edge");
    assert_eq!(h.bg(0, 0), Some(track(&h)), "the part after midnight belongs to the next day");
    assert_eq!(h.bg(1, 0), Some(track(&h)));

    let mut shift = Demo::new(night);
    shift.day_start = Some(t(18, 0));
    let h = Harness::new(shift, 24, 1);
    assert_eq!(h.bg(4, 0), Some(track(&h)));
    assert_eq!(h.bg(5, 0), Some(accent(&h)), "a day from 18:00 holds the whole night: 23:00 is its fifth hour");
    assert_eq!(h.bg(6, 0), Some(accent(&h)));
    assert_eq!(h.bg(7, 0), Some(track(&h)), "and 01:30 falls in its eighth");
}

#[test]
fn a_range_across_midnight_zooms_into_a_night() {
    let mut demo = Demo::new([TimeBlock::new("Sleep", t(23, 0), t(1, 30))]);
    demo.day_start = Some(t(18, 0));
    demo.range = Some((t(22, 0), t(6, 0)));
    demo.axis = true;
    let h = Harness::new(demo, 48, 2);
    // Eight hours over 48 cells: ten minutes a cell.
    assert_eq!(h.bg(5, 0), Some(track(&h)));
    assert_eq!(h.bg(6, 0), Some(accent(&h)), "23:00 is an hour into the range");
    assert_eq!(h.bg(20, 0), Some(accent(&h)));
    assert_eq!(h.bg(21, 0), Some(track(&h)), "01:30 is three and a half hours in");
    assert_eq!(
        line(&h, 1).split_whitespace().collect::<Vec<_>>(),
        ["22:00", "23:00", "00:00", "01:00", "02:00", "03:00", "04:00", "05:00"]
    );
    assert_eq!(line(&h, 1).find("23:00"), Some(6), "the axis stands over the cells of its hours");
}

#[test]
fn a_range_shows_a_stretch_across_the_whole_width() {
    let mut demo = Demo::new([TimeBlock::new("Work", t(9, 0), t(11, 10))]);
    demo.range = Some((t(9, 0), t(12, 0)));
    demo.axis = true;
    let h = Harness::new(demo, 36, 2);
    // Three hours over 36 cells: five minutes a cell, so 11:10 is cell 26.
    assert_eq!(h.bg(0, 0), Some(accent(&h)));
    assert_eq!(h.bg(25, 0), Some(accent(&h)));
    assert_eq!(h.bg(26, 0), Some(track(&h)));
    assert_eq!(line(&h, 1).split_whitespace().next(), Some("09:00"));
    let outside = {
        let mut demo = Demo::new([TimeBlock::new("Late", t(20, 0), t(21, 0))]);
        demo.range = Some((t(9, 0), t(12, 0)));
        Harness::new(demo, 36, 1)
    };
    assert!((0..36).all(|x| outside.bg(x, 0) == Some(track(&outside))), "a block outside the range is not drawn");
}

// ---- Reading and selecting -------------------------------------------------------------------

#[test]
fn the_pointer_and_the_keyboard_read_the_same_block() {
    let mut demo = Demo::new(morning()).interactive();
    demo.readout = true;
    let mut h = Harness::new(demo, 48, 2);
    assert_eq!(line(&h, 1), "", "nothing is read yet");
    h.hover(19, 0);
    let by_pointer = line(&h, 1);
    assert_eq!(by_pointer, "Work  09:00–11:10  2 h 10 min");
    assert_eq!(h.app().picks, Vec::<usize>::new(), "hovering reads, it does not select");
    assert_eq!(h.fg(0, 1), h.env().theme().color("text"), "the name is in the text colour");
    assert_eq!(h.fg(6, 1), h.env().theme().color("dim"), "the times are quieter");

    h.hover(0, 1);
    assert_eq!(line(&h, 1), "", "the readout follows the pointer off the blocks");
    h.press("tab");
    h.press("right");
    assert_eq!(h.app().picks, vec![0], "the first key selects the first block");
    assert_eq!(line(&h, 1), by_pointer, "the keyboard reads the same words as the pointer");
    h.press("right");
    assert_eq!(line(&h, 1), "Mail  11:30–12:00  30 min");
    h.press("end");
    assert_eq!(line(&h, 1), "Lunch  12:00–13:00  1 h");
    h.press("home");
    h.press("l");
    assert_eq!(h.app().picks, vec![0, 1, 2, 0, 1], "h and l walk the blocks too");
    h.click(24, 0);
    assert_eq!(h.app().selected, Some(2), "a click selects the block under it");
    h.click(2, 0);
    assert_eq!(h.app().selected, Some(2), "a click on a gap selects nothing");
}

#[test]
fn a_narrow_readout_drops_the_length_then_the_times_then_cuts_the_name() {
    for (width, expected) in
        [(40, "Work  09:00–11:10  2 h 10 min"), (24, "Work  09:00–11:10"), (10, "Work"), (3, "Wo…")]
    {
        let mut demo = Demo::new([TimeBlock::new("Work", t(9, 0), t(11, 10))]).interactive();
        demo.readout = true;
        demo.selected = Some(0);
        let h = Harness::new(demo, width, 2);
        assert_eq!(line(&h, 1), expected, "{width} cells");
    }
}

#[test]
fn the_readout_writes_times_across_midnight_and_short_lengths() {
    let mut demo = Demo::new([TimeBlock::new("Sleep", t(23, 0), t(6, 45)), TimeBlock::new("Ping", t(9, 0), t(9, 0))]);
    demo.readout = true;
    demo.selected = Some(0);
    let mut h = Harness::new(demo, 60, 2);
    assert_eq!(line(&h, 1), "Sleep  23:00–06:45  7 h 45 min", "the length counts on past midnight");
    h.set_locale("tr");
    assert_eq!(line(&h, 1), "Sleep  23:00–06:45  7 sa 45 dk", "the length is written in the active language");
    let mut moment = Demo::new([TimeBlock::new("Ping", t(9, 0), t(9, 0))]);
    moment.readout = true;
    moment.selected = Some(0);
    assert_eq!(line(&Harness::new(moment, 60, 2), 1), "Ping  09:00–09:00  0 s");
}

#[test]
fn nothing_moves_when_a_block_is_hovered_or_selected() {
    let mut demo = Demo::new(morning()).interactive();
    demo.axis = true;
    let mut h = Harness::new(demo, 48, 2);
    let resting = h.screen();
    let labels = |h: &Harness<Demo>| (h.find("Work"), h.find("Lunch"));
    let places = labels(&h);
    h.hover(19, 0);
    assert_eq!(h.screen(), resting, "hovering changes tones, never characters");
    assert_ne!(h.bg(19, 0), Some(accent(&h)), "the hovered block steps towards the text colour");
    h.press("tab").press("right");
    assert_eq!(h.screen(), resting, "selecting changes tones, never characters");
    assert_eq!(labels(&h), places, "no name slides");
    assert!(!h.screen().contains('▌'), "a strip has no pillar: it is not a list");
}

#[test]
fn the_tone_ladder_climbs_from_rest_to_hover_to_selected_to_focus() {
    for theme in THEMES {
        let mut demo = Demo::new([TimeBlock::new("A", t(0, 0), t(6, 0)), TimeBlock::new("B", t(12, 0), t(18, 0))]);
        demo.selectable = true;
        let mut h = Harness::new(demo, 24, 1);
        h.set_theme(theme);
        let rest = h.bg(2, 0).expect("drawn");
        let track = h.bg(8, 0).expect("drawn");
        assert!(rest.perceptual_distance(track) >= 0.03, "{theme}: a block stands off the track");
        h.hover(2, 0);
        let hover = h.bg(2, 0).expect("drawn");
        h.hover(8, 0);
        h.click(14, 0);
        let selected = h.bg(14, 0).expect("drawn");
        h.press("left");
        h.press("right");
        let focus = h.bg(14, 0).expect("drawn");
        let from_rest = |c: Rgb| c.perceptual_distance(rest);
        assert!(from_rest(hover) >= 0.03, "{theme}: hover shows");
        assert!(from_rest(selected) > from_rest(hover), "{theme}: selected steps further than hover");
        assert!(from_rest(focus) > from_rest(selected), "{theme}: the keyboard steps further still");
    }
}

#[test]
fn a_timeline_without_a_message_is_a_picture() {
    let mut h = Harness::new(Demo::new(morning()), 48, 1);
    let resting = h.bg(19, 0);
    h.hover(19, 0);
    assert_eq!(h.bg(19, 0), resting, "a picture does not answer the pointer");
    h.press("tab");
    assert!(!h.is_focused("timeline"), "and takes no focus");
    h.click(19, 0);
    assert_eq!(h.app().picks, Vec::<usize>::new());
}

#[test]
fn a_disabled_timeline_is_quiet_and_answers_nothing() {
    let mut demo = Demo::new(morning()).interactive();
    demo.disabled = true;
    demo.readout = true;
    demo.axis = true;
    demo.zoomable = true;
    demo.selected = Some(0);
    let mut h = Harness::new(demo, 48, 3);
    let muted = h.env().theme().color("muted");
    assert_eq!(h.bg(19, 0), muted, "blocks go muted");
    assert_eq!(h.bg(25, 0), muted, "a series tone goes muted too");
    assert_eq!(h.fg(0, 1), muted, "the hours go faint");
    assert_eq!(h.fg(0, 2), muted, "and so does the readout");
    h.press("tab");
    assert!(!h.is_focused("timeline"));
    h.click(24, 0);
    h.press("+");
    assert_eq!(h.app().picks, Vec::<usize>::new());
    assert_eq!(h.app().zooms, Vec::new());
}

// ---- Zoom ------------------------------------------------------------------------------------

fn zoomable() -> Demo {
    let mut demo = Demo::new(morning()).interactive();
    demo.zoomable = true;
    demo.axis = true;
    demo.selected = Some(0);
    demo
}

#[test]
fn the_keyboard_zooms_around_the_selected_block_and_back_out() {
    let mut h = Harness::new(zoomable(), 48, 2);
    h.press("tab");
    h.press("+");
    // Work's middle is 10:05, 42% into the day; half a day keeps it 42% in: from 05:02 to 17:02.
    assert_eq!(h.app().zooms, vec![(t(5, 2), t(17, 2))]);
    h.press("+").press("+").press("+");
    let (from, to) = *h.app().zooms.last().expect("zoomed");
    assert_eq!(super::super::axis::span_between(from, to), 3_600, "the closest zoom is one hour");
    assert!(from <= t(10, 5) && t(10, 5) <= to, "and it keeps the block in view: {from}–{to}");
    let zooms = h.app().zooms.len();
    h.press("+");
    assert_eq!(h.app().zooms.len(), zooms, "past the last step nothing is asked");
    h.press("-");
    let (from, to) = *h.app().zooms.last().expect("zoomed");
    assert_eq!(super::super::axis::span_between(from, to), 10_800, "minus steps back out");
    h.press("0");
    assert_eq!(h.app().zooms.last(), Some(&(t(0, 0), t(0, 0))), "0 is the whole day again");
    h.press("-");
    assert_eq!(h.app().zooms.last(), Some(&(t(0, 0), t(0, 0))), "a whole day zooms out no further");
}

#[test]
fn zooming_in_draws_the_same_block_wider() {
    let mut h = Harness::new(zoomable(), 48, 2);
    let wide = |h: &Harness<Demo>| (0..48).filter(|x| h.bg(*x, 0) != Some(track(h))).count();
    let before = wide(&h);
    h.press("tab").press("+").press("+");
    assert!(wide(&h) > before, "a closer range gives the blocks more cells: {before} then {}", wide(&h));
    assert!(line(&h, 1).contains(":00"), "and the axis follows the range: {}", line(&h, 1));
}

#[test]
fn the_wheel_zooms_around_the_pointer_only_once_the_timeline_holds_the_focus() {
    let mut h = Harness::new(zoomable(), 48, 2);
    h.mouse(MouseKind::ScrollUp, 36, 0);
    assert_eq!(h.app().zooms, Vec::new(), "a wheel passing over leaves the range alone");
    h.click(19, 0);
    assert!(h.is_focused("timeline"), "a click gives the timeline the focus");
    h.mouse(MouseKind::ScrollUp, 36, 0);
    // Cell 36 is 18:00, three quarters into the day; half a day keeps it there: 09:00 to 21:00.
    assert_eq!(h.app().zooms, vec![(t(9, 0), t(21, 0))]);
    h.mouse(MouseKind::ScrollDown, 36, 0);
    assert_eq!(h.app().zooms.last(), Some(&(t(0, 0), t(0, 0))), "the wheel steps back out");
}

#[test]
fn selecting_a_block_outside_a_zoomed_range_moves_the_range_to_it() {
    let mut demo = zoomable();
    demo.range = Some((t(9, 0), t(10, 0)));
    let mut h = Harness::new(demo, 48, 2);
    h.press("tab");
    h.press("end");
    assert_eq!(h.app().selected, Some(2), "lunch is selected");
    let (from, to) = *h.app().zooms.last().expect("the range moved");
    assert_eq!((from, to), (t(12, 0), t(13, 0)), "an hour around lunch");
    assert!(h.bg(24, 0) != Some(track(&h)), "and lunch is on screen");
}

// ---- Every environment -----------------------------------------------------------------------

#[test]
fn every_glyph_mode_draws_the_same_strip() {
    let mut screens = Vec::new();
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        let mut demo = zoomable();
        demo.readout = true;
        let mut h = Harness::new(demo, 48, 3);
        h.set_glyph_mode(mode);
        assert!(h.bg(19, 0).is_some(), "{mode:?}");
        screens.push((h.screen(), h.bg(19, 0), h.bg(24, 0)));
    }
    assert!(screens.windows(2).all(|pair| pair[0] == pair[1]), "the strip is made of colour: {screens:?}");
}

#[test]
fn a_terminal_with_few_colours_still_tells_blocks_track_and_steps_apart() {
    for depth in [ColorDepth::Ansi256, ColorDepth::Ansi16] {
        for theme in THEMES {
            let blocks = [TimeBlock::new("A", t(0, 0), t(6, 0)), TimeBlock::new("B", t(6, 0), t(12, 0))];
            let mut h = Harness::new(Demo::new(blocks).interactive(), 24, 1);
            h.set_theme(theme);
            h.set_depth(depth);
            let cell = |h: &Harness<Demo>, x: u16| h.buffer()[(x, 0)].bg;
            let (rest, seam, gap) = (cell(&h, 2), cell(&h, 6), cell(&h, 14));
            assert_ne!(rest, gap, "{theme} {depth:?}: a block is not the track");
            assert_ne!(seam, rest, "{theme} {depth:?}: the seam shows");
            h.hover(2, 0);
            assert_ne!(cell(&h, 2), rest, "{theme} {depth:?}: hovering shows");
            h.hover(20, 0);
            h.click(8, 0);
            assert_ne!(cell(&h, 8), rest, "{theme} {depth:?}: selecting shows");
        }
    }
}

#[test]
fn tiny_areas_draw_what_they_can_without_panicking() {
    for (width, height) in [(1, 1), (2, 1), (3, 2), (5, 3), (9, 1), (12, 4), (80, 1)] {
        let mut demo = zoomable();
        demo.readout = true;
        demo.blocks.push(TimeBlock::new("Overlap", t(9, 30), t(10, 0)));
        demo.blocks.push(TimeBlock::new("Night", t(23, 0), t(2, 0)));
        let mut h = Harness::new(demo, width, height);
        h.press("tab").press("right").press("+").press("end").press("-");
        h.hover(0, 0);
        h.click(i32::from(width) - 1, 0);
        h.mouse(MouseKind::ScrollUp, 0, 0);
        assert!(h.screen().lines().count() <= usize::from(height), "{width}×{height}");
        assert!(h.bg(0, 0).is_some(), "{width}×{height}: the strip keeps its row");
    }
}

#[test]
fn a_short_area_gives_up_the_axis_then_the_readout() {
    let mut demo = zoomable();
    demo.readout = true;
    demo.height = Some(2);
    let h = Harness::new(demo, 48, 2);
    assert!(line(&h, 1).starts_with("Work"), "the readout outlives the axis: {}", line(&h, 1));
    let mut demo = zoomable();
    demo.readout = true;
    demo.height = Some(1);
    let h = Harness::new(demo, 96, 1);
    assert!(h.screen().contains("Work"), "one row is the strip itself:\n{}", h.screen());
    assert_eq!(h.screen().lines().count(), 1);
}

#[test]
fn every_theme_draws_blocks_and_their_names() {
    for theme in THEMES {
        let mut h = Harness::new(Demo::new(morning()), 96, 1);
        h.set_theme(theme);
        let block = h.bg(37, 0).expect("drawn");
        let name = h.fg(37, 0).expect("the name is drawn in the block");
        assert_eq!(h.screen().find("Work"), Some(37), "{theme}");
        assert!(block.contrast_ratio(name) >= 3.0, "{theme}: the name reads on its block");
        assert_eq!(block, h.env().theme().color("accent").expect("token"), "{theme}");
    }
}

#[test]
fn the_strip_measures_its_lanes_and_rows() {
    let mut demo = Demo::new(morning());
    demo.axis = true;
    demo.readout = true;
    let h = Harness::new(demo, 48, 10);
    assert!(h.bg(0, 0).is_some() && h.bg(0, 0) == Some(track(&h)), "the strip is the first row");
    assert!(line(&h, 1).starts_with("00:00"), "then the axis");
    assert_eq!(h.bg(0, 3), h.env().theme().color("canvas"), "and nothing after the readout");
}

// ---- Open ends and faint blocks --------------------------------------------------------------

/// The background of every cell in the first row, as the terminal gets it.
fn row_bg(h: &Harness<Demo>, width: u16) -> Vec<ratatui_core::style::Color> {
    (0..width).map(|x| h.buffer()[(x, 0)].bg).collect()
}

/// The columns where two rows differ.
fn changed(a: &[ratatui_core::style::Color], b: &[ratatui_core::style::Color]) -> Vec<usize> {
    (0..a.len()).filter(|x| a[*x] != b[*x]).collect()
}

#[test]
fn a_block_without_the_new_options_draws_as_before_and_each_option_touches_only_its_cells() {
    // 24 cells for a day: an hour a cell. The evening runs 18:00 to midnight, cells 18 to 23.
    let evening = || TimeBlock::new("Evening", t(18, 0), t(0, 0));
    let plain = Harness::new(Demo::new([evening()]), 24, 1);
    let toned = Harness::new(Demo::new([evening().tone(2)]), 24, 1);
    assert_ne!(plain.buffer(), toned.buffer(), "a tone is a colour of its own, so the buffers do compare");
    let again = Harness::new(Demo::new([evening()]), 24, 1);
    assert_eq!(plain.buffer(), again.buffer());
    let before = row_bg(&plain, 24);
    assert_eq!(changed(&before, &row_bg(&toned, 24)), (18..24).collect::<Vec<_>>());

    let open = Harness::new(Demo::new([evening().open_end()]), 24, 1);
    assert_eq!(changed(&before, &row_bg(&open, 24)), vec![22, 23], "an open end changes its last two cells only");
    assert_eq!(plain.screen(), open.screen(), "and no characters");
    let open = Harness::new(Demo::new([evening().open_start()]), 24, 1);
    assert_eq!(changed(&before, &row_bg(&open, 24)), vec![18, 19], "an open start changes its first two cells");
    let faint = Harness::new(Demo::new([evening().faint()]), 24, 1);
    assert_eq!(changed(&before, &row_bg(&faint, 24)), (18..24).collect::<Vec<_>>(), "faint changes its own cells");
}

#[test]
fn an_open_end_fades_towards_the_track_and_never_becomes_it() {
    for theme in THEMES {
        let mut h = Harness::new(Demo::new([TimeBlock::new("Evening", t(18, 0), t(0, 0)).open_end()]), 24, 1);
        h.set_theme(theme);
        let (block, track) = (accent(&h), track(&h));
        let (inner, outer) = (h.bg(22, 0).expect("drawn"), h.bg(23, 0).expect("drawn"));
        assert_eq!(h.bg(21, 0), Some(block), "{theme}: the block keeps its tone up to the fade");
        assert!(inner.perceptual_distance(block) >= 0.03, "{theme}: the fade starts a visible step in");
        assert!(outer.perceptual_distance(track) >= 0.03, "{theme}: the last cell is still the block, not the gap");
        assert!(
            outer.perceptual_distance(track) < inner.perceptual_distance(track),
            "{theme}: each cell is closer to the track than the one before it"
        );
        assert!(inner.perceptual_distance(track) < block.perceptual_distance(track), "{theme}");
    }
}

#[test]
fn an_open_start_fades_in_from_the_track() {
    let h = Harness::new(Demo::new([TimeBlock::new("Night", t(0, 0), t(4, 0)).open_start()]), 24, 1);
    let track = track(&h);
    let (outer, inner) = (h.bg(0, 0).expect("drawn"), h.bg(1, 0).expect("drawn"));
    assert!(outer.perceptual_distance(track) < inner.perceptual_distance(track), "fades in from the left");
    assert_ne!(outer, track);
    assert_eq!(h.bg(2, 0), Some(accent(&h)));
    assert_eq!(h.bg(3, 0), Some(accent(&h)), "the far end is closed");
}

#[test]
fn a_short_open_block_keeps_a_cell_of_its_own_tone() {
    // One cell: no room for a fade, so the block keeps its tone and the readout says it.
    let one = |block: TimeBlock| {
        let mut demo = Demo::new([block]);
        demo.readout = true;
        demo.selected = Some(0);
        Harness::new(demo, 48, 2)
    };
    let (open, closed) =
        (one(TimeBlock::new("Now", t(9, 0), t(9, 20)).open_end()), one(TimeBlock::new("Now", t(9, 0), t(9, 20))));
    // 48 cells: half an hour a cell, so twenty minutes from 09:00 is cell 18 alone.
    assert_eq!(row_bg(&open, 48), row_bg(&closed, 48), "a one-cell block keeps its tone, selected or not");
    assert_eq!(line(&open, 1), "Now  09:00–09:20  running  20 min");

    // Two cells open at one end: one fading cell, one of the block's own tone.
    let two = Harness::new(Demo::new([TimeBlock::new("Two", t(9, 0), t(11, 0)).open_end()]), 24, 1);
    assert_eq!(two.bg(9, 0), Some(accent(&two)));
    assert_ne!(two.bg(10, 0), Some(accent(&two)));
    assert_ne!(two.bg(10, 0), Some(track(&two)));

    // Open at both ends: the cells are shared, and at least one in the middle keeps the tone.
    let both = |hours: u8| {
        let block = TimeBlock::new("Both", t(9, 0), t(9 + hours, 0)).open_start().open_end();
        Harness::new(Demo::new([block]), 24, 1)
    };
    let two = both(2);
    assert_eq!((two.bg(9, 0), two.bg(10, 0)), (Some(accent(&two)), Some(accent(&two))), "two cells: no fade");
    let three = both(3);
    assert_ne!(three.bg(9, 0), Some(accent(&three)));
    assert_eq!(three.bg(10, 0), Some(accent(&three)), "three cells: a fading cell at each end");
    assert_ne!(three.bg(11, 0), Some(accent(&three)));
    assert_eq!(three.bg(9, 0), three.bg(11, 0), "both ends fade alike");
    let six = both(6);
    let row = row_bg(&six, 24);
    assert_eq!(row[9], row[14], "six cells: two at each end, mirrored");
    assert_eq!(row[10], row[13]);
    assert_ne!(row[9], row[10]);
    assert_eq!((six.bg(11, 0), six.bg(12, 0)), (Some(accent(&six)), Some(accent(&six))));
}

#[test]
fn only_the_real_open_edge_fades_not_the_edge_of_a_zoomed_range() {
    let block = || TimeBlock::new("Work", t(9, 0), t(12, 0)).open_end();
    // 09:00 to 11:00 over 24 cells: five minutes a cell. The block runs on past the right edge.
    let mut cut = Demo::new([block()]);
    cut.range = Some((t(9, 0), t(11, 0)));
    let h = Harness::new(cut, 24, 1);
    assert!((0..24).all(|x| h.bg(x, 0) == Some(accent(&h))), "the range's edge is not the block's end");
    // 09:00 to 13:00: ten minutes a cell, so the real end, 12:00, is cell 18.
    let mut whole = Demo::new([block()]);
    whole.range = Some((t(9, 0), t(13, 0)));
    let h = Harness::new(whole, 24, 1);
    assert_eq!(h.bg(15, 0), Some(accent(&h)));
    assert_ne!(h.bg(16, 0), Some(accent(&h)), "the real end fades where it falls");
    assert_ne!(h.bg(17, 0), Some(accent(&h)));
    assert_eq!(h.bg(18, 0), Some(track(&h)));
    // An open start before the range is not drawn either.
    let mut late = Demo::new([TimeBlock::new("Night", t(0, 0), t(4, 0)).open_start()]);
    late.range = Some((t(2, 0), t(6, 0)));
    let h = Harness::new(late, 24, 1);
    assert_eq!(h.bg(0, 0), Some(accent(&h)));
}

#[test]
fn a_faint_block_sits_between_its_tone_and_the_track() {
    for theme in THEMES {
        let blocks = [TimeBlock::new("Before", t(0, 0), t(6, 0)).faint(), TimeBlock::new("Now", t(12, 0), t(18, 0))];
        let mut h = Harness::new(Demo::new(blocks), 24, 1);
        h.set_theme(theme);
        let (faint, full, track) = (h.bg(2, 0).expect("drawn"), h.bg(14, 0).expect("drawn"), track(&h));
        assert_eq!(full, accent(&h), "{theme}");
        assert!(faint.perceptual_distance(track) >= 0.03, "{theme}: a faint block is not the gap");
        assert!(faint.perceptual_distance(full) >= 0.03, "{theme}: nor a full block");
        assert!(faint.perceptual_distance(track) < full.perceptual_distance(track), "{theme}: it is quieter");
    }
}

#[test]
fn a_faint_block_still_answers_hover_and_selection() {
    for theme in THEMES {
        let blocks = [TimeBlock::new("Before", t(0, 0), t(6, 0)).faint(), TimeBlock::new("B", t(12, 0), t(18, 0))];
        let mut h = Harness::new(Demo::new(blocks).interactive(), 24, 1);
        h.set_theme(theme);
        let (rest, track) = (h.bg(2, 0).expect("drawn"), track(&h));
        h.hover(2, 0);
        let hover = h.bg(2, 0).expect("drawn");
        h.hover(20, 0);
        h.click(2, 0);
        let selected = h.bg(2, 0).expect("drawn");
        let from_rest = |c: Rgb| c.perceptual_distance(rest);
        assert!(from_rest(hover) >= 0.03, "{theme}: hover shows on a faint block");
        assert!(from_rest(selected) > from_rest(hover), "{theme}: selected steps further");
        assert!(hover.perceptual_distance(track) >= 0.03 && selected.perceptual_distance(track) >= 0.03, "{theme}");
    }
}

#[test]
fn a_terminal_with_few_colours_still_shows_faint_blocks_and_open_edges() {
    for depth in [ColorDepth::Ansi256, ColorDepth::Ansi16] {
        for theme in THEMES {
            let blocks = [
                TimeBlock::new("Before", t(0, 0), t(6, 0)).faint().open_start(),
                TimeBlock::new("Now", t(12, 0), t(18, 0)).open_end(),
            ];
            let mut h = Harness::new(Demo::new(blocks).interactive(), 24, 1);
            h.set_theme(theme);
            h.set_depth(depth);
            let cell = |h: &Harness<Demo>, x: u16| h.buffer()[(x, 0)].bg;
            let (faint, full, gap) = (cell(&h, 3), cell(&h, 14), cell(&h, 9));
            assert_ne!(faint, gap, "{theme} {depth:?}: a faint block is not the track");
            assert_ne!(faint, full, "{theme} {depth:?}: nor a full block");
            for x in [0, 1, 16, 17] {
                assert_ne!(cell(&h, x), gap, "{theme} {depth:?}: the fading cell {x} is still the block");
            }
            assert_ne!(cell(&h, 17), full, "{theme} {depth:?}: the open end shows");
            h.hover(3, 0);
            let hover = cell(&h, 3);
            assert_ne!(hover, faint, "{theme} {depth:?}: hovering a faint block shows");
            assert_ne!(hover, gap, "{theme} {depth:?}: and does not turn it into the track");
            h.hover(9, 0);
            h.click(3, 0);
            assert_ne!(cell(&h, 3), faint, "{theme} {depth:?}: selecting a faint block shows");
            assert_ne!(cell(&h, 3), gap, "{theme} {depth:?}");
        }
    }
}

#[test]
fn open_and_faint_blocks_are_made_of_colour_in_every_glyph_mode() {
    let mut screens = Vec::new();
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        let mut demo = Demo::new([
            TimeBlock::new("Rust", t(0, 0), t(1, 30)).faint().open_start(),
            TimeBlock::new("Docs", t(9, 0), t(11, 0)),
            TimeBlock::new("Rust", t(15, 0), t(17, 40)).open_end(),
        ])
        .interactive();
        demo.readout = true;
        demo.selected = Some(2);
        let mut h = Harness::new(demo, 96, 2);
        h.set_glyph_mode(mode);
        assert!(!h.screen().contains('▌'), "{mode:?}: the pillar means focus, never an open edge");
        screens.push((h.screen(), row_bg(&h, 96)));
    }
    assert!(screens.windows(2).all(|pair| pair[0] == pair[1]), "the edges are colour, not glyphs");
}

#[test]
fn the_readout_says_what_an_open_edge_means() {
    let blocks = [
        TimeBlock::new("Rust", t(0, 0), t(1, 30)).faint().open_start(),
        TimeBlock::new("Docs", t(9, 0), t(10, 0)).open_start(),
        TimeBlock::new("Rust", t(15, 0), t(17, 40)).open_end(),
        TimeBlock::new("Night", t(22, 0), t(2, 0)).open_end(),
        TimeBlock::new("Both", t(9, 0), t(12, 0)).open_start().open_end(),
    ];
    let expected = [
        ("Rust  00:00–01:30  from the previous day  1 h 30 min", "Rust  00:00–01:30  önceki günden  1 sa 30 dk"),
        ("Docs  09:00–10:00  from earlier  1 h", "Docs  09:00–10:00  önceden  1 sa"),
        ("Rust  15:00–17:40  running  2 h 40 min", "Rust  15:00–17:40  sürüyor  2 sa 40 dk"),
        ("Night  22:00–02:00  continues next day  4 h", "Night  22:00–02:00  ertesi güne sürüyor  4 sa"),
        ("Both  09:00–12:00  from earlier, running  3 h", "Both  09:00–12:00  önceden, sürüyor  3 sa"),
    ];
    for (index, (english, turkish)) in expected.into_iter().enumerate() {
        let mut demo = Demo::new(blocks.clone());
        demo.readout = true;
        demo.selected = Some(index);
        // Both overlaps Docs, so the strip has two lanes and the readout is the third row.
        let mut h = Harness::new(demo, 96, 3);
        assert_eq!(line(&h, 2), english, "block {index}");
        h.set_locale("tr");
        assert_eq!(line(&h, 2), turkish, "block {index}");
    }
}

#[test]
fn a_narrow_readout_keeps_the_open_edge_longer_than_the_times() {
    for (width, expected) in [
        (40, "Rust  15:00–17:40  running  2 h 40 min"),
        (30, "Rust  15:00–17:40  running"),
        (20, "Rust  running"),
        (8, "Rust"),
    ] {
        let mut demo = Demo::new([TimeBlock::new("Rust", t(15, 0), t(17, 40)).open_end()]);
        demo.readout = true;
        demo.selected = Some(0);
        let h = Harness::new(demo, width, 2);
        assert_eq!(line(&h, 1), expected, "{width} cells");
    }
}

#[test]
fn a_name_stands_clear_of_a_fading_edge() {
    // 96 cells: a quarter of an hour a cell. 15:00 is cell 60, 17:40 falls in cell 70.
    let open = Harness::new(Demo::new([TimeBlock::new("Rust", t(15, 0), t(17, 40)).open_start()]), 96, 1);
    assert_eq!(open.screen().find("Rust"), Some(62), "the name starts after the two fading cells");
    let closed = Harness::new(Demo::new([TimeBlock::new("Rust", t(15, 0), t(17, 40))]), 96, 1);
    assert_eq!(closed.screen().find("Rust"), Some(61), "a closed block keeps its cell of air");
    // Seven cells open at both ends: two fading cells each side leave three, too few for four letters.
    let tight = TimeBlock::new("Rust", t(15, 0), t(16, 45)).open_start().open_end();
    let tight = Harness::new(Demo::new([tight]), 96, 1);
    assert_eq!(tight.screen().find("Rust"), None, "a name that would sit on a fade is left out");
    for theme in THEMES {
        let mut h = Harness::new(Demo::new([TimeBlock::new("Rust", t(15, 0), t(17, 40)).faint()]), 96, 1);
        h.set_theme(theme);
        let (block, name) = (h.bg(61, 0).expect("drawn"), h.fg(61, 0).expect("the name is drawn"));
        assert!(block.contrast_ratio(name) >= 3.0, "{theme}: the name reads on a faint block");
    }
}

#[test]
fn a_disabled_timeline_keeps_open_and_faint_blocks_apart_from_the_track() {
    let mut demo = Demo::new([
        TimeBlock::new("Before", t(0, 0), t(6, 0)).faint().open_start(),
        TimeBlock::new("B", t(12, 0), t(18, 0)).open_end(),
    ]);
    demo.disabled = true;
    let h = Harness::new(demo, 24, 1);
    let muted = h.env().theme().color("muted");
    assert_eq!(h.bg(14, 0), muted, "a disabled block is muted");
    for x in [0, 1, 3, 16, 17] {
        assert_ne!(h.bg(x, 0), Some(track(&h)), "cell {x} is still a block");
        assert_ne!(h.bg(x, 0), muted, "cell {x} keeps its open edge or its faintness");
    }
}
