## When to use

Use an accordion when a page has several groups of details and people usually need one or two of them at a time: a container's overview, ports, volumes and environment. When every group is needed at once, show them as panels; when the groups are views of the same thing, use tabs.

## Step by step

1. Keep which sections are open in your application: `open: [bool; 4]`.
2. Give the titles: `Accordion::new(["Overview", "Ports", "Volumes"])`.
3. Add one child per section, in the same order: `ui.add_with(accordion, |ui| { ui.column(..); ui.column(..); ... })`.
4. Pass the state and handle changes: `.open(state.open).on_toggle(|index, open| Msg::Section(index, open))`.
5. Turn on what you need: `.single(true)` keeps one section open; `Section::new(title).icon("folder").detail("3 mounts")` adds an icon and a faint detail.

## How it works

- **Surfaces, not frames.** A title row sits one tone above the page, the open body sits on the surface under it, and one row of ground separates sections.
- **Title rows behave like list rows.** Hover and keyboard focus raise the row, show the pillar and slide the icon and title one cell right. The chevron and the detail stay anchored.
- **Opening unfolds.** The body's rows are reserved at once and the content appears row by row over twice `motion.enter`. Closing is immediate, so nothing below jumps back and forth. Reduced motion opens at once.
- **Your state decides.** Clicking only sends a message; the section opens when your `update` stores it.
- **One at a time** sends close messages for the other open sections together with the open message.

## Common mistakes

- **Children that do not match the titles.** The n-th child is the body of the n-th title; keep them in the same order.
- **Important actions inside closed sections.** People do not open every section; keep the main action outside.
- **An accordion in a fixed short area.** Open bodies take their natural height; put the accordion in a scroll view, or use a widget dock that shares a fixed height.
