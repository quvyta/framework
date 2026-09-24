## Methods

- `Window::new(title)` — a window named `title`, unfocused, only a surface.
- `.subtitle(text)` — a faint second title after the name, such as a program's own title or its folder.
- `.icon(glyph)` — the glyph before the name: an icon key, or `Glyph::literal` for one the application looked up.
- `.focused(on)` — the window the user works in: one tone raised, bright bold name, pillar down the left edge.
- `.maximized(on)` — the maximize mark offers to restore instead.
- `.shadow(on)` — one column right and one row below are darkened.
- `.on_event(|event| msg)` — makes the window movable and shows its marks; every `WindowEvent` becomes a message.
- `.on_drag(|drag| msg)` — moves and resizes arrive here as a `WindowDrag` instead of through `on_event`: `step` is the `Move` or `Resize`, `total_dx` and `total_dy` count cells from where the button went down (0 on an axis the edge does not move). Does nothing without `on_event`.
- `cx.pointer_shape(rect, shape)` — while painting, asks for the pointer to take a `PointerShape` over `rect`: `Default`, `EwResize`, `NsResize`, `NwseResize`, `NeswResize`. The last shape asked for over a cell wins, from the widget under the pointer or one around it; a widget holding the pointer keeps the first shape it asked for. `Harness::pointer_shape()` answers the shape a test's pointer asks for.
- `ui.place(rect, |ui| ..)` — where a child of a stack goes, counted from the stack's corner; `.id(name)` names it.
- `Ghost::new()` — the tone that shows where something will land; `.mix(ratio)` says how much accent goes into the ground (a quarter by default). It draws no glyph and takes no pointer.

## Events

- `WindowEvent::Focus` — a press anywhere in a window that is not focused, sent before whatever else the press does.
- `Move { dx, dy }` — the title was dragged, or alt with the left button anywhere.
- `Resize { edge, dx, dy }` — an edge or corner moves by that many cells; `dx` is 0 for the top and bottom edges, `dy` for the left and right ones. A left or top side moves the window by the delta and changes its size by the opposite, so the other side stays.
- `Minimize`, `ToggleMaximize` (the maximize mark or a double click on the title), `Close` — a mark was clicked.
- `Dropped` — the button came up after at least one move or resize; a press that moved nothing sends nothing.
- `WindowEdge` — `Left`, `Right`, `Top`, `Bottom` and the four corners; `left()`, `right()`, `top()`, `bottom()` say which sides move.

## Keys

- The window itself takes no keyboard focus: an application binds its own keys and acts through the same messages. The demo binds `ctrl alt w` to focus the next window, and its arrow buttons (reached with `tab`, pressed with `enter`) move or resize the focused one by two cells.

## Mouse

- Title: drag moves, double click maximizes or restores. The cells between its two corner cells move; the top left and top right corners resize.
- Marks: minimize, maximize or restore, close; the three cells of each light up together. They end one cell before the right edge, which is the top right corner.
- Sides: the left column, the right column and the bottom row resize, with all four corners; the rest of the body belongs to what is inside it.
- `alt` and the left button: move from anywhere. `alt` and the right button: resize from the nearest edge or corner, the top edge included.
- A drag reaches the window until the button comes up, wherever the pointer goes.
- Pointer: a resize arrow over each handle (`ew-resize` on the columns, `ns-resize` on the bottom row, `nwse-resize` and `nesw-resize` on the corners), the usual pointer elsewhere; a resize keeps its arrow until the button comes up. Sent as OSC 22 to foot, kitty and WezTerm only, and only when it changes; `QUVYTA_POINTER_SHAPES=on|off` decides for any terminal. Inside tmux or screen nothing is sent.

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
- `split-handle` — the edge handles; `hover`, `active` while dragged. A column lights whole, the bottom row whole, and the top side, which has no row of its own, lights its two corner cells.
- `ghost` — `bg`, the colour a ghost mixes into the ground; the accent when the theme is silent.
