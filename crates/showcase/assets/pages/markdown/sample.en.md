## Release notes

Quvyta **0.2** brings the showcase you are looking at. Every component has a live demo, its own code, a guide and a reference.

### What changed

- Themes check readability when they load
- Lists slide the selected label one cell to the right
- `Command::perform` runs slow work on a background thread
  and keeps the screen responsive

1. Update the framework
2. Run the showcase
3. Press `f12` to see how a frame is built

> Surfaces, not frames: grouping comes from tone and space.

```toml
[style."list-item:selected:focus"]
pillar = "pulse($accent, $accent-2)"
```
