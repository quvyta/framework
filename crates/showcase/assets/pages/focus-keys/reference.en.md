## Global actions

- `quit` — `ctrl+q`; leaves the application.
- `focus-next` — `tab`; `focus-prev` — `shift+tab`.
- `debug` — `f12`; toggles the debug layer.
- `copy` — `ctrl+c`; copies the mouse selection clean.
- `paste` — `ctrl+v`; pastes into the focused widget from the system clipboard, then the terminal's, then the last copy inside the application.
- `toggle-panel` — `alt+b`; opens or closes a side panel from anywhere inside the panel or its body.
- `help` — `?` and `palette` — `ctrl+p`; the runtime does not act on them itself but hands them to `App::action`, where applications open their help layer and command palette.

## Keymap file

- `[global]` and `[app]` tables; values are one key like `"ctrl+s"` or a list of keys.
- Key names: characters, `f1`–`f24`, `enter esc tab space backspace delete insert home end pgup pgdn up down left right menu`.
- Modifiers: `ctrl`, `alt`, `shift`. Shift is folded into symbols: write `?`, not `shift+/`. Letters keep it: write `shift+s`, or just `S`, since an uppercase letter means shift plus that letter; modifier names and named keys (`Enter`, `Tab`, `PgDn`) are case-insensitive.

## Code

- `App::action(name) -> Option<Msg>` — application actions.
- `Command::focus(name)` — focus a named widget.
- `Keymap::parse`, `.overlay`, `.bind`, `.action_for(chord)`, `.chords_for(scope, action)`, `.iter()`, `.conflicts()`.
- `Scope::Global`, `Scope::App`, `scope.label_key(action)`.
- `KeyChord` parses `"ctrl+shift+p"`; `.label()` gives `ctrl shift p`.
- `Runtime::keymap_file(path)` — layer a file over the built-in keymap.
- `Runtime::keymap_source(file, text)` — layer a keymap given as text, such as an `include_str!`, so an installed binary carries its keys and needs no file beside it. It wins over `keymap_file`, and a file named as well is then optional: when it cannot be read the text stands in for it and the reason becomes a diagnostic instead of stopping the program.

## Locale keys

- `quvyta.keys.<action>` for global actions, `keys.<action>` for application actions.
