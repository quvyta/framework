## When to use

Use shimmer text for a message that stands for ongoing work, where the words matter more than a symbol: a search running, a request being processed, someone typing. It is quieter than a spinner and reads as part of the conversation.

## Step by step

1. Add the message: `ui.add(ShimmerText::new(t!("processing")))`. Light sweeps across it.
2. For a typing indicator choose the dots style: `.style(ShimmerStyle::Dots)`.
3. Pair it when it helps: a `Spinner` before it and a faint elapsed time after it make a complete working line.
4. Replace it with the result as soon as the work is done.

## How it works

- **Per-cell light.** Each letter gets its own colour, blended from the resting colour towards the highlight by its distance from a moving band of light that fades out over five cells on either side of its centre. Nothing but colour changes; the text never moves.
- **One pass per `motion.shimmer`.** The band eases in and out so it slows near the edges.
- **Dots count up** from none to three over the same period, then start again. Room for the three dots is reserved, so the layout never shifts.
- **Reduced motion.** The sweep rests in its plain colour; dots show all three.

## Common mistakes

- **Shimmering real content.** Only use it for the working message, never for text people need to read closely.
- **Long sentences.** The light takes the same time for any length, so long text sweeps fast. Keep it to a few words.
