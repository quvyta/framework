## Methods

- `Button::new(label)` — a button showing `label`.
- `.on_press(msg)` — the message sent when pressed. Required to make it interactive.
- `.variant(name)` — theme variant such as `primary` or `danger`. Default: none.
- `.shortcut(label)` — key label in a darker segment on the left, e.g. `⏎` or `ctrl s`. The segment starts with the pillar cell. Default: none.
- `.icon(key)` — icon from the icon set before the label. Default: none.
- `.disabled(bool)` — greys it out; not focusable, ignores input. Default: `false`.
- `.loading(bool)` — spinner instead of the icon; ignores presses. Default: `false`.
- `.selected(bool)` — makes it a choice button, such as one of a few view modes: selected shows it raised with a steady pillar. A choice button does not flash when pressed. Default: an ordinary button.

## Messages

- The `on_press` message, once per press.

## Keyboard and mouse

- `enter` or `space` while focused — press.
- Left button released over the button — press. Releasing elsewhere cancels.
- Holding a key — one press.

## Layout

- Pillar cell, shortcut segment (a space, the key, a space), left padding, icon and a space, label, right padding.
- The pillar cell exists only with a shortcut and a left padding of at least one cell; without a shortcut the pillar sits in the first padding cell. Either way it is the leftmost cell and appears on hover and focus without moving anything.

## Theme keys

- `button` with states `hover`, `focus`, `pressed`, `selected`, `disabled` — `bg`, `fg`, `bold`, `padding`, `pillar`. `focus` applies only when the focus came from the keyboard.
- `button.<variant>` with the same states.
- `button-key` and `button-key.<variant>` — the shortcut segment.

## Icons and text

- While loading, the frames of the default spinner style (animation `spinner-arc`); any icon key for `.icon`.
