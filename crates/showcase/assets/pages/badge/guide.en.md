## When to use

Use a badge for a short state that belongs to something else: a container that is running, a deploy that failed, a branch that is pinned. It also counts things waiting for attention, like alerts or updates. For a sentence or an action, use text or a button instead.

## Step by step

1. Add a neutral badge: `ui.add(Badge::new("Paused"))`.
2. Give it a tone that tells what the state means: `.variant("success")`, `"warning"`, `"danger"`, `"info"` or `"accent"`.
3. Put it next to what it describes, usually at the end of a row: the name first, the state after it.
4. Add a count when a number matters: `Badge::new("Alerts").variant("danger").count(3)`.
5. Keep the label to one or two words; in narrow places give the badge a width and it cuts the label with `…`.

## How it works

- **Shape from colour.** The pill is a tinted surface with one cell of padding on each side. About 16% of the status colour is blended over the surface, so the tone is clear but the pill stays quiet.
- **Never colour alone.** Every badge carries a `●` marker before its word (`*` in ASCII mode), so the state reads without colour too.
- **Counts.** The count sits in a stronger segment right after the label; anything above 99 reads `99+` so the pill does not grow.
- **Neutral.** Without a variant, the badge sits on the raised surface with dim text, for states that are not good or bad news.

## Common mistakes

- **A rainbow.** Tones mean something. A list where every badge has a different colour tells nothing; most states should be neutral.
- **Sentences in a pill.** "Waiting for the health check to pass" belongs in text; the badge says "Starting".
- **Accent for status.** The accent tone is for your own emphasis, such as pinned or new, not for success or failure.
