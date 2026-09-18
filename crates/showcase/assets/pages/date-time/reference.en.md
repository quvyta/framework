## Uptime

- `Uptime::now()` — reads both monotonic clocks at once.
- `.awake` — time counted by the clock that stops while the machine sleeps.
- `.elapsed` — time counted by the clock that keeps going; equal to `.awake` where there is no second clock.
- `.suspended_since(&earlier)` — how long the machine slept between two readings; zero when the readings are in the wrong order, and zero where sleep cannot be told apart.
- `Uptime::detects_suspend()` — `true` on Linux and Android, `false` everywhere else.
- Clocks used: Linux and Android `CLOCK_MONOTONIC` and `CLOCK_BOOTTIME`, read through `rustix`, so no `unsafe` is needed. Other Unix systems read `CLOCK_MONOTONIC` alone; macOS's `CLOCK_UPTIME_RAW` and Windows's `QueryUnbiasedInterruptTime` need a foreign function call and are not read. Off Unix, `.awake` is measured from the first reading in the process.

## Date

- `Date::new(year, month, day) -> Option<Date>`, `Date::today_utc()`, `Date::today_local()`, `Date::from_days(n)`.
- `Date::parse(text) -> Result<Date, String>` and `text.parse::<Date>()` — `YYYY-MM-DD`, four or more digits of year, two each of month and day, an optional leading `-`, spaces around it ignored. The calendar is checked.
- `Display` — writes `YYYY-MM-DD`, the form `parse` reads.
- `.year()`, `.month()`, `.day()`, `.weekday()`, `.to_days()` (days since 1970-01-01).
- `.add_days(n)`, `.add_months(n)`, `.first_of_month()`, `.start_of_week(Weekday)`.
- `date::is_leap_year(year)`, `date::days_in_month(year, month)`.

## TimeOfDay

- `TimeOfDay::new(hour, minute, second)` — each part capped at its largest value; `TimeOfDay::LARGEST` is 23:59:59.
- `TimeOfDay::parse(text) -> Option<TimeOfDay>` — `h:mm` or `h:mm:ss`; out of range is refused, not capped.
- `.seconds_since_midnight()`, `TimeOfDay::from_seconds_since_midnight(n)` (a value past a day wraps).
- `Display` — `hh:mm:ss`. The fields `hour`, `minute`, `second` are public and times compare in clock order.
- It lives in `date` and is re-exported from `widgets`, so `TimeInput` and `date` speak of the same type.

## DateTime and the local offset

- `DateTime { date, time, offset_minutes }` — a local date and time with the offset from UTC that belongs to them.
- `DateTime::now_local()`, `DateTime::from_unix(seconds, offset_minutes)`, `.to_unix()`.
- `date::local_offset() -> Option<i16>` — minutes local time is ahead of UTC, `None` when the system does not say.
- `date::local_offset_minutes() -> i16` — the same, with 0 for unknown.
- Source: the TZif file `TZ` names, or `/etc/localtime`, read once per process. A `TZ` holding a POSIX rule such as `EST5EDT,M3.2.0,M11.1.0` is not parsed and counts as unknown, and moments past the last change the file records keep the last offset it gives.

## Input idleness

- `ui.idle_for() -> Duration` — how long no input has reached this terminal; from the start of the application until the first input. Reading it draws the view again on each whole second of silence, while the view reads it.
- `ui.on_idle(after, |away| …)` — `message(true)` once, at the moment the silence reaches `after`; `message(false)` at the first input after that, before the input reaches a widget. Declare it in every frame it should stay active; watches with different `after` are independent.
- Input: key press, repeat and release; mouse buttons, wheel, drags and the pointer moving over the window; paste; the end of a `Handoff`.
- Not input: a terminal resize, messages, perform work and tasks, events the runtime makes itself.
- Tests: `Harness::advance` moves the silence forward and tells due watches; `press`, `click`, `hover`, `paste` and `events` start it again; `send` and `resize` do not.
- Out of scope: idleness of the whole machine, other terminals and other programs.

## Keys

- The date field takes text like any text input; nothing on this page has keys of its own.
