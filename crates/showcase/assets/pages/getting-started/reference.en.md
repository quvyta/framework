## App

- `type Msg` — everything that can happen; must be `Send`.
- `fn update(&mut self, msg) -> Command<Msg>` — applies a message.
- `fn view(&self, ui: &mut View<Msg>)` — describes the screen; no I/O.
- `fn action(&self, name) -> Option<Msg>` — turns a keymap action into a message: your `[app]` actions and the global ones the runtime leaves to you, such as `help` and `palette`. Optional.
- `fn clipboard(&self, &ClipboardEvent) -> Option<Msg>` — hears copies made by widgets and the mouse selection, and pastes no widget took. Optional.

## Command

- `Command::none()` — nothing to do.
- `Command::batch([..])` — several commands in order.
- `Command::perform(|| msg)` — runs work on a background thread, then delivers its message.
- `Command::quit()` — leaves the application.
- `Command::focus("name")` — focuses the widget named with `.id("name")`.
- `Command::set_theme(id)`, `Command::set_locale(code)`, `Command::set_icon_mode(mode)` — switch the look and language while running.
- `Command::set_reduced_motion(bool)`, `Command::set_pillar(style)`, `Command::set_slide(bool)` — change motion, the pillar and the selection slide while running.
- `Command::copy(text)` — copies to the clipboard, also over SSH; `Command::read_clipboard(|text| msg)` reads it.
- `Command::confirm(confirm)` — asks a question in a dialog and delivers the answer.
- `Command::toast(toast)`, `Command::dismiss_toast(key)`, `Command::toast_corner(corner)` — notifications.
- `Command::task(task)`, `Command::cancel_task(id)` — background work with progress that can be cancelled.

## Runtime

- `Runtime::new(app)` — a runtime with built-in files.
- `.theme_dir(path)`, `.icon_dir(path)`, `.locale_dir(path)`, `.keymap_file(path)` — load your own files over the built-ins.
- `.locale_source(file, text)` — loads a locale given as text, e.g. `include_str!("../locales/en.toml")`, so an installed program needs no files beside it.
- `.theme(id)` — start with a theme other than Monochrome.
- `.settings(&settings)` — start with the look the user saved; saved values win over `.theme`.
- `.run()` — takes over the terminal until the application quits; restores it on exit and on panic.

## Harness

- `Harness::new(app, width, height)` — built-in environment, rendered at once; `Harness::with_env(app, env, width, height)` for your own.
- `.press("ctrl+s")`, `.key(event)`, `.type_text("hi")`, `.paste(text)` — keyboard input.
- `.click(x, y)`, `.click_text("Save")`, `.hover(x, y)`, `.drag(from, to)`, `.mouse(kind, x, y)` — mouse input.
- `.send(msg)` — delivers a message as if a widget sent it.
- `.advance(duration)` — moves the fake clock; animations and flashes follow it. `.render()` paints again.
- `.set_theme(id)`, `.set_locale(code)`, `.set_glyph_mode(mode)`, `.set_reduced_motion(bool)`, `.set_system_clipboard(Some(text))` — change the environment.
- `.screen()`, `.find(text)`, `.fg(x, y)`, `.bg(x, y)`, `.is_bold(x, y)`, `.buffer()`, `.html(caption)` — read what was drawn.
- `.app()`, `.env()`, `.is_focused("name")`, `.copied()`, `.clipboard()`, `.quit_requested()` — inspect the application and the runtime.

## Keys

- `tab` and `shift tab` move focus; `ctrl q` quits; `f12` shows the debug layer.
