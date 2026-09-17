## Methods

- `ProgressBar::new(value)` — a bar at `value`, clamped to 0..1, with the percentage shown.
- `ProgressBar::indeterminate()` — a sweeping bar for work of unknown size.
- `.percent(bool)` — shows or hides the percentage after a determinate bar.
- `.variant(name)` — theme variant such as `"success"`, `"warning"`, `"danger"`.

## Behaviour

- Measures the full width it gets and one row.
- The percentage takes five cells on the right; the bar uses the rest.
- Determinate bars fill in eighths of a cell, whole cells in ASCII mode, rounded to the nearest cell.
- Indeterminate bars request frames while motion is not reduced; with reduced motion they show an even tint.

## Theme keys

- `progress`, `progress.<variant>` — `track`, `fill`.
- `progress-label`, `progress-label.<variant>` — `fg`, `bold`.
- `[motion]` — `shimmer` for the sweep.
