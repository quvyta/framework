## When to use

Use an icon button for a small, well-known action at the edge of a header or a row: settings, search, add. It is one glyph, so it fits where a labelled button would crowd the line. When the action needs words to be understood, or it is the main action of the screen, use a `Button` with a label instead.

## Step by step

1. Create it with an icon key from the icon set: `IconButton::new("settings")`. The glyph follows the glyph mode by itself: a cog with a Nerd Font, `▤` in Unicode, `*` in ASCII.
2. Give it the message it sends: `.on_press(Msg::OpenSettings)`. Without a message it is drawn but cannot be focused or pressed.
3. Name it: `.tooltip(t!("header.settings"))`. A glyph alone is a guess; the words show below it when the pointer rests on it, and at once when it is reached with Tab.
4. Put it at the end of a row after a filling title, as on this page: `ui.add(Text::new(title)).fill_width();` then the buttons.

## How it works

- **Three cells, every mode.** A space, the glyph and a space. The whole three cells are the target, so a click on the space beside the glyph counts.
- **No surface at rest.** It stands on the ground around it, so a row of them reads as quiet marks, not as a row of buttons.
- **The tone is the state.** Under the pointer all three cells lighten to the hover tone, keyboard focus lightens them one step further, and a press flashes them one step more: rest < hover < focus < pressed, never inverted. There is no pillar: three cells have no room for one before the glyph, and a pillar would push it off centre.
- **Keyboard and mouse are equal.** Enter or Space presses the focused button; a click presses it when the mouse is released over it, so moving away before releasing cancels.
- **Disabled means gone for input.** A disabled icon button is faint, skipped by Tab and ignores clicks.

## Why not a Button option

A `Button` is a raised surface with padding and a pillar cell before its content. An icon button has none of these: no ground, no padding from the theme and no pillar. Turning a button into one would take three options that each switch off something every button has, so it is a component of its own, with its own style key.

## Styling with a theme

```toml
[style.icon-button]
fg = "$dim"

[style."icon-button:hover"]
bg = "$active"
fg = "$text"

[style."icon-button:focus"]
bg = "mix($accent, $active, 22%)"
fg = "$text"
```

## Common mistakes

- **Leaving out the tooltip.** A glyph means different things to different people; say it in words.
- **Using it for the main action.** The primary action of a screen deserves a labelled button.
- **Writing the tooltip in code.** Put it in the locale file so it follows the language.
- **Choosing an icon that is two cells wide.** Every glyph of the icon set is one cell; an application's own icon should be too, or the button grows.
