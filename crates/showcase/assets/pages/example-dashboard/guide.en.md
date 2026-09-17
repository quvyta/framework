## When to use

This page is an example application, not a component: the status screen of a container host, built only from framework parts. Read it when you build a monitoring or overview screen and want to see how badges, big text, sparklines, gauges, bar charts, empty states and skeletons work together.

## Step by step

1. **Header.** A panel with the host name as a title, status badges under it, a big clock and a refresh button on the right. Only the host's overall state uses a status tone; the container count stays neutral.
2. **Trend and capacity side by side.** Two panels share a row with `Length::Fill(1)`: CPU history as a three-row sparkline with its current value as text, and resources as gauges with the same `label_width`, so their meters line up.
3. **Comparison and problems.** Services are a bar chart with a fixed maximum of 100%; only services above the limit get the danger tone and its marker. Next to it, the alerts panel.
4. **Empty is designed.** When nothing is wrong, the alerts panel shows an empty state that says so, instead of a blank surface.
5. **Loading is designed.** While data is on its way, every panel keeps its layout and shows skeletons in the shape of what will come; the clock and header stay.
6. Use the playground to switch between loading, alerts and normal operation, and press Refresh to take a new sample.

## How it works

- **One accent.** The clock is the one figure in the accent; charts use a step below it so highlighted extremes can stand out. Status colours appear only where something means good, warning or bad, always with a dot, `▲` or `✕`.
- **No lines anywhere.** Panels separate by surface tone and space; there is not a single frame or divider character on the screen.
- **State lives in the app.** The page keeps a tick, a loading flag and an alerts flag. Every widget is rebuilt from that state in `view`, so refreshing is just changing the tick.
- **Repeatable samples.** The numbers come from a small deterministic generator, so tests and screenshots show the same dashboard every time.

## Common mistakes

- **Everything in colour.** When every gauge, bar and badge has a tone, the one real problem disappears. Keep most things neutral.
- **Blank panels.** An alerts panel with nothing in it looks broken. Say "No alerts".
- **Jumping layouts.** If loading shows a spinner in place of a three-row chart, the whole screen moves when data arrives. Use skeletons of the same size.
