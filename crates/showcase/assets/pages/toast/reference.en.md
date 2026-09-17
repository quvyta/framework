## Methods

- `Toast::new(ToastKind, title)`, `Toast::success(title)`, `Toast::warning(title)`, `Toast::danger(title)`, `Toast::info(title)`.
- `.body(text)` — faint detail under the title, wrapped.
- `.action(label, msg)` — a button on the title row; sends `msg` and dismisses.
- `.duration(Duration)` — time on screen while not hovered. Default: 5 s, 8 s with an action.
- `.key(name)` — a toast with the same key replaces it in place.
- `.on_press(msg)` — makes the toast pressable: a click on it, outside its action and close mark, sends `msg` and the toast stays. Needs `Msg: Clone`, because each press sends a copy.
- `.icon_motion(SpinnerStyle)` — plays that one-cell animation in the icon cell, in the kind's colour; reduced motion shows the kind's icon.

## Commands

- `Command::toast(toast)` — shows it.
- `Command::dismiss_toast(key)` — removes the toast with that key.
- `Command::toast_corner(Corner)` — `TopRight`, `BottomRight` (default), `BottomLeft`, `TopLeft`. `Corner::name()` and `ToastKind::name()` give short names for settings screens.

## Mouse

- Hover pauses the countdown. The close mark keeps its resting tone; only the pointer on the mark lights its three cells.
- A click on the close mark dismisses. A click on the action sends its message and dismisses. A click elsewhere does nothing, or sends the `on_press` message and keeps the toast. Clicks on a toast never reach what is under it.

## Theme keys

- `toast` — `bg` (default `$overlay`), `padding` (default `[1, 2]`); `hover` on a pressable toast under the pointer (default `bg = $active`).
- `toast-title`, `toast-body`.
- `toast-action` with `hover`, and `active` while its pressable toast is raised (default a touch of accent over `$active`, `active:hover` a step above).
- `close-mark` — `fg`, `bg`, `bold`; `hover` on the mark. Shared with tabs, the tab rail and dialogs.
- `spinner` with the variants `success`, `warning`, `danger`, `info` for an animated icon.
- The marker and icon use `success`, `warning`, `danger` and `info`.
- `[motion] enter` — how long a toast takes to slide in and out.

## Icons

- `success`, `warning`, `error`, `info` for the kinds; `close` in the close mark; the spinner frames for an animated icon.
