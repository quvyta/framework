## When to use

Use a side panel for a secondary area that people open while they need it and close to make room: a project explorer, a search view, an outline. With several views, give it an icon strip, the activity bar of editors such as VS Code. When both areas matter equally and only their sizes change, use a splitter.

## Step by step

1. Keep the panel's state in your application: `open: bool`, `width: u16` and, with several views, `view: u16`.
2. Build it: `SidePanel::new(state.width).open(state.open).panel(|ui| ...).body(|ui| ...).show(ui)`.
3. Dock it right if needed: `.side(Side::Right)`.
4. Let people open and close it: `.on_toggle(|open| Msg::Panel(open))`.
5. Let them resize it: `.on_resize(|width| Msg::PanelWidth(width))`. Without limits the panel can grow until the body keeps a quarter of the area; choose your own with `.limits(18, 48)`, or only a minimum with `.limits(18, None)`.
6. Give several views an icon strip: `.strip(["folder", "search"], |index| Msg::ShowView(index)).active_view(state.view)`. In `update`, `ShowView(index)` sets `view = index` and `open = true`; nothing else is needed, the strip decides when a click closes instead.
7. Choose what closing leaves: `.closed(Closed::Collapse)` (the default) keeps the strip, `.closed(Closed::Hide)` leaves only the edge.

## How it works

- **The edge is a tone difference.** The panel sits on its own surface next to the body; nothing is drawn between them. Pointing at the edge brightens that column one step and raises a two-cell toggle at its middle: a `‹` or `›` on a raised surface, reaching one cell into the body so the panel's own content stays untouched, with its first cell left plain in the toggle's tone. Pointing at the toggle itself lifts it one more step and shows the pillar `▌` in that first cell, pressing it lifts it one more, and a click opens or closes the panel. Dragging the rest of the edge resizes, in the accent; a press that starts on the toggle never resizes.
- **Keyboard.** Tab reaches the edge and shows the same toggle with a breathing pillar, as every pressable shows keyboard focus: Enter or Space toggles and the toggle flashes one step brighter, ← → resize. The global action `toggle-panel` (`alt+b`) toggles from anywhere inside the panel or its body; bind an application action of your own for a shortcut that works everywhere.
- **It slides.** Opening and closing move the panel cell by cell over twice `motion.enter`; its content keeps its width and slides out from the edge instead of reflowing. Reduced motion switches at once.
- **The strip is an activity bar.** It sits at the outer edge whether the panel is open or closed, with the open panel between it and the body. The icon of the shown view is raised with a steady pillar `▌`; pointing at another icon raises it a step. Clicking another icon switches the open panel to its view, clicking the shown view's icon closes the panel, and while the panel is closed any icon opens it on its view. Tab reaches the strip after the edge: ↑ ↓ move, Enter or Space act like a click, and the icon under the keyboard breathes its pillar.
- **Collapse or hide.** With `Closed::Collapse` the panel folds into the strip, which stays. With `Closed::Hide` the strip goes too and the body takes the whole width but one column at the edge. That column looks exactly like the body; only when the pointer reaches it does it light up and show the same `▌›` toggle as an open panel's edge. Click the toggle, press `alt+b`, or drag the edge into the body to open the panel again, on the view it showed last. Without a strip both behave the same: one edge column is left.
- **Dragging a closed edge opens it.** Once the pointer is half the minimum width into the body, the panel opens there.
- **Limits are yours.** By default the panel is at least 8 columns wide and has no upper limit. `.limits(min, max)` takes a number or `None` for `max`; the playground switches between no limit, 18–48 and 30–60.
- **Your state decides.** Every change is a message; the width you get is already within the limits, and the body always keeps a quarter of the area.

## Common mistakes

- **Closing to nothing without a way back.** With `Closed::Hide` or without a strip, only the edge and the shortcut reopen the panel; show the shortcut in your hint bar.
- **Forgetting `active_view`.** Without it no icon looks selected and clicking the shown view's icon cannot close the panel.
- **Closing in `ShowView` as well.** Let the strip decide: it sends `ShowView` only to show a view and closes through `on_toggle`.
- **Important state inside the panel content only.** A fully closed panel is not painted; keep what matters in your application state.
- **Drawing a separator.** The tone difference is the separator.
