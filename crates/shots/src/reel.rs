//! Moving pictures of a harness: a scripted visit drawn frame by frame on the fake clock, then
//! joined into a GIF and an MP4 by ffmpeg.

mod encode;
#[cfg(test)]
mod tests;

use std::fmt::{self, Write as _};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use qframe::runtime::{App, Harness};

use crate::Shot;

/// Time between two frames: smooth enough for the framework's cell-step animations, which move
/// no faster. [`Recording::encode`] plays the frames at the matching 20 a second.
const STEP: Duration = Duration::from_millis(50);

/// How long the pointer takes to cross one cell when it glides: slow enough to follow.
const GLIDE_PER_CELL: Duration = Duration::from_millis(12);

/// The shortest glide, so even a one-cell move is seen easing in.
const GLIDE_MIN: Duration = Duration::from_millis(200);

/// The longest glide, so crossing the whole screen does not drag.
const GLIDE_MAX: Duration = Duration::from_millis(900);

/// The pause between arriving on a target and clicking it, as a hand settles before it presses.
const SETTLE: Duration = Duration::from_millis(100);

/// The name of the list [`Reel::finish`] writes next to the frames.
const LIST: &str = "frames.txt";

/// An assertion run on every frame's screen text, see [`Reel::check`].
type Check = Box<dyn FnMut(&str)>;

/// A recorder of a [`Harness`] as a moving picture, for READMEs and launch posts.
///
/// Each step lets the harness's fake clock run and draws a [`Shot`] every 50 ms, so the
/// framework's animations play as they would in a terminal and the same script gives the same
/// frames on every machine. A picture equal to the one before is not written again; the earlier
/// frame is held longer instead, so pauses cost nothing. Frames are square PNGs (the corners
/// filled with the theme's ground, see [`Shot::square`]) at twice the terminal's size, numbered
/// `0000.png`, `0001.png` and so on in the folder given to [`Reel::new`].
///
/// A mouse pointer is drawn once [`Reel::pointer_at`] placed one; [`Reel::glide`] and the clicks
/// move it cell by cell. For steps the recorder has no word for, [`Reel::harness_mut`] reaches
/// the harness, and [`Reel::hold`] then records what follows.
///
/// ```no_run
/// # use qframe::prelude::*;
/// # struct Tour;
/// # impl App for Tour {
/// #     type Msg = ();
/// #     fn update(&mut self, (): ()) -> Command<()> { Command::none() }
/// #     fn view(&self, ui: &mut View<'_, ()>) { ui.add(Text::new("Settings")); }
/// # }
/// use std::time::Duration;
///
/// let harness = Harness::new(Tour, 100, 30);
/// let mut reel = qshots::Reel::new(harness, "target/tour-frames").title("tour").pointer_at(90, 20);
/// reel.hold(Duration::from_secs(2));
/// reel.click_on("Settings").hold(Duration::from_secs(1));
/// reel.press("ctrl+p", Duration::from_millis(600)).type_slowly("theme", Duration::from_millis(180));
/// let recording = reel.finish()?;
/// recording.encode("docs/tour.gif", "docs/tour.mp4", 1000)?;
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Panics
///
/// The recording steps (everything that draws a frame) panic, as a failed test assertion does,
/// when a frame cannot be written to the folder, when a glyph on screen is missing from the
/// embedded fonts (see [`Shot::missing`]), or when the [`Reel::check`] closure panics.
pub struct Reel<A: App> {
    harness: Harness<A>,
    dir: PathBuf,
    title: Option<String>,
    pointer: Option<(i32, i32)>,
    check: Option<Check>,
    /// Each distinct picture written, with how long it stays.
    frames: Vec<(String, Duration)>,
    /// The last picture as SVG, to tell a repeated screen from a new one.
    last: String,
    /// Whether the frame folder was made, so it is made once, on the first frame.
    dir_ready: bool,
}

impl<A: App> fmt::Debug for Reel<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reel")
            .field("dir", &self.dir)
            .field("title", &self.title)
            .field("pointer", &self.pointer)
            .field("frames", &self.frames.len())
            .field("total", &self.total())
            .finish_non_exhaustive()
    }
}

impl<A: App> Reel<A> {
    /// A recorder of `harness` that writes its frames into `dir`, made on the first frame.
    ///
    /// Nothing is drawn yet; set the harness up (size, theme, the first screen) before or
    /// through [`Reel::harness_mut`].
    pub fn new(harness: Harness<A>, dir: impl Into<PathBuf>) -> Self {
        Self {
            harness,
            dir: dir.into(),
            title: None,
            pointer: None,
            check: None,
            frames: Vec::new(),
            last: String::new(),
            dir_ready: false,
        }
    }

    /// Draws every frame with a title strip, as [`Shot::title`] does.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Puts the mouse pointer on the cell at column `x`, row `y`, telling the application with
    /// a hover, and draws it on every frame from now on.
    #[must_use]
    pub fn pointer_at(mut self, x: i32, y: i32) -> Self {
        self.move_pointer((x, y));
        self
    }

    /// Runs `check` on the screen's text before every frame is drawn, for assertions that must
    /// hold on the whole recording, such as no real home folder ever showing. The closure fails
    /// the recording by panicking, as `assert!` does.
    #[must_use]
    pub fn check(mut self, check: impl FnMut(&str) + 'static) -> Self {
        self.check = Some(Box::new(check));
        self
    }

    /// The harness being recorded.
    #[must_use]
    pub fn harness(&self) -> &Harness<A> {
        &self.harness
    }

    /// The harness being recorded, for steps the recorder has no word for: a message, a resize,
    /// a theme. Nothing is drawn until the next recording step.
    pub fn harness_mut(&mut self) -> &mut Harness<A> {
        &mut self.harness
    }

    /// The cell the pointer is on, once [`Reel::pointer_at`] or a glide placed it.
    #[must_use]
    pub fn pointer(&self) -> Option<(i32, i32)> {
        self.pointer
    }

    /// Draws the screen as it is now and holds it for `duration`, without moving the clock.
    ///
    /// For applications whose screen changes in real time (a terminal's output arriving), where
    /// the test waits for the screen itself and then records it; [`Reel::hold`] is the step
    /// for everything the fake clock drives.
    pub fn capture(&mut self, duration: Duration) -> &mut Self {
        if let Some(check) = self.check.as_mut() {
            check(&self.harness.screen());
        }
        let mut shot = Shot::of(&self.harness).square();
        if let Some(title) = &self.title {
            shot = shot.title(title.clone());
        }
        if let Some((x, y)) = self.pointer {
            // A pointer off the grid draws nothing, which a cell beyond `u16` also is.
            shot = shot.pointer(u16::try_from(x).unwrap_or(u16::MAX), u16::try_from(y).unwrap_or(u16::MAX));
        }
        let missing = shot.missing();
        assert!(missing.is_empty(), "the embedded fonts lack {missing:?}:\n{}", self.harness.screen());
        let svg = shot.to_svg();
        if svg == self.last
            && let Some((_, held)) = self.frames.last_mut()
        {
            *held += duration;
            return self;
        }
        let name = format!("{:04}.png", self.frames.len());
        let png = shot.to_png().unwrap_or_else(|error| panic!("frame {name} does not rasterise: {error}"));
        self.write(&name, &png);
        self.frames.push((name, duration));
        self.last = svg;
        self
    }

    /// Lets `duration` pass on the fake clock, a frame every 50 ms, so animations play.
    pub fn hold(&mut self, duration: Duration) -> &mut Self {
        let mut left = duration;
        while !left.is_zero() {
            let step = left.min(STEP);
            self.harness.advance(step);
            self.capture(step);
            left -= step;
        }
        self
    }

    /// Presses the key chord `chord` (as [`Harness::press`] reads it) and holds for `stay`.
    pub fn press(&mut self, chord: &str, stay: Duration) -> &mut Self {
        self.harness.press(chord);
        self.hold(stay)
    }

    /// Types `text` a character at a time, holding `gap` after each, as a calm typist.
    pub fn type_slowly(&mut self, text: &str, gap: Duration) -> &mut Self {
        let mut buffer = [0; 4];
        for c in text.chars() {
            self.harness.type_text(c.encode_utf8(&mut buffer));
            self.hold(gap);
        }
        self
    }

    /// Moves the pointer to `to` in a straight line, easing out as it arrives, and hovers every
    /// cell it passes, so hover highlights follow it as they would under a hand.
    ///
    /// The glide takes 12 ms a cell of its longer leg, kept between 0.2 and 0.9 seconds. With
    /// no pointer yet, the pointer appears at `to`.
    pub fn glide(&mut self, to: (i32, i32)) -> &mut Self {
        let Some(from) = self.pointer else {
            self.move_pointer(to);
            return self.hold(STEP);
        };
        let cells = (to.0 - from.0).abs().max((to.1 - from.1).abs()).max(1);
        let time = (GLIDE_PER_CELL * u32::try_from(cells).unwrap_or(u32::MAX)).clamp(GLIDE_MIN, GLIDE_MAX);
        let steps = u32::try_from(time.as_millis() / STEP.as_millis()).unwrap_or(1).max(1);
        for step in 1..=steps {
            let t = f64::from(step) / f64::from(steps);
            let eased = 1.0 - (1.0 - t).powi(3);
            #[allow(clippy::cast_possible_truncation)] // A cell between `from` and `to` fits.
            let at = |a: i32, b: i32| a + (f64::from(b - a) * eased).round() as i32;
            let next = (at(from.0, to.0), at(from.1, to.1));
            let current = self.pointer.unwrap_or(from);
            for cell in cells_between(current, next) {
                self.move_pointer(cell);
            }
            self.harness.advance(STEP);
            self.capture(STEP);
        }
        self
    }

    /// Glides to the cell `at`, settles for a moment and clicks it with the left button.
    pub fn click(&mut self, at: (i32, i32)) -> &mut Self {
        self.glide(at);
        self.hold(SETTLE);
        self.harness.click(at.0, at.1);
        self
    }

    /// Clicks the first occurrence of `text` on screen, resting a cell inside the word, which
    /// reads better than touching its first letter.
    ///
    /// # Panics
    ///
    /// Panics when `text` is not on screen.
    pub fn click_on(&mut self, text: &str) -> &mut Self {
        self.click_on_below(text, 0)
    }

    /// Clicks the first occurrence of `text` at or below row `row`, as [`Reel::click_on`] does;
    /// for a word that also shows higher up, such as a theme's name in a top bar and in the list
    /// it opens.
    ///
    /// # Panics
    ///
    /// Panics when `text` is not on screen at or below `row`.
    pub fn click_on_below(&mut self, text: &str, row: i32) -> &mut Self {
        let found = self.find_below(text, row);
        let (x, y) =
            found.unwrap_or_else(|| panic!("`{text}` is not on screen below row {row}:\n{}", self.harness.screen()));
        let inside = if text.chars().nth(1).is_some() { 1 } else { 0 };
        self.click((x + inside, y))
    }

    /// The cell of the first occurrence of `text` at or below row `row`, or `None`.
    #[must_use]
    pub fn find_below(&self, text: &str, row: i32) -> Option<(i32, i32)> {
        let screen = self.harness.screen();
        let skip = usize::try_from(row).unwrap_or(0);
        screen.lines().enumerate().skip(skip).find_map(|(y, line)| {
            let column = line[..line.find(text)?].chars().count();
            Some((i32::try_from(column).ok()?, i32::try_from(y).ok()?))
        })
    }

    /// How many distinct frames were written so far.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames.len()
    }

    /// How long the recording so far plays.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.frames.iter().map(|(_, held)| *held).sum()
    }

    /// Ends the recording: writes `frames.txt` next to the frames, the list ffmpeg's concat
    /// reader joins them by, each with how long it stays. The last frame is named once more, as
    /// the reader wants, or its pause would be cut.
    ///
    /// # Errors
    ///
    /// Returns the error of making the folder or writing the list.
    pub fn finish(self) -> io::Result<Recording> {
        let mut list = String::new();
        for (name, held) in &self.frames {
            let _ = writeln!(list, "file '{name}'\nduration {:.3}", held.as_secs_f64());
        }
        if let Some((name, _)) = self.frames.last() {
            let _ = writeln!(list, "file '{name}'");
        }
        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(LIST);
        std::fs::write(&path, list)?;
        Ok(Recording { list: path, frames: self.frames.len(), total: self.total() })
    }

    /// Moves the pointer onto `cell` and tells the application.
    fn move_pointer(&mut self, cell: (i32, i32)) {
        self.pointer = Some(cell);
        self.harness.hover(cell.0, cell.1);
    }

    /// Writes one frame into the folder, making the folder first.
    fn write(&mut self, name: &str, png: &[u8]) {
        if !self.dir_ready {
            std::fs::create_dir_all(&self.dir)
                .unwrap_or_else(|error| panic!("the frame folder {} cannot be made: {error}", self.dir.display()));
            self.dir_ready = true;
        }
        let path = self.dir.join(name);
        std::fs::write(&path, png).unwrap_or_else(|error| panic!("{} cannot be written: {error}", path.display()));
    }
}

/// A finished recording: its frames and the list that joins them, ready to encode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    list: PathBuf,
    frames: usize,
    total: Duration,
}

impl Recording {
    /// The concat list, `frames.txt` in the frame folder.
    #[must_use]
    pub fn list(&self) -> &Path {
        &self.list
    }

    /// How many distinct frames the recording has.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// How long the recording plays.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.total
    }

    /// Encodes the recording as a looping GIF at `gif` and an H.264 MP4 at `mp4`, both `width`
    /// pixels wide, with ffmpeg from the `PATH`.
    ///
    /// The frames are drawn at twice the terminal's size and scaled down with Lanczos, so every
    /// cell stays crisp. The GIF takes one 256-colour palette for the whole recording, with no
    /// dithering (flat terminal colours would only gain noise) and only the changed rectangle
    /// stored per frame, which keeps it small; the MP4 is x264 at CRF 23, `yuv420p` so every
    /// player shows it, with its index at the front so a page can start playing it early.
    ///
    /// # Errors
    ///
    /// Returns an error naming the cause when ffmpeg is not on the `PATH`, and one carrying
    /// ffmpeg's own message when it fails.
    pub fn encode(&self, gif: impl AsRef<Path>, mp4: impl AsRef<Path>, width: u32) -> io::Result<()> {
        self.encode_using("ffmpeg", gif.as_ref(), mp4.as_ref(), width)
    }

    /// [`Recording::encode`] with the ffmpeg found as `program`.
    pub(crate) fn encode_using(
        &self,
        program: &str,
        gif: impl AsRef<Path>,
        mp4: impl AsRef<Path>,
        width: u32,
    ) -> io::Result<()> {
        encode::run(program, &self.list, gif.as_ref(), mp4.as_ref(), width, STEP)
    }
}

/// The cells from `from` (left out) to `to` (included) along a straight line, each a neighbour
/// of the one before, so a hover reaches every cell the pointer crosses.
fn cells_between(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = ((to.0 - from.0).abs(), -(to.1 - from.1).abs());
    let (sx, sy) = ((to.0 - from.0).signum(), (to.1 - from.1).signum());
    let (mut x, mut y, mut error) = (from.0, from.1, dx + dy);
    let mut cells = Vec::new();
    while (x, y) != to {
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
        cells.push((x, y));
    }
    cells
}
