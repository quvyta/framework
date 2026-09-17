## When to use

Use a tooltip for a short hint that helps but is not needed to use the screen: what a terse toolbar button does, the full text of a name shortened with `…`, the meaning of a status marker. Keep it to one line. Anything the user must read to succeed belongs on the screen itself, and anything interactive belongs in a popover.

## Step by step

1. Wrap the widget: `ui.add_with(Tooltip::new(t!("restart-tip")), |ui| { ui.add(Button::new(t!("restart"))); })`.
2. If the hint matters for keyboard users too, add `.on_focus(true)`; it shows while focus is inside.
3. Choose a side with `.placement(Placement::Above)` when the space below is busy.

## How it works

- **It waits.** The tooltip appears after the pointer has rested on the widget for the theme's `motion.hover-delay` (450 ms in the built-in themes), and disappears as soon as the pointer leaves.
- **It is a layer that never stands in the way.** One row of text on the overlay surface, no frame, drawn over everything. It catches no clicks: a click on the widget it explains, or anywhere else, works as if the tooltip were not there.
- **It finds room without hiding the pointer.** Below by default; it flips to the other side when there is no room and moves aside rather than covering the cell under the pointer.
- **It fades in** over `motion.enter`: the text colour blends from the surface to full strength. With reduced motion it is there at once.
- **Keyboard focus is opt-in.** With `.on_focus(true)` the tooltip shows at once while focus is inside, so keyboard users get the same hint.
- **Plain content works too.** A text with no mouse behaviour still gets hover, because the tooltip listens for the whole area it wraps.

## Common mistakes

- **Hiding essential information.** Touch and keyboard users may never see it; put required text on screen.
- **Long sentences.** A tooltip is one line and is shortened with `…` when the screen is narrow.
- **Tooltips on everything.** A clear label does not need one; tooltips on every control turn resting the pointer into noise.
