## Methods

- `PageTransition::new(key)` — a container that animates when `key` changes; cross-fades by default.
- `.slide(bool)` — the incoming page also slides into place. Default: `false`.
- `.direction(Navigation)` — which side a sliding page comes from. Default: `Forward`.
- `Router::direction() -> Navigation` — `Forward` after `push` or `replace`, `Back` after `back`.
- `Navigation::Forward`, `Navigation::Back`.

## Behaviour

- Duration: `motion.page`, eased out. Themes without `page` use twice `motion.enter`.
- First half: old symbols, their text colour blending towards the background. Second half: new symbols rising from it. Backgrounds blend throughout.
- Slide distance: an eighth of the width, at most 6 cells, stepped one cell at a time with the same progress as the colours; forward from the right, back from the left.
- No transition on the first paint, when the area changed size, or with reduced motion.
- Colours that are not 24-bit switch at the halfway point instead of blending.
- Wide glyphs cut by the area edge are drawn as spaces during the transition.

## Theme keys

- `[motion] page` — duration such as `"260ms"`.
