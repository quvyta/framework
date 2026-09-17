## Methods

This example adds no API of its own. It uses:

- `Panel` with `.title(..)` for every section.
- `Badge::new(..).variant(..).count(..)` for the host state, the container count and hot services.
- `BigText::new("14:32").variant("accent")` for the clock.
- `Sparkline::new(..).range(0.0, 100.0).highlight_extremes().baseline(80.0)` for CPU history.
- `Gauge::new(..).range(..).label(..).label_width(8).thresholds(..).value_text(..)` for resources.
- `BarChart::new(bars).max(100.0)` with `Bar::new(..).value_text(..).variant("danger")` for services.
- `EmptyState::new(..).icon("success").message(..)` when there are no alerts.
- `Skeleton::block()`, `Skeleton::lines(..)` while loading.

## Behaviour

- Refresh advances the sample clock by one tick: the clock moves a minute and every chart takes a new sample.
- A service above 70% CPU turns its bar to the danger tone and adds a "Hot services" badge with a count.
- Loading replaces the charts with skeletons of the same size; alerts replace the empty state with an alert list and an Acknowledge all button.
- Every interaction is written to the event log.

## Theme keys

The page uses the theme keys of its components: `panel`, `badge`, `badge-count`, `big-text`, `sparkline`, `gauge`, `gauge-label`, `gauge-value`, `bar-chart`, `bar-chart-label`, `bar-chart-value`, `empty-state-*`, `skeleton` and `button`.
