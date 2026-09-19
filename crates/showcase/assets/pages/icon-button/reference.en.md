## Methods

- `IconButton::new(key)` — a button showing the icon `key` of the icon set, such as `settings`, `search` or `add`.
- `.on_press(msg)` — the message sent when pressed. Required to make it interactive.
- `.tooltip(text)` — words shown below it after the theme's `motion.hover-delay`, and at once when it has keyboard focus. Default: none.
- `.disabled(bool)` — faint; not focusable, ignores input. Default: `false`.

## Messages

- The `on_press` message, once per press.

## Keyboard and mouse

- `enter` or `space` while focused — press.
- Left button released over any of its three cells — press. Releasing elsewhere cancels.

## Layout

- A space, the glyph and a space: three cells for every one-cell glyph, in Nerd Font, Unicode and ASCII mode. One row high.
- No surface at rest: the cells keep the ground they stand on.

## Theme keys

- `icon-button` with states `hover`, `focus`, `pressed`, `disabled` — `bg`, `fg`, `bold`. `focus` applies only when the focus came from the keyboard. At rest the built-in themes give no `bg`.
- `tooltip` — `bg`, `fg`, `padding` of the words.

## Icons and text

- Any icon key of the icon set; `settings` is a cog with a Nerd Font, `▤` in Unicode and `*` in ASCII.
