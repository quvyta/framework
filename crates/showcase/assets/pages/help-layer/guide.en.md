## When to use

Give every application a help layer. People press `?` when they forget a key, and the layer answers from the same keymap the application runs on, so it is never out of date. Keys that only mean something on the current screen are listed too: the focused widget says which of them it takes, and the application adds hints for the rest.

## Step by step

1. Keep whether it is open: `help_open: bool`.
2. Open it from the global `help` action (bound to `?`): in `App::action`, map `"help"` to `Msg::Help(true)`.
3. In `view`, add it while open: `ui.add(HelpLayer::new(Msg::Help(false)))`.
4. Leave the keys of your widgets alone: a table, list, tree or field answers with the keys it takes, so nobody writes a hint for them.
5. Add hints for the keys of the screen that no widget takes, such as the `r` of an application that restarts the selected row: `.hint("r", t!("hints.restart"))`.
6. Label your own actions in the language files under `[keys]`; framework actions are labelled already.

## How it works

- **Three groups.** "This screen" lists the keys of the focused widget first and your hints after them, "Application" your `[app]` actions, "General" the framework's `[global]` actions. Empty groups are hidden.
- **The focused widget fills itself in.** Whatever has the focus is asked which keys it takes while it paints, and the layer puts those at the top of "This screen". A widget of your own answers with `Widget::keys`.
- **The widget the keyboard came from is the one asked.** A layer takes the keyboard with it, so the keys listed are those of the widget that had the focus when the layer opened, not those of the layer's own filter. Open the help with a key while the table has the focus; a click on a button that opens it leaves the focus on that button, which declares nothing.
- **Only the keys that are really there.** A table without a row to open says nothing about Enter, a list without check marks says nothing about Space, and a tree that cannot expand says nothing about the side arrows. Turning a capability on adds its key.
- **Keys look like keys.** Each chord sits on a raised chip, the label follows in a quieter colour; a binding with several keys shows them side by side.
- **Type to filter.** Every letter narrows the list with fuzzy matching over labels and keys; matched characters take the accent.
- **Long lists scroll.** Arrows, PgUp/PgDn and the wheel scroll; a scrollbar appears only when needed, and it can be pressed and dragged.
- **It is a modal layer.** The screen dims, a pillar runs down the layer's left edge, focus stays inside, application shortcuts pause, and focus returns when it closes.
- **Dismissable means Esc and × together.** Esc and the close mark `×` in the top right corner both send the close message. `.dismissable(false)` turns both off and hides the mark, for a layer the application closes itself, such as a first-run tour that stays for a few seconds.
- **Fresh every time.** Reopening starts unfiltered at the top.

## Common mistakes

- **Writing a separate help text.** It drifts from the real keys; let the keymap and the widgets fill the layer.
- **Hints for a widget's keys.** They are listed already, and a hint for a key a widget no longer takes is a lie the layer cannot see.
- **A layer nobody can close.** With `.dismissable(false)`, the application must remove the layer itself; nothing on screen will.
- **Missing labels.** An action without a `keys.<action>` entry shows its key path as `⟦keys.<action>⟧`; the locale tests catch that.
