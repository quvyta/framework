use std::time::Duration;

use super::*;
use crate::event::MouseKind;
use crate::i18n::I18n;
use crate::icons::GlyphMode;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;

fn hm(hours: u64, minutes: u64) -> Duration {
    Duration::from_secs(hours * 3600 + minutes * 60)
}

fn hms(hours: u64, minutes: u64, seconds: u64) -> Duration {
    Duration::from_secs(hours * 3600 + minutes * 60 + seconds)
}

fn i18n(code: &str) -> I18n {
    let mut i18n = I18n::builtin();
    assert!(i18n.set_active(code));
    i18n
}

// Parsing.

#[test]
fn reads_the_unit_words_of_every_built_in_language() {
    let i18n = i18n("en");
    let read = |text: &str| parse_duration(text, &i18n);
    assert_eq!(read("1 Std 30 Min"), Ok(hm(1, 30)));
    assert_eq!(read("2 Stunden 10 Sekunden"), Ok(hms(2, 0, 10)));
    assert_eq!(read("1 hora 5 minutos"), Ok(hm(1, 5)));
    assert_eq!(read("2 heures 5 minutes"), Ok(hm(2, 5)));
    assert_eq!(read("1 ч 30 мин"), Ok(hm(1, 30)));
    assert_eq!(read("5 часов 20 секунд"), Ok(hms(5, 0, 20)));
    assert_eq!(read("1小时30分钟"), Ok(hm(1, 30)));
    assert_eq!(read("1時間30分"), Ok(hm(1, 30)));
    assert_eq!(read("45秒"), Ok(Duration::from_secs(45)));
}

#[test]
fn every_built_in_language_explains_every_error() {
    for code in ["de", "es", "fr", "pt-BR", "ru", "zh-Hans", "ja"] {
        let i18n = i18n(code);
        for error in [
            DurationError::Empty,
            DurationError::Character('/'),
            DurationError::UnknownUnit("x".to_owned()),
            DurationError::BadNumber("1.2.3".to_owned()),
            DurationError::MissingNumber("h".to_owned()),
            DurationError::MissingUnit("1".to_owned()),
            DurationError::RepeatedUnit(DurationUnit::Hours),
            DurationError::BadClock("1:75".to_owned()),
            DurationError::TooLarge,
        ] {
            let message = error.message(&i18n);
            assert!(!message.contains('⟦') && !message.contains('{'), "{code} {error:?}: {message}");
        }
    }
}

#[test]
fn reads_numbers_with_units_in_both_languages_whatever_is_active() {
    for active in ["en", "tr"] {
        let i18n = i18n(active);
        let read = |text: &str| parse_duration(text, &i18n);
        assert_eq!(read("1 h 30 min"), Ok(hm(1, 30)));
        assert_eq!(read("1 sa 30 dk"), Ok(hm(1, 30)));
        assert_eq!(read("90 dk"), Ok(hm(1, 30)));
        assert_eq!(read("90 min"), Ok(hm(1, 30)));
        assert_eq!(read("1sa30dk"), Ok(hm(1, 30)), "no spaces needed");
        assert_eq!(read("1h30m"), Ok(hm(1, 30)));
        assert_eq!(read("2 hours 5 minutes 10 seconds"), Ok(hms(2, 5, 10)));
        assert_eq!(read("2 saat 5 dakika 10 saniye"), Ok(hms(2, 5, 10)));
        assert_eq!(read("45 sn"), Ok(Duration::from_secs(45)));
        assert_eq!(read("45 s"), Ok(Duration::from_secs(45)), "s is seconds, sa is hours");
        assert_eq!(read("30 dk 1 sa"), Ok(hm(1, 30)), "any order");
        assert_eq!(read("1 h, 30 min"), Ok(hm(1, 30)), "a comma between parts only separates");
        assert_eq!(read("  1 SA 30 DK  "), Ok(hm(1, 30)), "case and spaces do not matter");
        assert_eq!(read("5 DAKİKA"), Ok(hm(0, 5)), "Turkish capital İ");
        assert_eq!(read("5 DAKIKA"), Ok(hm(0, 5)), "or a plain I typed on another keyboard");
        assert_eq!(read("5 MIN"), Ok(hm(0, 5)), "an English capital I under a Turkish locale");
    }
}

#[test]
fn reads_clock_faces_bare_numbers_fractions_and_carries() {
    let i18n = i18n("en");
    let read = |text: &str| parse_duration(text, &i18n);
    assert_eq!(read("2:15"), Ok(hm(2, 15)), "hours and minutes, never minutes and seconds");
    assert_eq!(read("0:45:30"), Ok(hms(0, 45, 30)));
    assert_eq!(read("2:5"), Ok(hm(2, 5)));
    assert_eq!(read("25"), Ok(hm(0, 25)), "a bare number is minutes");
    assert_eq!(read("1 h 30"), Ok(hm(1, 30)), "a bare number after a unit takes the next smaller one");
    assert_eq!(read("2 min 30"), Ok(hms(0, 2, 30)));
    assert_eq!(read("1.5 h"), Ok(hm(1, 30)));
    assert_eq!(read("1,5 sa"), Ok(hm(1, 30)), "a decimal comma");
    assert_eq!(read("0.5 min"), Ok(Duration::from_secs(30)));
    assert_eq!(read("1 h 90 min"), Ok(hm(2, 30)), "minutes past 60 carry over");
    assert_eq!(read("0"), Ok(Duration::ZERO));
    assert_eq!(read("99:59:59"), Ok(hms(99, 59, 59)), "the longest length");
}

#[test]
fn says_precisely_what_it_cannot_read() {
    let i18n = i18n("en");
    let read = |text: &str| parse_duration(text, &i18n);
    assert_eq!(read("   "), Err(DurationError::Empty));
    assert_eq!(read(", ."), Err(DurationError::Empty));
    assert_eq!(read("-5 min"), Err(DurationError::Character('-')));
    assert_eq!(read("3 days"), Err(DurationError::UnknownUnit("days".to_owned())));
    assert_eq!(read("1.2.3 h"), Err(DurationError::BadNumber("1.2.3".to_owned())));
    assert_eq!(read(".5 h"), Err(DurationError::BadNumber(".5".to_owned())));
    assert_eq!(read("h 30"), Err(DurationError::MissingNumber("h".to_owned())));
    assert_eq!(read("1 30 min"), Err(DurationError::MissingUnit("1".to_owned())));
    assert_eq!(read("30 s 5"), Err(DurationError::MissingUnit("5".to_owned())), "nothing is below seconds");
    assert_eq!(read("1 h 2 h"), Err(DurationError::RepeatedUnit(DurationUnit::Hours)));
    assert_eq!(read("1 h 30"), Ok(hm(1, 30)));
    assert_eq!(read("1 min 1"), Ok(hms(0, 1, 1)));
    assert_eq!(read("1 min 2 s 3"), Err(DurationError::MissingUnit("3".to_owned())));
    assert_eq!(read("1:75"), Err(DurationError::BadClock("1:75".to_owned())));
    assert_eq!(read("1:30 h"), Err(DurationError::BadClock("1:30 h".to_owned())));
    assert_eq!(read("1::30"), Err(DurationError::BadClock("1::30".to_owned())));
    assert_eq!(read("1:2:3:4"), Err(DurationError::BadClock("1:2:3:4".to_owned())));
    assert_eq!(read("1:005"), Err(DurationError::BadClock("1:005".to_owned())));
    assert_eq!(read("100 h"), Err(DurationError::TooLarge));
    assert_eq!(read("100:00"), Err(DurationError::TooLarge));
    assert_eq!(read("99999999999999999999999 min"), Err(DurationError::TooLarge), "no overflow");
}

#[test]
fn reasons_come_from_the_language_files() {
    let en = i18n("en");
    let tr = i18n("tr");
    let unit = DurationError::UnknownUnit("days".to_owned());
    assert_eq!(unit.message(&en), "“days” is not a unit of time; use h, min or s");
    assert_eq!(unit.message(&tr), "“days” bir zaman birimi değil; sa, dk ya da sn kullan");
    assert_eq!(DurationError::RepeatedUnit(DurationUnit::Minutes).message(&en), "The minutes are written twice");
    assert_eq!(DurationError::RepeatedUnit(DurationUnit::Hours).message(&tr), "Saatler iki kez yazılmış");
    assert_eq!(DurationError::TooLarge.message(&en), "Longer than the longest length, 99 h 59 min 59 s");
    assert_eq!(DurationError::TooLarge.message(&tr), "En uzun süreden, 99 sa 59 dk 59 sn, daha uzun");
    for error in [
        DurationError::Empty,
        DurationError::Character('-'),
        DurationError::BadNumber("1.2.3".to_owned()),
        DurationError::MissingNumber("h".to_owned()),
        DurationError::MissingUnit("1".to_owned()),
        DurationError::BadClock("1:75".to_owned()),
    ] {
        for i18n in [&en, &tr] {
            let message = error.message(i18n);
            assert!(!message.contains('⟦') && !message.contains('{'), "{error:?}: {message}");
        }
    }
}

#[test]
fn unit_words_do_not_clash_across_languages() {
    let i18n = I18n::builtin();
    let mut seen: Vec<(String, DurationUnit)> = Vec::new();
    for (unit, stem) in DurationUnit::ALL.into_iter().zip(["hour", "minute", "second"]) {
        let key = format!("quvyta.duration.{stem}-words");
        for list in i18n.in_every_locale(&key) {
            for word in list.split(',').map(str::trim) {
                assert!(!seen.iter().any(|(other, owner)| other == word && *owner != unit), "`{word}` names two units");
                seen.push((word.to_owned(), unit));
            }
        }
    }
    assert!(seen.len() > 12, "both languages give words: {seen:?}");
}

// The field.

struct Demo {
    length: Duration,
    seconds: bool,
    disabled: bool,
    rejected: Vec<DurationError>,
    reject: bool,
}

#[derive(Debug, Clone)]
enum Msg {
    Changed(Duration),
    Rejected(DurationError),
}

impl App for Demo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Changed(length) => self.length = length,
            Msg::Rejected(error) => self.rejected.push(error),
        }
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        let mut field =
            DurationInput::new(self.length).seconds(self.seconds).disabled(self.disabled).on_change(Msg::Changed);
        if self.reject {
            field = field.on_reject(Msg::Rejected);
        }
        ui.add(field).id("target");
    }
}

fn demo(length: Duration, seconds: bool) -> Demo {
    Demo { length, seconds, disabled: false, rejected: Vec::new(), reject: false }
}

fn harness(length: Duration, seconds: bool) -> Harness<Demo> {
    Harness::new(demo(length, seconds), 24, 3)
}

#[test]
fn draws_digits_with_faint_unit_words_in_the_active_language() {
    let mut h = harness(hm(1, 30), false);
    assert_eq!(h.screen().lines().next(), Some("  1 h 30 min"));
    let theme = h.env().theme();
    assert_eq!(h.fg(4, 0), theme.color("muted"), "the unit word is faint");
    assert_ne!(h.fg(2, 0), theme.color("muted"), "the digits are not");
    h.set_locale("tr");
    assert_eq!(h.screen().lines().next(), Some("  1 sa 30 dk"));
    let mut h = harness(hms(12, 5, 9), true);
    assert_eq!(h.screen().lines().next(), Some(" 12 h 05 min 09 s"));
    h.set_glyph_mode(GlyphMode::Ascii);
    assert!(h.screen().is_ascii());
    assert!(!h.screen().contains(['[', ']', '(', ')', '|']), "no brackets or bars: {}", h.screen());
}

#[test]
fn the_active_segment_is_raised_and_the_pillar_stays_in_its_cell() {
    let mut h = harness(hm(1, 30), false);
    let idle = h.bg(2, 0);
    h.press("tab");
    assert_eq!(h.screen().lines().next(), Some("▌ 1 h 30 min"), "nothing slides");
    assert_ne!(h.bg(2, 0), h.bg(7, 0), "the active segment stands out");
    assert_ne!(h.bg(2, 0), idle);
    h.press("right");
    assert_eq!(h.bg(2, 0), h.bg(4, 0), "the hours are back on the field surface");
    assert_eq!(h.screen().lines().next(), Some("▌ 1 h 30 min"));
}

#[test]
fn a_zero_length_rests_faint_until_hovered_or_focused() {
    let mut h = harness(Duration::ZERO, false);
    assert_eq!(h.screen().lines().next(), Some("  0 h 00 min"));
    let muted = h.env().theme().color("muted");
    assert_eq!((h.fg(2, 0), h.fg(6, 0)), (muted, muted), "the empty state");
    h.hover(6, 0);
    assert_ne!(h.fg(6, 0), muted, "hover brings the digits back");
    h.hover(6, 2).press("tab");
    assert_ne!(h.fg(6, 0), muted, "and so does focus");
    let mut h = harness(hm(0, 1), false);
    assert_ne!(h.fg(6, 0), muted, "any length that is not zero is written plainly");
    h.press("tab");
}

#[test]
fn arrows_step_one_unit_and_carry_between_units() {
    let mut h = harness(hm(0, 59), false);
    h.press("tab").press("right").press("up");
    assert_eq!(h.app().length, hm(1, 0), "0 h 59 min up is 1 h 00 min");
    h.press("down");
    assert_eq!(h.app().length, hm(0, 59));
    h.press("left").press("up");
    assert_eq!(h.app().length, hm(1, 59), "the hours step by an hour");
    h.press("down").press("down");
    assert_eq!(h.app().length, Duration::ZERO, "down stops at zero");
    h.press("down");
    assert_eq!(h.app().length, Duration::ZERO);
    h.send(Msg::Changed(hms(99, 59, 0))).press("up");
    assert_eq!(h.app().length, hms(99, 59, 59), "up stops at the longest length");
}

#[test]
fn typing_fills_two_digits_a_segment_and_carries_minutes_past_59() {
    let mut h = harness(Duration::ZERO, false);
    h.press("tab").type_text("0130");
    assert_eq!(h.app().length, hm(1, 30));
    h.press("left").press("backspace");
    assert_eq!(h.app().length, hm(0, 30), "backspace zeroes the active hours");
    h.press("right").type_text("90");
    assert_eq!(h.app().length, hm(1, 30), "90 minutes carry into the hours");
    assert_eq!(h.screen().lines().next(), Some("▌ 1 h 30 min"));
    h.press("left").type_text("2").press(":").type_text("15");
    assert_eq!(h.app().length, hm(2, 15), "`:` moves on after a single digit");
    h.press("left").type_text("3").press("space").type_text("05");
    assert_eq!(h.app().length, hm(3, 5), "so does space");
}

#[test]
fn seconds_are_a_third_segment_and_hidden_seconds_are_kept() {
    let mut h = harness(hms(0, 1, 30), true);
    h.press("tab").press("right").press("right").press("up");
    assert_eq!(h.app().length, hms(0, 1, 31));
    h.type_text("75");
    assert_eq!(h.app().length, hms(0, 2, 15), "75 seconds carry into the minutes");
    let mut h = harness(hms(0, 10, 20), false);
    h.press("tab").press("right").type_text("15");
    assert_eq!(h.app().length, hms(0, 15, 20), "seconds off screen stay as they were");
}

#[test]
fn the_wheel_changes_the_segment_under_the_pointer_like_the_time_field() {
    let mut h = harness(hm(1, 15), false);
    h.hover(7, 0).mouse(MouseKind::ScrollUp, 7, 0).mouse(MouseKind::ScrollUp, 7, 0);
    assert_eq!(h.app().length, hm(1, 17), "one minute per notch, no click needed");
    assert!(!h.is_focused("target"), "the wheel does not take focus");
    h.hover(2, 0).mouse(MouseKind::ScrollDown, 2, 0);
    assert_eq!(h.app().length, hm(0, 17), "one hour per notch");
    // Over a unit word: the segment last hovered inside the field.
    h.hover(7, 0).hover(10, 0).mouse(MouseKind::ScrollUp, 10, 0);
    assert_eq!(h.app().length, hm(0, 18));
    h.hover(2, 0).hover(4, 0).mouse(MouseKind::ScrollUp, 4, 0);
    assert_eq!(h.app().length, hm(1, 18));
    // Leaving the field forgets it: straight onto a word, the active segment changes.
    h.click(7, 0);
    h.hover(7, 2).hover(4, 0).mouse(MouseKind::ScrollDown, 4, 0);
    assert_eq!(h.app().length, hm(1, 17), "the active minutes");
    h.press("up");
    assert_eq!(h.app().length, hm(1, 18), "the keys still change the active segment");
}

#[test]
fn the_wheel_does_nothing_on_a_disabled_field_and_disabled_ignores_input() {
    let mut h = Harness::new(Demo { disabled: true, ..demo(hm(1, 15), false) }, 24, 1);
    h.hover(7, 0).mouse(MouseKind::ScrollUp, 7, 0).press("tab").press("up").click(2, 0).type_text("1");
    h.paste("2 h");
    assert_eq!(h.app().length, hm(1, 15));
    let muted = h.env().theme().color("muted");
    assert_eq!(h.fg(7, 0), muted);
}

#[test]
fn select_all_copies_cuts_and_pastes_in_any_form() {
    let mut h = Harness::new(demo(hm(1, 30), false), 30, 6);
    h.set_reduced_motion(true);
    h.press("tab").press("ctrl+c");
    assert!(h.copied().is_empty(), "nothing is selected yet");
    h.press("ctrl+a");
    let selection = h.env().theme().style("text-input-selection", None, &[]).paint("bg").map(|paint| paint.at(0.0));
    assert_eq!((h.bg(2, 0), h.bg(4, 0), h.bg(9, 0)), (selection, selection, selection), "unit words included");
    h.press("ctrl+c");
    assert_eq!(h.clipboard(), Some("1 h 30 min"));
    h.press("ctrl+a").press("ctrl+x");
    assert_eq!(h.app().length, Duration::ZERO);
    h.paste("90 dk");
    assert_eq!(h.app().length, hm(1, 30));
    h.paste("2:15");
    assert_eq!(h.app().length, hm(2, 15));
    h.paste("1 saat 5 dakika");
    assert_eq!(h.app().length, hm(1, 5));
    h.paste("3 days").paste("noon");
    assert_eq!(h.app().length, hm(1, 5), "text that is no length is ignored");
    h.set_locale("tr").press("ctrl+a").press("ctrl+c");
    assert_eq!(h.clipboard(), Some("1 sa 5 dk"), "copied in the active language");
}

#[test]
fn a_rejected_paste_can_tell_the_application_why() {
    let mut h = Harness::new(Demo { reject: true, ..demo(hm(1, 30), false) }, 30, 3);
    h.press("tab").paste("3 days").paste("1:75");
    assert_eq!(h.app().length, hm(1, 30));
    assert_eq!(
        h.app().rejected,
        [DurationError::UnknownUnit("days".to_owned()), DurationError::BadClock("1:75".to_owned())]
    );
}

#[test]
fn right_click_opens_the_edit_menu() {
    let mut h = Harness::new(demo(hms(1, 30, 0), true), 30, 6);
    h.set_reduced_motion(true);
    h.mouse(MouseKind::Down(MouseButton::Right), 7, 0).mouse(MouseKind::Up(MouseButton::Right), 7, 0);
    assert!(h.screen().contains("Select all"), "{}", h.screen());
    let muted = h.env().theme().color("muted");
    let (x, y) = h.find("Copy").expect("the menu lists Copy");
    let (x, y) = (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0));
    assert_eq!(h.fg(x, y), muted, "Copy needs the whole length selected: {}", h.screen());
    h.click_text("Select all");
    h.mouse(MouseKind::Down(MouseButton::Right), 7, 0).mouse(MouseKind::Up(MouseButton::Right), 7, 0);
    h.click_text("Copy");
    assert_eq!(h.clipboard(), Some("1 h 30 min"));
    h.set_system_clipboard(Some("45 sn")).mouse(MouseKind::Down(MouseButton::Right), 2, 0);
    h.click_text("Paste");
    assert_eq!(h.app().length, Duration::from_secs(45));
}

#[test]
fn a_click_activates_the_segment_or_the_one_before_a_unit_word() {
    let mut h = harness(hm(1, 15), false);
    h.click(7, 0).press("up");
    assert_eq!(h.app().length, hm(1, 16));
    h.click(4, 0).press("up");
    assert_eq!(h.app().length, hm(2, 16), "the word `h` belongs to the hours");
    h.click(11, 0).press("up");
    assert_eq!(h.app().length, hm(2, 17), "the word `min` belongs to the minutes");
}

#[test]
fn narrow_fields_fall_back_to_the_clock_form() {
    let mut h = Harness::new(demo(hm(1, 30), false), 10, 1);
    assert_eq!(h.screen(), "  1 : 30\n", "the words do not fit in ten cells");
    h.hover(6, 0).mouse(MouseKind::ScrollUp, 6, 0);
    assert_eq!(h.app().length, hm(1, 31), "the segments move with the form");
    h.click(1, 0).type_text("2");
    assert_eq!(h.app().length, hm(2, 31));
    let tiny = Harness::new(demo(hms(1, 30, 5), true), 4, 1);
    assert_eq!(tiny.screen(), "  1\n", "cut at the edge, never wrapped");
    let empty = Harness::new(demo(hm(1, 30), false), 0, 0);
    assert_eq!(empty.screen(), "");
}

#[test]
fn lengths_beyond_the_longest_are_shown_whole_and_only_step_down() {
    let mut h = harness(hm(250, 0), false);
    assert_eq!(h.screen().lines().next(), Some(" 250 h 00 min"), "the hours widen rather than lie");
    h.press("tab").press("up");
    assert_eq!(h.app().length, hm(250, 0), "up does nothing past the longest length");
    h.press("down");
    assert_eq!(h.app().length, hm(249, 0));
}
