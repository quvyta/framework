## Methods

- `ShimmerText::new(text)` — text with light sweeping across it.
- `.style(ShimmerStyle)` — `Sweep` (default) or `Dots`.

## Behaviour

- Measures the text's width and one row; `Dots` adds three cells for the dots.
- `Sweep`: a band that fades out over five cells on either side of its centre travels across the letters once per `motion.shimmer`, eased in and out.
- `Dots`: zero to three dots over `motion.shimmer`.
- Requests frames only while motion is not reduced.

## Theme keys

- `shimmer` — `fg` for resting letters, `highlight` for the light.
- `[motion]` — `shimmer`.
