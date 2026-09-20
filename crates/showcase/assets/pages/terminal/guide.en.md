## When to use

Use the terminal when the user needs a real shell or a real interactive program inside your application: a console next to a container, a REPL, a database client. For output you only show, a log view is lighter and searchable.

## Step by step

1. Turn on the framework's `pty` feature in `Cargo.toml`.
2. Start a program: `TerminalSession::shell(&folder)` or `TerminalSession::spawn(program, &args, &folder)`, and keep the session in your state.
3. Watch it: return `Command::perform(move || Msg::Changed(watch.next()))` with `let watch = session.watch();`.
4. When `TerminalEvent::Output` arrives, start the watch again; the view redraws on its own. Stop when `TerminalEvent::Exited(code)` arrives.
5. Show it: `Terminal::new(&session)`, sized like any node. Stop the program with `session.terminate(grace)`, `session.kill()` or by dropping the session.

For more control, describe the start with `TerminalSession::builder(program)`: `.args(..)`, `.folder(..)`, `.env(name, value)`, `.size(columns, rows)`, `.scrollback(lines)` and `.coalesce(interval)`, then `.spawn()`. Watch it with `watch.next_change()` to hear the program's title, folder, bell and notifications as well.

## How it works

- **A real pseudo-terminal.** The program runs in a PTY, so shells, editors and full-screen tools behave as in any terminal. Output is parsed on a background thread into a screen with 5000 lines of scrollback, or as many as `.scrollback(lines)` asks; the playground sets it for the next shell.
- **Theme colours, not raw ANSI.** The sixteen classic colours come from the theme: red is the theme's danger tone, green its success tone, the background its surface. 256-colour and 24-bit colours are drawn as asked.
- **Keys go to the program.** While focused, every key is sent, Tab and Esc included; `shift tab` moves focus away and `ctrl q` still quits. By default nothing else reaches the application. Pastes use bracketed paste when the program supports it.
- **Chosen actions can pass through.** `.pass_through(Scope::Global, "help")` lets the keys of that keymap action skip the program and take the usual route: to a side panel around the terminal, to key listeners and to `App::action`. Call it once per action. A key that types a character, a character or space with at most `shift`, always goes to the program: with help on `?` and `f1`, `?` types into the shell and `f1` opens the help. This page passes help, so `f1` works while the shell has focus.
- **One key out and back in.** A passed action can mean "leave" inside the terminal and "go back" outside it. Pass it, answer it on the terminal's node with `.on_action(Scope::App, "terminal-focus", Msg::Leave)`, and map it in `App::action` to `Msg::Enter`; each message returns `Command::focus` with the other place. Focus decides which message comes, however it moved, so a click into the shell is followed too. On this page `ctrl alt space` moves from the shell to the start button and back.
- **Start options.** `TerminalSession::spawn` is the builder with every default: an 80 × 24 screen, 5000 lines of scrollback, every read reported. Give the builder the size the widget will have, so a full-screen program draws its first screen at the right size. Extra variables reach the program on top of `TERM=xterm-256color` and `COLORTERM=truecolor`, which they may replace.
- **What the program says about itself.** `TerminalWatch::next_change` returns a `TerminalChange`: `Output`, `Title` (OSC 0 and 2), `WorkingFolder` (OSC 7, percent-decoded), `Bell`, `Notify { title, body }` (OSC 9 and OSC 777) and `Exited(code)`. A title and a folder replace the unread one before, unread bells count as one and at most eight notifications wait, so a flood never grows memory. `next` keeps reporting only output and the end, as before. On this page the strip above the shell shows the last title, folder, notification and how often the bell rang.
- **Output at a steady pace.** `.coalesce(Duration::from_millis(16))` reports output at most once per interval, so `yes` or a fast log asks for one frame per interval instead of one per read; the last bytes are never held back longer than the interval. This page uses one frame.
- **A bounded wait for tests.** A screen test runs a `Command::perform` on the spot, so an unbounded wait on a live but silent program never comes back. `watch.next_change_within(Duration::from_secs(2))` answers `Some(change)` or `None` when the program said nothing in that time, and the session is still usable: ask again, or write to the program and ask again. A running application has a thread for its watch and keeps using `next_change`.
- **A polite end.** `session.terminate(grace)` sends SIGHUP to the program's process group, as a closing terminal window does, and SIGKILL when it has not ended within `grace`. It returns at once; the watch reports the end. Keep the session while the grace runs: dropping the last handle ends the program at once, so Stop on this page leaves the session in place. `session.pid()` gives the process id. Stop on this page ends the shell that way.
- **Size follows the widget.** The widget asks for its size while drawing; the watch applies it on its background thread, so drawing never touches the process.
- **Scrollback with the wheel.** The wheel scrolls back through earlier output, a quiet note says how far; typing returns to the live screen. Full-screen programs receive arrow keys instead, and programs that asked for the mouse receive the wheel itself.
- **The output is selectable.** A drag over the screen selects the program's output; `ctrl c` copies it (while text is selected, `ctrl c` copies instead of interrupting the program) and a right click offers Copy and Raw copy. The scrolled-back and exited notes are left out of clean copies.
- **The mouse goes to programs that ask for it.** When a program turns on mouse reporting, clicks, drags and the wheel go to the program instead: try `htop`, or `vim` after `:set mouse=a`. `shift` + drag still selects the output, and `shift` + wheel still scrolls back.

## Common mistakes

- **Not restarting the watch.** Without a running watch the screen stops updating and size changes wait.
- **Keeping sessions you no longer show.** A session keeps its program alive; drop it, or call `terminate` or `kill`.
- **Matching `TerminalChange` without a catch-all.** It can gain kinds of notice later; end the match with `_ => {}` and restart the watch there too.
- **Starting shells in tests through the runtime.** Tests run background work inline and an unbounded watch blocks; drive `TerminalWatch::next` yourself, or ask for a change within a bound with `next_change_within`.
