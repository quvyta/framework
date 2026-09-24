//! How often the runtime draws: the frame limit an application sets and its defaults.

use std::time::Duration;

/// How many frames a second the runtime draws at most, for a local and for a remote connection.
///
/// An application answers with one from [`App::frame_limit`](super::App::frame_limit). The limit
/// merges frames the application's own work causes: output of an embedded terminal, messages of
/// background work, a running animation. It never holds back a frame that answers a key, a paste,
/// a mouse button going down or a button coming up — the echo of a typed character, the answer to
/// a click and the place a dragged window comes to rest are drawn at once, so the limit cannot be
/// felt in the keyboard or in a click.
///
/// The pointer moving is merged too: a drag, the pointer passing over the screen and the wheel
/// arrive as fast as the hand moves, and only the latest state is worth a frame. Every one of
/// those events still reaches the widgets and the application, which apply them all; only the
/// drawing waits, at most one gap of the limit, and the first motion after a rest that long is
/// drawn at once. So a two-second window drag at 5 frames a second writes ten frames, not a frame
/// for every cell the pointer crossed.
///
/// The default draws 60 frames a second locally and 20 over a remote connection
/// ([`Env::remote`](crate::env::Env::remote)), because on a slow link every frame is a screen
/// written down a network: a program pouring out lines would otherwise spend the connection on
/// frames nobody can read apart.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::runtime::FrameLimit;
///
/// // The default: 60 frames a second at the machine, 20 over SSH.
/// assert_eq!(FrameLimit::default().frames_per_second(false), Some(60));
/// assert_eq!(FrameLimit::default().frames_per_second(true), Some(20));
///
/// // The same number everywhere, or a quieter one on a remote connection.
/// assert_eq!(FrameLimit::per_second(30).frames_per_second(true), Some(30));
/// assert_eq!(FrameLimit::per_second(30).remote(10).frames_per_second(true), Some(10));
///
/// // Every frame that is wanted is drawn.
/// assert_eq!(FrameLimit::none().frames_per_second(false), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLimit {
    /// Frames a second at the machine the application runs on; `None` is no limit.
    local: Option<u32>,
    /// Frames a second over a remote connection; `None` is no limit.
    remote: Option<u32>,
}

impl Default for FrameLimit {
    fn default() -> Self {
        Self { local: Some(Self::LOCAL), remote: Some(Self::REMOTE) }
    }
}

impl FrameLimit {
    /// Frames a second the default draws locally: as many as a screen at 60 Hz shows.
    pub const LOCAL: u32 = 60;
    /// Frames a second the default draws over a remote connection: fast enough to read a
    /// scrolling program by, cheap enough for a link that carries every frame.
    pub const REMOTE: u32 = 20;

    /// The same number of frames a second on every connection. A `frames` of zero is no limit,
    /// the way [`FrameLimit::none`] is; a limit of no frames would draw nothing at all.
    #[must_use]
    pub fn per_second(frames: u32) -> Self {
        let frames = (frames > 0).then_some(frames);
        Self { local: frames, remote: frames }
    }

    /// No limit: every frame the application asks for is drawn.
    #[must_use]
    pub fn none() -> Self {
        Self { local: None, remote: None }
    }

    /// Draws `frames` a second over a remote connection instead, and leaves the local number as
    /// it is. A `frames` of zero lifts the limit for remote connections.
    #[must_use]
    pub fn remote(self, frames: u32) -> Self {
        Self { remote: (frames > 0).then_some(frames), ..self }
    }

    /// How many frames a second this limit allows on the connection in hand; `None` is no limit.
    /// `remote` is what [`Env::remote`](crate::env::Env::remote) reports.
    #[must_use]
    pub fn frames_per_second(self, remote: bool) -> Option<u32> {
        if remote { self.remote } else { self.local }
    }

    /// The shortest time between two frames this limit allows, or `None` when it allows any.
    pub(crate) fn gap(self, remote: bool) -> Option<Duration> {
        self.frames_per_second(remote).map(|frames| Duration::from_secs(1) / frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_draws_fewer_frames_over_a_remote_connection() {
        let limit = FrameLimit::default();
        assert_eq!(limit.frames_per_second(false), Some(60));
        assert_eq!(limit.frames_per_second(true), Some(20));
        assert_eq!(limit.gap(false), Some(Duration::from_nanos(16_666_666)));
        assert_eq!(limit.gap(true), Some(Duration::from_millis(50)));
    }

    #[test]
    fn a_number_given_holds_on_every_connection_until_the_remote_one_is_given_too() {
        let same = FrameLimit::per_second(30);
        assert_eq!(same.frames_per_second(false), Some(30));
        assert_eq!(same.frames_per_second(true), Some(30));
        let quieter = same.remote(10);
        assert_eq!(quieter.frames_per_second(false), Some(30), "the local number stays");
        assert_eq!(quieter.frames_per_second(true), Some(10));
    }

    #[test]
    fn no_limit_and_a_limit_of_zero_frames_both_allow_every_frame() {
        assert_eq!(FrameLimit::none().gap(false), None);
        assert_eq!(FrameLimit::none().gap(true), None);
        assert_eq!(FrameLimit::per_second(0).frames_per_second(false), None, "zero frames would draw nothing");
        assert_eq!(FrameLimit::default().remote(0).frames_per_second(true), None);
        assert_eq!(FrameLimit::default().remote(0).frames_per_second(false), Some(60), "only the remote one is lifted");
    }
}
