## When to use

Use a popover for a small, optional panel that belongs to one control and should not take over the screen: filters under a Filters button, the details of a deploy, a colour picker next to a field. The rest of the screen stays visible and usable once it closes. For a choice from a list use a select; for a decision the user must make before going on, use a dialog.

## Step by step

1. Keep whether it is open in your state: `filters_open: bool`.
2. Build it with the anchor and the content: `Popover::new(self.filters_open).anchor(|ui| …).content(|ui| …).show(ui)`.
3. Toggle it from the anchor, usually a button: `Button::new(t!("filters")).on_press(Msg::ToggleFilters)`.
4. Close it when asked: `.on_dismiss(Msg::CloseFilters)` arrives on Esc and on a click outside.
5. If the content is a form, add `.focus_inside(true)` so the keyboard starts in the layer.

## How it works

- **It is a layer.** The content is drawn over everything else on the overlay surface, with no frame; the tone difference is its edge. Nothing below moves.
- **It stands apart from what it opens over.** Over the screen ground the layer keeps the overlay tone. Opened inside a panel whose tone is nearly the same, it moves one small step towards the theme's text colour (lighter on a dark theme, darker on a light one) until the edge shows again. The step comes from the theme's own colours, so it works in every theme without a colour written by hand. Menus, lists, tooltips, toasts and dialogs do the same; a widget of your own gets it with `PaintCx::floating`.
- **It finds room.** Below the anchor by default, or the side chosen with `.placement(…)`. When that side has no room it flips to the opposite one, and it is always slid back onto the screen.
- **It unfolds from the anchor** over the theme's `motion.enter`, row by row (column by column for side placements). With reduced motion it appears at once.
- **Dismissing is yours.** Esc and a click outside send the dismiss message.
- **The click still counts.** A popover does not dim the screen, so it does not hold the screen either: the click that closes it also reaches what it landed on. With the filters open, one click on another button closes the filters and presses that button; one click on a menu entry changes the page. Clicking the anchor itself reaches the anchor, which usually toggles. If a button outside the anchor opened the popover, a click on that button only closes it, so it never closes and reopens in one go. Dialogs are different: they dim the screen and swallow every click until they close.
- **Focus inside is opt-in.** With `.focus_inside(true)` focus moves to the first focusable widget in the layer when it opens and returns to where it was when it closes.
- **Layers stack.** A select inside a popover opens its own list above it; a click outside that list closes the list and then counts inside the popover as usual.

## Common mistakes

- **Forgetting `on_dismiss`.** Without it Esc and outside clicks do nothing, and the popover stays open until the anchor is pressed again.
- **Putting a whole page in it.** A popover is for a few controls; long content wants its own screen or a side panel.
- **Wrapping text.** Give long lines `.no_wrap()` or a fixed width, or the layer grows as wide as the screen.
