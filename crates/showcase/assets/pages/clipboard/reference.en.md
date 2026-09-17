## Methods

- `CopyValue::new(value)` — a value on a raised surface that copies itself.
- `.masked(bool)` — draws mask dots but copies the real value. Default: `false`.
- `.disabled(bool)` — greyed out, not focusable, never copies. Default: `false`.
- `.on_copy(msg)` — sent after every copy.
- `Command::copy(text)` — copies to the terminal clipboard (OSC 52) and the in-process clipboard.
- `Command::read_clipboard(|Option<String>| msg)` — the clipboard's text in a later update: the system clipboard, then the terminal's (OSC 52), then the last copy inside the application; `None` when none has text.
- `EventCx::copy(text)` — the same copy from inside a widget; reported to `App::clipboard`.
- `App::clipboard(&self, &ClipboardEvent) -> Option<Msg>` — `Copied(text)` from widgets, menus and the copy key, `Pasted(text)` when no widget took a paste.
- `Harness::clipboard()` and `Harness::copied()` — the in-process copy and every copy so far, for tests; `Harness::paste(text)` simulates the terminal's paste; `Harness::set_system_clipboard(Some(text))` stands in for the system clipboard, which a harness never reads.

## Behaviour

- Enter, Space, `c` or a click copies a CopyValue, flashes it and shows the success mark for 1.4 seconds, fading back over `motion.enter`.
- Narrow: the value is truncated with `…`; the marker stays whole.
- The `paste` action (`ctrl+v`) and Paste menu entries paste into the focused widget from the first source with text: the system tool (`wl-paste --no-newline --type text`, `xclip -o -selection clipboard`, `xsel --clipboard --output`, `pbpaste`; no shell, 500 ms at most, on a thread), the terminal's answer to an OSC 52 query (200 ms), the last copy inside the application. Nothing happens when none has text, and Paste entries are disabled.
- Releasing a mouse selection copies nothing; the `copy` action copies it clean, its right click menu offers Copy and Raw copy.
- Copies the application asked for with `Command::copy` are not reported to `App::clipboard`.

## Keys

- `[global] paste = "ctrl+v"`, label `quvyta.keys.paste`.
- `[global] copy = "ctrl+c"`, label `quvyta.keys.copy` — copies the mouse selection clean.

## Theme keys

- `copy-value` with `hover`, `focus`, `pressed`, `disabled` — `bg`, `fg`, `padding`.
- `copy-value-marker` (`fg`) and `copy-value-marker.copied`.

## Language keys

- `quvyta.copy-value.copy`, `quvyta.copy-value.copied`; the check comes from the `check` icon.
- `quvyta.edit.copy`, `quvyta.edit.raw-copy`, `quvyta.edit.cut`, `quvyta.edit.paste`, `quvyta.edit.select-all` for the right click menus.
