## When to use

Use a select when the user picks one option from a known set that is too long to show at once, or when space is short: a theme, a language, a region. For two or three options that should all stay visible, a segmented control or radio group reads faster.

## Step by step

1. Give it the options: `Select::new([t!("runtime.podman"), t!("runtime.docker")])`.
2. Show the choice: `.selected(self.runtime)`, an `Option<usize>`.
3. Receive choices: `.on_select(Msg::Runtime)`.
4. Say what to pick while nothing is chosen: `.placeholder(t!("runtime.choose"))`.
5. For long lists, limit the height with `.max_visible(rows)`; the list scrolls.

## How it works

- **The list is a layer.** Opening draws the options on top of everything else, below the field when there is room and above it when there is not. It never pushes content around.
- **The keyboard stays inside while it is open.** Arrows move, Home and End jump, PgUp and PgDn page, typing a letter jumps to the next option starting with it, Enter or Space chooses, Esc closes. Tab closes and moves on.
- **Clicking elsewhere closes it, and the click still counts.** With the language list open, one click on the theme field closes it and opens the theme list; one click on a menu entry changes the page. A click on the open select's own field only closes it.
- **One highlight.** The keyboard and the pointer move the same highlighted row. A pointer resting where the list unfolds takes nothing until it moves; the keys go on from the row the pointer left the highlight on.
- **Long lists scroll** with the wheel, the keys, or by pressing and dragging the scrollbar. Whether the scrollbar shows is decided by the fully unfolded height, so it never flashes while the list opens.
- **Rows behave like list rows.** The highlighted row raises its surface, shows the pillar and slides its label one cell; the chosen option has a check mark on the right.
- **Only real changes send messages.** Choosing the option that is already chosen closes the list quietly.

## Styling with a theme

```toml
[style.select-menu]
bg = "$overlay"

[style."select-option:hover"]
bg = "$active"
pillar = "pulse($accent, $accent-2)"
```

## Common mistakes

- **Using the option text as the value.** Keep ids in your code and map the index; labels are translated.
- **Hundreds of options.** Add search, or use a list with a filter field.
- **Placing it at the very bottom of a short screen.** It opens upward, but give it room when you can.
