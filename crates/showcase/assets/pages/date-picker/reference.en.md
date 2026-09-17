## Methods

- `DatePicker::new(Option<Date>)` — the chosen date.
- `.on_change(|date| msg)` — sent when a different day is chosen.
- `.placeholder(text)` — shown while no date is chosen.
- `.today(Date)` — the day marked as today. Default: `Date::today_utc()`.
- `.disabled(bool)` — not focusable, cannot open. Default: `false`.

## Date

- `Date::new(year, month, day) -> Option<Date>`, `Date::today_utc()`, `Date::from_days(n)`.
- `.year()`, `.month()`, `.day()`, `.weekday()`, `.to_days()` (days since 1970-01-01).
- `.add_days(n)`, `.add_months(n)` (clamps to the month's last day), `.first_of_month()`, `.start_of_week(Weekday)`.
- `date::is_leap_year(year)`, `date::days_in_month(year, month)`; `Weekday::ALL`, `.number()`, `Weekday::from_number(n)`, `.days_since(start)` (days forward from `start`, `0..7`).

## Keys

- Closed: `enter`, `space` or `down` opens.
- Open: `left` `right` a day, `up` `down` a week, `pgup` `pgdn` a month, `shift pgup` `shift pgdn` a year, `home` `end` the ends of the week, `enter` or `space` chooses, `esc` closes, `tab` closes and moves focus.

## Mouse

- Click the field to open or close; click a day to choose it; click the arrows or use the wheel to change the month.
- Moving the pointer over the days moves the one highlighted day.
- The field, each month arrow and the highlighted day show the pillar in their leftmost, always blank cell. Nothing slides, whatever the slide setting says. The pillar breathes only when the keyboard moved the highlight last.
- A click outside closes and still reaches what it landed on.

## Locale keys

- `quvyta.date.month-1` … `month-12`, `weekday-1` (Monday) … `weekday-7` (Sunday).
- `quvyta.date.first-weekday` — `1` Monday to `7` Sunday.
- `quvyta.date.format` — the field, with `{day}`, `{month}`, `{year}`; `quvyta.date.title` — the calendar title.

## Theme keys

- Field: `select`, `select-placeholder`, `select-chevron`.
- `calendar` — `bg` (default `$overlay`), `padding` (default `[1, 2]`).
- `calendar-title`, `calendar-arrow` with `hover` (`bg` lights its three cells, `pillar` in its first cell), `calendar-weekday`.
- `calendar-day` with `hover` (the highlighted day: `bg`, `pillar`), `hover:focus` (`pillar` while the keyboard moved it), `selected`, `selected:hover` and `selected:hover:focus` (`pillar` on the accent fill); variants `outside` and `today`.
