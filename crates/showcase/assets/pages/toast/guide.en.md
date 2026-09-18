## When to use

Use a toast to tell the user that something happened in the background and needs no answer: a deploy finished, a disk is filling up, a build failed. It does not block the screen and goes away on its own. Use a dialog for decisions, and show errors about a field next to that field.

## Step by step

1. Return a toast from `update`: `Command::toast(Toast::success(t!("deployed")))`.
2. Add detail when it helps: `.body(t!("deployed-body"))`.
3. Offer one follow-up action: `.action(t!("retry"), Msg::Retry)`; the message arrives like any other.
4. For something that progresses, give it a key: `.key("upload")`. Showing it again replaces it in place; `Command::dismiss_toast("upload")` removes it.
5. While the work runs, let the icon move: `.icon_motion(SpinnerStyle::Pulse)`. When it is done, show a toast of the result under the same key, without the animation: `Toast::success(t!("uploaded")).key("upload")`.
6. When the news has a place to go, make the toast pressable: `.on_press(Msg::OpenDeployLog)`. A press on the toast sends the message and the toast stays; the playground's "Pressable deploy toast" turns this on for the deploy toast.
7. Pick the corner once, for example at start: `Command::toast_corner(Corner::TopRight)`.

## How it works

- **The runtime owns the stack.** Toasts are not part of your view: they outlive the screen that raised them and are drawn above the screen and its menus. Your application keeps no toast state.
- **A toast never covers a dialog.** While a dialog, the command palette or another modal layer is open, the stack keeps to the rows between its corner and the dialog, with one free row between them. A toast that finds no room there waits, its time stopped, and slides in when the dialog closes or the screen grows; a toast already shown when a dialog opens over it steps aside the same way. "Save under a dialog" above opens a dialog and reports a save at the same moment; make the terminal short to see the toast wait.
- **Status is a marker, never colour alone.** A column in the status colour and an icon (success, warning, danger, info) sit at the left; the title and body stay in neutral text.
- **They slide in** from the screen edge over `motion.enter`, cell by cell, with their colours arriving at the same pace, and slide out the same way. With reduced motion they appear and vanish at once.
- **They stack in a corner**, newest nearest the corner, with one row of space between them. Toasts that do not fit wait until others leave. A toast's time runs only while it is on screen, so a waiting toast is never missed.
- **The icon can move.** Any one-cell animation of the Spinner (the styles on the Spinner and Animation studio pages, Pulse among them) can play in the icon cell, in the kind's colour, and arrives with the toast like the rest of its colours. The title keeps its column, so a keyed toast that settles from the animation into its icon does not jump. With reduced motion the kind's icon stands still. The playground's "Icon animation" picks the style of the upload toast.
- **They leave on their own** after 5 seconds, or 8 seconds when they carry an action, or the `.duration(…)` you set. Resting the pointer on a toast pauses its countdown.
- **The close mark is the one tabs and dialogs use.** Three cells at the end of the title row, a whisper at rest. Pointing at the toast leaves it alone; only the pointer on the mark lights its three cells together. It is always there, because every toast can be dismissed.
- **Only the close mark dismisses.** A click on the mark closes the toast; a click on the action sends its message and closes it. A click anywhere else on a plain toast does nothing, so a toast never vanishes under a stray click.
- **A pressable toast goes somewhere.** With `.on_press(msg)` a click on the toast (not on its action or mark) sends `msg`, for example to open the log or the page the news came from, and the toast stays until its mark or its timer closes it. While the pointer is on it the toast rises one step (`$overlay` to `$active`, as a pressable panel does) and its action climbs with it. A plain toast shows no hover, because it has nothing to press.
- **Clicks stay on the toast.** No click on a toast reaches what is under it.

## Common mistakes

- **Using a toast for something the user must act on.** It disappears; use a dialog.
- **A toast per progress step.** Give progress one key so it updates in place instead of stacking.
- **Long text.** Keep the title short; the body wraps, but a toast is not a log.
- **An animation that never stops.** A moving icon says "still running"; replace the toast when the work ends, or it pulses until it leaves.
