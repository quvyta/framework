//! Tests of the recorder: frames, the pointer's path and the list ffmpeg reads.

use std::path::PathBuf;
use std::time::Duration;

use qframe::event::{Event, MouseButton, MouseKind};
use qframe::keymap::Modifiers;
use qframe::prelude::*;
use qframe::widget::{EventCx, MeasureCx, PaintCx, Widget};

use crate::Reel;

/// A folder for one test's frames under the system's temporary folder. It is not cleared here:
/// `Reel::new` empties it.
fn frame_dir(test: &str) -> PathBuf {
    std::env::temp_dir().join(format!("qshots-reel-{}-{test}", std::process::id()))
}

/// A screen that never changes.
struct Still;

impl App for Still {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Text::new("still"));
    }
}

/// A counter the test moves on with `send`, so the screen changes when the test says.
struct Counter(u32);

impl App for Counter {
    type Msg = ();
    fn update(&mut self, (): ()) -> Command<()> {
        self.0 += 1;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Text::new(format!("count {}", self.0)));
    }
}

/// What the pointer did on the tracker's surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pointer {
    Moved(i32, i32),
    Pressed(MouseButton, Modifiers, i32, i32),
}

/// A surface filling the screen that reports every cell the pointer moves onto or presses.
struct Surface;

impl Widget<Pointer> for Surface {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        cx.track_pointer_moves();
    }
    fn event(&self, cx: &mut EventCx<'_, Pointer>, event: &Event) -> bool {
        let Event::Mouse(mouse) = event else { return false };
        match mouse.kind {
            MouseKind::Moved => cx.emit(Pointer::Moved(mouse.x, mouse.y)),
            MouseKind::Down(button) => cx.emit(Pointer::Pressed(button, mouse.mods, mouse.x, mouse.y)),
            _ => return false,
        }
        true
    }
}

/// Everything the pointer did, in order.
#[derive(Default)]
struct Tracker(Vec<Pointer>);

impl Tracker {
    /// The cells the pointer was moved onto, in order.
    fn path(&self) -> Vec<(i32, i32)> {
        self.0
            .iter()
            .filter_map(|pointer| match *pointer {
                Pointer::Moved(x, y) => Some((x, y)),
                Pointer::Pressed(..) => None,
            })
            .collect()
    }
}

impl App for Tracker {
    type Msg = Pointer;
    fn update(&mut self, pointer: Pointer) -> Command<Pointer> {
        self.0.push(pointer);
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, Pointer>) {
        ui.add(Surface);
    }
}

#[test]
fn the_same_screen_held_twice_is_one_frame_with_both_durations() {
    let dir = frame_dir("same");
    let mut reel = Reel::new(Harness::new(Still, 12, 2), &dir);
    reel.hold(Duration::from_millis(200));
    reel.hold(Duration::from_millis(150));
    assert_eq!(reel.frames(), 1);
    assert_eq!(reel.total(), Duration::from_millis(350));
    let files = std::fs::read_dir(&dir).expect("the frame folder exists").count();
    assert_eq!(files, 1, "only the one distinct picture is written");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_changed_screen_is_a_new_frame() {
    let dir = frame_dir("changed");
    let mut reel = Reel::new(Harness::new(Counter(0), 12, 2), &dir);
    reel.hold(Duration::from_millis(100));
    reel.harness_mut().send(());
    reel.hold(Duration::from_millis(100));
    assert_eq!(reel.frames(), 2);
    assert_eq!(reel.total(), Duration::from_millis(200));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn glide_hovers_every_cell_it_passes() {
    let dir = frame_dir("glide");
    let mut reel = Reel::new(Harness::new(Tracker::default(), 40, 12), &dir).pointer_at(2, 1);
    reel.glide((30, 8));
    let path = reel.harness().app().path();
    assert_eq!(path.first(), Some(&(2, 1)), "placing the pointer hovers its cell");
    assert_eq!(path.last(), Some(&(30, 8)));
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        assert!((a.0 - b.0).abs() <= 1 && (a.1 - b.1).abs() <= 1, "the pointer jumped from {a:?} to {b:?}");
    }
    for column in 2..=30 {
        assert!(path.iter().any(|cell| cell.0 == column), "column {column} was skipped: {path:?}");
    }
    assert_eq!(reel.pointer(), Some((30, 8)));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_list_names_each_frame_with_its_duration_and_the_last_twice() {
    let dir = frame_dir("list");
    let mut reel = Reel::new(Harness::new(Counter(0), 12, 2), &dir);
    reel.hold(Duration::from_millis(250));
    reel.harness_mut().send(());
    reel.hold(Duration::from_millis(1500));
    let recording = reel.finish().expect("the list is written");
    assert_eq!(recording.list(), dir.join("frames.txt"));
    assert_eq!(recording.frames(), 2);
    assert_eq!(recording.total(), Duration::from_millis(1750));
    let list = std::fs::read_to_string(recording.list()).expect("the list is there");
    assert_eq!(list, "file '0000.png'\nduration 0.250\nfile '0001.png'\nduration 1.500\nfile '0001.png'\n");
    assert!(dir.join("0000.png").is_file() && dir.join("0001.png").is_file());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_check_sees_every_captured_screen() {
    let dir = frame_dir("check");
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let log = std::rc::Rc::clone(&seen);
    let mut reel = Reel::new(Harness::new(Counter(0), 12, 2), &dir)
        .check(move |screen| log.borrow_mut().push(screen.lines().next().unwrap_or_default().trim_end().to_owned()));
    reel.hold(Duration::from_millis(100));
    reel.harness_mut().send(());
    reel.capture(Duration::from_millis(100));
    assert_eq!(*seen.borrow(), ["count 0", "count 0", "count 1"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[should_panic(expected = "on screen")]
fn a_failing_check_stops_the_recording() {
    let dir = frame_dir("refused");
    let mut reel = Reel::new(Harness::new(Still, 12, 2), &dir)
        .check(|screen| assert!(!screen.contains("still"), "`still` is on screen"));
    reel.hold(Duration::from_millis(50));
}

#[test]
fn click_glides_to_the_cell_and_presses_there() {
    let dir = frame_dir("click");
    let mut reel = Reel::new(Harness::new(Tracker::default(), 20, 3), &dir).pointer_at(0, 2);
    reel.click((5, 0));
    assert_eq!(reel.pointer(), Some((5, 0)));
    assert_eq!(reel.harness().app().0.last(), Some(&Pointer::Pressed(MouseButton::Left, Modifiers::default(), 5, 0)));
    let _ = std::fs::remove_dir_all(dir);
}

/// The last press the tracker heard.
fn last_press(reel: &Reel<Tracker>) -> Pointer {
    *reel.harness().app().0.iter().rev().find(|pointer| matches!(pointer, Pointer::Pressed(..))).expect("a press")
}

#[test]
fn right_click_presses_the_right_button_where_it_glided() {
    let dir = frame_dir("right-click");
    let mut reel = Reel::new(Harness::new(Tracker::default(), 20, 3), &dir).pointer_at(0, 2);
    reel.right_click((5, 0));
    assert_eq!(reel.pointer(), Some((5, 0)));
    assert_eq!(last_press(&reel), Pointer::Pressed(MouseButton::Right, Modifiers::default(), 5, 0));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn click_with_carries_the_button_and_the_held_modifiers() {
    let dir = frame_dir("click-with");
    let ctrl = Modifiers { ctrl: true, ..Modifiers::default() };
    let mut reel = Reel::new(Harness::new(Tracker::default(), 20, 3), &dir).pointer_at(0, 2);
    reel.click_with(MouseButton::Left, ctrl, (7, 1));
    assert_eq!(last_press(&reel), Pointer::Pressed(MouseButton::Left, ctrl, 7, 1));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_right_click_draws_the_same_frames_as_a_plain_one() {
    let (left, right) = (frame_dir("frames-left"), frame_dir("frames-right"));
    let mut plain = Reel::new(Harness::new(Tracker::default(), 20, 3), &left).pointer_at(0, 2);
    plain.click((5, 0));
    let mut other = Reel::new(Harness::new(Tracker::default(), 20, 3), &right).pointer_at(0, 2);
    other.right_click((5, 0));
    assert_eq!(plain.frames(), other.frames());
    assert_eq!(plain.total(), other.total());
    for frame in 0..plain.frames() {
        let name = format!("{frame:04}.png");
        let (a, b) = (std::fs::read(left.join(&name)), std::fs::read(right.join(&name)));
        assert_eq!(a.expect("the frame is there"), b.expect("the frame is there"), "{name} differs");
    }
    let _ = std::fs::remove_dir_all(left);
    let _ = std::fs::remove_dir_all(right);
}

#[test]
fn right_click_on_rests_a_cell_inside_the_word() {
    let dir = frame_dir("right-click-on");
    let mut reel = Reel::new(Harness::new(Counter(0), 20, 3), &dir).pointer_at(10, 2);
    reel.right_click_on("count");
    assert_eq!(reel.pointer(), Some((1, 0)));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn click_on_rests_a_cell_inside_the_word() {
    let dir = frame_dir("click-on");
    let mut reel = Reel::new(Harness::new(Counter(0), 20, 3), &dir).pointer_at(10, 2);
    reel.click_on("count");
    assert_eq!(reel.pointer(), Some((1, 0)));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn encoding_without_ffmpeg_on_the_path_is_an_error() {
    let dir = frame_dir("no-ffmpeg");
    let mut reel = Reel::new(Harness::new(Still, 12, 2), &dir);
    reel.hold(Duration::from_millis(100));
    let recording = reel.finish().expect("the list is written");
    let error = recording
        .encode_using("qshots-no-such-ffmpeg", dir.join("out.gif"), dir.join("out.mp4"), 100)
        .expect_err("a missing ffmpeg is reported");
    assert!(error.to_string().contains("qshots-no-such-ffmpeg"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "needs ffmpeg on the PATH"]
fn encoding_writes_a_gif_and_an_mp4_of_the_asked_width() {
    let dir = frame_dir("encode");
    let mut reel = Reel::new(Harness::new(Counter(0), 20, 3), &dir).title("reel");
    reel.hold(Duration::from_millis(300));
    reel.harness_mut().send(());
    reel.hold(Duration::from_millis(300));
    let (gif, mp4) = (dir.join("out.gif"), dir.join("out.mp4"));
    reel.finish().expect("the list is written").encode(&gif, &mp4, 200).expect("ffmpeg encodes");
    let gif_bytes = std::fs::read(&gif).expect("the GIF is there");
    assert!(gif_bytes.starts_with(b"GIF89a"));
    // A GIF's logical screen width is the little-endian word after the signature.
    assert_eq!(u16::from_le_bytes([gif_bytes[6], gif_bytes[7]]), 200);
    assert!(std::fs::metadata(&mp4).expect("the MP4 is there").len() > 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_new_reel_empties_the_frame_folder() {
    let dir = frame_dir("empties");
    std::fs::create_dir_all(dir.join("inside")).expect("the folder is made");
    std::fs::write(dir.join("0000.png"), b"a frame of an earlier run").expect("the stale frame is written");
    let mut reel = Reel::new(Harness::new(Still, 12, 2), &dir);
    reel.hold(Duration::from_millis(50));
    let left: Vec<_> = std::fs::read_dir(&dir)
        .expect("the frame folder exists")
        .map(|entry| entry.expect("the entry reads").file_name())
        .collect();
    assert_eq!(left, ["0000.png"], "only the new frame is there");
    let frame = std::fs::read(dir.join("0000.png")).expect("the frame reads");
    assert!(frame.starts_with(b"\x89PNG"), "the stale file was replaced by a real frame");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn hold_with_moves_the_application_clock_on_every_frame() {
    let dir = frame_dir("hold-with");
    let mut reel = Reel::new(Harness::new(Counter(0), 12, 2), &dir);
    let mut steps = Vec::new();
    reel.hold_with(Duration::from_millis(200), |harness, step| {
        steps.push(step);
        harness.send(());
    });
    assert_eq!(steps, [Duration::from_millis(50); 4], "the tick sees each 50 ms as it passes");
    assert_eq!(reel.harness().app().0, 4);
    assert_eq!(reel.frames(), 4, "the counter changes the screen on every frame");
    assert_eq!(reel.total(), Duration::from_millis(200));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn skip_lets_time_pass_without_drawing() {
    let dir = frame_dir("skip");
    let mut reel = Reel::new(Harness::new(Still, 12, 2), &dir);
    reel.skip(Duration::from_secs(5));
    assert_eq!(reel.frames(), 0);
    assert_eq!(reel.total(), Duration::ZERO);
    reel.hold(Duration::from_millis(50));
    assert_eq!(reel.frames(), 1);
    assert_eq!(reel.total(), Duration::from_millis(50));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "needs ffmpeg and ffprobe on the PATH"]
fn the_gif_and_the_mp4_last_as_long_as_the_recording() {
    let dir = frame_dir("length");
    let mut reel = Reel::new(Harness::new(Counter(0), 20, 3), &dir);
    reel.hold(Duration::from_millis(300));
    reel.harness_mut().send(());
    // A long last pause: without `-t` the last frame would be held for it twice over.
    reel.hold(Duration::from_millis(2000));
    let (gif, mp4) = (dir.join("out.gif"), dir.join("out.mp4"));
    let recording = reel.finish().expect("the list is written");
    recording.encode(&gif, &mp4, 200).expect("ffmpeg encodes");
    let total = recording.total();
    for picture in [&gif, &mp4] {
        let length = duration_of(picture);
        let slack = length.abs_diff(total);
        assert!(slack <= Duration::from_millis(50), "{} lasts {length:?}, the recording {total:?}", picture.display());
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// How long the picture at `path` plays, counting its frames with ffprobe: a GIF's container
/// duration is written per frame, so the frames at the recording's 20 a second are the measure.
fn duration_of(path: &std::path::Path) -> Duration {
    let run = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-count_frames", "-select_streams", "v:0"])
        .args(["-show_entries", "stream=nb_read_frames", "-of", "csv=p=0"])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    assert!(run.status.success(), "ffprobe failed: {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8_lossy(&run.stdout);
    let frames: u32 = text.trim().trim_end_matches(',').parse().unwrap_or_else(|error| panic!("{text:?}: {error}"));
    Duration::from_millis(50) * frames
}
