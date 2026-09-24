## Feature

- `image` on `quvyta-framework`, off by default. It brings the decoders for PNG, JPEG, GIF and WebP and nothing else.

## ImageData

- `ImageData::decode_file(path, max)` — reads and decodes the picture at `path`, shrunk to fit within `max` (width, height in pixels) with its shape kept. The format is read from the file's first bytes. GIF gives its first frame. Blocks: call it from `Command::perform`.
- `ImageData::from_rgb(width, height, &bytes)` — a picture from three bytes (red, green, blue) per pixel, row after row; `None` when a side is zero or the length is wrong.
- `.width()`, `.height()` — the size kept, in pixels.
- `.original_size()` — the size before shrinking.
- `.name()` — the file name, for a decoded picture.
- `.pixel(x, y)` — one pixel's colour, `None` outside the picture.
- Cloning is cheap: the pixels are shared. Transparency is not kept.

## ImageError

- `Missing` — nothing is at the path.
- `Unreadable` — it cannot be read: no permission, a folder, or too large to decode.
- `UnknownFormat` — not a PNG, JPEG, GIF or WebP picture.
- `Broken` — a picture whose data is damaged or cut short.
- Its `Display` is a sentence from the language files (`quvyta.image.*`).

## Image

- `Image::new(&data)` — draws `data`, keeping a clone.
- `.fit(Fit)` — `Fit::Contain` (the default), `Fit::Cover` or `Fit::Center`.

## Behaviour

- Measures the cells its fit needs within the room given: all of it for `Cover`, the picture's shape for `Contain`, its own size for `Center`, where a cell is one pixel wide and two tall.
- Every cell is `▀` with the upper pixel as its colour and the lower one as its background; a half cell at the picture's edge is `▀` or `▄` over what is under it. Cells the picture does not reach are left alone.
- Resamples with a box filter. The cells are kept in the widget's memory and recomputed only when the area's size, the fit or the picture changes.
- Marks its cells as decoration: a clean copy of a text selection leaves them out.
- `ColorDepth::Ansi256`: both halves take their nearest palette entry. `ColorDepth::Ansi16` and `GlyphMode::Ascii`: draws an empty state with the file name (or "Picture"), the format, the original size and "This terminal cannot show pictures."
- Not focusable; sends no messages.

## Theme keys

- `empty-state-icon`, `empty-state-title`, `empty-state-message` — through the empty state shown where pictures cannot be drawn.
