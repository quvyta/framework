#!/usr/bin/env bash
# Records the showcase tour as showcase.gif and showcase.mp4 in this folder.
#
# The frames come from the showcase's test harness with a fake clock (the ignored `reel` test,
# built on qshots::Reel), so they are the same on every machine; only ffmpeg's encoding may
# differ between versions. The test encodes them itself. Needs ffmpeg on the PATH.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
# Release: rasterising a few hundred full frames is slow unoptimised. The recorder empties its
# frame folder itself, so nothing of an earlier run is left behind.
(cd "$root" && cargo test --release -p quvyta-framework-showcase --lib \
    tests::reel::reel -- --ignored --exact --nocapture)

ls -l "$here/showcase.gif" "$here/showcase.mp4"
