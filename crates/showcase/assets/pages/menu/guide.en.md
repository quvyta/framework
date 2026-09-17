## When to use

Use a menu for the places of an application that are always there: the sidebar of a console, the sections of a settings screen. For a list of data rows use `List`; for open things that can be closed use `Tabs` or `TabRail`.

## Step by step

1. Build items with a key your application understands: `MenuItem::new("deploys", t!("menu.deploys"))`.
2. Add an icon and a badge where they help: `.icon("success", Some("success")).badge("3")`.
3. Put items in titled groups: `MenuGroup::new("workspace", items).title(t!("menu.workspace"))`.
4. Show the menu with the current page: `Menu::new(groups).selected(Some(&self.page)).on_select(|key| Msg::Go(key.to_owned()))`.
5. Put it in the sidebar of an `AppShell`: `.sidebar(|ui| { ui.add(menu).fill(); })`.
6. To let groups fold, add `.collapsible(|group, open| Msg::Group(..))` and pass the closed groups with `.collapsed(..)`.

## How it works

- **Rows, not buttons.** Headings are faint text. The current item is raised with the accent pillar, which breathes while the menu has focus. A hovered item rises softly. Icons and labels slide one cell; badges stay anchored at the right.
- **The keyboard moves a cursor.** ↑ and ↓ move a highlight without opening anything, so passing through the menu does not load every page on the way. Typing a letter jumps to the next item that starts with it. Enter or Space opens; a click opens at once. There is only ever one highlight: moving the pointer onto a row moves the cursor there, and the keys continue from it.
- **Folding is an option.** Without `collapsible` headings are plain text. With it, titled headings get a chevron and can be reached by the keyboard; Enter or a click toggles, → opens, ← closes, and ← on an item goes to its heading. A foldable heading rises and slides like an item; its chevron stays at the right.
- **Long menus scroll.** The wheel scrolls, the cursor and the current item stay in view, and a scrollbar appears when needed.
- **The same rows as List.** Menu, List, Tree, Accordion and TabRail draw rows with one shared painter, so hover, selection and slide match everywhere: fixed marks first, then the sliding icon and label, then the anchored right side.

## Common mistakes

- **Navigating on every arrow key.** The menu sends `on_select` only for Enter, Space or a click; do not open pages from anything else.
- **Status colour without a mark.** Colour an icon, not the label.
- **Folding groups without titles.** A group needs a title to be folded.
