## Feature

- `image` on `quvyta-framework`, off by default. It brings the decoders for PNG, JPEG, GIF and WebP, and zlib to compress pictures sent to a kitty terminal, and nothing else.

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
- `Env::graphics()` is `Graphics::Kitty`: no half blocks. The picture's cells get the theme's `canvas` ground and a space, and the terminal draws the picture over them at `z=-1`, under text and over the cells' ground. The pixels are sent once as RGB (`a=t,f=24,o=z`, base64 in chunks of 4096 bytes) under a number of their own; every frame after only places them (`a=p` with the source rectangle and the cells), deletes a place no longer used (`a=d,d=i`) and frees a picture shown nowhere (`a=d,d=I`). A frame whose places did not change writes nothing. Every command carries `q=2`, so the terminal never answers. After a program had the terminal, everything is sent and placed again.
- What is painted over a kitty picture hides it: when the cells left showing form one rectangle (a menu along one side), the picture is cut to it; otherwise (something in the middle or a corner, a dialog's dimmed backdrop) that frame draws it with half blocks, dimmed like the backdrop. `Graphics::Sixel` draws half blocks for now.
- Marks its cells as decoration: a clean copy of a text selection leaves them out.
- `ColorDepth::Ansi256`: both halves take their nearest palette entry. `ColorDepth::Ansi16` and `GlyphMode::Ascii`: draws an empty state with the file name (or "Picture"), the format, the original size and "This terminal cannot show pictures."
- Not focusable; sends no messages.

## Theme keys

- `empty-state-icon`, `empty-state-title`, `empty-state-message` — through the empty state shown where pictures cannot be drawn.
