## When to use

Use a button when the user starts an action: save, send, delete, open a dialog. Use a list row or a tab to move between places, not a button.

Give each screen **one primary button**. When everything is emphasised, nothing is.

## Step by step

1. Create it with a label: `Button::new(t!("actions.save"))`. Labels come from language files, never from code.
2. Give it the message it sends: `.on_press(Msg::Save)`. Without a message the button is drawn but cannot be focused or pressed.
3. Choose a variant when it matters: `.variant("primary")` for the main action, `.variant("danger")` for destructive ones.
4. Add a shortcut segment when a key triggers the same action: `.shortcut("ctrl s")`. It only shows the key; bind the key in your keymap.
5. While the action runs, set `.loading(true)`. The label stays, a spinner replaces the icon and presses are ignored.

## How it works

- **The shape is colour.** A button is a surface of its own tone with two cells of padding. There are no brackets and no frame, in every glyph mode.
- **The pillar comes first.** Hover and focus show the pillar `▌` in the button's very first cell, before the shortcut segment and the icon: `▌ ⏎   Save`, never `⏎ ▌ Save`. The cell is part of the button at rest, so nothing moves when the pillar appears. Buttons never slide.
- **States come from the theme.** Hover raises the surface and shows a soft pillar. Focus reached with the keyboard raises it too and the pillar breathes between the two accent tones; a button focused by a click stays calm under the pointer. A press flashes it one tone brighter for the theme's `motion.flash` duration.
- **Keyboard and mouse are equal.** Enter or Space presses the focused button; a click presses it when the mouse is released over it, so moving away before releasing cancels.
- **Held keys never repeat.** Holding Enter presses once, even on terminals that report the hold as fast repeated presses. Enter pressed again after another key is a new press, however soon.
- **Disabled means gone for input.** A disabled or loading button is skipped by Tab and ignores clicks.

## Styling with a theme

```toml
[style."button.primary"]
bg = "$accent"
fg = "$ink"

[style."button:focus"]
bg     = "$active"
pillar = "pulse($accent, $accent-2)"

[style."button.warning"]
bg = "mix($warning, $surface, 22%)"
fg = "$warning"
```

Variants are free names: `.variant("warning")` works as soon as a theme styles `button.warning`.

## Common mistakes

- **Two primary buttons side by side.** Pick the one the user most likely wants.
- **Writing the label in code.** Put it in the locale file so the button follows the language.
- **Using a button as navigation.** Moving between pages belongs to lists, tabs and the router.
