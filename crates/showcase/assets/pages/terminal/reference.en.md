## Methods

- `Terminal::new(&session)` — draws a session; available with the `pty` feature.
- `.pass_through(scope, action)` — the keys bound to that keymap action skip the program and go on to ancestors, key listeners and `App::action`; call it once per action. The keymap in force when the key arrives decides, so rebinding follows. Keys that type a character (a character or space with at most `shift`) always reach the program. Default: nothing passes but `shift tab` and `ctrl q`.
- `TerminalSession::shell(folder)`, `TerminalSession::spawn(program, args, folder)` — start a program in a pseudo-terminal.
- `.watch()` — a `TerminalWatch`; `.write(bytes)`, `.kill()`, `.exit()`.
- `TerminalWatch::next()` — blocks until `TerminalEvent::Output` or `TerminalEvent::Exited(code)`; run it in `Command::perform`.

## Behaviour

- Keys while focused are encoded like xterm: control letters, `alt` as an escape prefix, arrows in normal or application mode with modifier codes, function keys, `shift tab` excepted (it moves focus).
- `ctrl q` is left to the application.
- Pastes are wrapped in bracketed-paste markers when the program turned that mode on.
- The wheel scrolls back up to 5000 lines, or sends three arrow keys on the alternate screen.
- When the program turns on mouse reporting (modes 9, 1000, 1002 and 1003, in the default, UTF-8 or SGR encoding), presses, releases, drags, moves and the wheel (buttons 64 and 65) are sent as xterm sends them, in cells counted from the terminal's corner. A press the program takes keeps the pointer until the release; a drag past the edge reports the nearest cell. `shift` with a press or the wheel selects and scrolls back instead.
- The cursor is drawn with `terminal-cursor` on the live screen unless the program hides it.
- After the program exits a note shows its exit code; keys are no longer sent.
- The program starts with `TERM=xterm-256color` and `COLORTERM=truecolor`.
- The screen is a text selection region; the scrolled-back and exited notes are decoration for clean copies. While a selection exists, `ctrl c` copies it instead of reaching the program.

## Theme keys

- `terminal` — `bg`, `fg`: the default colours.
- `terminal-cursor` — `bg`, `fg`, with `focus`.
- `terminal-note` — `bg`, `fg` of the scrolled-back and exit notes.
- Classic colours come from the tokens `raised`, `danger`, `success`, `warning`, `info`, `dim`, `muted` and `text`.
- Strings: `quvyta.terminal.scrolled`, `quvyta.terminal.exited`.
