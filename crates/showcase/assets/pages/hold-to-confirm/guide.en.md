## When to use

Use hold to confirm for an action that must not happen by accident but is done often enough that a dialog would annoy: quitting with running work, deleting a volume, purging a cache. The hold is the confirmation, so nothing opens and nothing needs answering. For rare, heavy decisions that deserve an explanation, ask with `Command::confirm` instead.

## Step by step

1. Add the control where the action lives: `HoldToConfirm::new(t!("delete-volume"))`.
2. Give it the message: `.on_confirm(Msg::DeleteVolume)`. It arrives once, when the bars are full.
3. To make a chord work from anywhere, add `.key("ctrl+q")`.
4. If the control should only appear while it is held, add `.floating(true)`; it then shows a small card in the top left corner.
5. Change the time only with reason: `.duration(Duration::from_millis(2000))`.
6. To give the bars a colour that fits the action, name a theme colour: `.color("$danger")` for deleting, `.color("$success")` for publishing. Without it they take the theme's warning tone.

## How it works

- **Three bars fill in turn, each as a whole.** The hold is split into three equal parts. During the first part the whole first bar blends from the track colour to the theme's target colour (the warning tone in the built-in themes); then the second bar, then the third. When the third bar reaches the full colour the action happens. The total time is the duration you set.
- **Colours come from the theme.** `track` is where a bar starts and `to` where it ends. `.color(...)` replaces `to` with an expression written exactly as in a theme file: a token (`"$danger"`, `"$success"`, `"$accent"`), a blend (`"mix($accent, $danger, 50%)"`) or a fixed `"#RRGGBB"`. The goal is a theme colour, because it changes with the theme; a fixed colour is possible but stays the same in every theme.
- **A bad colour never breaks the control.** An expression the theme cannot turn into one colour (a typo, an unknown token, `pulse()`) is ignored and the bars fill towards the theme's `to`. `Theme::solid(expression)` tells you why it was rejected.
- **Letting go empties the bars.** Released before the end, the bars drain back quickly, over `motion.enter`, the last filled bar first.
- **Hover and focus raise the pillar** in the control's first cell, like a button.
- **It sends once.** After completing, nothing else happens until the key is let go and pressed again.
- **How a held key is noticed.** Terminals with the kitty keyboard protocol report repeats and the release; other terminals repeat the press every few dozen milliseconds after the keyboard's repeat delay. The control counts a key as released on a release event, or when no repeat comes within 650 ms of the press or 350 ms of the last repeat.
- **The mouse works without events.** A held button sends nothing while still, so the runtime delivers a quiet repeat to the control; moving off the control cancels.
- **Reduced motion** switches a bar to the full colour at once when its third of the hold is over, and letting go empties the bars at once. The duration itself never shortens.

## Common mistakes

- **Very long durations.** Over two seconds feels broken; 1.2 s is enough to stop accidents.
- **Hiding the only way to an action behind a floating chord.** People cannot discover a key they cannot see; show the chord in the key hints or next to the action.
- **Using it for choices.** A hold says "yes, really"; it cannot say which.
- **Hard-coding a hex colour.** `"#E5484D"` looks right in one theme and wrong in the others; `"$danger"` follows every theme.
