## Methods

- `HelpLayer::new(on_close)` — the layer; Esc and the close mark send `on_close`.
- `.dismissable(bool)` — whether Esc and the close mark close it, together. Default: `true`. `false` hides the mark; the application closes the layer.
- `.hint(key, label)` — a key of the current screen that no widget declares, listed under "This screen" after the focused widget's own keys.
- `.width(cells)` — width with padding. Default: 64; narrow screens shrink it.

## Keys

- Typing, `backspace`, `ctrl w`, `ctrl u`, `left` `right` `home` `end` edit the filter; pasting works too.
- `up` `down` scroll by a row, `pgup` `pgdn` by a page.
- `esc` sends the close message while dismissable.
- The global `help` action (`?` by default) reaches `App::action("help")`; open the layer there.

## Mouse

- The wheel scrolls the list; the scrollbar can be pressed and dragged.
- The close mark `×` in the top right corner of the surface, on the row above the title, closes: three cells that light up under the pointer, drawn only while dismissable.
- Everything else on the dimmed screen is swallowed.

## Behaviour

- Groups: the keys of the focused widget, then screen hints, then `[app]` actions, then `[global]` actions; groups without matches are hidden.
- Labels: `keys.<action>` and `quvyta.keys.<action>` in the active language for bindings, and the widget's own `quvyta.widget.<label>` for the keys it declares; every chord of an action is shown.
- Up to 18 rows before scrolling, fewer on short screens; the filter and scroll position reset when it opens again.
- A modal layer: a pillar down the left edge, focus trap, paused application shortcuts, focus restored on close.
- The hint line shows `esc close` while dismissable and `↑↓ scroll` while the list scrolls.

## Widget keys

- `Widget::keys(&self, env: &Env) -> Vec<WidgetKey>` — the keys the widget takes while it has the focus; nothing by default. The runtime asks only the widget the keys are with, once a frame, while it paints, and the layer lists the answer at the top of "This screen". A layer takes the keyboard with it, so the widget asked is the one that had the focus when the layer opened: a layer opened while a table has the focus lists the table's keys, not those of the filter inside it.
- `WidgetKey::new(keys, label)` — one row: the keys as drawn, `"↑↓"` or `"pgup pgdn"`, and the label. `WidgetKey` is `#[non_exhaustive]`, so build it with the constructor.
- The framework's own widgets declare: `Table` (move, scroll, `enter` with an activate message, `space` when it checks rows, `s` when it sorts), `List` (move, scroll, `enter`, `space` with check marks), `Tree` (move, scroll, `←→` and `enter` when it expands), `TextInput` (`enter` when it submits) and `TextArea` (`ctrl enter` when it submits).
- Arrows come from the icon set, so they are `↑↓` in Unicode and `^v` in ASCII.

## Theme keys

- `modal` (`bg`, `padding`, `pillar`), `modal-title`, `layer-backdrop` — the surface; `close-mark` — the close mark.
- `layer-filter` (`bg`, `fg`), `layer-filter-mark`, `layer-filter-placeholder`, `layer-filter-cursor`, `layer-match`.
- `help-group`, `help-key` (`bg`, `fg`, `bold`), `help-label`.
- `layer-hint-key`, `layer-hint-label`, `scrollbar`.
- `[icons]` — `search`, `close`, `pillar`, `scroll-thumb`, `scroll-track`.
- Language — `quvyta.help.title`, `.screen`, `.app`, `.global`, `.empty`; `quvyta.layer.filter`, `.close`, `.scroll`; `quvyta.widget.move`, `.scroll`, `.open`, `.check`, `.sort`, `.fold`, `.submit`.
