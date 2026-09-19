## Keyboard first

Everything in an application built with quvyta-framework works from the keyboard. The mouse is a shortcut to the same actions, never the only way.

## Focus

- Widgets that take input are focusable: buttons, fields, selects, lists, tabs, scroll views.
- **Tab** moves to the next one and **shift tab** to the previous one, in the order they appear on screen.
- Clicking a widget focuses it.
- Keys go to the focused widget first. What it does not use travels up to its parents, then to the keymap.
- An open dropdown keeps the keys until it closes, so arrows move inside it and Esc closes it.
- `Command::focus("name")` moves focus to the widget named with `.id("name")`, for example back to the first field of a form.

Focus is shown by the theme: the focused widget breathes between the two accent tones, through its pillar or its own colours, so the eye finds it at once. Buttons, cards, tabs and other pressable controls breathe only when the focus came from the keyboard; a control you just clicked stays calm under the pointer.

## The keymap

A keymap binds **action names** to keys. Framework actions live in `[global]`; your application's actions live in `[app]`:

```toml
[app]
save = "ctrl+s"
search = ["/", "ctrl+f"]
```

Your `App::action` turns a name into a message. Users can rebind keys with a file, without any change to the code. When the same key is bound to two actions of one table, a warning says so.

An action can also mean something else while focus is inside one part of the screen. `.on_action(Scope::App, "switch", Msg::Leave)` on a node answers the action with that message while the node or anything inside it has focus; with focus elsewhere the key reaches `App::action` as usual. The focus in force when the key arrives decides, whether it came from Tab, a click or `Command::focus`, so the application keeps no flag of its own. The innermost answering node wins, a key the focused widget uses (a letter in a field) stays with it, and the runtime's own actions such as `focus-next` are never answered. One key can so leave a terminal and, pressed again outside it, go back in: the terminal page shows it with `ctrl alt space`.

The application's own bindings do not have to be a file on disk. `Runtime::keymap_source(file, text)` takes the TOML text itself, usually an `include_str!` of the keymap in your repository, so the installed binary carries its keys; a path built from `CARGO_MANIFEST_DIR` breaks once the binary is installed somewhere else. A `keymap_file` named as well is then optional: when it cannot be read the text stands in for it and the reason becomes a diagnostic instead of stopping the program. A broken entry is skipped with its file, line and column, and the built-in bindings keep working.

## Hint labels come from the language

The hint bar reads labels from locale keys: `quvyta.keys.<action>` for global actions and `keys.<action>` for yours. Switch the language and every hint follows.

## Held keys

Holding Enter or Space never repeats an activation. Terminals that cannot report key release send a held key as fast presses; quvyta-framework treats presses closer than 100 ms as one.

## The debug layer

Press **f12** in any application. Clickable areas are tinted, every focusable widget shows its position in the focus order, and a panel shows the frame number, paint time, hit areas and the focused widget. Press f12 again to hide it.

## Common mistakes

- **Mouse-only actions.** Every click target needs a key path: focusable widget or keymap action.
- **Keys hard-coded in widgets.** Bind application shortcuts in the keymap so users can change them.
- **Taking Tab.** Leave Tab to focus movement unless the widget is a text editor that needs it.
