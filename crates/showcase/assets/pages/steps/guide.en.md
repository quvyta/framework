## When to use

Use steps to show where a sequence stands: the pages of a wizard, the stages of a release pipeline, an onboarding checklist. They answer "how far am I and what is left". For progress that is a quantity rather than named stages, use a progress bar.

## Step by step

1. Keep the current step in your state: `current: usize`.
2. Draw the sequence: `Steps::new(labels).current(state.current)`.
3. Move `current` in `update` when the work advances; a value past the last step shows everything finished.
4. Add capabilities only where they fit: `.vertical(true)` for a checklist in a side column, `.running(true)` while the current stage is working, `.failed(true)` when it stopped, `.on_select(Msg::GoTo)` to let people return to a finished step.

## How it works

- **Markers and tone carry the state.** Finished steps show a check in the success colour, the current step an accent dot and a bold label, upcoming steps a faint outline. Colour never works alone.
- **Nothing connects the steps.** No lines or arrows; steps are set apart by space.
- **Narrow rows compact themselves.** When the labels do not fit, every marker stays and only the current step keeps its label.
- **Running breathes.** The current marker pulses between the accent colours; with reduced motion it stands still.
- **Going back is opt-in.** Without `on_select` the steps are only a display and take no focus. With it, finished steps rise with the pillar under the pointer, like every pressable thing, and the keyboard moves between them with the arrow keys; Enter or Space chooses.
- **The application decides.** Choosing sends a message; your `update` sets `current`.

## Common mistakes

- **Letting people jump ahead.** Only finished steps are choosable; skipping validation is a flow bug, not a feature.
- **Long labels.** One or two words per step; details belong on the page itself.
- **Using steps for a single task.** One step is not a sequence; show a spinner or a progress bar.
