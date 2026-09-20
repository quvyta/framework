//! What a language does with numbers, dates and the width of its words, in the text the framework
//! itself draws. These sit beside an application's own words on one screen, so a point where the
//! language writes a comma, or a date in the other order, reads as a mistake.

use std::sync::Arc;

use qframe::date::Date;
use qframe::i18n::I18n;

/// Runs `f` with `code` as the active language.
fn in_language<R>(code: &str, f: impl FnOnce() -> R) -> R {
    let mut i18n = I18n::builtin();
    assert!(i18n.set_active(code), "{code} is a built-in language");
    qframe::i18n::scope(Arc::new(i18n), f)
}

#[test]
fn a_language_that_writes_a_comma_gets_a_comma() {
    // A point in English, Japanese and Chinese; a comma in the rest.
    for code in ["en", "ja", "zh-Hans"] {
        assert_eq!(in_language(code, || qframe::i18n::number(0.5, 1)), "0.5", "{code}");
        assert_eq!(in_language(code, qframe::i18n::decimal_separator), '.', "{code}");
    }
    for code in ["de", "fr", "es", "ru", "pt-BR", "tr"] {
        assert_eq!(in_language(code, || qframe::i18n::number(0.5, 1)), "0,5", "{code}");
        assert_eq!(in_language(code, qframe::i18n::decimal_separator), ',', "{code}");
    }
    // Whole numbers carry no separator at all.
    assert_eq!(in_language("fr", || qframe::i18n::number(12.0, 0)), "12");
}

#[test]
fn every_language_says_what_it_writes_between_a_number_and_its_decimals() {
    let i18n = I18n::builtin();
    for (code, _) in i18n.list() {
        assert!(i18n.has(&code, "quvyta.number.decimal"), "{code} gives its own separator");
    }
}

#[test]
fn a_date_reads_in_the_order_its_language_writes_dates_in_every_form() {
    let day = Date::new(2026, 9, 18).expect("a real date");
    let cases = [
        ("en", "September 18, 2026", "Sep 18, 2026", "September 18", "Sep 18"),
        ("tr", "18 Eylül 2026", "18 Eyl 2026", "18 Eylül", "18 Eyl"),
        ("de", "18. September 2026", "18. Sep 2026", "18. September", "18. Sep"),
        ("ja", "2026年9月18日", "2026年9月18日", "9月18日", "9月18日"),
    ];
    for (code, long, short, day_month, day_month_short) in cases {
        assert_eq!(in_language(code, || day.written()), long, "{code}");
        assert_eq!(in_language(code, || day.written_short()), short, "{code}");
        // The heading form and the field form put the day and the month the same way round.
        assert_eq!(in_language(code, || day.day_and_month()), day_month, "{code}");
        assert_eq!(in_language(code, || day.day_and_month_short()), day_month_short, "{code}");
    }
}

#[test]
fn every_language_gives_the_day_and_month_with_the_whole_month_name() {
    let i18n = I18n::builtin();
    for (code, _) in i18n.list() {
        assert!(i18n.has(&code, "quvyta.date.format-day-month-long"), "{code} gives the heading form");
    }
}

#[test]
fn the_chinese_minute_is_one_character_beside_a_number() {
    // `分钟` is the word on its own; after a number, and in a column of a table, Chinese writes
    // `分`, which is also what an application's own unit says.
    let unit = in_language("zh-Hans", || qframe::t!("quvyta.duration.minutes"));
    assert_eq!(unit, "分");
    let span = in_language("zh-Hans", || qframe::t!("quvyta.time.hours-minutes", hours = 1, minutes = 30));
    assert_eq!(span, "1小时30分");
    // The long word is still read when someone types it.
    let words = in_language("zh-Hans", || qframe::t!("quvyta.duration.minute-words"));
    assert!(words.contains("分钟"), "{words}");
}

/// A screen with the three framework parts that write a number, a date and a length of time.
mod screen {
    use qframe::date::Date;
    use qframe::prelude::*;
    use qframe::widgets::{Bar, BarChart, DatePicker, DurationInput, NumberInput};

    #[derive(Default)]
    pub struct Customs {
        pub hours: f64,
    }

    impl App for Customs {
        type Msg = f64;
        fn update(&mut self, hours: f64) -> Command<f64> {
            self.hours = hours;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, f64>) {
            ui.add(BarChart::new([Bar::new("Mon", 0.5)]).unit("h")).fill_width().id("chart");
            ui.add(NumberInput::new(self.hours).range(0.0, 9.0).step(0.5).on_change(|value| value))
                .width(Length::Cells(12))
                .id("number");
            ui.add(DurationInput::new(std::time::Duration::from_secs(90 * 60))).width(Length::Cells(16)).id("duration");
            ui.add(DatePicker::new(Date::new(2026, 9, 18))).width(Length::Cells(30)).id("date");
        }
    }
}

#[test]
fn the_numbers_dates_and_units_a_screen_draws_follow_its_language() {
    use qframe::runtime::Harness;

    let mut h = Harness::new(screen::Customs { hours: 1.5 }, 40, 12);
    assert!(h.screen().contains("0.5 h"), "English writes a point:\n{}", h.screen());
    assert!(h.screen().contains("1.5"), "and so does the field:\n{}", h.screen());
    assert!(h.screen().contains("September 18, 2026"), "{}", h.screen());

    h.set_locale("fr");
    assert!(h.screen().contains("0,5 h"), "French writes a comma in the chart:\n{}", h.screen());
    assert!(h.screen().contains("1,5"), "and in the field:\n{}", h.screen());
    // Typed back with the comma the language shows, the field reads it as a number.
    assert!(h.screen().contains("18 septembre 2026"), "and the date reads in French order:\n{}", h.screen());

    h.set_locale("zh-Hans");
    assert!(h.screen().contains("分"), "{}", h.screen());
    assert!(!h.screen().contains("分钟"), "the long word does not fit beside a number:\n{}", h.screen());
}
