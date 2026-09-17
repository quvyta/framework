## When to use

Use a date picker when the user chooses a single day and seeing the surrounding weeks helps: a release date, a maintenance window, a report range start. For dates far in the past, such as a birthday, typing is faster; for a time of day, use a time field.

## Step by step

1. Keep the value in your state as `Option<Date>`; build dates with `Date::new(2026, 9, 16)`.
2. Show it: `DatePicker::new(self.release)`.
3. Receive choices: `.on_change(Msg::Release)`.
4. Say what to choose while it is empty: `.placeholder(t!("choose-date"))`.
5. If your application knows the user's time zone, pass the local day with `.today(date)`; otherwise today comes from the system clock in UTC.

## How it works

- **The field reads like a select** and writes the date the way the language does: "September 16, 2026" in English, "16 Eylül 2026" in Turkish.
- **The calendar is a layer** on the overlay surface that unfolds below the field over `motion.enter`, or above it when there is no room.
- **Six weeks, always.** The grid keeps its height from month to month. The chosen day is filled with the accent, today carries the accent in bold underlined digits, days of the neighbouring months are faint, and the highlighted day raises its surface.
- **The language decides the week.** Month and weekday names and the first day of the week come from the `quvyta.date` keys: English weeks start on Sunday, Turkish weeks on Monday. Your own locale files can change them.
- **Keys:** ← and → move a day, ↑ and ↓ a week, PgUp and PgDn a month, Shift with them a year, Home and End the ends of the week, Enter or Space chooses, Esc closes. The arrows beside the month (three cells each, lit under the pointer) and the mouse wheel change the month; clicking a day chooses it.
- **One highlighted day.** The keyboard and the pointer move the same highlight. Pointing at a day of a neighbouring month lights it without turning the calendar; the keys go on from the day you last pointed at in the shown month.
- **The pillar marks what you point at, and nothing slides.** The field, a month arrow and the highlighted day show the pillar `▌` in their leftmost cell. That cell is always blank (a day is four cells: a blank, two digits, a blank), so no label or number moves, even with slide turned on in the settings. The pillar is soft under the pointer and breathes only when the keyboard moved the highlight last. The chosen day keeps its accent fill and shows a dark pillar when highlighted.
- **A click elsewhere closes the calendar and still counts**, on a button, a menu entry or another field. A click on the date field itself only closes it.
- **Dates are plain values.** `Date` has no time or zone. It adds days and months (January 31 plus a month is the last day of February), tells the weekday and counts days between dates with `to_days`.

## Common mistakes

- **Storing the formatted text.** Keep the `Date`; the text changes with the language.
- **Trusting UTC for "today" near midnight.** Pass `.today(…)` when the local day matters.
- **Expecting the calendar to slide like a list.** Slide is for lists, menus and tabs. The date picker is a grid of fixed columns, so it never slides and shows the pillar instead.
- **Using it for ranges.** Two pickers work, as in the demo; check the order in `update`.
