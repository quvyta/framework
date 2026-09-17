## When to use

Use a tab rail to switch between a handful of open workspaces that each fill the screen: projects in an IDE, clusters in a dashboard, connections in a database client. For views of one thing use horizontal `Tabs`; for navigating many pages use a menu.

## Step by step

1. Build a `RailTab` per workspace: `RailTab::new("quvyta").icon("folder")`.
2. Add what is worth seeing at a glance: `.status("success").badge("3")` for three running containers.
3. Show the rail with the open tab: `TabRail::new(tabs).active(self.active).on_select(Msg::Open)`.
4. Give it a width and the height you have: `.width(Length::Cells(24)).fill_height()`.
5. Turn on what you need: `.collapsed(true)` for a thin strip of icons, `.on_add(..)` for a `+` row, `.closable(..)` and `.reorderable(..)` exactly as on `Tabs`, applied with `TabEdit::apply`.
6. For project tabs you hit often, make them thicker: `.row_height(3)` turns every tab into a three-line block with one empty line between blocks. Any height works, and `.gap(n)` sets any other spacing; `.gap(0)` stacks the blocks. Try both with the number fields in the playground.
7. Choose what the collapsed strip shows: `.collapsed_marker(CollapsedMarker::Number)` for positions, `Initial` for letters, `Icon` (the default) for icons. Let users pick per tab, as in an editor's activity bar: keep their choice in your state and pass it as `RailTab::marker(..)`, or change the tab's `.icon(..)`.
8. Give tabs a right-click menu: `.context_menu(|index| vec![ContextItem::new("Close others", Msg::CloseOthers(index)), ..])`. Build the entries from your state, disable the ones that would do nothing, and handle their messages in `update` like any other.

## How it works

- **Rows, not boxes.** By default each tab is one row. The open tab is raised with the accent pillar, which breathes while the rail has keyboard focus. A hovered tab rises softly. The icon and name of the open tab and of a hovered tab slide one cell; the status dot, badge and close mark stay where they are.
- **Tall blocks.** With `row_height(n)` above one, every tab is a raised block of `n` lines, still without a frame: the shape is the tone. The icon, name, status and badge sit on the middle line, the close mark sits in the block's top right corner like the close mark of a dialog, and the pillar of a hovered or open tab runs down the whole block. With a `row_height` above 1 tabs are separated by one empty row unless `gap` says otherwise; `gap(0)` stacks the blocks. Every line of a block is the tab: hover, click, middle click, drag and right click work anywhere on it except on the close mark itself. In a two-line block the first line is also the middle line, so the mark ends the name's line as in a one-line row. The add row and a collapsed strip keep the same height, and the name card of a collapsed rail is as tall as its block. A rail shorter than one block shows blocks cut to its height rather than half a tab.
- **Right-click menu.** With `context_menu`, a right click on a tab (or on the name card of a collapsed rail) opens that tab's menu at the pointer without opening the tab; the menu key or shift+F10 opens the menu of the open tab below it, with its first entry highlighted. The menu is the same `ContextMenu` as everywhere: arrows, typing a letter, Enter, Esc, and a click beside it closes it and still does what it pressed. Without the option a right click does nothing.
- **Markers.** A collapsed tab shows one cell: its icon (or its initial when it has none), its initial, or its position counted from 1. The initial keeps accents and turns upper case when that stays one character. Positions past 9 show `…` rather than a digit that would point at another tab; the name card still names the tab. A tab's own `marker` wins over the rail's `collapsed_marker`, and a number follows its tab when it moves. The demo's tab menu has a Marker submenu that sets a project's icon, letter or number, or returns it to the rail's choice.
- **Status needs a marker.** The status colour is always a dot, and the badge is plain faint text beside it.
- **Collapsed.** The rail narrows to icons; a status colours the icon. The icon never moves: on a hovered or open tab the pillar appears in the cell right before it, so the strip does not look wider and the icon never reaches the scrollbar column. A tab without an icon shows the first letter of its name. Hovering a tab shows a name card beside the rail with its name and badge. The card is part of the rail: move the pointer onto it and it stays, rises under the pointer, opens the tab when clicked and, with closing on, ends in the tab's close mark on its middle line. It disappears when the pointer leaves both the row and the card. While the rail has keyboard focus the card names the open tab, so the icons are never a guessing game.
- **Same model as Tabs.** Closing (a faint `×`, middle click, ctrl+w), pinning and dragging behave exactly like `Tabs`; while dragging, the other tabs move to open a tinted slot where the tab will land. ctrl+shift+↑ and ↓ move the open tab.
- **Many tabs.** When the rows do not fit, the rail keeps the open tab visible and shows a scrollbar; the wheel scrolls it, and a click or drag on the scrollbar moves straight there.
- **Dragging to hidden rows.** Hold a dragged tab on the first or last row in view, or past the rail's top or bottom: the scrollbar lights up and after 400 ms the rail scrolls one row (a whole block with tall rows), then another every 150 ms, faster the further past the edge you pull. The tinted slot follows, so the tab lands where you see it. Moving back into the middle stops at once, and the rail stops at its first and last row. Passing over an end row on the way to a drop scrolls nothing, because of the wait.
- **Scrolling tall blocks.** Tall blocks scroll one whole block at a time and never stop half way.
- **Mouse.** Click to open, `×` or middle click to close, drag to reorder, right click for the menu, wheel or scrollbar to scroll, and in a collapsed rail the same on the name card.

## Common mistakes

- **Using the rail as a page menu.** Tabs are open things that can close; a menu lists places that are always there.
- **Colour without a dot.** Do not tint the name to show status; use `.status(..)`.
- **Long names in a narrow rail.** Names end in `…`; give the rail room or collapse it.
- **Tall blocks in a short rail.** Three-line blocks take four lines each with the empty line after them; give the rail the height for the tabs you expect, or keep one-line rows. A rail shorter than one block still works, it just shows one cut block at a time.
- **Numbers for many tabs.** Past nine tabs numbers turn into `…`; use icons or letters when you expect more.
- **Menu entries that do nothing.** A "Close others" with nothing else to close should be disabled, not silently ignored.
