## When to use

Wrap content in a scroll view when it can be taller than the space it gets: documents, long forms, settings pages. Lists scroll on their own and do not need one.

## Step by step

1. Add it with its content: `ui.add_with(ScrollView::new(), |ui| { ... })`.
2. Give it a height, usually `.fill()` or `.height(Length::Cells(n))`.
3. Name it with `.id(...)` when several scroll views can appear in the same place, so each keeps its own position.

## How it works

- **Content is laid out at full height** and drawn through a window. Everything outside the window is clipped, including half-visible widgets.
- **Three ways to scroll.** The mouse wheel moves three rows, the scrollbar can be clicked and dragged, and while the view is focused ↑ ↓ PgUp PgDn Home End move it.
- **Focus pulls the view.** When Tab moves focus to a widget inside that is out of sight, the view scrolls just enough to show it.
- **Following the end is one option.** `.follow_end(true)` suits content that grows while the person watches: a conversation, a build's output. The view opens at the end and stays there as lines arrive, gliding to each new end, or jumping when motion is reduced. Scrolling up to read holds the view where the person put it, and a faint note at the bottom counts the lines below; End, scrolling back to the bottom or a click on the note follows again.
- **Focus and following agree.** A move to show a focused widget that leaves the end stops following, like scrolling up, so the widget stays in view. A focused widget at the end, such as a reply field under a conversation, keeps following and stays in view as the content grows.
- **The scrollbar is quiet.** The scrollbar column appears only when content overflows, in the style the theme picks; the thumb brightens while hovered or dragged. The content column leaves room for it, so nothing draws under the bar.
- **Position is remembered.** Inside a router page, coming back to the page restores where the user was.

## Common mistakes

- **Scrolling to the end from the application.** Growing content does not need the application to track the position; `.follow_end(true)` does it and respects the person scrolling up.

- **Scroll view inside a scroll view.** Nest only when the inner one has a fixed height.
- **Putting a list inside.** Lists virtualise and scroll themselves; give them a height instead.
