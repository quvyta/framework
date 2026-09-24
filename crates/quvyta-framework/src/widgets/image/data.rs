//! Pictures decoded into pixels: from a file, shrunk while it is read, or from raw RGB bytes.

use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::color::Rgb;
use crate::i18n::translate_active;

/// Hands every picture made an identity of its own, so a widget can tell a picture it has already
/// worked out from a new one, even when the new one reuses the old one's memory.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The file formats a picture can be decoded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Png,
    Jpeg,
    Gif,
    WebP,
}

impl Kind {
    fn from_format(format: ::image::ImageFormat) -> Option<Self> {
        match format {
            ::image::ImageFormat::Png => Some(Self::Png),
            ::image::ImageFormat::Jpeg => Some(Self::Jpeg),
            ::image::ImageFormat::Gif => Some(Self::Gif),
            ::image::ImageFormat::WebP => Some(Self::WebP),
            _ => None,
        }
    }

    /// The format's own name, the same in every language.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Gif => "GIF",
            Self::WebP => "WebP",
        }
    }
}

struct Inner {
    id: u64,
    width: u32,
    height: u32,
    /// Row after row, left to right.
    pixels: Box<[Rgb]>,
    /// The size the picture had before it was shrunk to be kept.
    original: (u32, u32),
    name: Option<String>,
    kind: Option<Kind>,
}

/// A decoded picture: its pixels in RGB, shared, so a clone costs a reference count.
///
/// Make one with [`ImageData::decode_file`], which reads PNG, JPEG, GIF (its first frame) and
/// WebP and shrinks a large picture while it is read, or with [`ImageData::from_rgb`] from pixels
/// an application already has. [`Image`](super::Image) draws it.
///
/// Transparency is not kept: a transparent pixel takes the colour stored under it, often black.
#[derive(Clone)]
pub struct ImageData {
    inner: Arc<Inner>,
}

impl fmt::Debug for ImageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageData")
            .field("width", &self.inner.width)
            .field("height", &self.inner.height)
            .field("original", &self.inner.original)
            .field("name", &self.inner.name)
            .finish_non_exhaustive()
    }
}

impl ImageData {
    fn new(width: u32, height: u32, pixels: Box<[Rgb]>, original: (u32, u32)) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Self { inner: Arc::new(Inner { id, width, height, pixels, original, name: None, kind: None }) }
    }

    /// A picture of `width` × `height` pixels from raw bytes: red, green and blue for every
    /// pixel, row after row, left to right.
    ///
    /// Gives `None` when either side is zero or `rgb` does not hold exactly three bytes for every
    /// pixel.
    #[must_use]
    pub fn from_rgb(width: u32, height: u32, rgb: &[u8]) -> Option<Self> {
        let count = usize::try_from(u64::from(width) * u64::from(height)).ok()?;
        if count == 0 || rgb.len() != count.checked_mul(3)? {
            return None;
        }
        let pixels = rgb.chunks_exact(3).map(|c| Rgb::new(c[0], c[1], c[2])).collect();
        Some(Self::new(width, height, pixels, (width, height)))
    }

    /// Reads and decodes the picture at `path`, shrunk to fit within `max` pixels (width, height)
    /// with its shape kept; a picture already that small keeps its size.
    ///
    /// Only the shrunk pixels are kept, so a 4000 × 3000 photo asked for at 300 × 200 keeps
    /// 267 × 200 pixels, about 160 KB. Decoding still passes through the full picture once, so
    /// this is slow for large files: call it from [`Command::perform`](crate::runtime::Command::perform),
    /// never from `view`. For an [`Image`](super::Image) filling an area, ask for the area's width in
    /// cells and twice its height: a cell shows two pixels, one above the other. Where
    /// [`Env::graphics`](crate::env::Env::graphics) is [`Graphics::Kitty`](crate::graphics::Graphics::Kitty)
    /// the terminal shows every pixel it is sent, which is the size kept here and never more, so
    /// ask for about ten times the width and twenty times the height.
    ///
    /// # Errors
    ///
    /// [`ImageError::Missing`] when nothing is at `path`, [`ImageError::Unreadable`] when it cannot
    /// be read (no permission, a folder, too large to decode), [`ImageError::UnknownFormat`] when it
    /// is not a PNG, JPEG, GIF or WebP picture, and [`ImageError::Broken`] when it is one but its
    /// data is damaged or cut short.
    pub fn decode_file(path: &Path, max: (u32, u32)) -> Result<Self, ImageError> {
        let mut file = File::open(path).map_err(|error| ImageError::from_io(&error))?;
        // The format is read from the file's first bytes, never from its name: a text file called
        // `notes.png` is not a picture, not a broken one.
        let mut header = [0u8; 32];
        let mut read = 0;
        while read < header.len() {
            match file.read(&mut header[read..]) {
                Ok(0) => break,
                Ok(count) => read += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(ImageError::from_io(&error)),
            }
        }
        let format = ::image::guess_format(&header[..read]).map_err(|_| ImageError::UnknownFormat)?;
        let kind = Kind::from_format(format).ok_or(ImageError::UnknownFormat)?;
        file.seek(SeekFrom::Start(0)).map_err(|error| ImageError::from_io(&error))?;
        let reader = ::image::ImageReader::with_format(BufReader::new(file), format);
        let mut picture = reader.decode().map_err(|error| ImageError::from_decoding(&error))?;
        let original = (picture.width(), picture.height());
        if original.0 == 0 || original.1 == 0 {
            return Err(ImageError::Broken);
        }
        let (max_width, max_height) = (max.0.max(1), max.1.max(1));
        if original.0 > max_width || original.1 > max_height {
            // Averages every pixel into the one it shrinks to, in the picture's own layout, before
            // anything is converted: the full-size picture is dropped right after.
            picture = picture.thumbnail(max_width, max_height);
        }
        let rgb = picture.into_rgb8();
        let (width, height) = rgb.dimensions();
        let pixels = rgb.pixels().map(|p| Rgb::new(p.0[0], p.0[1], p.0[2])).collect();
        let mut data = Self::new(width, height, pixels, original);
        if let Some(inner) = Arc::get_mut(&mut data.inner) {
            inner.name = path.file_name().map(|name| name.to_string_lossy().into_owned());
            inner.kind = Some(kind);
        }
        Ok(data)
    }

    /// Width in pixels, as kept.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.inner.width
    }

    /// Height in pixels, as kept.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.inner.height
    }

    /// The size, width and height in pixels, the picture had before it was shrunk; the kept size
    /// for one made from raw pixels.
    #[must_use]
    pub fn original_size(&self) -> (u32, u32) {
        self.inner.original
    }

    /// The name of the file it was decoded from, e.g. `"harbour.png"`.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    /// The colour of the pixel at column `x`, row `y`; `None` outside the picture.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<Rgb> {
        if x >= self.inner.width || y >= self.inner.height {
            return None;
        }
        let index = usize::try_from(u64::from(y) * u64::from(self.inner.width) + u64::from(x)).ok()?;
        self.inner.pixels.get(index).copied()
    }

    /// Every pixel, row after row.
    pub(crate) fn pixels(&self) -> &[Rgb] {
        &self.inner.pixels
    }

    /// This picture's identity: clones share it, every picture made has its own.
    pub(crate) fn id(&self) -> u64 {
        self.inner.id
    }

    pub(super) fn kind(&self) -> Option<Kind> {
        self.inner.kind
    }
}

/// Why a picture could not be decoded. Its text, from the language files, reads as a sentence a
/// person can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    /// Nothing is at the path.
    Missing,
    /// Something is there but cannot be read: no permission, a folder, or a picture too large to
    /// decode.
    Unreadable,
    /// The file is not a PNG, JPEG, GIF or WebP picture.
    UnknownFormat,
    /// The file is a picture, but its data is damaged or cut short.
    Broken,
}

impl ImageError {
    fn from_io(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::Missing,
            _ => Self::Unreadable,
        }
    }

    fn from_decoding(error: &::image::ImageError) -> Self {
        use ::image::ImageError as E;
        match error {
            E::Unsupported(_) => Self::UnknownFormat,
            E::Limits(_) => Self::Unreadable,
            // A file that ends early is the usual way a picture is damaged.
            E::Decoding(_) | E::Parameter(_) | E::Encoding(_) | E::IoError(_) => Self::Broken,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Missing => "quvyta.image.missing",
            Self::Unreadable => "quvyta.image.unreadable",
            Self::UnknownFormat => "quvyta.image.unknown-format",
            Self::Broken => "quvyta.image.broken",
        }
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&translate_active(self.key(), &[]))
    }
}

impl std::error::Error for ImageError {}
