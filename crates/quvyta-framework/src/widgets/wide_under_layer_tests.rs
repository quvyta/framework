//! A layer painted over double-width text never leaves half a character behind. A terminal draws
//! a wide character across two cells from the first one; when a dialog's pillar or edge lands on
//! either half and only that half is replaced, the terminal shows the glyph spilling over the
//! pillar or shifts the rest of the row by a column. Whatever writes a cell of a wide pair turns
//! the other half into a blank, so every pair on screen stays whole.

use std::time::Duration;

use ratatui_core::buffer::Buffer;

use crate::runtime::{App, Command, Confirm, Harness};
use crate::text;
use crate::widget::View;
use crate::widgets::{Text, Toast};

/// A screen full of Chinese text, shifted by one ASCII letter when `odd`, so the wide characters
/// start on odd columns and a surface edge on an even column lands on their second half.
struct Wide {
    odd: bool,
}

#[derive(Clone)]
enum Msg {
    Ask,
    Toast,
    Answered,
}

impl App for Wide {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Ask => Command::confirm(Confirm::new("清理缓存?", Msg::Answered).message("缓存清理").danger()),
            Msg::Toast => Command::toast(Toast::info("缓存清理完成").body("缓存清理缓存清理")),
            Msg::Answered => Command::none(),
        }
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let line = format!("{}{}", if self.odd { "a" } else { "" }, "缓存清理".repeat(20));
        ui.column(|ui| {
            for _ in 0..14 {
                ui.add(Text::new(line.as_str()).no_wrap());
            }
        });
    }
}

/// Every wide character is followed by its blank second half, and every second half follows a
/// wide character.
fn assert_pairs_whole(buffer: &Buffer, screen: &str) {
    let area = buffer.area;
    for y in 0..area.height {
        for x in 0..area.width {
            let symbol = buffer[(x, y)].symbol();
            if text::width(symbol) == 2 {
                let next = (x + 1 < area.width).then(|| buffer[(x + 1, y)].symbol());
                assert_eq!(next, Some(""), "`{symbol}` at ({x}, {y}) lost its second half:\n{screen}");
            }
            if symbol.is_empty() {
                let before = x.checked_sub(1).map_or(0, |left| text::width(buffer[(left, y)].symbol()));
                assert_eq!(before, 2, "a second half at ({x}, {y}) lost its first:\n{screen}");
            }
        }
    }
}

fn asked(odd: bool) -> Harness<Wide> {
    let mut h = Harness::new(Wide { odd }, 60, 14);
    h.send(Msg::Ask).advance(Duration::from_millis(300));
    h
}

#[test]
fn a_dialog_pillar_on_the_second_half_of_a_wide_character_blanks_the_first() {
    let h = asked(true);
    let screen = h.screen();
    let (_, title) = h.find("清理缓存?").unwrap_or_else(|| panic!("the question is on screen:\n{screen}"));
    let row = u16::try_from(title).unwrap_or(0);
    let pillar = (0..60).find(|&x| h.buffer()[(x, row)].symbol() == "▌").expect("the pillar shows");
    assert_eq!(pillar % 2, 0, "the pillar sits on the second half of a wide character");
    assert_eq!(h.buffer()[(pillar - 1, row)].symbol(), " ", "the first half turned blank:\n{screen}");
    assert_pairs_whole(h.buffer(), &screen);
}

#[test]
fn a_dialog_edge_on_the_first_half_of_a_wide_character_blanks_the_second() {
    for odd in [false, true] {
        let h = asked(odd);
        assert_pairs_whole(h.buffer(), &h.screen());
    }
}

#[test]
fn a_toast_over_wide_text_leaves_every_pair_whole() {
    for odd in [false, true] {
        let mut h = Harness::new(Wide { odd }, 60, 14);
        h.send(Msg::Toast).advance(Duration::from_millis(300));
        assert!(h.screen().contains("完成"), "the toast shows:\n{}", h.screen());
        assert_pairs_whole(h.buffer(), &h.screen());
    }
}
