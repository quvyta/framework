## When to use

Use an empty state wherever a list, table or panel can have nothing to show: no containers yet, no search results, no alerts. An empty area with no words looks broken; an empty state says why it is empty and what to do next.

## Step by step

1. Start with a title that states the fact: `EmptyState::new("No containers yet")`.
2. Explain in one or two sentences what will appear here or why nothing does: `.message(..)`.
3. Offer the way out when there is one: `.action(Button::new("Run a container").variant("primary").on_press(..))`. Call `.action(..)` once for each equal choice; the first one is the primary choice.
4. When the empty state reports success, a warning, an error or neutral news, add `.tone(ToastKind::Danger)` (or the matching kind). The icon and title take the status colour, and the icon carries the matching sign.
5. Add a quiet icon for a large area with no status tone: `.icon("inbox")`.
6. Give it the area of the missing content, usually `.fill()` or a fixed height; it centres itself.

## How it works

- **Centred block.** Icon, title, explanation and actions are centred as one block, both ways, in the area it gets.
- **Several actions.** Buttons share one centred row with the same two-cell gap as dialog actions. If the row is too wide, each button stands on its own centred row with a blank row between them. The first button added is first in the reading and focus order, so it is the primary choice.
- **Readable width.** The explanation wraps to at most 52 cells, so on wide screens it stays a short paragraph.
- **Status tone.** `ToastKind` reuses the colours and signs used by `Toast`. The sign stays visible in a sixteen-colour terminal and in ASCII icon mode; colour never carries the meaning by itself.
- **Small areas.** When rows run out, the icon goes first, then the space above the actions, then explanation lines (the last visible line ends with `…`). The title and the actions stay as long as there is room.
- **Real buttons.** Every action is a normal Button: it takes focus with Tab and the arrow keys, and works with Enter, Space and the mouse.

## Common mistakes

- **Blaming the user.** "You have no containers" reads as a reproach; "No containers yet" states the fact.
- **An action that does nothing useful.** If there is no real next step, leave the action out.
- **A colour without a sign.** Use `.tone(..)` for a status so the meaning survives a limited terminal.
- **Using it while loading.** Empty is a result. While data is on its way, show a skeleton or a spinner, otherwise the empty state flashes before the data arrives.
