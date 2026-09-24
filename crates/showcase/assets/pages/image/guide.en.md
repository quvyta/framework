## When to use

Use an image for a picture the person chose or needs to see: a desktop wallpaper, the preview of a file, an avatar, a chart rendered elsewhere. It draws in every terminal with 256 colours or more, over SSH too, because it needs nothing but coloured cells; a terminal that speaks the kitty graphics protocol draws it itself, in real pixels.

## Step by step

1. Turn on the `image` feature of `quvyta-framework` in `Cargo.toml`: `features = ["image"]`.
2. Decode the file in the background, since decoding a large photo takes a moment: `Command::perform(move || ImageData::decode_file(&path, (width, height * 2)), Msg::Decoded)`. Ask for the area's width in cells and twice its height, so only as many pixels as the screen can show are kept. When `env.graphics()` is `Graphics::Kitty` the terminal shows every pixel it is sent, so ask for more, about ten times the width and twenty times the height; the picture is sent at the size kept and never enlarged.
3. While it decodes, show a `Spinner`; when it fails, show an `EmptyState` with the error, which reads as a sentence: `error.to_string()`.
4. Draw it: `ui.add(Image::new(&data).fit(Fit::Cover)).fill()`.
5. For pixels you already have, use `ImageData::from_rgb(width, height, &bytes)`.

## How it works

- **Two pixels a cell.** Every cell is drawn as `▀`: the upper pixel is the colour of the glyph, the lower one the background behind it. A cell is about twice as tall as it is wide, so the two halves are close to square and the picture keeps its shape.
- **Three fits.** `Contain` shows the whole picture as large as fits, leaving the ground beside it. `Cover` fills the whole area and cuts what spills over evenly from both sides; it is the one for wallpapers. `Center` shows the picture at its own size, one pixel per half cell, and cuts what does not fit.
- **Averaged, not picked.** Shrinking takes the average of every pixel a new pixel covers, so a photo keeps its tones instead of turning into noise.
- **Worked out once.** The cells are kept in the widget's memory and worked out again only when the area's size, the fit or the picture changes. A frame that draws the same picture again only copies them.
- **A ground to draw on.** Cells the picture does not reach keep what is under them, and anything painted after the picture lands on it, so windows and icons can sit on a wallpaper.
- **Real pixels where the terminal can.** When `Env::graphics` says `Kitty`, the widget paints no half blocks: its cells get the plain ground and the terminal draws the picture over them. The pixels cross the wire once and are kept under a number; a later frame only says where to show them, and a frame where nothing moved writes nothing at all. A picture no longer shown is freed from the terminal's memory.
- **Menus stay on top.** The picture sits under text and over the cells' ground. Whatever is painted over it wins: a menu or panel along one side cuts the picture to the part left showing, and something in its middle (a window, a dialog) or the dimmed backdrop of a dialog draws it with half blocks until it moves away.
- **Every terminal.** With 256 colours both halves of every cell take their nearest palette entry. With only the sixteen standard colours, or with ASCII glyphs, pixels are never drawn with characters: the picture's name, format and size are shown with a sentence saying that this terminal cannot show pictures.

## Common mistakes

- **Decoding in `view`.** A large photo takes a noticeable time to decode; do it once, in `Command::perform`, and keep the `ImageData` in your state.
- **Asking for the full picture.** A 4000 by 3000 photo kept whole is tens of megabytes. Pass the size the screen can show as `max`; only that is kept.
- **A new `ImageData` every frame.** Every decode or `from_rgb` is a new picture, and a new picture is worked out again. Clone the one in your state; a clone is the same picture and costs a reference count.
