//! Telling a terminal that speaks the kitty graphics protocol which pictures to show, and where.
//!
//! A picture's pixels cross the wire once: the terminal keeps them under a number and every later
//! frame only says where to show them. What a frame writes is the difference from the one before:
//! a new picture is sent, a picture that moved or was cut differently is placed again, a place no
//! longer used is deleted, and a picture no longer shown anywhere is freed so the terminal's
//! memory does not grow. A frame with the same places as the one before writes nothing at all.
//!
//! Every command carries `q=2`, so the terminal answers none of them: an answer would arrive as
//! input and read as keys.

use std::io::Write;

use crate::widgets::ImageData;
use crate::widgets::image::PicturePlacement;

/// The most base64 bytes one command carries; the protocol's limit.
const CHUNK: usize = 4096;

/// The kitty pictures a screen has shown, and what the terminal holds of them.
#[derive(Debug, Default)]
pub(crate) struct KittyPictures {
    /// The numbers of the pictures whose pixels the terminal holds.
    sent: Vec<u32>,
    /// The places shown now.
    shown: Vec<PicturePlacement>,
    /// Whether every place must be written again, as after the terminal was cleared.
    place_all: bool,
    /// Whether every picture must be sent again, since the terminal may have lost them.
    send_all: bool,
}

impl KittyPictures {
    /// The commands that take the terminal from the places shown to `placements`, and notes them
    /// as shown. Empty when they are the same. The pictures in `painted` (their identities) are
    /// still on screen, drawn another way this frame, such as with half blocks while a window
    /// covers their middle: their pixels stay in the terminal, only their places are deleted,
    /// so they need not be sent again when they are placed again.
    pub(crate) fn commands(&mut self, placements: &[PicturePlacement], painted: &[u64]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut kept: Vec<u32> = placements.iter().map(|placement| number(&placement.data)).collect();
        kept.extend(painted.iter().map(|id| number_of(*id)));
        // Freeing a picture also deletes its places, so only the places of pictures kept are
        // deleted one by one.
        self.sent.retain(|sent| {
            let keep = kept.contains(sent);
            if !keep {
                let _ = write!(out, "\x1b_Ga=d,d=I,i={sent},q=2\x1b\\");
            }
            keep
        });
        for old in &self.shown {
            let image = number(&old.data);
            let still = placements.iter().any(|new| number(&new.data) == image && new.number == old.number);
            if kept.contains(&image) && !still {
                let _ = write!(out, "\x1b_Ga=d,d=i,i={image},p={},q=2\x1b\\", old.number);
            }
        }
        if self.send_all {
            self.sent.clear();
        }
        let mut sent_now = Vec::new();
        for placement in placements {
            let image = number(&placement.data);
            if !self.sent.contains(&image) {
                transmit(&mut out, &placement.data, image);
                self.sent.push(image);
                sent_now.push(image);
            }
        }
        for placement in placements {
            // Sending a picture again replaces it along with its places.
            let again = self.place_all || sent_now.contains(&number(&placement.data));
            if again || !self.shown.contains(placement) {
                place(&mut out, placement);
            }
        }
        self.shown = placements.to_vec();
        self.place_all = false;
        self.send_all = false;
        out
    }

    /// Frees every picture the terminal holds, for before the terminal is handed to a program
    /// or left. The next frame sends what it shows again.
    pub(crate) fn release(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        for sent in self.sent.drain(..) {
            let _ = write!(out, "\x1b_Ga=d,d=I,i={sent},q=2\x1b\\");
        }
        self.shown.clear();
        out
    }

    /// Forgets what the terminal holds, after a program wrote over it: the next frame sends every
    /// picture it shows and places it again.
    pub(crate) fn forget(&mut self) {
        self.place_all = true;
        self.send_all = true;
    }

    /// Notes that the terminal was cleared for a new size: the next frame places every picture
    /// again.
    pub(crate) fn resized(&mut self) {
        self.place_all = true;
    }
}

/// The number the terminal keeps `data` under: its identity, within the protocol's range, never 0.
pub(super) fn number(data: &ImageData) -> u32 {
    number_of(data.id())
}

/// The number the terminal keeps the picture with identity `id` under.
fn number_of(id: u64) -> u32 {
    let wrapped = (id.saturating_sub(1) % u64::from(u32::MAX)) + 1;
    u32::try_from(wrapped).unwrap_or(u32::MAX)
}

/// Writes the command that sends `data`'s pixels under `image`: RGB, compressed with zlib, in
/// base64 split into chunks the protocol allows.
fn transmit(out: &mut Vec<u8>, data: &ImageData, image: u32) {
    let rgb: Vec<u8> = data.pixels().iter().flat_map(|pixel| [pixel.r, pixel.g, pixel.b]).collect();
    let (payload, compressed) = match compress(&rgb) {
        Some(zipped) => (zipped, ",o=z"),
        None => (rgb, ""),
    };
    let encoded = base64(&payload);
    let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(CHUNK).collect();
    let (width, height) = (data.width(), data.height());
    for (index, chunk) in chunks.iter().enumerate() {
        let more = u8::from(index + 1 < chunks.len());
        if index == 0 {
            let _ = write!(out, "\x1b_Ga=t,f=24{compressed},s={width},v={height},i={image},q=2,m={more};");
        } else {
            let _ = write!(out, "\x1b_Gm={more},q=2;");
        }
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\x1b\\");
    }
}

/// Writes the command that shows `placement`: the cursor moved to its first cell, then the part
/// of the picture stretched over its cells. `z=-1` puts the picture under text, above the ground
/// colours of cells; `C=1` leaves the cursor where it is.
fn place(out: &mut Vec<u8>, placement: &PicturePlacement) {
    let (column, row, columns, rows) = placement.cells;
    let (x, y, width, height) = placement.crop;
    let image = number(&placement.data);
    let _ = write!(
        out,
        "\x1b[{};{}H\x1b_Ga=p,i={image},p={},x={x},y={y},w={width},h={height},c={columns},r={rows},z=-1,C=1,q=2\x1b\\",
        u32::from(row) + 1,
        u32::from(column) + 1,
        placement.number,
    );
}

/// `bytes` compressed with zlib; `None` should compressing fail, which writing to memory does not.
fn compress(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).ok()?;
    encoder.finish().ok()
}

/// `bytes` in standard base64, padded.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let joined =
            group.iter().enumerate().fold(0u32, |all, (index, byte)| all | u32::from(*byte) << (16 - 8 * index));
        for index in 0..4 {
            if index <= group.len() {
                out.push(char::from(ALPHABET[(joined >> (18 - 6 * index) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}
