//! Joining the frames into a GIF and an MP4 with ffmpeg.

use std::io;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Runs `program` (ffmpeg) three times over the concat list `list`: a palette for the whole
/// recording, the GIF with it, and the MP4. Frames are played every `step` and scaled to
/// `width`.
pub(super) fn run(program: &str, list: &Path, gif: &Path, mp4: &Path, width: u32, step: Duration) -> io::Result<()> {
    let palette = list.with_file_name("palette.png");
    let fps = (1000 / step.as_millis().max(1)).max(1);
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
    ffmpeg(program, &input, list, &["-i", &palette_input, "-lavfi", &gif_filter, "-loop", "0"], gif)?;
    let video = ["-vf", &scale, "-c:v", "libx264", "-crf", "23", "-preset", "slow", "-pix_fmt", "yuv420p"];
    ffmpeg(program, &input, list, &[&video[..], &["-movflags", "+faststart"]].concat(), mp4)
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
