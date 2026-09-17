## When to use

Use a page transition wherever one screen replaces another: the pages of a router, the steps of a flow, the panes of a settings screen. It tells the user that the place changed and, with slide, which way they went. Do not use it for content that updates in place, such as a list refreshing.

Layers (a dialog popping, a toast sliding in, a dropdown unfolding) animate inside their own widgets; this is the page-level mechanism.

## Step by step

1. Keep a `Router` in your state and navigate with `push` and `back`.
2. In `view`, wrap the page: `ui.add_with(PageTransition::new(page.clone()), |ui| ui.page(page, build)).fill()`.
3. Add `.slide(true).direction(router.direction())` for a short slide that follows the navigation.
4. Leave the duration to the theme: `[motion] page = "260ms"`.

## How it works

- **The key decides.** When the key given to `PageTransition::new` differs from the last frame, a transition starts; anything else that changes on the same page just redraws.
- **Cell by cell.** The outgoing screen is remembered. In the first half the old text fades into the blending background, in the second half the new text rises out of it. Backgrounds blend the whole time, so surfaces never jump.
- **Slide in whole cells.** With slide the incoming page starts up to six cells away and steps into place one cell at a time; its colours blend with the same progress, so each step is also a step in colour.
- **Direction.** `Router::direction()` is `Forward` after `push` or `replace` and `Back` after `back`. Forward pages come from the right, back pages from the left.
- **Never in the way.** The new page is live from the first frame. Reduced motion, a resized area or a first paint show the new page at once.

## Common mistakes

- **Using a key that changes every frame**, such as a counter or a time: the page transitions forever.
- **Wrapping each section instead of the page.** Put the transition around the content that is replaced as a whole.
- **Long durations.** Pages change often; keep `motion.page` short so navigation stays quick.
