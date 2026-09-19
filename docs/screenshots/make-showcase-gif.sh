#!/usr/bin/env bash
# Records the showcase tour as showcase.gif and showcase.mp4 in this folder.
#
# The frames come from the showcase's test harness with a fake clock (the ignored `reel` test),
# so they are the same on every machine; only ffmpeg's encoding may differ between versions.
# Needs ffmpeg on the PATH.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
frames="$(mktemp -d)"
trap 'rm -rf "$frames"' EXIT

export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
# Release: rasterising a few hundred full frames is slow unoptimised.
(cd "$root" && QUVYTA_REEL_DIR="$frames" cargo test --release -p quvyta-framework-showcase --lib \
    tests::reel::reel -- --ignored --exact --nocapture)

# The frames are drawn at twice the size for sharpness; half size keeps the GIF small while
# every cell stays readable. A steady 20 frames a second matches the recording's step. The frames
# have square corners filled with the terminal's ground: a GIF with transparent pixels loses
# ffmpeg's frame-difference cropping and grows about thirty times, and dropping the alpha of
# rounded corners would turn them black.
scale="fps=20,scale=iw/2:-1:flags=lanczos,format=rgb24"
ffmpeg -v error -y -f concat -safe 0 -i "$frames/frames.txt" \
    -vf "$scale,palettegen=max_colors=256:stats_mode=full:reserve_transparent=0" "$frames/palette.png"
ffmpeg -v error -y -f concat -safe 0 -i "$frames/frames.txt" -i "$frames/palette.png" \
    -lavfi "$scale[v];[v][1:v]paletteuse=dither=none:diff_mode=rectangle" \
    -loop 0 "$here/showcase.gif"
ffmpeg -v error -y -f concat -safe 0 -i "$frames/frames.txt" \
    -vf "$scale" -c:v libx264 -crf 23 -preset slow -pix_fmt yuv420p -movflags +faststart \
    "$here/showcase.mp4"

ls -l "$here/showcase.gif" "$here/showcase.mp4"
