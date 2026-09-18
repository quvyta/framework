## When to use

Use a heatmap when the question is "how much, day after day, over a long stretch": days of focus, commits, workouts, incidents. A year fits in seven rows and fifty-two columns, and a reader sees the streaks and the gaps at once. For the exact value of a few days use a bar chart, and for the shape of the last minutes a sparkline; a heatmap answers about the pattern, not the number.

Use a legend wherever tone stands for a category: several heatmaps side by side, or a chart whose shares are only told apart by colour. The theme carries five series tones, so the legend is what keeps a sixth series from being a mystery.

## Step by step

1. Keep one value per day in your state, oldest first, with a zero for a day that holds nothing.
2. Draw it: `ui.add(Heatmap::new(values)).height(Length::Cells(7))`.
3. Give the first value its weekday when the range does not start on one: `.starts_at(2)` leaves two cells blank, and a blank cell is a day outside the range, not an empty day.
4. Fix the scale against a goal with `.max(120.0)`; otherwise the busiest day of the range is the top step and a quiet year looks as busy as a loud one.
5. Let a reader read a single day: `.on_select(|day| Msg::Pick(day))` with `.selected(state.picked)`, and write that day's value beside the grid yourself. A tone cannot be read as a number.
6. For one category among several, `.series(n)` takes the theme's n-th series tone, and `Legend::new(["Rust", "Docs", "Review"])` names them in the same order; `.tones([0, 1, 2])` pins each name to the tone its heatmap was given, so a legend that shows only some categories still names the right colours.

## How it works

- **Four steps, not a gradient.** A day at or below zero takes the empty tone; anything above it takes one of four steps towards the full tone, and only the largest day (or `max`) reaches the top. Steps can be named and compared; a smooth ramp only looks precise.
- **Made of colour, never characters.** Every cell is a filled cell, so the grid is the same in a Nerd Font, in Unicode and in ASCII.
- **Low colour depth.** On a 16-colour terminal the steps that would land on the same terminal colour are dropped and the levels are spread over the tones that are left: fewer steps, but never two different levels in one tone, and a day that holds something never looks like a day that holds nothing.
- **Narrow areas keep the newest weeks.** Whole columns are dropped from the left, never squeezed, and `columns(width)` tells you how many are left so you can write "the last 12 weeks". A short area keeps the rows that fit from the top.
- **Mouse and keyboard are equal.** The cell under the pointer lights up; the arrow keys move a cursor, a column at a time with ← and →, a day at a time with ↑ and ↓, and Home and End go to the ends. A click and Enter do the same thing: they report the cell. Without `on_select` the heatmap is a picture: no focus, no messages.
- **Every state is drawn.** No values at all draws nothing and measures nothing, which leaves room for an empty state; a year of zeroes is a grid in the empty tone, which says "nothing happened" rather than "nothing is here".

## Common mistakes

- **A tone as the only answer.** Show the picked day's value as text; nobody reads minutes out of a shade.
- **An auto scale for a goal.** A week where the best day was ten minutes fills the top step. Give `.max()` the goal.
- **Gaps as zeroes.** A day the range does not cover is not a quiet day. Use `starts_at` for the lead-in and leave the range where the data ends.
- **A legend left out.** Series tones repeat after five; without names two categories are one colour and nothing tells them apart.
