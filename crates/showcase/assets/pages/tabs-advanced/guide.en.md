## When to use

Use these options when tabs stand for things the user opens and closes: files in an editor, terminals, open queries. Plain `Tabs` stays right for fixed views such as Overview and Settings; add only the options your tabs need.

## Step by step

1. Keep the tabs in your state as a list and the open one as an index.
2. Start plain: `Tabs::new(labels).active(self.active).on_select(Msg::Open)`.
3. Let people close tabs: `.closable(|i| Msg::Edit(TabEdit::Close(i)))`. Keep a start page open with `.pinned([0])`.
4. Let them reorder: `.reorderable(|from, to| Msg::Edit(TabEdit::Move { from, to }))`.
5. Apply both in `update` with one line: `edit.apply(&mut self.tabs, &mut self.active)`. It removes or moves the tab and keeps the open tab pointing at the same one.
6. Choose a width with `.tab_width(TabWidth::Fixed(16))` or `TabWidth::Fill`, and what happens when tabs do not fit with `.overflow(Overflow::Arrows)` or `Overflow::Menu`.
7. Add a right-click menu with `.context_menu(|index| items)`: return the `ContextItem`s for tab `index` (Close, Close others, Close to the right, Pin, Duplicate) and handle their messages in `update`.

## How it works

- **Every option is independent.** Without an option there is no trace of it: no close marks, no arrows, no extra keys. Turn on any combination.
- **Closing.** A faint `×` sits at the end of each tab. It brightens on the open tab and on a hovered tab, and lights up more when the pointer is on the mark itself. A click on it, a middle click anywhere on the tab, or ctrl+w on the focused strip closes. Pinned tabs have no mark and ignore all three.
- **Widths.** `Fit` follows the label. `Fixed(n)` makes every tab n cells and ends long labels with `…`. `Fill` shares the strip equally and stops shrinking at a readable minimum, after which the strip overflows.
- **Overflow.** `Arrows`, the default, puts an arrow button at both ends that scrolls one tab without opening anything; ctrl+PgUp, ctrl+PgDn and the wheel do the same. `Menu` puts a control at the end with the number of hidden tabs; clicking it or pressing ↓ lists them. Both controls are small buttons: hover brightens them and shows the pillar in their first cell, an arrow flashes when pressed, and the menu control stays lit with a steady pillar while its menu is open.
- **Mouse.** Everything the keys do has a mouse way: click a tab to open it, an arrow or the wheel to scroll, the count to list hidden tabs, `×` or a middle click to close, drag to reorder. With the menu open, one click on a tab closes the menu and opens the tab; a click on the control only closes the menu.
- **Reordering.** Drag a tab: it floats as a ghost under the pointer and the other tabs make room around a tinted slot where it will land. Releasing sends `from` and `to`. ctrl+shift+← and → move the open tab from the keyboard.
- **Dragging past what is on screen.** When tabs are hidden, hold the dragged tab on an arrow (or past the end of the strip): the arrow lights up, and after 400 ms the strip scrolls one tab, then one more every 150 ms, a little faster the further past the end you pull. The tinted slot follows the tabs coming into view, so letting go drops the tab exactly where the slot is. Move off the arrow and scrolling stops at once; at the last tab the arrow sinks and nothing more happens. The wait means passing over an arrow on the way to a drop never scrolls. Reduced motion changes nothing here: each step is a jump anyway. `.on_drag_scroll(|first| msg)` tells you about every step; this page logs them.
- **Right-click menu.** A right click on a tab opens that tab's menu at the pointer and leaves the open tab alone; the menu key or shift+F10 opens the open tab's menu under it. The entries are yours, so the menu can say "Unpin" on a pinned tab and grey out "Close to the right" on the last one. A right click on another tab moves the menu, and one left click beside it closes the menu and does what it pressed. The menu of hidden tabs and the right-click menu never show together. Without the option a right click does nothing.
- **One model.** `TabRail` uses the same options and the same behaviour, laid out vertically.

## Common mistakes

- **Changing the list without adjusting the open index.** Use `TabEdit::apply`; closing the open tab should open its neighbour, not jump to the first tab.
- **Closable tabs for fixed views.** If a tab cannot come back, do not let people close it, or pin it.
- **Arrows on a strip that always fits.** They only appear when needed; do not reserve space for them.
- **A menu that repeats the options.** The right-click menu sends your messages; it does not close or pin by itself. Pinning from the menu means keeping pins in your state and passing them to `.pinned(..)`.
