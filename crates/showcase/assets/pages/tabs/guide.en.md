## When to use

Use tabs to switch between views of the same thing: the sections of a page, the open files of an editor. Each tab is a place, not an action.

## Step by step

1. Give it labels: `Tabs::new([t!("tabs.overview"), t!("tabs.activity")])`.
2. Show the open one from your state: `.active(self.tab)`.
3. Receive switches: `.on_select(Msg::Tab)`.
4. Add `.numbered(true)` when numbers help, like the Demo, Code, Guide, Reference tabs of this showcase.

## How it works

- **The open tab is a surface.** It sits on the active tone with bold text and the pillar, which breathes while the strip has keyboard focus; the others are quiet text. A hovered tab rises onto the surface with a soft pillar. No boxes, no brackets, no underline characters.
- **Slide.** With slide on, a resting label sits one cell left and moves to its place when its tab is hovered or open; the tab keeps that cell at its right, so no tab changes width.
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
