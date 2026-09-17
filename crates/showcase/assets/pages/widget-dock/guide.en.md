## When to use

Use a widget dock for a side area made of independent tools, like an IDE's side panel: source control, containers, ports, activity. People open the tools they care about, close the rest and put them in their own order, and the application remembers it.

## Step by step

1. Keep the layout in your application so it can be saved: the widget ids in display order and which are open.
2. Build the titles in that order: `WidgetDock::new(order.iter().map(|id| title(id)))`.
3. Add one child per widget in the same order, usually `.fill()` columns or lists.
4. Pass the open state in display order and handle toggles: `.open(open).on_toggle(|position, open| Msg::Toggle(position, open))`.
5. To let people reorder, add `.on_move(|from, to| Msg::Move(from, to))` and in `update` remove the id at `from` and insert it at `to`.
6. Give the dock a fixed height, for example `.fill()` inside a side panel.

## How it works

- **Open widgets share the height.** A widget that needs less than its share keeps its natural height; taller ones, such as lists, split what is left evenly and scroll inside.
- **The same sections as the accordion.** Title rows, hover and focus, chevrons, icons and details look and behave the same; only the height rule differs.
- **Dragging a title** lifts the widget: its title follows the pointer as a ghost on the overlay tone with the accent pillar, the other widgets close ranks, and a tinted row shows where it will land. Releasing sends one move message. A click that does not move toggles instead.
- **Keyboard:** `ctrl shift ↑↓` moves the focused widget one place.
- **Opening and closing animate** within the dock's fixed height; reduced motion switches at once.

## Common mistakes

- **Keeping the order inside the widget.** The dock has no order of its own; if you do not apply the move message nothing moves.
- **Indexing by position after a move.** Messages carry display positions; map them to your ids through the order.
- **No height.** In a column that only measures its content the dock cannot share anything; give it `.fill()` or cells.
