## When to use

Use a date picker when the user chooses a single day and seeing the surrounding weeks helps: a release date, a maintenance window, a report range start. For dates far in the past, such as a birthday, typing is faster; for a time of day, use a time field.

## Step by step

1. Keep the value in your state as `Option<Date>`; build dates with `Date::new(2026, 9, 16)`.
2. Show it: `DatePicker::new(self.release)`.
3. Receive choices: `.on_change(Msg::Release)`.
4. Say what to choose while it is empty: `.placeholder(t!("choose-date"))`.
5. Today is marked on the machine's own day, from the system clock and time zone. Pass `.today(date)` when a different day should count as today, such as the day of a user in another zone.

## How it works

- **The field reads like a select** and writes the date the way the language does: "September 16, 2026" in English, "16 Eylül 2026" in Turkish.
- **The calendar is a layer** on the overlay surface that unfolds below the field over `motion.enter`, or above it when there is no room.
- **Six weeks, always.** The grid keeps its height from month to month. The chosen day is filled with the accent, today carries the accent in bold underlined digits, days of the neighbouring months are faint, and the highlighted day raises its surface.
- **The region decides the week, or else the language.** Month and weekday names come from the `quvyta.date` keys. The first day of the week comes from `I18n::first_weekday()`: when the system names a country (`LANG=en_GB.UTF-8`) it is that country's day from the Unicode CLDR, Monday in the United Kingdom and Sunday in the United States; without one it is the language's `quvyta.date.first-weekday` key, Sunday for English and Monday for Turkish, and Monday for a language that does not give it. Call the same method wherever your application counts weeks, so a weekly goal and the calendar agree; in `update`, where there is no `Env`, `qframe::i18n::first_weekday()` gives the same answer.
- **An application can name the region.** `Command::set_locale("en-GB")` switches to English and to the British week at once; a code without a region, such as `tr`, keeps the region the system gave. `Command::set_region(Some("GB"))` changes the region alone and `Command::set_region(None)` leaves the week to the language again.
- **Keys:** ← and → move a day, ↑ and ↓ a week, PgUp and PgDn a month, Shift with them a year, Home and End the ends of the week, Enter or Space chooses, Esc closes. The arrows beside the month (three cells each, lit under the pointer) and the mouse wheel change the month; clicking a day chooses it.
- **One highlighted day.** The keyboard and the pointer move the same highlight. Pointing at a day of a neighbouring month lights it without turning the calendar; the keys go on from the day you last pointed at in the shown month.
- **The pillar marks what you point at, and nothing slides.** The field, a month arrow and the highlighted day show the pillar `▌` in their leftmost cell. That cell is always blank (a day is four cells: a blank, two digits, a blank), so no label or number moves, even with slide turned on in the settings. The pillar is soft under the pointer and breathes only when the keyboard moved the highlight last. The chosen day keeps its accent fill and shows a dark pillar when highlighted.
- **A click elsewhere closes the calendar and still counts**, on a button, a menu entry or another field. A click on the date field itself only closes it.
- **Dates are plain values.** `Date` has no time or zone. It adds days and months (January 31 plus a month is the last day of February), tells the weekday and counts days between dates with `to_days`.

## Common mistakes

- **Storing the formatted text.** Keep the `Date`; the text changes with the language.
- **Marking today with `Date::today_utc()`.** Near midnight the UTC day is not the user's day; leave `.today(…)` out to get the local day, or pass `Date::today_local()` yourself.
- **Expecting the calendar to slide like a list.** Slide is for lists, menus and tabs. The date picker is a grid of fixed columns, so it never slides and shows the pillar instead.
- **Using it for ranges.** Two pickers work, as in the demo; check the order in `update`.
