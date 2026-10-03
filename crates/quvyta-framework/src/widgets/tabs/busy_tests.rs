use std::time::Duration;

use super::*;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::{Length, View};

/// Tabs whose jobs run in the background: some may be working, one may wait for the person.
struct Jobs {
    active: usize,
    busy: Vec<usize>,
    status: Option<(usize, &'static str)>,
    tab_width: TabWidth,
    width: u16,
}

fn jobs(busy: &[usize]) -> Jobs {
    Jobs { active: 0, busy: busy.to_vec(), status: None, tab_width: TabWidth::Fit, width: 60 }
}

/// Opening a tab, or every job finishing.
#[derive(Clone)]
enum Msg {
    Open(usize),
    Finished,
}

impl App for Jobs {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open(index) => self.active = index,
            Msg::Finished => self.busy.clear(),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut tabs = Tabs::new(["Planner", "Builder", "Reviewer"])
            .active(self.active)
            .tab_width(self.tab_width)
            .on_select(Msg::Open);
        for index in 0..3 {
            tabs = tabs.busy(index, self.busy.contains(&index));
        }
        if let Some((index, token)) = self.status {
            tabs = tabs.status(index, token);
        }
        ui.add(tabs).width(Length::Cells(self.width));
    }
}

fn harness(app: Jobs) -> Harness<Jobs> {
    let width = app.width;
    let mut h = Harness::new(app, width, 1);
    h.set_glyph_mode(GlyphMode::Unicode);
    h
}

/// The character two cells before `name`, where its tab's mark sits.
fn mark(h: &Harness<Jobs>, name: &str) -> char {
    let (x, _) = h.find(name).unwrap_or_else(|| panic!("{name} on screen: {}", h.screen()));
    let x = usize::try_from(x).expect("on screen") - 2;
    h.screen().lines().next().and_then(|row| row.chars().nth(x)).expect("a cell before the name")
}

fn braille(c: char) -> bool {
    ('\u{2801}'..='\u{28FF}').contains(&c)
}

#[test]
fn a_busy_tab_shows_a_turning_mark_before_its_name_and_the_others_do_not() {
    let mut h = harness(jobs(&[1]));
    assert!(braille(mark(&h, "Builder")), "{}", h.screen());
    assert!(!braille(mark(&h, "Planner")), "{}", h.screen());
    assert_eq!(mark(&h, "Reviewer"), ' ', "{}", h.screen());
    let first = mark(&h, "Builder");
    let mut seen = vec![first];
    for _ in 0..6 {
        h.advance(Duration::from_millis(120));
        seen.push(mark(&h, "Builder"));
    }
    assert!(seen.iter().all(|c| braille(*c)), "{seen:?}");
    assert!(seen.iter().any(|c| *c != first), "the mark turns on the framework's clock alone: {seen:?}");
}

#[test]
fn the_open_tab_shows_its_mark_too() {
    let h = harness(Jobs { active: 1, ..jobs(&[1]) });
    assert!(braille(mark(&h, "Builder")), "{}", h.screen());
}

#[test]
fn with_reduced_motion_the_mark_is_one_dot_standing_still() {
    let mut h = harness(jobs(&[1]));
    h.set_reduced_motion(true);
    assert_eq!(mark(&h, "Builder"), '●', "{}", h.screen());
    h.advance(Duration::from_secs(1));
    assert_eq!(mark(&h, "Builder"), '●', "{}", h.screen());
}

#[test]
fn the_mark_is_in_the_accent_colour() {
    let h = harness(jobs(&[1]));
    let (x, _) = h.find("Builder").expect("the name");
    let cell = u16::try_from(x - 2).expect("on screen");
    assert_eq!(h.fg(cell, 0), h.env().theme().color("accent"));
}

#[test]
fn busy_takes_no_room_so_no_other_tab_moves_when_it_starts_or_stops() {
    let idle = harness(jobs(&[]));
    for busy in [vec![1], vec![0, 1], vec![2]] {
        let marked = harness(jobs(&busy));
        for (index, name) in ["Planner", "Builder", "Reviewer"].into_iter().enumerate() {
            let (before, after) = (idle.find(name).expect("idle"), marked.find(name).expect("busy"));
            if busy.contains(&index) {
                // The name steps one cell on, into the spare cell every tab keeps.
                assert_eq!(after.0 - before.0, 1, "{name}\n{}\n{}", idle.screen(), marked.screen());
            } else {
                assert_eq!(after, before, "{name}\n{}\n{}", idle.screen(), marked.screen());
            }
        }
    }
    let open = harness(Jobs { active: 1, ..jobs(&[]) });
    let open_busy = harness(Jobs { active: 1, ..jobs(&[1]) });
    assert_eq!(open.find("Reviewer"), open_busy.find("Reviewer"));
}

#[test]
fn a_tab_that_stops_being_busy_looks_as_if_it_never_was() {
    struct Plain;
    impl App for Plain {
        type Msg = usize;
        fn update(&mut self, _: usize) -> Command<usize> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(Tabs::new(["Planner", "Builder", "Reviewer"]).on_select(|i| i)).width(Length::Cells(60));
        }
    }
    let mut plain = Harness::new(Plain, 60, 1);
    plain.set_glyph_mode(GlyphMode::Unicode);
    let mut h = harness(jobs(&[1]));
    h.send(Msg::Finished);
    assert_eq!(h.screen(), plain.screen());
}

#[test]
fn a_narrow_tab_shortens_its_name_and_keeps_the_mark() {
    let h = harness(Jobs { tab_width: TabWidth::Fixed(10), ..jobs(&[1]) });
    let screen = h.screen();
    assert!(!screen.contains("Builder"), "{screen}");
    let row: Vec<char> = screen.chars().collect();
    let at = row.iter().position(|c| braille(*c)).expect("the mark stays");
    assert_eq!(row[at + 1], ' ', "{screen}");
    assert_eq!(row[at + 2], 'B', "{screen}");
    assert!(row[at + 3..].iter().take(6).any(|c| *c == '…'), "{screen}");
}

#[test]
fn a_status_puts_a_dot_in_its_colour_before_the_name() {
    let h = harness(Jobs { status: Some((2, "success")), ..jobs(&[]) });
    assert_eq!(mark(&h, "Reviewer"), '●', "{}", h.screen());
    let (x, _) = h.find("Reviewer").expect("the name");
    let cell = u16::try_from(x - 2).expect("on screen");
    assert_eq!(h.fg(cell, 0), h.env().theme().color("success"));
    assert_eq!(mark(&h, "Builder"), ' ');
}

#[test]
fn a_busy_tab_with_a_status_turns_in_the_status_colour() {
    let h = harness(Jobs { status: Some((1, "warning")), ..jobs(&[1]) });
    assert!(braille(mark(&h, "Builder")), "{}", h.screen());
    let (x, _) = h.find("Builder").expect("the name");
    let cell = u16::try_from(x - 2).expect("on screen");
    assert_eq!(h.fg(cell, 0), h.env().theme().color("warning"));
}
