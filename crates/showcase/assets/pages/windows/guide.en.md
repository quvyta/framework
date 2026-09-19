## When to use

Use a window when the user decides where a surface sits and how big it is: a desktop of programs, a tool box dragged out of the way, two documents held side by side for comparison. When the layout is the application's decision, a row, a column or a splitter is the right thing; a window costs the user the work of placing it.

## Step by step

1. Keep the windows in your own state, in stacking order, bottom first: a name, a rectangle and whatever the body shows.
2. Give the desktop a stack and place each window: `ui.place(win.rect, |ui| ..).id(win.name)`. The rectangle counts from the stack's top left corner; name every placed window, so its state and a drag in progress follow it when the order changes.
3. Build the window inside: `ui.add_with(Window::new(name).subtitle(..).icon(..).focused(..), |ui| ..)` and put the body's widgets in the closure.
4. Make it movable: `.on_event(move |event| Msg::Window(name, event))`. Without it the window is only a surface, with no marks and no handles.
5. Apply the events in `update`: add the deltas to the rectangle, raise the window on `Focus`, remove it on `Close`.
6. Set your own limits while applying them: a smallest size, staying on the desktop, what maximizing means. The window reports the pointer; the rules are yours.
7. Act on the drop: `WindowEvent::Dropped` arrives when the button comes up after a move or a resize. Snapping to an edge, landing a ghost and saving a size belong there, not in every delta.

## How it works

- **A surface, not a box.** One row of title, then the body. No lines anywhere: the title strip is one tone lighter than the body, and the focused window climbs one tone above the others, with a bright bold name and the pillar `▌` down its left edge.
- **Three marks.** Minimize, maximize (restore while maximized) and close sit at the right end of the title, three cells each, and light up together under the pointer like every close mark in the family. A press that is released somewhere else does nothing.
- **Moving and resizing.** The title drags the window; a double click on it maximizes. The body's right column, bottom row and their corner are handles: invisible until the pointer reaches them, then one tone brighter, and the accent while dragged, exactly like a splitter's boundary. The left edge is the pillar's column and the top edge is the title, so they have other work: hold alt and drag with the left button to move from anywhere, or with the right button to resize from the edge or corner nearest the pointer, including left and top.
- **A drag belongs to the window.** Once a button goes down on the window, every drag and the release reach it, even far outside the screen, so the pointer never loses the window it carries.
- **A terminal in the body keeps its mouse.** While a program inside a [`Terminal`](terminal) reads the mouse, plain presses in the body are its own. The title, the marks, the handles and every alt drag are still the window's, so a window around htop stays movable.
- **The deltas are cells since the last message.** `Move { dx, dy }` and `Resize { edge, dx, dy }` say how far the pointer went, not where the window should be. `edge.left()`, `right()`, `top()` and `bottom()` say which sides move.
- **Every state is drawn.** Focused and unfocused, hovered marks and handles, a narrow title that shortens the subtitle first and then the name with `…` while the marks stay, a window that reaches past the screen and is cut off, and an empty desktop, which is the application's to fill (the demo offers to open the windows again). A window with no `on_event` is the disabled state: a surface that cannot be moved, with no marks.
- **A ghost for a slow line.** Over SSH every drawn frame costs bytes, so a window can stay where it is while only a `Ghost` follows the pointer; on the drop the window jumps there in one frame. The same surface previews where a window would snap: a rectangle of the accent mixed into whatever lies under it, no lines, nothing to click. Place it in the same stack after the windows, and choose the strength with `.mix(0.25)` for a dragged ghost, `.mix(0.20)` for a snap preview. The playground's Ghost drag switch shows both.
- **Optional shadow.** `.shadow(true)` darkens one column right of the window and one row below it, so it floats over the ground. With reduced motion or in 16 colours it is not drawn.
- **Sixteen colours.** Every surface tone falls to black there, so the title strip takes the accent on the focused window and a grey on the others; the name is dark on both.

## Common mistakes

- **Letting the window decide.** It never moves itself. Nothing happens until `update` applies the event, which is what makes snapping, tiling and a smallest size possible at all.
- **Forgetting the name.** An unnamed placed window loses its memory when the stacking order changes, and a drag ends with it.
- **A rectangle smaller than the content.** The body keeps the pillar column, one cell after it, the right column and the bottom row; below about 20 × 5 nothing readable is left. Keep your own minimum, as the demo does.
- **Windows for a fixed layout.** Two panes that always share the screen belong in a splitter; the user should not have to place them.
