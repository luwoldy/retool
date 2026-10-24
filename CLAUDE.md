# re — project notes for Claude

Read `README.md` first for what this repo is and how the tools run.

## Conventions

- Research before building: decisions live in `RESEARCH-*.md`, experiment
  results in `EXPERIMENT-*.md`. When a decision changes, amend the doc that
  made it (with a dated correction note) rather than leaving it stale.
- One binary, subcommands, no CLI framework. Pipeline stages are plain
  functions in `src/main.rs`; keep them individually runnable.
- Image math in f32 linear light end-to-end; quantize once at JPEG export.
- `src/dino.rs` is vendored from candle-transformers — don't "improve" it
  beyond what keeps it loading timm's DINOv2-reg4 weights.
- Verify pipeline changes by eye: process a station, downscale with
  `sips -Z 1400`, and look at the image, not just the exit code.

## Environment quirks (hard-won, don't rediscover)

- cargo needs `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"` in
  non-interactive shells.
- enfuse binary: `/Applications/Hugin/HuginStitchProject.app/Contents/MacOS/enfuse`
  (tools_mac symlink is dangling; cask needs quarantine stripped).
- `image` crate decoders hit a memory limit on our 40MP 16-bit TIFFs —
  use `reader.no_limits()` (already done where needed).
- rawler 0.8's `RawDevelop` demosaics X-Trans bilinear (soft); `develop_to_tiff`
  injects its Markesteijn pass by running Rescale alone, demosaicing, and
  feeding the RGB back as a `cpp = 3` raw for the remaining steps.
- candle 0.11's fused layer-norm has no Metal kernel — `src/dino.rs`
  carries a composed-ops LayerNorm instead.
- Bracket frames are `DSCF*.RAF` (sRGB), flash pops `_DSF*.RAF`
  (Adobe RGB) — two filename families, one card.
- Video: exiftool prints QuickTime CreateDate as stored (the camera's wall
  clock, same as the RAFs); ffmpeg stores a naive `creation_time` as UTC
  after a local conversion, so synthesised clips need the `Z` suffix.
  `-hwaccel videotoolbox` makes 4K HEVC decode 5× faster. vid.stab's
  detect file is binary and `vidstabtransform` cannot read it back on this
  ffmpeg, hence the block-match motion estimate in `src/video.rs`.
- zsh: `$4:s=…` inside a string triggers the `:s` history modifier; brace
  parameters (`${4}`) in scripts that build ffmpeg filter strings.
- Resolve scripting (Studio 21, must be running): `tools/resolve_build.py`
  appends the API's Modules folder itself. A timeline with `useCustomSettings`
  carries its own colour-management block and defaults to *unmanaged*; copy
  the project's keys onto it, `rcmPresetMode` Custom before `colorSpaceOutput`.
  `GetSourceStartFrame` floor-rounds a float (reads one low); `GetLeftOffset`
  is exact. One marker per frame, and none past the media on the timeline.
  Plain `python3` here has an ASCII locale: `encoding="utf-8"` on writes.
  `timelinePlaybackFrameRate` is read-only to scripts (stays at the default 24):
  a 30 fps project needs `--template` from a .drp set right once in the UI.

## The user's shooting recipe

X-H2, RAF lossless only. Stations = 5-frame ±2EV bracket (ES, f/6.4,
ISO 125, WB fixed 5000K, DR100) + one bounced Godox V1 pop (MS, ISO 320).
Compliance checks in `run_card` encode this; the recipe doc vs. reality
deviations found so far are logged in EXPERIMENT-first-shoot.md. The
reasoning behind each number (and why not f/8) is `RECIPE.md` — amend it
when a setting changes.
