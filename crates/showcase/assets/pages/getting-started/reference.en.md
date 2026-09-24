## App

- `type Msg` — everything that can happen; must be `Send`.
- `fn update(&mut self, msg) -> Command<Msg>` — applies a message.
- `fn view(&self, ui: &mut View<Msg>)` — describes the screen; no I/O.
- `ui.map(f, |ui| screen.view(ui))` — adds a column built with a screen's own messages, each converted by `f`; handlers, nested containers, layers and overlays inside all arrive converted.
- `fn action(&self, name) -> Option<Msg>` — turns a keymap action into a message: your `[app]` actions and the global ones the runtime leaves to you, such as `help` and `palette`. Optional.
- `fn clipboard(&self, &ClipboardEvent) -> Option<Msg>` — hears copies made by widgets and the mouse selection, and pastes no widget took. Optional.
- `fn init(&mut self) -> Command<Msg>` — runs once at the start of the first frame, before its view is built; a `Command::focus` it returns is in place before the first key. Optional.
- `fn resized(&self, Size) -> Option<Msg>` — the terminal size at start (before `init`) and after every resize, the same size `ui.size()` reports; its message goes through `update`. Optional.
- `fn before_quit(&self) -> Option<Msg>` — asked before the runtime quits for the user (the quit binding, the quit action from the command palette); a message keeps the application running and is delivered instead. `Command::quit()` is never asked about. Optional.
- `fn terminating(&self, Termination) -> Option<Msg>` — hears that the system is ending the application: `Termination::Terminate` for a `SIGTERM` or an outside `SIGINT`, `Termination::Hangup` for a `SIGHUP`. `None` quits at once; a message keeps it running to save and quit. By default a terminate is answered by `before_quit` and a hangup quits. Optional.

- `fn frame_limit(&self) -> FrameLimit` — how many frames a second the runtime may draw; asked before every frame, so it may follow the application's own state. Optional.

## FrameLimit

- `FrameLimit::default()` — 60 frames a second at the machine, 20 over a remote connection.
- `FrameLimit::per_second(n)` — the same number on every connection; `n` of zero is no limit.
- `.remote(n)` — a different number over a remote connection, leaving the local one as it is.
- `FrameLimit::none()` — every frame that is wanted is drawn.
- `limit.frames_per_second(remote)` — the number in force on the connection in hand, or `None` for no limit.
- A frame that answers input — a key, a click, the pointer, a paste — is never held back, and a held frame is drawn as soon as the gap is over. Only frames the application's own work causes are merged.

## Pace and connection

- `env.remote()` — whether the terminal is at the other end of a remote connection: `SSH_CONNECTION` or `SSH_TTY` set and not empty. Detected once by `Env::load`; `Env::builtin`, the environment of tests, is never remote.
- `Env::load_with(&dirs, lookup)` — `Env::load` with every variable it reads (`LANG`, `LC_ALL`, `LC_TIME`, `TERM`, `SSH_CONNECTION`…) answered by `lookup`, and the operating system's own language never asked. For a test that runs an application with its real files: `|_| None` is a machine with nothing set, so the language, the first day of the week and the decimal mark are the same on every machine.

## Graphics

- `env.graphics()` — how a picture can be drawn here: `Graphics::Kitty`, `Graphics::Sixel`, `Graphics::HalfBlock` or `Graphics::None`. The terminal's answer to one question at start (150 ms at most, never delivered as keys), then: 16 colours or ASCII glyphs give `None`; `TMUX` or `STY` turn kitty and sixel into half blocks. `Env::builtin` gives half blocks.
- `QUVYTA_GRAPHICS=kitty|sixel|halfblock|none` — decides over the answer and every rule; another value is ignored and becomes a diagnostic.
- `Graphics::name()`, `Graphics::from_name(name)` — the names the variable takes.

## Command

- `Command::none()` — nothing to do.
- `Command::batch([..])` — several commands in order.
- `Command::perform(|| msg)` — runs work on a background thread, then delivers its message.
- `Command::quit()` — leaves the application; it is the application's own decision, so `before_quit` is not asked.
- `Command::focus("name")` — focuses the widget named with `.id("name")`.
- `Command::set_theme(id)`, `Command::set_locale(code)`, `Command::set_icon_mode(mode)` — switch the look and language while running.
- `Command::set_reduced_motion(bool)`, `Command::set_pillar(style)`, `Command::set_slide(bool)` — change motion, the pillar and the selection slide while running.
- `Command::copy(text)` — copies to the clipboard, also over SSH; `Command::read_clipboard(|text| msg)` reads it.
- `Command::confirm(confirm)` — asks a question in a dialog and delivers the answer.
- `Command::toast(toast)`, `Command::dismiss_toast(key)`, `Command::toast_corner(corner)` — notifications.
- `Command::task(task)`, `Command::cancel_task(id)` — background work with progress that can be cancelled.
- `command.map(f)` — the same work delivering `f(message)`: for a screen's commands in the application's `update`. Messages that performs and tasks send later go through `f` too; `f` must be `Send + Sync`.

## Runtime

- `Runtime::new(app)` — a runtime with built-in files.
- `.theme_dir(path)`, `.icon_dir(path)`, `.locale_dir(path)`, `.keymap_file(path)` — load your own files over the built-ins.
- `.theme_source(file, text)`, `.icon_source(file, text)`, `.locale_source(file, text)`, `.keymap_source(file, text)` — load the same files given as text, e.g. `include_str!("../locales/en.toml")`, so an installed program needs no files beside it. Text wins over the matching path, and for themes and icon sets the file stem is the id. A path named as well is then optional: when it cannot be read the text stands in for it and the reason becomes a diagnostic instead of stopping the program.
- `.theme(id)` — start with a theme other than Monochrome.
- `.settings(&settings)` — start with the look the user saved; saved values win over `.theme`.
- `.run()` — takes over the terminal until the application quits; restores it on exit and on panic. On Unix it catches `SIGTERM`, `SIGINT` and `SIGHUP`, tells `App::terminating`, and always ends in bounded time: after the grace, at once on a second `SIGTERM` or `SIGINT`, and by the signal itself a second later when the loop is stuck. A signal during a handoff reaches its program first.

## Termination

- `Termination::Terminate` — a `SIGTERM`, or a `SIGINT` from outside; the terminal is still there.
- `Termination::Hangup` — a `SIGHUP`: the terminal went away and nothing is drawn after it; a repeated hangup is told once.
- `cause.grace()` — how long the application has to quit on its own: five seconds after a terminate, three after a hangup.

## Harness

- `Harness::new(app, width, height)` — built-in environment, rendered at once, which reports the size to `resized` and runs `init`; `Harness::with_env(app, env, width, height)` for your own.
- `.resize(width, height)` — resizes the screen, reports the new size to `resized` and draws in full.
- `.press("ctrl+s")`, `.key(event)`, `.type_text("hi")`, `.paste(text)` — keyboard input.
- `.click(x, y)`, `.click_text("Save")`, `.hover(x, y)`, `.drag(from, to)`, `.mouse(kind, x, y)` — mouse input.
- `.send(msg)` — delivers a message as if a widget sent it.
- `.advance(duration)` — moves the fake clock; animations and flashes follow it, and a termination whose grace is over quits. `.render()` paints again.
- `.terminate(Termination::Terminate)`, `.terminate(Termination::Hangup)` — simulates a `SIGTERM` or a `SIGHUP`: `terminating` hears it as in a terminal, a second terminate quits, a repeated hangup changes nothing.
- `.set_theme(id)`, `.set_locale(code)`, `.set_glyph_mode(mode)`, `.set_graphics(graphics)`, `.set_reduced_motion(bool)`, `.set_system_clipboard(Some(text))` — change the environment.
- `.screen()`, `.find(text)`, `.fg(x, y)`, `.bg(x, y)`, `.is_bold(x, y)`, `.buffer()`, `.html(caption)` — read what was drawn. A double-width character reads once, without the cell it covers: `screen().contains("防火墙")` holds and `find` gives the column it is drawn in.
- `.app()`, `.env()`, `.is_focused("name")`, `.copied()`, `.clipboard()`, `.quit_requested()` — inspect the application and the runtime.

## Keys

- `tab` and `shift tab` move focus; `ctrl q` quits, after asking `before_quit`; `f12` shows the debug layer.
