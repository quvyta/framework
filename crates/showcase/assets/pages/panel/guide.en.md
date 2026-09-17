## When to use

A panel groups things that belong together: a form, a set of choices, a live preview. It is a surface one step above its background, so the grouping is visible without a frame.

## Step by step

1. Add it with its content: `ui.add_with(Panel::new(), |ui| { ... })`.
2. Give it a short title when the group needs a name: `.title(t!("settings.network"))`. Titles are quiet on purpose; the content is the point.
3. Set `.gap(rows)` between children; one row by default.
4. To make a card the user can pick, add `.on_press(msg)` and `.selected(bool)`.
5. Use `.variant("inset")` for a surface inside a surface, such as the cards above: on a panel, a card must be one step higher to be seen.

## How it works

- **Elevation, not borders.** Canvas, surface, raised, active: each step is a lighter tone. A panel paints its own tone; a selected panel paints the active tone.
- **The pillar marks selection.** A selected panel shows the accent pillar along its whole left edge, like a dialog: selection is a state that stays. Idle panels never do, because emphasis everywhere is emphasis nowhere.
- **Pressable panels answer the pointer like buttons.** Hovering lifts the surface one clear tone and wakes the title, with a soft pillar down the panel's whole left edge. A press flashes one tone brighter. Focus reached with the keyboard looks like hover with a breathing pillar; a click does not leave that behind.
- **Pressable panels are real controls.** They join the focus order and respond to Enter, Space and click. `.disabled(true)` takes them out: no hover, no focus, no press.
- **Plain panels stay still.** Without `on_press` nothing changes under the pointer, so the user never hovers a group that does nothing.

## Styling with a theme

```toml
[style.panel]
bg = "$surface"
padding = [1, 3]

[style."panel:hover"]
bg = "mix($text, $surface, 8%)"
pillar = "mix($accent, $active, 45%)"

[style."panel:selected"]
bg = "$active"
pillar = "pulse($accent, $accent-2)"
```

## Common mistakes

- **A panel around everything.** Group only what belongs together; the canvas is a valid background.
- **Nested panels three deep.** Two levels read clearly; beyond that, restructure the screen.
- **Long titles.** A title names the group; explanations go inside.
