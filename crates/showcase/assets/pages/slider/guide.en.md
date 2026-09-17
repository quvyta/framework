## When to use

Use a slider when the exact number matters less than where it sits in a range: traffic sent to a canary release, a CPU limit, how long logs are kept. People see the whole range at a glance and drag to a value. When the exact number matters, or the range is huge, use a number input.

## Step by step

1. Keep the value in your application: `canary: f64`.
2. Draw it: `Slider::new(state.canary)`. The plain slider runs from 0 to 100 in steps of 1.
3. Handle changes: `.on_change(|value| Msg::Canary(value))` and store the value in `update`.
4. Shape the range only when you need to: `.range(0.5, 8.0).step(0.5)`.
5. Say what the number means: `.suffix(t!("cores"))`, or write it yourself with `.format(|mb| …)`.
6. Give it a width with the layout, such as `.width(Length::Cells(48))`; a slider takes all the width it is given.

## How it works

- **The value comes first.** It is written before the rail in the accent colour, right-aligned in room for the widest value, so the rail never moves while the number changes.
- **The rail is the shape.** The part up to the value is drawn in the accent, the knob `◆` sits at the value, the rest of the rail is a quiet raised tone. In ASCII mode the same rail is drawn with cell colours instead of glyphs.
- **Values snap to steps** counted from the minimum and never leave the range. Decimals follow the step: a step of `0.5` writes `2.5`, a step of `1` writes `3`.
- **Keyboard:** ← and → move one step, Page Up and Page Down a tenth of the range, Home and End jump to the ends.
- **Mouse:** pressing the rail jumps the knob there, dragging moves it, even when the pointer leaves the rail. Pressing the value only focuses the slider.
- **Wheel:** with the pointer anywhere over a slider, wheel up moves one step up and wheel down one step down, without taking focus. The slider keeps the wheel, so a scrolling page around it stays still while the pointer rests on the slider; move the pointer off it to scroll the page. A disabled slider lets the wheel scroll the page.
- **Motion:** a keyboard jump steps the knob cell by cell, one cell every `motion.step`; long jumps take at most eight steps. The pointer moves the knob at once, and with reduced motion every change is instant.
- **Focus** makes the knob breathe between the accent and its second tone.

## Common mistakes

- **Sliders for exact numbers.** Nobody drags to exactly 1 337; offer a number input.
- **Units in the label only.** "CPU limit 2.5" leaves people guessing; add `.suffix(" cores")`.
- **Sliders in a long scrolling form with nothing else to rest the pointer on.** The wheel changes whichever slider is under the pointer; leave some room beside sliders so people can scroll the page.
- **Too narrow a rail.** With only a few cells each cell covers many steps; give the slider room or a coarser step.
