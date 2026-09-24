## Feature

- `image` on `quvyta-framework`, off by default. It brings the decoders for PNG, JPEG, GIF and WebP, and zlib to compress pictures sent to a kitty terminal, and nothing else; the sixel encoder is the framework's own.

## ImageData

- `ImageData::decode_file(path, max)` — reads and decodes the picture at `path`, shrunk to fit within `max` (width, height in pixels) with its shape kept. The format is read from the file's first bytes. GIF gives its first frame. Blocks: call it from `Command::perform`.
- `ImageData::decode_bytes(&bytes, max)` — decodes a picture held in memory (`include_bytes!`), shrunk the way `decode_file` shrinks it. It has no name. Errors: `UnknownFormat`, `Broken`, `Unreadable`.
- `ImageData::EXTENSIONS` — `["png", "jpg", "jpeg", "gif", "webp"]`: the extensions of the formats the decoder reads, lower case, for `FileBrowser::extensions`. A test holds it to the formats compiled in.
- `ImageData::reads(path)` — whether `path`'s extension is one of them, in any case. The decoder itself goes by the first bytes, never the name.
- `ImageData::from_rgb(width, height, &bytes)` — a picture from three bytes (red, green, blue) per pixel, row after row; `None` when a side is zero or the length is wrong.
- `.width()`, `.height()` — the size kept, in pixels.
- `.original_size()` — the size before shrinking.
- `.name()` — the file name, for a picture decoded from a file; `None` otherwise.
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

## Graphics

- `Graphics::can_draw()` — whether a picture is drawn at all: true for `Kitty`, `Sixel` and `HalfBlock`, false for `None`. It is the image's own test; ask it before showing an image to skip its "cannot show" state.
- `App::graphics(graphics)` — hears the graphics before the first frame (after `App::resized`, before `App::init`) and whenever they change, e.g. the glyph mode switched to ASCII. Decode there at the size the terminal shows.

## Behaviour

- Measures the cells its fit needs within the room given: all of it for `Cover`, the picture's shape for `Contain`, its own size for `Center`, where a cell is one pixel wide and two tall.
- Every cell is `▀` with the upper pixel as its colour and the lower one as its background; a half cell at the picture's edge is `▀` or `▄` over what is under it. Cells the picture does not reach are left alone.
- Resamples with a box filter. The cells are kept in the widget's memory and recomputed only when the area's size, the fit or the picture changes.
- `Env::graphics()` is `Graphics::Kitty`: no half blocks. The picture's cells get the theme's `canvas` ground and a space, and the terminal draws the picture over them at `z=-1`, under text and over the cells' ground. The pixels are sent once as RGB (`a=t,f=24,o=z`, base64 in chunks of 4096 bytes) under a number of their own; every frame after only places them (`a=p` with the source rectangle and the cells), deletes a place no longer used (`a=d,d=i`) and frees a picture shown nowhere (`a=d,d=I`). A frame whose places did not change writes nothing. Every command carries `q=2`, so the terminal never answers. After a program had the terminal, everything is sent and placed again.
- What is painted over a kitty picture hides it. The cells left showing are split into rectangles, each row's runs merged downward while they line up, and the picture is placed once in each (`p=1`, `p=2`, …) with its source cut to match; neighbouring crops meet at the same pixel. Cells under a blend recorded by `PaintCx::tint` (a dialog's backdrop, a window's shadow) are drawn with half blocks in the same frame, dimmed by that blend; a cell counts as dimmed only when the recorded blends turn the mark and the ground into exactly its colours, anything else painted there covers it. More than 64 rectangles draw the whole picture with half blocks for that frame.
- `Env::graphics()` is `Graphics::Sixel`: the cells are marked as for kitty. On a local terminal the picture is split as for kitty: the cells that still hold its mark, at most 64 rectangles, dimmed cells in half blocks, and the whole picture in half blocks for a frame that needs more. Over a remote connection (`Env::remote()`) it is shown only if every cell it may paint (within its clip) still holds its mark; anything over any of it, a blend included, draws the whole picture with half blocks for that frame. Shown, a part is shrunk with the box filter from its exact crop to its cells' pixels: a cell is the size the terminal reports for its window divided by its cells, or 10 × 20 where it reports none. The pixels are worked out at the cells' full height and cut down to whole bands of six, so a part never reaches below its last row and parts meet without stretching; the cells of a last row the bands leave short are written, before the pixels, with the average colour of the rows left out as their ground. Colours go to a fixed palette of 6 × 7 × 6 levels (red, green, blue; 252 entries, only those used are defined). Pixels are written after the cells, inside the frame's synchronized update: the cursor moves to a part's first cell, then `ESC P 0;1;0 q "1;1;w;h`, the palette, six-pixel bands with `!` repeats, `ST`. The screen keeps, cell by cell, whose pixels it shows; a cell loses them when it is written, and after a new size or a program had the terminal every cell has none. A frame sends pixels only for the cells of its places that do not show them, cut into rectangles the same way (up to 64 a picture, or the places holding them whole beyond that), so a frame whose places and cells are what the terminal shows writes nothing. A cell that shows pixels no place wants any more and that the frame does not change is written again, so the pixels do not stay behind. Encodings are kept per picture, crop and size while their picture is painted.
- Marks its cells as decoration: a clean copy of a text selection leaves them out.
- `ColorDepth::Ansi256`: both halves take their nearest palette entry. `Graphics::None` (`ColorDepth::Ansi16`, `GlyphMode::Ascii`, or `QUVYTA_GRAPHICS=none`): draws an empty state with the file name (or "Picture"), the format, the original size and "This terminal cannot show pictures."
- Not focusable; sends no messages.

## Theme keys

- `empty-state-icon`, `empty-state-title`, `empty-state-message` — through the empty state shown where pictures cannot be drawn.
