## When to use

Use tabs to switch between views of the same thing: the sections of a page, the open files of an editor. Each tab is a place, not an action.

## Step by step

1. Give it labels: `Tabs::new([t!("tabs.overview"), t!("tabs.activity")])`.
2. Show the open one from your state: `.active(self.tab)`.
3. Receive switches: `.on_select(Msg::Tab)`.
4. Add `.numbered(true)` when numbers help, like the Demo, Code, Guide, Reference tabs of this showcase.
5. Put a count on a tab with `.badge(index, count)`, such as the updates waiting behind an Updates tab. Pass the count as it is: zero shows nothing.
6. Show that a tab's work is running with `.busy(index, true)`, such as a program writing or a build, and mark a tab that waits for the person with `.status(index, "success")`.

## How it works

- **The open tab is a surface.** It sits on the active tone with bold text and the pillar, which breathes while the strip has keyboard focus; the others are quiet text. A hovered tab rises onto the surface with a soft pillar. No boxes, no brackets, no underline characters.
- **Slide.** With slide on, a resting label sits one cell left and moves to its place when its tab is hovered or open; the tab keeps that cell at its right, so no tab changes width.
- **A count stays readable.** A badge sits one space after the name, a step quieter than it, and above 99 reads `99+` like a `Badge` count. When a tab is too narrow, its name gives way to `…` first and the count keeps its cells; the count does not slide with the name. A tab hidden in the overflow menu brings its count along.
- **A working tab turns.** A busy tab shows a thin spinner before its name in the accent colour; it turns on the framework's own clock, so you only say when the work starts and stops. With reduced motion it is one dot standing still. A status puts a dot in its colour in the same cell, and a busy tab with a status turns in that colour. The mark sits in the tab's padding and the spare cell its name slides into, so no other tab moves when it comes or goes, and a narrow tab shortens its name and keeps the mark.
- **Keyboard.** While focused, ← and → (or h and l) open the neighbouring tab, and 1–9 open a numbered tab directly.
- **Overflow has arrows.** When the tabs do not fit, an arrow button sits at each end. It is three cells, a space, the chevron and a space, on a raised tone. Under the pointer it brightens and shows the pillar in its first cell; a press flashes it one tone brighter and scrolls the strip by one tab without opening anything. An arrow with nothing more to show sinks to the surface and ignores presses. Opening a tab always brings it into view. On a reorderable strip (see the advanced tabs page) a tab you drag and hold on an arrow scrolls the strip too, one tab at a time.
- **Mouse.** Click a tab to open it, click an arrow to scroll, or turn the wheel over the strip. A strip too narrow for its arrows cuts the open tab short and still scrolls with the wheel.
- **Keys for scrolling.** ctrl+PgUp and ctrl+PgDn scroll like the arrows and flash them. A strip that fits leaves those keys to your application.
- **Only changes send messages.** Opening the tab that is already open does nothing.

## Styling with a theme

```toml
[style."tab:selected"]
bg = "$active"
fg = "$text"
bold = true

[style."tab-index:selected:focus"]
fg = "pulse($accent, $accent-2)"
```

## Common mistakes

- **Tabs as buttons.** A tab shows a view; use a button to do something.
- **Tabs for a sequence.** For steps the user must follow in order, use steps or a wizard.
