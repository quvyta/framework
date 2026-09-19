use super::*;
use crate::runtime::{App, Command, Harness};
use crate::widget::{Length, View};

/// A package manager's views, the last one counting pending updates.
struct Views {
    active: usize,
    updates: u32,
    width: u16,
    tab_width: TabWidth,
    closable: bool,
}

fn views(updates: u32) -> Views {
    Views { active: 0, updates, width: 60, tab_width: TabWidth::Fit, closable: false }
}

impl App for Views {
    type Msg = usize;
    fn update(&mut self, index: usize) -> Command<usize> {
        self.active = index;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, usize>) {
        let mut tabs = Tabs::new(["Installed", "Updates", "Search"])
            .active(self.active)
            .tab_width(self.tab_width)
            .badge(1, self.updates)
            .on_select(|i| i);
        if self.closable {
            tabs = tabs.closable(|i| i);
        }
        ui.add(tabs).width(Length::Cells(self.width));
    }
}

#[test]
fn the_badge_follows_the_name_of_its_tab() {
    let mut h = Harness::new(Views { active: 1, ..views(3) }, 60, 1);
    assert!(h.screen().contains("Updates 3"), "one space after the name: {}", h.screen());
    let (badge, _) = h.find("3").expect("the badge");
    let (search, _) = h.find("Search").expect("the next tab");
    assert!(badge < search, "the badge belongs to its own tab");
    let (name, _) = h.find("Updates").expect("the name");
    let cell = |x: i32| u16::try_from(x).unwrap_or(0);
    assert_ne!(h.fg(cell(badge), 0), h.fg(cell(name), 0), "a quieter tone than the name");
    h.click_text("Search");
    assert_eq!(h.app().active, 2);
    // Resting, the name slides one cell towards the start; the badge keeps its cell.
    assert_eq!(h.find("3").map(|(x, _)| x), Some(badge), "{}", h.screen());
}

#[test]
fn a_count_of_zero_shows_no_badge_and_takes_no_room() {
    let none = Harness::new(views(0), 60, 1);
    struct Plain;
    impl App for Plain {
        type Msg = usize;
        fn update(&mut self, _: usize) -> Command<usize> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(Tabs::new(["Installed", "Updates", "Search"]).on_select(|i| i)).width(Length::Cells(60));
        }
    }
    let plain = Harness::new(Plain, 60, 1);
    assert_eq!(none.screen(), plain.screen());
    assert!(!none.screen().contains('0'), "{}", none.screen());
}

#[test]
fn large_counts_read_like_a_badge_count() {
    let h = Harness::new(Views { active: 1, ..views(250) }, 60, 1);
    assert!(h.screen().contains("Updates 99+"), "{}", h.screen());
}

#[test]
fn a_fixed_width_shortens_the_name_and_keeps_the_badge() {
    let h = Harness::new(Views { active: 1, tab_width: TabWidth::Fixed(10), ..views(12) }, 60, 1);
    let screen = h.screen();
    assert!(screen.contains("Up… 12"), "{screen}");
    assert!(!screen.contains("Updates"), "{screen}");
}

#[test]
fn a_narrow_screen_shortens_the_name_and_keeps_the_badge() {
    let h = Harness::new(Views { active: 1, width: 9, ..views(5) }, 9, 1);
    let screen = h.screen();
    assert!(screen.contains("… 5"), "the name gives way, the count stays: {screen}");
}

#[test]
fn the_badge_sits_before_the_close_mark() {
    let mut h = Harness::new(Views { active: 1, closable: true, ..views(4) }, 60, 1);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    let screen = h.screen();
    let badge = screen.find("Updates 4").expect("badge after the name");
    let close = screen[badge..].find('×').expect("close mark of the tab");
    assert!(close > "Updates 4".len(), "{screen}");
}

#[test]
fn a_hidden_tab_keeps_its_count_in_the_menu() {
    struct Crowded;
    impl App for Crowded {
        type Msg = usize;
        fn update(&mut self, _: usize) -> Command<usize> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            let tabs = Tabs::new(["Installed", "Search", "History", "Updates"])
                .overflow(Overflow::Menu)
                .badge(3, 7)
                .on_select(|i| i);
            ui.add(tabs).width(Length::Cells(30));
        }
    }
    let mut h = Harness::new(Crowded, 30, 8);
    h.press("tab").press("down").advance(std::time::Duration::from_secs(1));
    assert!(h.screen().contains("Updates  7"), "{}", h.screen());
}
