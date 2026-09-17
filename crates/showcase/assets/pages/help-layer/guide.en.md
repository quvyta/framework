## When to use

Give every application a help layer. People press `?` when they forget a key, and the layer answers from the same keymap the application runs on, so it is never out of date. Add screen hints for keys that only mean something on the current screen.

## Step by step

1. Keep whether it is open: `help_open: bool`.
2. Open it from the global `help` action (bound to `?`): in `App::action`, map `"help"` to `Msg::Help(true)`.
3. In `view`, add it while open: `ui.add(HelpLayer::new(Msg::Help(false)))`.
4. Add the keys of the current screen that are not keymap actions: `.hint("↑↓", t!("hints.move"))`.
5. Label your own actions in the language files under `[keys]`; framework actions are labelled already.

## How it works

- **Three groups.** "This screen" lists your hints, "Application" your `[app]` actions, "General" the framework's `[global]` actions. Empty groups are hidden.
- **Keys look like keys.** Each chord sits on a raised chip, the label follows in a quieter colour; a binding with several keys shows them side by side.
- **Type to filter.** Every letter narrows the list with fuzzy matching over labels and keys; matched characters take the accent.
- **Long lists scroll.** Arrows, PgUp/PgDn and the wheel scroll; a scrollbar appears only when needed, and it can be pressed and dragged.
- **It is a modal layer.** The screen dims, a pillar runs down the layer's left edge, focus stays inside, application shortcuts pause, and focus returns when it closes.
- **Dismissable means Esc and × together.** Esc and the close mark `×` in the top right corner both send the close message. `.dismissable(false)` turns both off and hides the mark, for a layer the application closes itself, such as a first-run tour that stays for a few seconds.
- **Fresh every time.** Reopening starts unfiltered at the top.

## Common mistakes

- **Writing a separate help text.** It drifts from the real keys; let the keymap fill the layer.
- **Hints for keymap actions.** They are listed already; hints are for keys the keymap does not know.
- **A layer nobody can close.** With `.dismissable(false)`, the application must remove the layer itself; nothing on screen will.
- **Missing labels.** An action without a `keys.<action>` entry shows its key path as `⟦keys.<action>⟧`; the locale tests catch that.
