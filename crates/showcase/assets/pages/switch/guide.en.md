## When to use

Use a switch for a setting that takes effect immediately: animations on, sounds off. If the change only applies after pressing Save, use a checkbox instead.

## Step by step

1. Keep the state in your application: `animations: bool`.
2. Draw it: `Switch::new(state.animations).label(t!("animations"))`.
3. Handle toggles: `.on_toggle(|on| Msg::Animations(on))`, apply the setting in `update`.
4. Choose a look only if the default does not fit: `.style(SwitchStyle::Rail)` next to sliders, `.style(SwitchStyle::Labeled)` where the state must be readable as a word.

## How it works

- **Capsule by default.** Five flat cells with a two-cell knob. Off, the knob rests left in a faint colour on a raised track; on, it rests right in the accent on a tinted track.
- **The knob steps, colours blend.** The knob moves one cell every `motion.step`, and the track and knob colours move with it, so the middle frame has a middle colour. With reduced motion it jumps.
- **The knob is always the brightest part**, so the state reads at a glance in every theme.
- **One tone ladder.** An off switch rests quiet: its knob sits well below the lit track of an on switch, so a column of switches never reads as filled and empty boxes. Hovering lifts both halves one step, on or off. Every tone is a mix of theme colours (`raised`, `muted`, `accent`); a theme changes them with the `switch` keys.
- **`Labeled`** writes `quvyta.switch.on` / `quvyta.switch.off` inside the capsule in the active language, with half-block rounded ends.
- **`Rail`** draws a knob on a thin rail; in ASCII mode it falls back to the capsule.

## Common mistakes

- **Switches that wait for Save.** People expect a switch to act now.
- **Labels that describe the state.** Write "Animations", not "Animations are on"; the switch shows the state.
