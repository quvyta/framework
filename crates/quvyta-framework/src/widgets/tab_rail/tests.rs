use std::time::Duration;

use super::*;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::{Length, View};
use crate::widgets::{ContextItem, TabEdit};

struct Projects {
    names: Vec<&'static str>,
    active: usize,
    collapsed: bool,
    pinned: Vec<usize>,
    added: usize,
    /// Row height and gap, set only when a test asks for them.
    blocks: Option<(u16, u16)>,
    /// Whether tabs have a context menu.
    menu: bool,
    /// Cells of a wide rail.
    width: u16,
    /// The first row in view after each step a dragged tab scrolled the rail.
    scrolled: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
enum Msg {
    Open(usize),
    Edit(TabEdit),
    Add,
    /// Opens a second tab of the same project after this one.
    Duplicate(usize),
    Scrolled(usize),
}

impl App for Projects {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open(index) => self.active = index,
            Msg::Edit(edit) => edit.apply(&mut self.names, &mut self.active),
            Msg::Add => self.added += 1,
            Msg::Duplicate(index) => {
                self.names.insert(index + 1, self.names[index]);
                self.active = index + 1;
            }
            Msg::Scrolled(first) => self.scrolled.push(first),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let tabs = self.names.iter().map(|name| {
            let tab = RailTab::new(*name).icon("folder");
            if *name == "quvyta" { tab.status("success").badge("3") } else { tab }
        });
        let mut rail = TabRail::new(tabs)
            .active(self.active)
            .collapsed(self.collapsed)
            .on_select(Msg::Open)
            .closable(|i| Msg::Edit(TabEdit::Close(i)))
            .pinned(self.pinned.clone())
            .reorderable(|from, to| Msg::Edit(TabEdit::Move { from, to }))
            .on_add(|| Msg::Add)
            .on_drag_scroll(Msg::Scrolled);
        if self.menu {
            rail = rail.context_menu(|index| {
                vec![
                    ContextItem::new("Duplicate", Msg::Duplicate(index)),
                    ContextItem::new("Close", Msg::Edit(TabEdit::Close(index))),
                ]
            });
        }
        if let Some((height, gap)) = self.blocks {
            rail = rail.row_height(height).gap(gap);
        }
        ui.row(|ui| {
            ui.add(rail)
                .width(Length::Cells(if self.collapsed { COLLAPSED } else { self.width }))
                .fill_height()
                .id("rail");
        })
        .fill();
    }
}

fn projects(collapsed: bool) -> Projects {
    Projects {
        names: vec!["quvyta", "qcode", "dotfiles", "website"],
        active: 0,
        collapsed,
        pinned: Vec::new(),
        added: 0,
        blocks: None,
        menu: false,
        width: 22,
        scrolled: Vec::new(),
    }
}

#[test]
fn open_tab_is_raised_with_pillar_and_trailing_marks_stay_put() {
    let mut h = Harness::new(projects(false), 30, 5);
    assert_eq!(
        h.screen(),
        "▌  ■ quvyta     ● 3 ×\n  ■ qcode           ×\n  ■ dotfiles        ×\n  ■ website         ×\n  + New tab\n"
    );
    assert_eq!(h.bg(5, 0), h.env().theme().color("active"));
    h.hover(4, 2);
    assert!(h.screen().contains("\n▌  ■ dotfiles       ×\n"), "{}", h.screen());
    h.press("tab").press("down").press("end");
    assert_eq!(h.app().active, 3);
    h.press("home");
    assert_eq!(h.app().active, 0);
}

#[test]
fn closes_by_mark_middle_click_and_ctrl_w() {
    let mut h = Harness::new(projects(false), 30, 4);
    h.click(20, 1);
    assert_eq!(h.app().names, vec!["quvyta", "dotfiles", "website"]);
    h.mouse(MouseKind::Down(MouseButton::Middle), 4, 2);
    assert_eq!(h.app().names, vec!["quvyta", "dotfiles"]);
    h.press("tab").press("ctrl+w");
    assert_eq!(h.app().names, vec!["dotfiles"]);
}

#[test]
fn drags_a_tab_down_and_moves_it_by_keys() {
    let mut h = Harness::new(projects(false), 30, 4);
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 2);
    let screen = h.screen();
    assert!(screen.starts_with("  ■ qcode"), "neighbours make room:\n{screen}");
    assert_ne!(h.bg(12, 2), h.env().theme().color("canvas"));
    h.mouse(MouseKind::Up(MouseButton::Left), 5, 2);
    assert_eq!(h.app().names, vec!["qcode", "dotfiles", "quvyta", "website"]);
    assert_eq!(h.app().active, 2);
    h.press("tab").press("ctrl+shift+up");
    assert_eq!(h.app().names, vec!["qcode", "quvyta", "dotfiles", "website"]);

    let mut h = Harness::new(projects(false), 30, 5);
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 4);
    h.mouse(MouseKind::Up(MouseButton::Left), 5, 4);
    assert_eq!(
        h.app().names,
        vec!["qcode", "dotfiles", "website", "quvyta"],
        "released over the add row, it lands last"
    );
    assert_eq!(h.app().added, 0, "and adds nothing");
}

#[test]
fn collapsed_rail_is_a_thin_strip_with_an_add_row() {
    let mut h = Harness::new(projects(true), 30, 5);
    // Pillar, a one-cell label that never moves, a cell of air and a kept scrollbar column.
    assert_eq!(h.screen(), "▌■\n ■\n ■\n ■\n +\n");
    assert_eq!(h.fg(1, 0), h.env().theme().color("success"), "a status colours the icon");
    h.hover(1, 2);
    assert!(h.screen().starts_with("▌■\n ■\n▌■  dotfiles ×\n"), "{}", h.screen());
    h.hover(1, 4);
    assert!(h.screen().contains("▌+  New tab"), "{}", h.screen());
    h.click(1, 4);
    assert_eq!(h.app().added, 1);
    assert_eq!(h.app().names.len(), 4, "the add row is not a tab");
}

#[test]
fn collapsed_rail_turns_its_last_column_into_a_scrollbar() {
    let wide = Harness::new(projects(true), 30, 5);
    let narrow = Harness::new(projects(true), 30, 3);
    let strip =
        |h: &Harness<Projects>| h.screen().lines().next().unwrap_or_default().chars().take(3).collect::<String>();
    assert_eq!(strip(&wide), strip(&narrow), "the strip keeps its width");
    assert!(super::super::scrollbar::column(&narrow, 3).contains('#'), "{}", narrow.screen());
}

/// The first three columns of every line: the strip without the scrollbar column and the name card.
fn strip(h: &Harness<Projects>) -> Vec<String> {
    h.screen().lines().map(|line| line.chars().take(3).collect::<String>().trim_end().to_owned()).collect()
}

#[test]
fn a_hovered_collapsed_tab_keeps_its_icon_in_place_beside_the_scrollbar() {
    let mut h = Harness::new(projects(true), 30, 3);
    assert_eq!(h.screen(), "▌■\n ■\n ■\n", "resting: the open tab's pillar sits right before its icon");
    assert_eq!(super::super::scrollbar::column(&h, 3), "#--", "the scrollbar shows");
    let resting = strip(&h);
    h.hover(1, 1);
    assert_eq!(h.screen(), "▌■\n▌■  qcode ×\n ■\n", "hovered: the icon stays, the pillar takes the cell before it");
    assert_eq!(resting, vec!["▌■", " ■", " ■"]);
    assert_eq!(strip(&h), vec!["▌■", "▌■", " ■"], "only the pillar appears");
    let raised = h.env().theme().color("raised");
    assert_eq!([h.bg(1, 1), h.bg(2, 1)], [raised; 2], "the cell of air is part of the raised row");
    h.hover(20, 1);
    assert_eq!(strip(&h), resting, "after the pointer leaves the strip is as it was at rest");
    assert_eq!(super::super::scrollbar::column(&h, 3), "#--", "the scrollbar column never changes");

    let mut tall = Harness::new(blocks(true, 3, 1), 30, 8);
    let screen = "▌\n▌■\n▌\n\n\n ■\n\n\n";
    assert_eq!(tall.screen(), screen, "resting tall strip");
    assert_eq!(super::super::scrollbar::column(&tall, 3), "###-----");
    tall.hover(1, 4);
    assert_eq!(tall.screen(), "▌\n▌■\n▌\n\n▌\n▌■  qcode ×\n▌\n\n", "hovered tall strip");
    tall.hover(20, 4);
    assert_eq!(tall.screen(), screen);
}

#[test]
fn follows_the_open_tab_when_rows_overflow() {
    let mut app = projects(false);
    app.active = 3;
    let h = Harness::new(app, 30, 2);
    let screen = h.screen();
    assert!(screen.contains("website") && !screen.contains("quvyta"), "{screen}");
}

#[test]
fn collapsed_name_card_takes_the_pointer_and_opens_or_closes_its_tab() {
    let mut h = Harness::new(projects(true), 30, 5);
    let theme = h.env().theme();
    let (raised, active) = (theme.color("raised"), theme.color("active"));
    h.hover(1, 2);
    let card = "▌■\n ■\n▌■  dotfiles ×\n ■\n +\n";
    assert_eq!(h.screen(), card, "the card ends in the close mark, as a wide rail row does");
    assert_eq!(h.bg(8, 2), raised, "a card the pointer is not on continues its raised row");
    assert_eq!(h.bg(1, 2), raised);

    h.hover(3, 2);
    assert_eq!(h.screen(), card, "the card stays while the pointer moves onto it");
    h.hover(8, 2);
    assert_eq!(h.screen(), card, "and its row stays raised");
    assert_eq!(h.bg(8, 2), active, "under the pointer the card rises one more step");

    h.hover(13, 2);
    let lit = h.bg(13, 2);
    assert!(lit != raised && lit != active, "the close mark lights up");
    assert_eq!([h.bg(12, 2), h.bg(14, 2)], [lit, lit], "all three cells of it");
    assert_eq!(h.bg(8, 2), raised, "while the card itself stays calm");

    h.click(8, 2);
    assert_eq!(h.app().active, 2, "a click on the card opens its tab");
    assert_eq!(h.screen(), " ■\n ■\n▌■  dotfiles ×\n ■\n +\n");
    h.click(13, 2);
    assert_eq!(h.app().names, vec!["quvyta", "qcode", "website"], "a click on its close mark closes it");

    h.hover(20, 2);
    assert_eq!(h.screen(), " ■\n ■\n▌■\n +\n\n", "leaving the row and the card hides the card");
    h.hover(1, 1).hover(8, 2);
    assert!(!h.screen().contains("qcode") && !h.screen().contains("website"), "{}", h.screen());
}

#[test]
fn pinned_tabs_and_the_add_row_get_cards_without_a_close_mark() {
    let mut app = projects(true);
    app.pinned = vec![1];
    let mut h = Harness::new(app, 30, 5);
    h.hover(1, 1);
    assert_eq!(h.screen(), "▌■\n▌■  qcode\n ■\n ■\n +\n", "a pinned tab's card has no close mark");
    h.click(10, 1);
    assert_eq!(h.app().names.len(), 4, "the end of its card closes nothing");
    h.hover(1, 0);
    assert_eq!(h.screen().lines().next(), Some("▌■  quvyta  3 ×"), "the badge sits before the close mark");
    h.hover(1, 4).hover(6, 4);
    assert!(h.screen().ends_with("▌+  New tab\n"), "{}", h.screen());
    h.click(6, 4);
    assert_eq!(h.app().added, 1, "the add row's card adds");
}

#[test]
fn keyboard_focus_names_the_open_tab_of_a_collapsed_rail() {
    let mut h = Harness::new(projects(true), 30, 5);
    h.press("tab");
    assert_eq!(h.screen().lines().next(), Some("▌■  quvyta  3 ×"), "{}", h.screen());
    h.press("down");
    assert_eq!(h.screen(), " ■\n▌■  qcode ×\n ■\n ■\n +\n", "the card follows the open tab");
    h.hover(1, 3);
    assert_eq!(h.screen(), " ■\n▌■\n ■\n▌■  website ×\n +\n", "the pointer's card wins while it is on the rail");
    h.hover(20, 0);
    assert_eq!(h.screen(), " ■\n▌■  qcode ×\n ■\n ■\n +\n", "and the open tab's card comes back when it leaves");

    let mut clicked = Harness::new(projects(true), 30, 5);
    clicked.click(1, 1).hover(20, 1);
    assert_eq!(clicked.screen(), " ■\n▌■\n ■\n ■\n +\n", "focus by a click shows no card");
}

#[test]
fn card_is_pulled_left_on_a_narrow_screen_and_drawn_in_ascii() {
    let mut h = Harness::new(projects(true), 12, 5);
    h.hover(1, 0);
    assert_eq!(h.screen().lines().next(), Some(" quvy…  3 ×"), "{}", h.screen());
    let mut h = Harness::new(projects(true), 30, 5);
    h.set_glyph_mode(GlyphMode::Ascii);
    h.hover(1, 2);
    assert_eq!(h.screen().lines().nth(2), Some(" #  dotfiles x"), "{}", h.screen());
    h.set_reduced_motion(true).hover(8, 2);
    assert_eq!(h.screen().lines().nth(2), Some(" #  dotfiles x"), "nothing about the card moves");
}

#[test]
fn trailing_marks_stay_anchored_while_icon_and_name_slide() {
    let mut app = projects(false);
    app.active = 1;
    let mut h = Harness::new(app, 30, 5);
    let columns = |h: &Harness<Projects>| {
        let line: Vec<char> = h.screen().lines().next().unwrap_or_default().chars().collect();
        ['■', 'q', '●', '3', '×'].map(|mark| line.iter().position(|c| *c == mark))
    };
    let rest = columns(&h);
    h.hover(8, 0);
    let hovered = columns(&h);
    assert_eq!(h.screen().lines().next(), Some("▌  ■ quvyta     ● 3 ×"), "{}", h.screen());
    assert_eq!(hovered[0], rest[0].map(|x| x + 1), "the icon slides one cell");
    assert_eq!(hovered[1], rest[1].map(|x| x + 1), "the name slides with it");
    assert_eq!(hovered[2..], rest[2..], "status dot, badge and close mark stay put");
}

#[test]
fn scrollbar_scrolls_by_click_and_drag() {
    let mut h = Harness::new(projects(false), 30, 3);
    assert!(h.screen().contains("quvyta") && !h.screen().contains("New tab"), "{}", h.screen());
    h.mouse(MouseKind::Down(MouseButton::Left), 21, 2);
    assert!(h.screen().contains("New tab") && !h.screen().contains("quvyta"), "a click jumps:\n{}", h.screen());
    h.mouse(MouseKind::Drag(MouseButton::Left), 21, 0);
    assert!(h.screen().contains("quvyta"), "a drag follows the pointer:\n{}", h.screen());
    h.mouse(MouseKind::Drag(MouseButton::Left), 21, 1);
    h.mouse(MouseKind::Up(MouseButton::Left), 21, 1);
    assert!(h.screen().contains("qcode") && h.screen().contains("website"), "{}", h.screen());
    assert_eq!(h.app().active, 0, "scrolling opens nothing");
    h.click_text("website");
    assert_eq!(h.app().active, 3, "rows take presses again after the drag");
}

#[test]
fn open_tab_pillar_breathes_only_with_keyboard_focus() {
    let mut clicked = Harness::new(projects(false), 30, 5);
    clicked.click(8, 0);
    let calm = clicked.fg(0, 0);
    let mut keyed = Harness::new(projects(false), 30, 5);
    keyed.press("tab");
    assert_ne!(keyed.fg(0, 0), calm, "reached by keyboard the pillar breathes");
    assert_eq!(Harness::new(projects(false), 30, 5).fg(0, 0), calm, "a click leaves it as calm as no focus");
}

/// The rail of `projects(collapsed)` with rows `height` lines tall and `gap` lines apart.
fn blocks(collapsed: bool, height: u16, gap: u16) -> Projects {
    Projects { blocks: Some((height, gap)), ..projects(collapsed) }
}

#[test]
fn row_height_one_is_the_plain_rail() {
    for collapsed in [false, true] {
        let mut plain = Harness::new(projects(collapsed), 30, 5);
        let mut one = Harness::new(blocks(collapsed, 1, 0), 30, 5);
        for h in [&mut plain, &mut one] {
            h.hover(1, 2).press("tab").mouse(MouseKind::Down(MouseButton::Left), 1, 0);
            h.mouse(MouseKind::Drag(MouseButton::Left), 1, 2);
        }
        assert_eq!(plain.html("rail"), one.html("rail"), "every cell and colour, collapsed = {collapsed}");
    }
}

/// A rail of three projects with rows this many lines tall and no `gap` set.
struct Tall(u16);

impl App for Tall {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        let tabs = ["quvyta", "qcode", "dotfiles"].map(RailTab::new);
        ui.add(TabRail::new(tabs).row_height(self.0)).width(Length::Cells(20)).fill_height().id("rail");
    }
}

#[test]
fn tall_rows_get_a_gap_of_one_line_unless_gap_is_set() {
    let implicit = Harness::new(Tall(3), 20, 12);
    let lines: Vec<String> = implicit.screen().lines().map(|line| line.trim_end().to_owned()).collect();
    assert_eq!(lines[1], "▌  quvyta", "{}", implicit.screen());
    assert_eq!(lines[3], "", "one canvas line after the first block");
    assert_eq!(lines[5], "  qcode", "the next block starts after the gap:\n{}", implicit.screen());
    assert_eq!(implicit.bg(8, 3), implicit.env().theme().color("canvas"), "the gap is canvas");
    assert_eq!(implicit.bg(8, 4), implicit.env().theme().color("raised"), "and the next block is raised");
    let explicit = Harness::new(blocks(false, 3, 1), 30, 12);
    assert!(explicit.screen().lines().nth(5).is_some_and(|line| line.starts_with("  ■ qcode")), "as gap(1)");
    let stacked = Harness::new(blocks(false, 3, 0), 30, 12);
    assert!(stacked.screen().lines().nth(4).is_some_and(|line| line.starts_with("  ■ qcode")), "gap(0) stacks");
    let plain = Harness::new(Tall(1), 20, 4);
    assert_eq!(plain.screen(), "▌  quvyta\n  qcode\n  dotfiles\n\n", "one-line rows keep no gap");
}

#[test]
fn tall_rows_are_raised_blocks_with_their_content_on_the_middle_line() {
    let h = Harness::new(blocks(false, 3, 0), 30, 15);
    assert_eq!(
        h.screen(),
        "▌                   ×\n▌  ■ quvyta       ● 3\n▌\n                    ×\n  ■ qcode\n\n                    ×\n  ■ dotfiles\n\n                    ×\n  ■ website\n\n\n  + New tab\n\n",
        "the close mark sits in the top right corner of every block"
    );
    let theme = h.env().theme();
    let (raised, active) = (theme.color("raised"), theme.color("active"));
    assert_eq!([h.bg(8, 0), h.bg(8, 1), h.bg(8, 2)], [active; 3], "the open tab is one block");
    assert_eq!([h.fg(0, 0), h.fg(0, 2)], [h.fg(0, 1); 2], "and its pillar runs down all of it");
    assert_eq!([h.bg(8, 3), h.bg(8, 5), h.bg(8, 13)], [raised; 3], "resting tabs and the add row are raised too");

    let two = Harness::new(blocks(false, 2, 0), 30, 10);
    assert_eq!(
        two.screen(),
        "▌  ■ quvyta     ● 3 ×\n▌\n  ■ qcode           ×\n\n  ■ dotfiles        ×\n\n  ■ website         ×\n\n  + New tab\n\n",
        "an even block keeps its content on the upper middle line"
    );

    let gap = Harness::new(blocks(false, 3, 1), 30, 19);
    assert_eq!(
        gap.screen(),
        "▌                   ×\n▌  ■ quvyta       ● 3\n▌\n\n                    ×\n  ■ qcode\n\n\n                    ×\n  ■ dotfiles\n\n\n                    ×\n  ■ website\n\n\n\n  + New tab\n\n"
    );
    assert_eq!(gap.bg(8, 3), theme.color("canvas"), "a gap is canvas");
    assert_eq!(gap.bg(8, 4), raised);
}

#[test]
fn a_tall_block_takes_the_pointer_on_every_line() {
    let mut h = Harness::new(blocks(false, 3, 1), 30, 19);
    let raised = h.env().theme().color("raised");
    h.hover(8, 4);
    assert_eq!(h.screen().lines().nth(5), Some("▌  ■ qcode"), "{}", h.screen());
    assert_eq!(h.screen().lines().nth(4), Some("▌                   ×"), "the pillar runs down the hovered block");
    let lit = h.bg(8, 6);
    assert!(lit != raised && lit == h.bg(8, 4), "the whole block rises");
    h.hover(8, 3);
    assert_eq!(h.screen().lines().nth(5), Some("  ■ qcode"), "the gap belongs to no tab");
    h.click(8, 3);
    assert_eq!(h.app().active, 0, "and a press on it opens nothing");
    h.click(8, 6);
    assert_eq!(h.app().active, 1, "the last line of a block opens its tab");
}

#[test]
fn closes_a_tall_tab_by_the_mark_in_its_top_right_corner() {
    let mut h = Harness::new(blocks(false, 3, 0), 30, 15);
    let theme = h.env().theme();
    let raised = theme.color("raised");
    h.hover(20, 3);
    let lit = h.bg(20, 3);
    assert!(lit != raised && lit != h.bg(8, 3), "under the pointer the mark lights up");
    assert_eq!([h.bg(19, 3), h.bg(21, 3)], [lit; 2], "all three cells of it");
    assert_eq!(h.screen().lines().nth(3), Some("▌                   ×"), "{}", h.screen());
    assert_eq!(h.screen().lines().nth(4), Some("▌  ■ qcode"), "the name stays on the middle line");
    h.click(20, 4);
    assert_eq!(h.app().names.len(), 4, "below the mark the block opens");
    assert_eq!(h.app().active, 1);
    h.click(20, 3);
    assert_eq!(h.app().names, vec!["quvyta", "dotfiles", "website"], "the mark on the first line closes");
    h.mouse(MouseKind::Down(MouseButton::Middle), 8, 5);
    assert_eq!(h.app().names, vec!["quvyta", "website"], "a middle click anywhere on the block closes too");

    let mut two = Harness::new(blocks(false, 2, 1), 30, 14);
    assert_eq!(
        two.screen(),
        "▌  ■ quvyta     ● 3 ×\n▌\n\n  ■ qcode           ×\n\n\n  ■ dotfiles        ×\n\n\n  ■ website         ×\n\n\n  + New tab\n\n",
        "in a two-line block the first line is the middle line: the mark ends the name's line"
    );
    two.hover(20, 3);
    let lit = two.bg(20, 3);
    assert_eq!([two.bg(19, 3), two.bg(21, 3)], [lit; 2]);
    assert_ne!(lit, two.bg(8, 3), "the mark lights up apart from its block");
    two.click(20, 4);
    assert_eq!(two.app().active, 1, "the second line opens");
    two.click(20, 3);
    assert_eq!(two.app().names, vec!["quvyta", "dotfiles", "website"], "the mark closes");
}

#[test]
fn drags_tall_tabs_by_whole_blocks() {
    let mut h = Harness::new(blocks(false, 3, 1), 30, 19);
    let theme = h.env().theme();
    let (raised, canvas) = (theme.color("raised"), theme.color("canvas"));
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 9);
    let screen = h.screen();
    assert!(screen.starts_with("                    ×\n  ■ qcode\n"), "the neighbours make room:\n{screen}");
    let ghost = h.bg(12, 9);
    assert!(ghost != raised && ghost != canvas, "the ghost is at the pointer");
    assert_eq!([h.bg(12, 10), h.bg(12, 11)], [ghost; 2], "a whole block tall");
    assert_eq!(screen.lines().nth(9), Some("                    ×"), "its close mark in its corner:\n{screen}");
    assert_eq!(screen.lines().nth(10), Some("  ■ quvyta        ● 3"), "its content on its middle line:\n{screen}");
    h.mouse(MouseKind::Up(MouseButton::Left), 5, 9);
    assert_eq!(h.app().names, vec!["qcode", "dotfiles", "quvyta", "website"]);
    assert_eq!(h.app().active, 2);
}

#[test]
fn tall_rows_scroll_and_follow_by_whole_blocks() {
    let mut h = Harness::new(blocks(false, 3, 1), 30, 8);
    assert_eq!(h.screen(), "▌                  ×\n▌  ■ quvyta      ● 3\n▌\n\n                   ×\n  ■ qcode\n\n\n");
    assert_eq!(super::super::scrollbar::column(&h, 21), "###-----", "two of five blocks show");
    h.mouse(MouseKind::ScrollDown, 5, 5);
    assert!(
        h.screen().starts_with("                   ×\n  ■ qcode\n"),
        "the wheel scrolls one block:\n{}",
        h.screen()
    );
    h.press("tab").press("end");
    assert!(h.screen().contains("website") && !h.screen().contains("qcode"), "{}", h.screen());

    let narrow = Harness::new(Projects { width: 16, ..blocks(false, 3, 1) }, 30, 8);
    assert_eq!(
        narrow.screen(),
        "▌            ×\n▌  ■ quvy… ● 3\n▌\n\n             ×\n  ■ qcode\n\n\n",
        "a narrow rail cuts names on the middle line and keeps the anchored marks"
    );

    let short = Harness::new(blocks(false, 3, 1), 30, 2);
    assert_eq!(short.screen(), "▌  ■ quvyta    ● 3 ×\n▌\n", "a rail shorter than a block cuts the blocks to it");
    let line = Harness::new(blocks(false, 3, 1), 30, 1);
    assert_eq!(line.screen(), "▌  ■ quvyta    ● 3 ×\n", "and keeps the mark clear of the name on a one-line block");
}

#[test]
fn a_collapsed_tall_rail_keeps_its_blocks_and_a_tall_name_card() {
    let mut h = Harness::new(blocks(true, 3, 1), 30, 19);
    assert_eq!(h.screen(), "▌\n▌■\n▌\n\n\n ■\n\n\n\n ■\n\n\n\n ■\n\n\n\n +\n\n");
    let theme = h.env().theme();
    let (raised, active) = (theme.color("raised"), theme.color("active"));
    h.hover(1, 4);
    assert_eq!(h.screen().lines().skip(4).take(3).collect::<Vec<_>>(), vec!["▌", "▌■  qcode ×", "▌"]);
    assert_eq!([h.bg(8, 4), h.bg(8, 6)], [raised; 2], "the card is as tall as its block");
    h.hover(8, 6);
    assert_eq!(h.bg(8, 4), active, "and rises as a whole under the pointer");
    h.click(8, 6);
    assert_eq!(h.app().active, 1, "a click on any line of the card opens its tab");
    h.hover(10, 5).click(10, 5);
    assert_eq!(h.app().names, vec!["quvyta", "dotfiles", "website"], "its close mark sits on the middle line");
}

#[test]
fn tall_rows_in_ascii_and_with_reduced_motion() {
    let mut h = Harness::new(blocks(false, 3, 0), 30, 15);
    h.set_glyph_mode(GlyphMode::Ascii).set_reduced_motion(true);
    let accent_cell = h.bg(0, 0);
    assert_eq!(
        h.screen().lines().take(3).collect::<Vec<_>>(),
        vec!["                    x", "   # quvyta       * 3", ""]
    );
    assert_eq!([h.bg(0, 1), h.bg(0, 2)], [accent_cell; 2], "the pillar is a coloured column in ASCII");
    assert_ne!(accent_cell, h.bg(1, 0));
    h.hover(8, 3);
    assert_eq!(h.screen().lines().nth(4), Some("   # qcode"), "{}", h.screen());
}

fn right_click(h: &mut Harness<Projects>, x: i32, y: i32) {
    h.mouse(MouseKind::Down(MouseButton::Right), x, y);
    h.mouse(MouseKind::Up(MouseButton::Right), x, y);
}

#[test]
fn a_right_click_on_the_rail_does_nothing_without_a_context_menu() {
    let mut h = Harness::new(blocks(false, 3, 0), 30, 15);
    h.set_reduced_motion(true);
    let screen = h.screen();
    right_click(&mut h, 8, 4);
    h.press("tab").press("menu").press("shift+f10");
    h.hover(26, 14);
    assert_eq!(h.screen(), screen, "no menu, no opened tab");
    assert_eq!(h.app().active, 0);
}

#[test]
fn a_right_click_opens_the_menu_of_a_tall_tab_and_runs_its_entries() {
    let mut app = blocks(false, 3, 1);
    app.menu = true;
    let mut h = Harness::new(app, 40, 19);
    h.set_reduced_motion(true);
    right_click(&mut h, 6, 6);
    let screen = h.screen();
    assert!(screen.lines().nth(7).is_some_and(|line| line.contains("Duplicate")), "below the pointer:\n{screen}");
    assert_eq!(h.app().active, 0, "a right click opens no tab");
    let raised = h.env().theme().color("raised");
    h.hover(30, 8);
    assert_ne!(h.bg(15, 4), raised, "the tab the menu acts on stays lifted while the pointer is on the menu");
    h.click_text("Duplicate");
    assert_eq!(h.app().names, vec!["quvyta", "qcode", "qcode", "dotfiles", "website"], "for the tab under the pointer");
    assert_eq!(h.app().active, 2);
    assert!(!h.screen().contains("Duplicate"), "{}", h.screen());

    right_click(&mut h, 6, 0);
    right_click(&mut h, 6, 13);
    h.press("down").press("down").press("enter");
    assert_eq!(h.app().names, vec!["quvyta", "qcode", "qcode", "website"], "a second right click moves the menu");

    right_click(&mut h, 6, 13);
    h.click(6, 1);
    assert_eq!(h.app().active, 0, "a left click beside the menu closes it and opens what it lands on");
    assert!(!h.screen().contains("Duplicate"), "{}", h.screen());
}

#[test]
fn the_menu_key_opens_the_menu_below_the_open_block() {
    let mut app = blocks(false, 3, 1);
    app.menu = true;
    app.active = 1;
    let mut h = Harness::new(app, 40, 19);
    h.set_reduced_motion(true);
    h.press("tab").press("menu");
    let screen = h.screen();
    assert!(screen.lines().nth(7).is_some_and(|line| line.contains("▌") && line.contains("Duplicate")), "{screen}");
    h.press("down").press("enter");
    assert_eq!(h.app().names, vec!["quvyta", "dotfiles", "website"]);
    h.press("shift+f10").press("esc").press("down");
    assert_eq!(h.app().active, 2, "esc closes the menu and the rail has the keys again");
}

/// A rail three rows tall over five tabs with a context menu, the last tab open, and room below.
struct Scrolled;

impl App for Scrolled {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        let tabs = ["quvyta", "qcode", "dotfiles", "website", "homelab"].map(RailTab::new);
        let rail = TabRail::new(tabs)
            .active(4)
            .on_select(|_| ())
            .context_menu(|_| vec![ContextItem::new("Duplicate", ()), ContextItem::new("Close", ())]);
        ui.column(|ui| {
            ui.add(rail).width(Length::Cells(20)).height(Length::Cells(3)).id("rail");
        })
        .fill();
    }
}

#[test]
fn the_menu_key_scrolls_the_open_tab_into_view_and_opens_below_it() {
    let mut h = Harness::new(Scrolled, 40, 12);
    h.set_reduced_motion(true);
    assert!(h.screen().lines().nth(2).is_some_and(|line| line.contains("homelab")), "{}", h.screen());
    h.mouse(MouseKind::ScrollUp, 2, 1).mouse(MouseKind::ScrollUp, 2, 1);
    assert!(!h.screen().contains("homelab"), "scrolled away from the open tab:\n{}", h.screen());
    h.press("tab").press("menu");
    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert!(lines[2].contains("homelab"), "the open tab is back in view:\n{screen}");
    assert!(lines[3].contains("Duplicate"), "the menu opens below its block:\n{screen}");
    h.press("esc").mouse(MouseKind::ScrollUp, 2, 1).mouse(MouseKind::ScrollUp, 2, 1);
    h.press("shift+f10");
    assert!(h.screen().lines().nth(3).is_some_and(|line| line.contains("Duplicate")), "shift+F10 too:\n{}", h.screen());
}

#[test]
fn a_collapsed_rail_opens_the_menu_from_a_tab_or_its_name_card() {
    let mut app = blocks(true, 3, 0);
    app.menu = true;
    let mut h = Harness::new(app, 40, 15);
    h.set_reduced_motion(true);
    h.hover(1, 4);
    assert!(h.screen().contains("qcode"), "{}", h.screen());
    right_click(&mut h, 8, 4);
    let screen = h.screen();
    assert!(!screen.contains("qcode"), "the menu replaces the name card:\n{screen}");
    assert!(screen.lines().nth(5).is_some_and(|line| line.contains("Duplicate")), "{screen}");
    h.click_text("Close");
    assert_eq!(h.app().names, vec!["quvyta", "dotfiles", "website"]);

    right_click(&mut h, 1, 0);
    h.press("down").press("down").press("enter");
    assert_eq!(h.app().names, vec!["dotfiles", "website"], "the strip itself opens it too");
}

#[test]
fn very_tall_blocks_with_a_wide_gap_stay_whole_in_small_rails() {
    let block = |content: &str| {
        let mut lines = ["▌"; 9];
        lines[0] = content.split('\n').next().unwrap_or_default();
        lines[4] = content.split('\n').nth(1).unwrap_or_default();
        lines.join("\n")
    };
    let h = Harness::new(Projects { width: 10, ..blocks(false, 9, 5) }, 12, 12);
    assert_eq!(h.screen(), format!("{}\n\n\n\n", block("▌      ×\n▌  ■ ● 3")), "one whole block and the gap");
    assert_eq!(super::super::scrollbar::column(&h, 9), "##----------", "a sixth of the rail shows");
    let tiny = Harness::new(Projects { width: 5, ..blocks(false, 9, 5) }, 12, 12);
    assert_eq!(
        tiny.screen(),
        format!("{}\n\n\n\n", block("▌ ×\n▌ 3")),
        "a rail too narrow for the status dot leaves it out rather than covering the pillar"
    );

    let mut short = Harness::new(blocks(false, 9, 5), 30, 6);
    let raised = short.env().theme().color("raised");
    assert_eq!(short.screen(), "▌                  ×\n▌\n▌  ■ quvyta      ● 3\n▌\n▌\n▌\n", "cut to the rail");
    short.hover(19, 0);
    assert_ne!(short.bg(19, 0), short.bg(8, 0), "its close mark still lights");
    assert_ne!(short.bg(8, 0), raised, "on the open block");
    short.click(19, 0);
    assert_eq!(short.app().names, vec!["qcode", "dotfiles", "website"], "and closes");

    let mut strip = Harness::new(blocks(true, 9, 5), 12, 30);
    strip.hover(1, 18);
    let lines: Vec<String> = strip.screen().lines().map(str::to_owned).collect();
    assert_eq!(lines[4], "▌■", "the open tab's icon on its middle line, right after the pillar");
    assert_eq!(lines[18], "▌■  qcode ×", "the hovered tab keeps its icon in the same column");
    assert_eq!(lines[13], "", "five lines of canvas between blocks");
}

/// A collapsed rail of `names` with the rail-wide `marker`; names starting with `icon-` carry the
/// folder icon and the tabs listed in `own` carry their own marker.
struct Markers {
    names: Vec<&'static str>,
    marker: CollapsedMarker,
    own: Vec<(usize, CollapsedMarker)>,
}

impl App for Markers {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        let tabs = self.names.iter().enumerate().map(|(index, name)| {
            let tab = match name.strip_prefix("icon-") {
                Some(rest) => RailTab::new(rest).icon("folder"),
                None => RailTab::new(*name),
            };
            match self.own.iter().find(|(own, _)| *own == index) {
                Some((_, marker)) => tab.marker(*marker),
                None => tab,
            }
        });
        let rail = TabRail::new(tabs).collapsed(true).collapsed_marker(self.marker).on_select(|_| ());
        ui.add(rail).width(Length::Cells(COLLAPSED)).fill_height().id("rail");
    }
}

fn markers(names: &[&'static str], marker: CollapsedMarker) -> Markers {
    Markers { names: names.to_vec(), marker, own: Vec::new() }
}

/// The marker column of every line of the strip.
fn marker_column(h: &Harness<Markers>) -> String {
    h.screen().lines().map(|line| line.chars().nth(1).unwrap_or(' ')).collect()
}

#[test]
fn the_icon_marker_is_the_default_and_falls_back_to_the_initial() {
    let names = ["icon-quvyta", "qcode", "ßeta", "e\u{301}lan", "界面"];
    let h = Harness::new(markers(&names, CollapsedMarker::Icon), 20, 5);
    assert_eq!(
        h.screen(),
        "▌■\n Q\n ß\n E\u{301}\n …\n",
        "icon, initials with their accents, and a wide initial that cannot fit one cell"
    );
    let plain =
        Harness::new(Markers { marker: CollapsedMarker::default(), ..markers(&names, CollapsedMarker::Icon) }, 20, 5);
    assert_eq!(plain.screen(), h.screen(), "Icon is the default");
}

#[test]
fn the_initial_marker_ignores_icons_and_the_number_marker_counts_from_one() {
    let h = Harness::new(markers(&["icon-quvyta", "icon-qcode", "dotfiles"], CollapsedMarker::Initial), 20, 3);
    assert_eq!(h.screen(), "▌Q\n Q\n D\n");

    let many = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];
    let mut h = Harness::new(markers(&many, CollapsedMarker::Number), 20, 12);
    assert_eq!(marker_column(&h), "123456789………", "past nine a position does not fit one cell");
    assert!(
        h.screen().lines().all(|line| text::width(&line.chars().take(3).collect::<String>()) <= 2),
        "every marker is one cell:\n{}",
        h.screen()
    );
    h.set_glyph_mode(GlyphMode::Ascii);
    assert_eq!(marker_column(&h), "123456789~~~", "numbers read the same in ASCII, cut with an ASCII mark");
    h.hover(1, 10);
    assert_eq!(h.screen().lines().nth(10), Some(" ~  k"), "the name card still names the tab");
}

#[test]
fn a_tab_marker_overrides_the_rail_marker_in_every_glyph_mode() {
    let names = ["icon-quvyta", "icon-qcode", "icon-homelab", "dotfiles"];
    let own = vec![(1, CollapsedMarker::Number), (2, CollapsedMarker::Initial)];
    let mut h = Harness::new(Markers { own, ..markers(&names, CollapsedMarker::Icon) }, 20, 4);
    assert_eq!(h.screen(), "▌■\n 2\n H\n D\n");
    h.set_glyph_mode(GlyphMode::Ascii);
    assert_eq!(marker_column(&h), "#2HD", "only the icon has an ASCII form");
    let expanded = Harness::new(projects(false), 30, 5);
    assert!(expanded.screen().starts_with("▌  ■ quvyta"), "the expanded rail is not affected");
}

const MS: fn(u64) -> Duration = Duration::from_millis;

/// A wide rail four lines tall with eight projects and the add row, so five rows are hidden.
fn many() -> Harness<Projects> {
    let names = vec!["quvyta", "qcode", "dotfiles", "website", "infra", "blog", "api", "notes"];
    Harness::new(Projects { names, ..projects(false) }, 30, 4)
}

/// The backgrounds of the scrollbar column.
fn bar(h: &Harness<Projects>) -> Vec<Option<crate::color::Rgb>> {
    (0..4).map(|y| h.bg(21, y)).collect()
}

#[test]
fn a_tab_held_on_the_last_row_scrolls_the_rail_after_a_delay_then_steadily_to_the_end() {
    let mut h = many();
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 2);
    let resting = bar(&h);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
    assert_ne!(bar(&h), resting, "the scrollbar lights up as the tab rests on the last row");
    h.advance(MS(399));
    assert_eq!(h.app().scrolled, Vec::<usize>::new(), "nothing scrolls before the delay");
    h.advance(MS(1));
    assert_eq!(h.app().scrolled, vec![1], "one row at the delay");
    assert!(h.screen().contains("infra"), "the next project comes into view:\n{}", h.screen());
    h.advance(MS(149));
    assert_eq!(h.app().scrolled, vec![1]);
    h.advance(MS(1));
    assert_eq!(h.app().scrolled, vec![1, 2], "then one row every 150 ms");
    h.advance(MS(150)).advance(MS(150)).advance(MS(150));
    assert_eq!(h.app().scrolled, vec![1, 2, 3, 4, 5]);
    h.advance(MS(150)).advance(MS(1000));
    assert_eq!(h.app().scrolled, vec![1, 2, 3, 4, 5], "and it stops at the end, the add row in view");
    h.mouse(MouseKind::Up(MouseButton::Left), 5, 3);
    assert_eq!(
        h.app().names,
        vec!["qcode", "dotfiles", "website", "infra", "blog", "api", "notes", "quvyta"],
        "dropped on the add row, it lands last"
    );
    assert_eq!((h.app().active, h.app().added), (7, 0));
}

#[test]
fn leaving_the_end_rows_stops_at_once_and_the_first_row_scrolls_back() {
    let mut h = many();
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
    h.advance(MS(400)).advance(MS(150));
    assert_eq!(h.app().scrolled, vec![1, 2]);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 1);
    h.advance(MS(150)).advance(MS(1000));
    assert_eq!(h.app().scrolled, vec![1, 2], "off the end rows nothing scrolls");
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 0);
    h.advance(MS(399));
    assert_eq!(h.app().scrolled, vec![1, 2], "the first row waits the delay too");
    h.advance(MS(1));
    assert_eq!(h.app().scrolled, vec![1, 2, 1], "and scrolls back");
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 2);
    h.mouse(MouseKind::Up(MouseButton::Left), 5, 2);
    assert_eq!(
        h.app().names,
        vec!["qcode", "dotfiles", "website", "quvyta", "infra", "blog", "api", "notes"],
        "it lands on the row shown under the pointer"
    );
}

#[test]
fn only_a_dragged_tab_scrolls_the_rail_and_only_when_rows_overflow() {
    let mut h = many();
    h.hover(5, 3).advance(MS(1000));
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 3);
    h.advance(MS(1000));
    h.mouse(MouseKind::Up(MouseButton::Left), 5, 3);
    assert!(h.app().scrolled.is_empty(), "a hover or a held press on the last row scrolls nothing");
    assert_eq!(h.app().active, 3);

    let mut fits = Harness::new(projects(false), 30, 6);
    fits.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
    fits.mouse(MouseKind::Drag(MouseButton::Left), 5, 5);
    fits.advance(MS(400)).advance(MS(1000));
    fits.mouse(MouseKind::Up(MouseButton::Left), 5, 5);
    assert!(fits.app().scrolled.is_empty(), "a rail that fits never scrolls");
}

#[test]
fn tall_blocks_scroll_by_whole_blocks_and_faster_past_the_edge() {
    let mut h = Harness::new(blocks(false, 3, 1), 30, 8);
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 1);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
    h.advance(MS(1000));
    assert!(h.app().scrolled.is_empty(), "the gap after the first block is not an end");
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 5);
    h.advance(MS(399));
    assert!(h.app().scrolled.is_empty());
    h.advance(MS(1));
    assert_eq!(h.app().scrolled, vec![1], "the last block scrolls one block");
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 9);
    h.advance(MS(150));
    assert_eq!(h.app().scrolled, vec![1, 2], "the step already due keeps its time");
    h.advance(MS(90));
    assert_eq!(h.app().scrolled, vec![1, 2, 3], "two lines past the bottom, a step every 90 ms");
    h.advance(MS(90)).advance(MS(1000));
    assert_eq!(h.app().scrolled, vec![1, 2, 3], "until the end");
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 1);
    h.mouse(MouseKind::Up(MouseButton::Left), 5, 1);
    assert_eq!(h.app().names, vec!["qcode", "dotfiles", "website", "quvyta"], "dropped on the block shown");
}

#[test]
fn a_dragged_tab_still_scrolls_the_rail_with_reduced_motion() {
    let mut h = many();
    h.set_reduced_motion(true);
    h.mouse(MouseKind::Down(MouseButton::Left), 5, 0);
    h.mouse(MouseKind::Drag(MouseButton::Left), 5, 3);
    h.advance(MS(400)).advance(MS(150));
    assert_eq!(h.app().scrolled, vec![1, 2]);
}
