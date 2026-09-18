## Methods

- `Gauge::new(value)` — a gauge at `value` in the range 0 to 100.
- `.range(min, max)` — the range of the value.
- `.label(text)` — a name before the meter.
- `.label_width(cells)` — reserves the label column, so stacked gauges line up.
- `.value_text(text)` — text after the meter instead of the percentage.
- `.thresholds(warning, danger)` — tones by value: success below `warning`, warning from it, danger from `danger`.

## Behaviour

- Measures the full width it gets and one row.
- The meter fills in eighths of a cell; whole cells in ASCII mode.
- With thresholds, track zones past each limit are tinted and the value carries the `dot`, `warning` or `error` icon.
- Below four cells of meter only the label and value are drawn.
- A readout: not focusable, answers no keys and no pointer events, and sends no messages. Tab goes past it. Use a `Slider` to set a value, a `Tooltip` for a label that had to be cut, a `Sparkline` to read a value out of a history.

## Theme keys

- `gauge`, `gauge.success`, `gauge.warning`, `gauge.danger` — `track`, `fill`, `zone`.
- `gauge-label` — `fg`.
- `gauge-value`, `gauge-value.<level>` — `fg`, `bold`.
