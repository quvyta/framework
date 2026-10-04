## When to use

Use a seek bar when a person is watching or listening to something and wants to go somewhere else in it: the transport row under a video, the play head of an audio file. It is a progress bar you can press and drag. When the work cannot be moved — an upload, a copy, a test run — use a progress bar; when the value is a number rather than a position in time, use a slider.

## Step by step

1. Keep the position in your application: `position: f32` from 0 to 1.
2. Draw it: `ui.add(SeekBar::new(state.position)).width(Length::Fill(1))`.
3. Say what a seek means: `.on_seek(|fraction| Msg::Seek(fraction))`, and store the fraction in `update`.
4. Name the position under the pointer when the bare fraction says nothing: `.hover_label(|fraction| clock(fraction))`, where `clock` writes `m:ss` of the track's length.
5. Give it a tone that tells the story: `.variant("success")` once the end is reached.

## How it works

- **The same picture as a progress bar.** A seek bar is drawn by the progress bar's own painting, so putting one where the other stood changes nothing on screen; only what the pointer can do changes. The percentage after the bar is written by both, so it takes the same `progress-label` style.
- **A press seeks the cell it landed on**, to that cell's centre rather than to a seam: on a twenty-cell bar, the fourth cell is `4.5 / 20`.
- **A drag seeks cell by cell** for as long as the button is held, and the release ends it. The pointer may leave the bar, and leave the row it is on: the fraction clamps to 0 or 1, so a drag past either end holds there instead of running away.
- **The keyboard reaches it** when it has a message: ← and → move a twentieth of the track, Home and End go to its ends. At an end, where there is nowhere to move, the bar says nothing.
- **Under the pointer** the bar steps a tone lighter, and the cell the pointer is on takes the accent, so where a seek would land is the one cell that stands out.
- **Without `on_seek`** the bar is a picture only: no focus, no hover, no pointer, exactly as a progress bar is.
- **The label** is drawn with the library's tooltip, anchored on the pointer's own cell, and appears at once rather than after the hover delay: it names where the pointer is rather than explaining the widget.

## Common mistakes

- **Rounding the position to the second.** The bar sends the fraction, not a time; a player that rounds to whole seconds before drawing cannot scrub smoothly.
- **Sending a message the application then ignores.** The bar does not move itself; it says where a person asked to go. Move it in `update`, or the fill stays where it was while the pointer carries on.
- **A label for a bare fraction.** "0.42" names nothing a person recognises; write the time of the track.
- **Too narrow a bar.** Each cell is a step; under about ten cells a drag jumps.