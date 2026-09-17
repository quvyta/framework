## Theme

- `[style.scrollbar]` — `style`: `block` (default), `half`, `thin` or `dots`; `track`, `thumb` colours. Any other word is a diagnostic naming the valid words.
- `[style."scrollbar:hover"]` — colours while the bar is hovered or dragged.
- `[style."scrollbar.<style>"]` — colours for one style only, e.g. `[style."scrollbar.dots"] track = "$muted"`.

## Code

- `ScrollbarStyle` — `Block` (default), `Half`, `Thin`, `Dots`; `ScrollbarStyle::ALL` in that order, `.name()`, `ScrollbarStyle::from_name(word)`.
- `List::scrollbar(style)`, `ScrollView::scrollbar(style)` — pin a style on one widget.
- `WidgetStyle::word(key)` and `StyleProps::word(key)` — read a word property in a custom widget.

## Behaviour

- One column at the right edge, drawn only when the content is taller than the view.
- The thumb brightens while hovered or dragged in every style.
- `block` draws only background colours in every glyph mode. ASCII mode draws the thumbs of `half` and `thin` as coloured cells; `dots` keeps a `.` track.

## Icons

- `scroll-track`, `scroll-thumb` (half), `scroll-thin`, `scroll-dot`, `scroll-dot-thumb`. `block` uses no icon.
