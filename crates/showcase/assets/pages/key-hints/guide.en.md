## When to use

Put a key hint bar at the bottom of a screen so people learn the keys without opening help. Show the handful that matter on this screen, not every binding.

## Step by step

1. Add hints for keys the screen handles itself: `.hint("↑↓", t!("hints.move"))`.
2. Add keymap actions by name: `.action(Scope::App, "search")`. The keys come from the keymap and the label from the language, so rebinding or translating needs no code.
3. Pin the few global actions on the right: `.action_right(Scope::Global, "quit")`.

## How it works

- **Keys are small surfaces, labels are faint.** Nothing is bracketed; the key's raised tone is its shape.
- **The bar adapts.** When it gets narrow, hints on the left are dropped from the end first; the right group always stays.
- **It follows the user.** Change a binding in the keymap file or switch the language and the bar updates.

## Common mistakes

- **Listing everything.** Five or six hints are read; fifteen are not.
- **Hints that lie.** Use `.action` for keymap actions instead of writing the key by hand, so the hint changes when the binding does.
