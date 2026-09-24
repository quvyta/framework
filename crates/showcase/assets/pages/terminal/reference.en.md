## Methods

- `Terminal::new(&session)` — draws a session; available with the `pty` feature.
- `.pass_through(scope, action)` — the keys bound to that keymap action skip the program and go on to ancestors, key listeners and `App::action`; call it once per action. The keymap in force when the key arrives decides, so rebinding follows. Keys that type a character (a character or space with at most `shift`) always reach the program. Default: nothing passes but `shift tab` and `ctrl q`.
- `.read_only()` — a terminal that only shows: it never writes to the session (no key, no paste, no mouse report, no wheel on the alternate screen, and no size request), takes no focus by Tab or by a click, and is drawn faint. Colours, wide characters, the cursor the program left, selecting the output and scrolling back are unchanged. For a window kept open after its program ended.
- `NodeMut::on_action(scope, action, msg)` on the terminal's node — a passed action sends `msg` while focus is in the terminal and reaches `App::action` elsewhere; the way to toggle focus with one key.
- `TerminalSession::shell(folder)`, `TerminalSession::spawn(program, args, folder)` — start a program in a pseudo-terminal.
- `TerminalSession::builder(program)` — a `TerminalBuilder`: `.args(args)`, `.folder(folder)` (default: the home folder), `.env(name, value)`, `.env_remove(name)` (the program does not see the variable at all, not even empty; the last `env` or `env_remove` of a name wins), `.size(columns, rows)` (default 80 × 24), `.scrollback(lines)` (default 5000), `.coalesce(interval)` (default zero), then `.spawn()`. `spawn` is the builder with these defaults.
- `.watch()` — a `TerminalWatch`; `.write(bytes)`, `.kill()`, `.exit()`. `write` counts as the person's input, because that is the path a keystroke takes; it fails once the program's end is known.
- `.paste(text)` — sends `text` as a paste: between `\x1b[200~` and `\x1b[201~` when the program turned bracketed paste on, plain when it did not. Both markers are removed from `text` first. Fails once the program's end is known. Does not move `last_input`: this is the application writing. The `Terminal` widget sends a person's paste through the same call, and that one does move `last_input`.
- `.last_output()` — an `Instant`: when the program last wrote anything, marked by the reading thread as the bytes arrive, or the session's start when it has written nothing.
- `.last_input()` — an `Instant`: when keys were last written to the program, by `write` or by the widget, or the session's start when none have been. Ask it together with `last_output` before writing text nobody asked for.
- `.pid()` — the program's process id, which is also its process group.
- `.terminate(grace)` — SIGHUP to the process group, SIGKILL after `grace` if it is still running; returns at once. Keep the session while the grace runs: dropping the last handle ends the program at once.
- `TerminalWatch::next()` — blocks until `TerminalEvent::Output` or `TerminalEvent::Exited(code)`; run it in `Command::perform`.
- `TerminalWatch::next_change()` — blocks until a `TerminalChange`: `Output`, `Title(text)`, `WorkingFolder(path)`, `Bell`, `Notify { title, body }` or `Exited(code)`. The enum is non-exhaustive.
- `TerminalWatch::next_change_within(bound)` — the same wait with a bound: `Some(change)`, or `None` when nothing was reported within `bound`. For tests, which run a `Command::perform` on the spot and would never return from an unbounded wait on a silent program; the session stays usable afterwards. A running application keeps using `next_change`.

## Behaviour

- Keys while focused are encoded like xterm: control letters, `alt` as an escape prefix, arrows in normal or application mode with modifier codes, function keys, `shift tab` excepted (it moves focus).
- `ctrl q` is left to the application.
- Pastes are wrapped in bracketed-paste markers when the program turned that mode on, by `paste`, which the widget uses for a person's paste as well. A `\x1b[200~` or `\x1b[201~` inside the text is left out, so text from elsewhere cannot end the paste early and have its rest read as keys.
- `write` and `paste` are refused once the program's end has been recorded: a pseudo-terminal keeps taking bytes after the program is gone and nobody ever reads them.
- The wheel scrolls back up to the scrollback (5000 lines by default), or sends three arrow keys on the alternate screen.
- When the program turns on mouse reporting (modes 9, 1000, 1002 and 1003, in the default, UTF-8 or SGR encoding), presses, releases, drags, moves and the wheel (buttons 64 and 65) are sent as xterm sends them, in cells counted from the terminal's corner. A press the program takes keeps the pointer until the release; a drag past the edge reports the nearest cell. `shift` with a press or the wheel selects and scrolls back instead.
- The cursor is drawn with `terminal-cursor` on the live screen unless the program hides it. On a `read_only()` terminal it fades with the screen.
- A `read_only()` terminal mixes every colour, the cursor's included, 45% into the background, the same share a cell the program itself marks faint is drawn with. The exit and scrolled-back notes keep their own colours: they are the widget's words, not the program's screen.
- After the program exits a note shows its exit code; keys are no longer sent. A paste and a wheel step on the alternate screen are handed back too, instead of being written into nothing: the paste then reaches `App::clipboard` as `ClipboardEvent::Pasted`, so the application can say the text went nowhere, and the wheel reaches whatever holds the terminal. Mouse reports are the one write dropped on purpose: a program that has ended has nothing to learn about the pointer, and a press handed back would act on what is behind the terminal.
- The program starts with `TERM=xterm-256color` and `COLORTERM=truecolor`, then the builder's variables, which may replace them.
- Notices: OSC 0 and 2 set the title, OSC 7 `file://host/path` the folder (percent-decoded, host not checked), BEL outside an escape sequence rings the bell, OSC 9 `body` and OSC 777 `notify;title;body` are notifications; OSC 9 starting with a number (ConEmu's progress and the like) is not. Sequences split across reads are heard whole. Unread titles and folders keep the newest, unread bells count as one, at most eight notifications wait. Notices come before the output they arrived with.
- An OSC string longer than 4096 bytes is cut there, so a program that never ends one cannot grow memory.
- With `.coalesce(interval)`, output is reported at most once per interval and never later than an interval after it arrived; notices and the end are not delayed.
- `next_change_within(bound)` comes back within the bound, answers `None` when the program said nothing, and applies pending size changes like the unbounded waits. With `.coalesce(interval)` a bound shorter than the interval can answer `None` although output has arrived, because the interval is still waited out.
- The screen is a text selection region; the scrolled-back and exited notes are decoration for clean copies. While a selection exists, `ctrl c` copies it instead of reaching the program.

## Theme keys

- `terminal` — `bg`, `fg`: the default colours.
- `terminal-cursor` — `bg`, `fg`, with `focus`.
- `terminal-note` — `bg`, `fg` of the scrolled-back and exit notes.
- Classic colours come from the tokens `raised`, `danger`, `success`, `warning`, `info`, `dim`, `muted` and `text`.
- Strings: `quvyta.terminal.scrolled`, `quvyta.terminal.exited`.
