## When to use

Use a number input when the exact number matters: a container port, the number of replicas, CPU cores to reserve. People can type the number they already know or nudge it with the arrow keys. When the position in a range matters more than the exact number, use a slider.

## Step by step

1. Keep the number in your application: `replicas: f64`.
2. Draw it: `NumberInput::new(state.replicas)`. The plain field has no limits and steps by 1.
3. Handle changes: `.on_change(|value| Msg::Replicas(value))` and store the value in `update`.
4. Set limits when the number has them: `.range(1.0, 12.0)`.
5. Choose the step: `.step(0.25)`. The step's decimals decide how the number is written and whether a decimal point can be typed.
6. Add `.steppers(true)` when the field is used mostly with the mouse.

## How it works

- **It is a text input.** The surface, prompt, focus, cursor, selection, undo and clipboard are those of `TextInput`; nothing new to learn.
- **Only numbers can be typed.** Digits always; a minus only when the range reaches below zero; a decimal point only when the step has decimals. Pasted text keeps just those characters.
- **Your application only receives valid numbers.** While the text is unfinished (`-`, `2.`), not a number or outside the range, the field shows the invalid tint and sends nothing; the last valid value stays in your state. An empty field shows the placeholder and is not marked.
- **Stepping:** ↑ and ↓ move one step, Page Up and Page Down ten steps, always clamped to the range. Stepping starts from the typed number, so typing 7 and pressing ↑ gives 8.
- **Stepper segments** are two surface segments on the right, like a button's key segment. Clicking one steps and flashes it; at a limit its sign turns faint.
- **Your value wins.** When your application changes the value, the field shows the new number at once.
- **The wheel steps.** Over a field the wheel moves one step per notch and the page around it does not scroll; a disabled field ignores it.
- **The edit menu.** A right click opens Cut, Copy, Paste and Select all as in a text input; pasted text keeps only what a number allows.

## Common mistakes

- **Validating twice.** The range already keeps out-of-range numbers away from `update`; use `.invalid(true)` only for rules of your own, such as a port that is taken.
- **Number inputs for choices.** Two to five fixed options read better as a segmented control or a radio group.
- **Rounding in `update`.** Choose a step instead; typed numbers keep their own writing, stepping writes the step's decimals.
