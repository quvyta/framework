## When to use

Use a checkbox for a setting or item that is on or off, especially in a list of independent choices that are applied together (a form saved with a button). For a setting that takes effect the moment it changes, a switch reads better.

## Step by step

1. Keep the state in your application: `autosave: bool`.
2. Draw the box with a label: `Checkbox::new(state.autosave).label(t!("autosave"))`.
3. Turn clicks into messages: `.on_toggle(|on| Msg::Autosave(on))`, and store `on` in `update`.
4. For a parent of several boxes, count the checked children and pass `.partial(some && !all)`. Toggling the parent asks for `true`; set every child in `update`.
5. Use `.disabled(true)` when the choice is not available right now.
6. The plain box is the default. For a box with a check mark, add `.style(CheckboxStyle::Check)`.

## How it works

- **The box is colour, not a drawing.** Two cells of the empty tone when unchecked, filled with the accent when checked, and only the left cell filled when partly checked. There is no glyph, so ASCII terminals show exactly the same box.
- **Changes blend.** Checking or unchecking blends the colour over three `motion.step`s, the time a switch knob takes; with reduced motion it changes at once.
- **Same box as a radio group.** Checkboxes and radio groups share this box on purpose. The difference is the meaning: each checkbox is on or off by itself, while a radio group keeps exactly one option chosen. Label and group the options so people can tell which one they are looking at.
- **The check style.** `CheckboxStyle::Check` keeps the three-cell box with a check `✓` and a dash for partly checked.
- **The whole row is clickable.** Clicking the label toggles, and so do Enter and Space while focused.
- **Hover and focus are colour.** Hovering lightens the empty box one step; a box reached with the keyboard warms towards the accent, and a focused checked box breathes. No pillar.
- **Your state wins.** The checkbox only asks; if `update` ignores the message the box stays as it was.

## Common mistakes

- **Negative labels.** "Don't send data" makes people think twice. Say what checking does.
- **Forgetting the parent.** When children change, recompute the parent's partial state from them rather than storing it.
