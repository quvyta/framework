## When to use

Use a breadcrumb to show where the user is in a hierarchy and to go back up: folders in a file browser, namespaces in a cluster, nested settings. It is not a replacement for back navigation or for tabs.

## Step by step

1. Keep the path in your state, from the root to the current place.
2. Show it: `Breadcrumb::new(self.path.clone())`.
3. Go up when a level is opened: `.on_select(Msg::Up)`, and in `update` `self.path.truncate(index + 1)`.
4. Give it the width it may use, for example `.fill_width()` in a header row.

## How it works

- **Text, not buttons.** Levels are bare dim text that rises on hover. The last level is the current place: bold and not clickable.
- **The separator is an icon.** A faint small chevron from the icon set sits between levels, never `/` or `>`. Themes can change it through `crumb-separator`.
- **Narrow paths collapse the middle.** When the path does not fit, it keeps the root and as many of the last levels as fit, with `…` in between. Opening `…` lists the hidden levels. In very little room it shows `…` and the current place. In ASCII mode the fold is drawn as `~`.
- **A faint path.** `.faint(true)` draws every level a step quieter, the current place too, for a place shown but not open to the person, such as a folder that cannot be read. Only the tone changes: the levels still rise on hover and still open, so the way back out stays where it always is.
- **Keyboard.** Focus the breadcrumb, move with ← and →, jump with Home and End, and open with Enter or Space.

## Common mistakes

- **Putting the path in the labels.** Each segment is one level name, not the whole path.
- **Showing a one-level path.** A single segment has nothing to go back to; the breadcrumb is then not focusable.
- **Using it as a menu of siblings.** It only goes up; show siblings in a list or menu.
