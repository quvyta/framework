## When to use

Use an empty state wherever a list, table or panel can have nothing to show: no containers yet, no search results, no alerts. An empty area with no words looks broken; an empty state says why it is empty and what to do next.

## Step by step

1. Start with a title that states the fact: `EmptyState::new("No containers yet")`.
2. Explain in one or two sentences what will appear here or why nothing does: `.message(..)`.
3. Offer the way out when there is one: `.action(Button::new("Run a container").variant("primary").on_press(..))`.
4. Add a quiet icon for large areas: `.icon("inbox")`.
5. Give it the area of the missing content, usually `.fill()` or a fixed height; it centres itself.

## How it works

- **Centred block.** Icon, title, explanation and action are centred as one block, both ways, in the area it gets.
- **Readable width.** The explanation wraps to at most 52 cells, so on wide screens it stays a short paragraph.
- **Small areas.** When rows run out, the icon goes first, then the space above the action, then explanation lines (the last visible line ends with `…`). The title and the action stay as long as there is room.
- **A real button.** The action is a normal Button: it takes focus with tab and works with Enter, Space and the mouse.

## Common mistakes

- **Blaming the user.** "You have no containers" reads as a reproach; "No containers yet" states the fact.
- **An action that does nothing useful.** If there is no real next step, leave the action out.
- **Using it while loading.** Empty is a result. While data is on its way, show a skeleton or a spinner, otherwise the empty state flashes before the data arrives.
