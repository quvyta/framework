## Painting helpers

- `cx.animate(name, target, duration, easing) -> f32` — a value that glides to `target`; starts settled on its first target.
- `cx.progress_since(start, duration, easing) -> f32` — eased progress 0 to 1 since `start`.
- `cx.cycle(period) -> f32` — phase 0 to 1 of a repeating animation.
- `cx.ticks(interval) -> u128` — whole intervals passed; the next frame lands exactly on the next step.
- `cx.pulse_phase() -> f32` — where the theme's breathing pulse is, 0 to 1; 0 with reduced motion.
- `cx.now()` — the frame's time, for `progress_since`; `cx.request_frame_in(delay)` — one more frame after `delay`, for motion the helpers do not cover.
- `cx.reduced_motion() -> bool` — whether the user asked for reduced motion.

## Building blocks

- `Easing` — `Linear`, `EaseIn`, `EaseOut` (default), `EaseInOut`; `.apply(t)`, `.name()`, `Easing::ALL`.
- `Tween` — a value moving between two numbers: `settled`, `value(now)`, `target`, `is_running(now)`, `retarget`.
- `steps(progress, count) -> u16` — which of `count` cells progress has reached.
- `Command::set_reduced_motion(bool)` — turns reduced motion on or off at runtime; store it with `Settings::REDUCED_MOTION` to keep it for the next start.

## Theme keys

`[motion]` — `enter`, `spinner`, `shimmer`, `pulse-period`, `flash`, `cursor-blink`, `step`, `page`, `hover-delay` as durations like `"140ms"` or `"1.6s"`, and `slide` as `true` or `false`.

## Environment

- `QUVYTA_REDUCED_MOTION=1` — reduced motion, `=0` — motion; either wins over the saved `reduced-motion` setting and `Command::set_reduced_motion`. Unset or empty leaves the choice to them.
- `Env::reduced_motion_forced() -> bool` — whether the variable decides; show a disabled switch with the reason while it does.
