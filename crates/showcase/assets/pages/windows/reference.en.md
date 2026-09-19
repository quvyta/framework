## Methods

- `Window::new(title)` — a window named `title`, unfocused, only a surface.
- `.subtitle(text)` — a faint second title after the name, such as a program's own title or its folder.
- `.icon(glyph)` — the glyph before the name: an icon key, or `Glyph::literal` for one the application looked up.
- `.focused(on)` — the window the user works in: one tone raised, bright bold name, pillar down the left edge.
- `.maximized(on)` — the maximize mark offers to restore instead.
- `.shadow(on)` — one column right and one row below are darkened.
- `.on_event(|event| msg)` — makes the window movable and shows its marks; every `WindowEvent` becomes a message.
- `ui.place(rect, |ui| ..)` — where a child of a stack goes, counted from the stack's corner; `.id(name)` names it.
- `Ghost::new()` — the tone that shows where something will land; `.mix(ratio)` says how much accent goes into the ground (a quarter by default). It draws no glyph and takes no pointer.

## Events

- `WindowEvent::Focus` — a press anywhere in a window that is not focused, sent before whatever else the press does.
- `Move { dx, dy }` — the title was dragged, or alt with the left button anywhere.
- `Resize { edge, dx, dy }` — an edge or corner moves by that many cells; `dx` is 0 for the top and bottom edges, `dy` for the left and right ones.
- `Minimize`, `ToggleMaximize` (the maximize mark or a double click on the title), `Close` — a mark was clicked.
- `Dropped` — the button came up after at least one move or resize; a press that moved nothing sends nothing.
- `WindowEdge` — `Left`, `Right`, `Top`, `Bottom` and the four corners; `left()`, `right()`, `top()`, `bottom()` say which sides move.

## Keys

- The window itself takes no keyboard focus: an application binds its own keys and acts through the same messages. The demo binds `ctrl alt w` to focus the next window, and its arrow buttons (reached with `tab`, pressed with `enter`) move or resize the focused one by two cells.

## Mouse

- Title: drag moves, double click maximizes or restores.
- Marks: minimize, maximize or restore, close; the three cells of each light up together.
- Body: right column, bottom row and their corner resize; the rest of the body belongs to what is inside it.
- `alt` and the left button: move from anywhere. `alt` and the right button: resize from the nearest edge or corner.
- A drag reaches the window until the button comes up, wherever the pointer goes.

## Behaviour

- Placed children are drawn in the order they are added, the last one on top, and it takes the pointer first where they overlap. A rectangle may reach past the stack on any side; what lies outside is not drawn and takes no pointer.
- A placed window may draw one cell past its right and bottom edges, which is where its shadow falls; that cell never takes the pointer.
- The body keeps the pillar column, one cell after it, the right column and the bottom row free.
- A narrow title shortens the subtitle first, then leaves it out, then shortens the name with `…`; the marks are always shown.
- ASCII glyph mode draws the marks as ` - `, ` + `, ` x ` (` o ` for restore) and the pillar as a cell of colour.
- Nothing about the window changes by itself: stacking order, focus, size limits, snapping and tiling stay in the application.

## Theme keys

- `window` — `bg`, `pillar`; state `focus`.
- `window-title` — `bg`, `fg`, `bold`; state `focus`.
- `window-subtitle` — `fg`; state `focus`.
- `window-shadow` — `scrim`, `strength` in percent.
- `close-mark` — the three marks; `active` while the window is focused, `hover` under the pointer.
- `split-handle` — the edge handles; `hover`, `active` while dragged.
- `ghost` — `bg`, the colour a ghost mixes into the ground; the accent when the theme is silent.
