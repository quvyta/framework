## When to use

Reach for these when your application measures or stores time: how long someone worked, which day a record belongs to, a date written into a file. They are plain values with no widget attached, so `Date` and `TimeOfDay` are what the date picker and the time field hand you, and what you keep in your own state.

Four questions, four answers:

- **How long has this been running?** `Uptime`, because it separates time worked from time the machine slept.
- **What day is it here?** `Date::today_local()` and `DateTime::now_local()`, because a wall clock alone does not know the zone.
- **Is anybody here?** `ui.idle_for()` and `ui.on_idle`, because an application never sees the keys itself.
- **What does the file say?** `Date::parse` and `Display`, because a stored date is text.

## Step by step

1. Take a reading when the work starts: `let started = Uptime::now();`.
2. Take another one later and ask the difference: `now.awake - started.awake` is the time worked, `now.suspended_since(&started)` the time the machine slept.
3. Tell your user when the answer is unknown: `Uptime::detects_suspend()` is `false` on every platform but Linux and Android, and there the slept time stays at zero rather than being invented.
4. For the day a record belongs to, use `Date::today_local()`; for a moment, `DateTime::now_local()`, which carries the offset with it.
5. Store a moment as `to_unix()` and read it back with `DateTime::from_unix(seconds, offset)`. Store a day as `date.to_string()` and read it back with `Date::parse`.

6. To notice that nobody is at the keyboard, declare a watch in `view`: `ui.on_idle(Duration::from_secs(300), Msg::Away)`. `update` gets `Msg::Away(true)` when five minutes pass without input and `Msg::Away(false)` at the next key or pointer movement. To show the silence, read `ui.idle_for()`.

## How it works

- **Two monotonic clocks, not a wall clock.** A wall clock lies twice over: a time-synchronisation step moves it while nobody slept, and on a platform whose monotonic clock keeps running through a suspend it moves by exactly as much as that clock, so the gap is zero and eight hours of sleep count as eight hours of work. The gap between `CLOCK_MONOTONIC`, which stops while the machine sleeps, and `CLOCK_BOOTTIME`, which does not, is the sleep by definition, and no clock correction touches it.
- **A platform that cannot answer says so.** Only Linux and Android offer the pair from safe code, so elsewhere `elapsed` equals `awake`, the slept time is zero and `Uptime::detects_suspend()` returns `false`. Show the user that, rather than a number nobody measured.
- **A reading means nothing alone.** `awake` and `elapsed` count from the machine's boot, or from the first reading in the process where the platform offers no boot clock. Keep the reading you started from and compare.
- **The local offset comes from the system.** It is read from the zone file `TZ` names, or `/etc/localtime`, once per process, and the file holds every offset change of the zone, so daylight saving is right without any rule of ours. Where there is no such file, `local_offset()` is `None`; `local_offset_minutes()` then answers 0, which is UTC, so use `local_offset()` when the difference matters.
- **`DateTime` carries its offset.** The date and time inside it are what a clock on the wall shows; `offset_minutes` is what turns them back into an instant. Two `DateTime`s with different offsets can be the same instant, as the playground shows.
- **ISO dates are checked against the calendar.** `Date::parse` wants `YYYY-MM-DD` and refuses 2026-13-40 and 2026-02-30; February 29 exists in 2024 and 2000 but not in 2026 or 2100. `Display` writes the same form back, so text goes out the way it came in.

- **Idleness is the runtime's clock.** An application never sees keys or the mouse; the runtime does, and it notes the moment of every key (press, repeat, release), every mouse event, the pointer moving over the window included, every paste and the end of a handoff. A resize is not the user: a window manager resizes windows nobody sits at. Before any input, the silence counts from the start.
- **The application is woken, not polled.** `on_idle(after, …)` puts one wake-up at the moment the silence reaches `after`; nothing is drawn in between. A view that reads `idle_for()` is drawn again on each whole second while it reads it, so the number on the screen is never stale, and stops when it no longer does. The demo on this page does both.

## Common mistakes

- **Measuring work with the wall clock.** `SystemTime` moves when the system's time is corrected, and a correction of ninety seconds silently deletes ninety seconds of work.
- **Treating an unknown offset as UTC.** `local_offset_minutes()` answers 0 when the system does not say. If your application writes the answer down, ask `local_offset()` first.
- **Storing the local time without the offset.** An hour written as `09:00` is not a moment until the offset says which `09:00`. Store `to_unix()`, or store the offset with it.
- **Trusting UTC for "today".** Near midnight the UTC day and the local day are different days; that is a record filed a day late.
- **Writing a date by hand.** `format!("{:04}-{:02}-{:02}", …)` is what `Display` already does, and `Date::parse` is what reads it back.
- **Measuring idleness with your own timer.** A task that ticks every second knows nothing about keys; `on_idle` is told the silence, and it does not keep the application awake while waiting.
- **Declaring the watch only while waiting for it.** Keep `on_idle` in every frame the watch should live, or it is not told when input comes back.
