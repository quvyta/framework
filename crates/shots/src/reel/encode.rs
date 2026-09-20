//! Joining the frames into a GIF and an MP4 with ffmpeg.

use std::io;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Runs `program` (ffmpeg) three times over the concat list `list`: a palette for the whole
/// recording, the GIF with it, and the MP4. Frames are played every `step` and scaled to
/// `width`, and both pictures stop after `total`.
pub(super) fn run(
    program: &str,
    list: &Path,
    gif: &Path,
    mp4: &Path,
    width: u32,
    step: Duration,
    total: Duration,
) -> io::Result<()> {
    let palette = list.with_file_name("palette.png");
    let fps = (1000 / step.as_millis().max(1)).max(1);
    // How long the story lasts. Without it the picture outlasts the recording: the concat list
    // names the last frame twice, so the reader honours its pause, and the `fps` filter then
    // holds that repeat for another full pause. A looping GIF would look stuck at the end.
    // One frame at least, so a recording of nothing still gives a picture.
    let length = format!("{:.3}", total.max(step).as_secs_f64());
    let stop = ["-t", &length];
    // The height follows the width and stays even, which H.264 in yuv420p needs. The frames
    // have square corners filled with the ground: a GIF with transparent pixels loses ffmpeg's
    // frame-difference cropping and grows about thirty times, and the reserve for transparency
    // would take a colour the palette needs.
    let scale = format!("fps={fps},scale={width}:-2:flags=lanczos,format=rgb24");
    let input = ["-f", "concat", "-safe", "0", "-i"];
    let palette_filter = format!("{scale},palettegen=max_colors=256:stats_mode=full:reserve_transparent=0");
    ffmpeg(program, &input, list, &["-vf", &palette_filter], &palette)?;
    let gif_filter = format!("{scale}[v];[v][1:v]paletteuse=dither=none:diff_mode=rectangle");
    let palette_input = palette.to_string_lossy();
    let picture = ["-i", &palette_input, "-lavfi", &gif_filter, "-loop", "0"];
    ffmpeg(program, &input, list, &[&picture[..], &stop].concat(), gif)?;
    let video = ["-vf", &scale, "-c:v", "libx264", "-crf", "23", "-preset", "slow", "-pix_fmt", "yuv420p"];
    ffmpeg(program, &input, list, &[&video[..], &["-movflags", "+faststart"], &stop].concat(), mp4)
}

/// One ffmpeg run: `input` options, the list, then `options` and the output file.
fn ffmpeg(program: &str, input: &[&str], list: &Path, options: &[&str], output: &Path) -> io::Result<()> {
    let run = Command::new(program)
        .args(["-v", "error", "-y"])
        .args(input)
        .arg(list)
        .args(options)
        .arg(output)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("`{program}` is not on the PATH; the GIF and the MP4 are encoded with ffmpeg"),
                )
            } else {
                io::Error::new(error.kind(), format!("`{program}` could not be started: {error}"))
            }
        })?;
    if run.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&run.stderr);
    Err(io::Error::other(format!(
        "{program} could not write {} ({}): {}",
        output.display(),
        run.status,
        message.trim()
    )))
}
