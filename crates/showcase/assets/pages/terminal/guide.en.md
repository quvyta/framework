## When to use

Use the terminal when the user needs a real shell or a real interactive program inside your application: a console next to a container, a REPL, a database client. For output you only show, a log view is lighter and searchable.

## Step by step

1. Turn on the framework's `pty` feature in `Cargo.toml`.
2. Start a program: `TerminalSession::shell(&folder)` or `TerminalSession::spawn(program, &args, &folder)`, and keep the session in your state.
3. Watch it: return `Command::perform(move || Msg::Changed(watch.next()))` with `let watch = session.watch();`.
4. When `TerminalEvent::Output` arrives, start the watch again; the view redraws on its own. Stop when `TerminalEvent::Exited(code)` arrives.
5. Show it: `Terminal::new(&session)`, sized like any node. Stop the program with `session.kill()` or by dropping the session.

## How it works

- **A real pseudo-terminal.** The program runs in a PTY, so shells, editors and full-screen tools behave as in any terminal. Output is parsed on a background thread into a screen with 5000 lines of scrollback.
- **Theme colours, not raw ANSI.** The sixteen classic colours come from the theme: red is the theme's danger tone, green its success tone, the background its surface. 256-colour and 24-bit colours are drawn as asked.
- **Keys go to the program.** While focused, every key is sent, Tab and Esc included; `shift tab` moves focus away and `ctrl q` still quits. By default nothing else reaches the application. Pastes use bracketed paste when the program supports it.
- **Chosen actions can pass through.** `.pass_through(Scope::Global, "help")` lets the keys of that keymap action skip the program and take the usual route: to a side panel around the terminal, to key listeners and to `App::action`. Call it once per action. A key that types a character, a character or space with at most `shift`, always goes to the program: with help on `?` and `f1`, `?` types into the shell and `f1` opens the help. This page passes help, so `f1` works while the shell has focus.
- **Size follows the widget.** The widget asks for its size while drawing; the watch applies it on its background thread, so drawing never touches the process.
- **Scrollback with the wheel.** The wheel scrolls back through earlier output, a quiet note says how far; typing returns to the live screen. Full-screen programs receive arrow keys instead, and programs that asked for the mouse receive the wheel itself.
- **The output is selectable.** A drag over the screen selects the program's output; `ctrl c` copies it (while text is selected, `ctrl c` copies instead of interrupting the program) and a right click offers Copy and Raw copy. The scrolled-back and exited notes are left out of clean copies.
- **The mouse goes to programs that ask for it.** When a program turns on mouse reporting, clicks, drags and the wheel go to the program instead: try `htop`, or `vim` after `:set mouse=a`. `shift` + drag still selects the output, and `shift` + wheel still scrolls back.

## Common mistakes

- **Not restarting the watch.** Without a running watch the screen stops updating and size changes wait.
- **Keeping sessions you no longer show.** A session keeps its program alive; drop it or call `kill`.
- **Starting shells in tests through the runtime.** Tests run background work inline and a watch blocks; drive `TerminalWatch::next` yourself.
