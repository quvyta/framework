## When to use

Reach for motion when a change would otherwise be hard to follow: a knob moving to the other side, a dropdown unfolding, a value counting up. Motion explains *where something went*; it is never decoration.

Most of the time you use motion without writing any: Select unfolds, Spinner turns and ProgressBar sweeps on their own. This page is for writing your own widget that moves.

## Step by step

1. In your widget's `paint`, ask for the value you want to show: `let x = cx.animate("knob", target, duration, Easing::EaseOut);`. The first time it simply returns `target`; when the target changes later it glides from wherever it is.
2. Turn the value into cells. Terminals move in whole cells, so blend colours with the fraction a cell is covered, like the lanes above: a knob half over a cell paints it halfway between track and knob.
3. For something that starts at a moment (a layer opening), store the start time from `cx.now()` and use `cx.progress_since(start, duration, easing)`.
4. For something that repeats (a spinner, a sweep) use `cx.cycle(period)` for smooth phases or `cx.ticks(interval)` for frame counts.
5. Take durations from the theme: `cx.env().theme().motion()`.

## How it works

- **Frames only while moving.** Every helper asks the runtime for the next frame only while something is still changing. A still screen costs nothing.
- **State lives with the widget.** `animate` keeps its tween in the widget's memory under the name you give, so one widget may animate several values.
- **Reduced motion is built in.** When the user turns it on (`QUVYTA_REDUCED_MOTION=1` or `Command::set_reduced_motion(true)`), `animate` and `progress_since` return the end state, `cycle` and `ticks` stand still at 0, and no frames are requested.
- **Remember the choice.** Reduced motion is a user preference: store it with `Settings::REDUCED_MOTION` when it changes, and `Runtime::settings` applies it before the first frame next time. `QUVYTA_REDUCED_MOTION` in the user's shell is stronger than any saved or runtime choice: `1` keeps motion reduced and `0` keeps it on, whatever the setting says, the way accessibility overrides work; unset or empty, the saved choice decides. The switch on this page and the one on the Theme, icons, language page store the same key, so each always shows what the other set.
- **Show when the environment decides.** `Env::reduced_motion_forced()` is true while `QUVYTA_REDUCED_MOTION` decides. Then a settings switch would snap back when pressed, so disable it, let it show `Env::reduced_motion()` and add a faint line saying why. Both switches here do that: "Set by the QUVYTA_REDUCED_MOTION environment variable." A disabled switch sends nothing, so nothing is changed or saved.
- **Easing.** `EaseOut` is right for things arriving, `EaseIn` for things leaving, `EaseInOut` for things travelling back and forth, `Linear` for loops.

## Common mistakes

- **Fixed durations in code.** Use the theme's timings so a theme can make the whole application calmer or brisker.
- **Moving by whole cells without blending.** It looks jumpy. Blend the edge cells with the covered fraction.
- **A reduced-motion switch that ignores the environment.** While `QUVYTA_REDUCED_MOTION` decides, a live switch flips, saves and snaps back with no reason given. Check `Env::reduced_motion_forced()`.
- **Ignoring reduced motion in hand-made loops.** Use `cycle` and `ticks` instead of reading the clock yourself; they already respect it.
