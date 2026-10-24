# Rust imaging stack for the photo-analysis tool

Research 2026-08-31, verified against crates.io / docs.rs / GitHub the same day.
Input reality (from the shoot recipe): X-H2-class 40MP X-Trans CMOS 5 HR,
RAF lossless compressed only, stations of 5-frame ±2EV bracket + 1 flash pop;
MOV video on slot 2. Reference/comparison images will mostly be listing JPEGs.

## 0. Verdict

Pure Rust covers the whole analysis pipeline as a single fast binary — no
OpenCV, no libvips. The one genuine ecosystem gap for us (high-quality X-Trans
demosaic) is **routed around entirely** for analysis by scoring the embedded
preview JPEG, reserving raw CFA data for clipping truth. **Correction
(2026-08-31, editing-feasibility pass):** the 40MP bodies embed a ~13MP
4416×2944 preview, NOT full-res (dpreview X-T5 thread) — plenty for analysis
(we downscale to ~3MP anyway), useless for full-res *delivery*; and rawler
0.8 ships `XTransMarkesteijnDemosaic` (darktable-quality) alongside bilinear
in `imgop::sensor::xtrans`, so the editing path can demosaic natively. See
RESEARCH-rust-editing.md.
Fuji makernotes (DR setting, shutter type, bracket/drive mode) are the one
place we shell out: `exiftool` (not yet installed — `brew install exiftool`).

## 1. Recommended stack

```toml
rawler            = "0.8"   # PRIMARY decoder: RAF (X-H2 confirmed, incl. lossless
                              # 2026-09-03: its developer demosaics X-Trans BILINEAR (soft at 100 %);
                              # develop_to_tiff runs rawler's own Markesteijn pass instead, 7 s a frame
                            # compressed) + ARW/CR3/NEF/DNG. full_image() hands us
                            # the embedded preview JPEG as an image::DynamicImage;
                            # decode_file() + RawDevelop for raw-value clipping stats
nom-exif          = "3.7"   # standard EXIF from RAF/JPEG/HEIC/CR3 + MP4/MOV, one API
image             = "0.25"  # JPEG/PNG/TIFF decode (zune-jpeg inside = turbo-class speed)
fast_image_resize = "6.1"   # NEON SIMD downscale 40MP → ~3MP analysis res
imageproc         = "0.27"  # Canny, Hough lines, Laplacian, integral images, stats
                            # (enable its rayon feature)
palette           = "0.7"   # RGB↔Lab/xyY; add ~10-line McCamy approximation for CCT
rayon             = "1.12"  # par_chunks over pixel rows for histograms/masks
```

Optional, only when the need actually arrives:
- `libheif-rs` 3.0 + `brew install libheif` — iPhone HEIC (we shoot RAF; only
  needed if phone snaps/reference HEICs enter the pipeline; `sips` is a
  zero-dep macOS fallback).
- `ort` 2.0-rc (CoreML EP works on macOS) — off-the-shelf ONNX aesthetic/IQA
  model later; `candle` 0.11 (Metal) if zero C deps matters more than ONNX.

Rejected: **opencv crate** (macOS build pain — libclang fights, bindgen, an
"install OpenCV first" distribution story — for nothing our list needs);
**libvips** (server thumbnail throughput, wrong shape, incomplete bindings,
not thread-safe wrapper); **zune-image umbrella** (we get zune-jpeg's speed
through `image` already); **rawloader** (no CR3, no demosaic, superseded by
rawler); **zenraw** (AGPL/commercial, 0.2.0, too young — but its
darktable-cli-fallback idea is worth remembering); **kamadak-exif** (no RAF,
22 months quiet; nom-exif strictly better for us).

## 2. Pipeline architecture

```
RAF ─ rawler::full_image() ──► embedded preview JPEG (~13MP, camera-processed)
 │                                 │ orientation-correct (EXIF)
 │                                 ▼
 │                          fast_image_resize → ~3MP analysis buffer
 │                                 │
 │                                 ├─ rayon: luma/RGB histograms, brightness,
 │                                 │         highlight/shadow %, WB (xyY→McCamy CCT),
 │                                 │         warmth-in-mids vs whites-neutral split
 │                                 ├─ imageproc: Canny → Hough → dominant vertical
 │                                 │         tilt (plumb check), keystone convergence
 │                                 └─ sharpness: Laplacian variance
 │                                    (on preview or green CFA plane — NOT on
 │                                     bilinear-demosaiced X-Trans output)
 │
 └─ rawler::decode_file() ──► raw CFA values → TRUE clipping detection
                              (the preview's tone curve lies about headroom;
                               window-blowout truth lives here)

JPEG/PNG references ─ image crate ──► same analysis path, minus raw truth
```

Why preview-first for analysis: no develop step, and the camera-processed
preview is close to what an editor/portal actually shows. When developed RGB
is needed (the editing pipeline), rawler 0.8 has both `XTransBilinearDemosaic`
(the `RawDevelop` default dispatch in `imgop/develop.rs`) and
`XTransMarkesteijnDemosaic` (darktable/RawTherapee's high-quality 3-pass) in
`imgop::sensor::xtrans` — full-quality native development, no LibRaw needed.
Fallbacks anyway: `rsraw` (LibRaw binding, Mar 2026) over `brew install
libraw`; or `sips`/CoreImage — macOS Tahoe 26.2+ decodes compressed +
lossless-compressed RAF natively (older macOS: uncompressed only — don't
rely on it).

## 3. EXIF strategy — two tiers

**Tier 1, pure Rust (nom-exif), always available:** DateTimeOriginal +
SubSecTimeOriginal, ExposureBiasValue (0x9204), Flash fired (0x9209 bit 0),
ISO, shutter, aperture, focal length, lens, orientation. RAF is an explicitly
supported nom-exif format. This tier is sufficient for bracket grouping and
most recipe checks.

**Tier 2, Fuji makernotes — `exiftool -j` shell-out** (no pure-Rust crate
walks the Fuji makernote pseudo-IFD; hand-rolling the parser is feasible —
TIFF IFD behind an 8-byte `FUJIFILM` prefix, offsets relative to makernote
start — but not v1 work). Verified tag table (exiv2/ExifTool Fujifilm.pm):

| Tag | Meaning | Recipe check |
|---|---|---|
| 0x1402 DynamicRangeSetting | DR100/200/400 | must be DR100 |
| 0x1050 ShutterType | mechanical/electronic | ES on brackets, MS on flash |
| 0x1100 AutoBracketing | bracket mode on/off | C1 stacks |
| 0x1101 SequenceNumber | in-burst index (0 = single) | robust stack grouping |
| 0x1103 DriveSetting | drive mode | continuous bracket |
| 0x1002 WhiteBalance | WB mode | fixed K, never AWB |
| 0x1010/0x1011 FlashMode/Strength | flash config | TTL pop frame |

## 4. Station grouping (bracket stack + flash pop)

No off-the-shelf Rust crate; ~50 lines. Prior art: HDRMerge's timestamp
auto-grouping, Lightroom/digiKam time-gap stacking.

1. Sort by (DateTimeOriginal, SubSec).
2. Cluster at time gaps > ~2 s (one-press burst gaps are ≪ 1–2 s; stations
   are tens of seconds apart).
3. A 5-frame cluster whose bias values form the symmetric ladder
   {0, −1, +1, −2, +2} → bracket stack. Match the *set*, not the order.
4. Temporally adjacent single frame with Flash-fired bit → that station's
   flash pop.
5. When exiftool is present, validate with SequenceNumber + AutoBracketing.

Recipe-compliance checks this unlocks per station, from metadata alone:
6 frames present; ladder complete; middle frame 1/30 s or slower (⇒ −2 at
1/125+); brackets f/6.4 ISO 125 ES; pop f/6.4 ISO 320 1/125 MS flash-fired;
DR100; WB mode fixed; focal length within the 16 mm-default rule.

## 5. Metric ↔ rulebook map (RESEARCH-shoot-craft.md)

| Rule (validated there) | Metric here |
|---|---|
| Level + plumb (§2, table stakes) | Hough dominant-vertical tilt°, keystone convergence |
| Window exposure / flambient | raw-CFA clip % in window regions; preview blown % |
| Bright/airy HDR default, whites stay white (A3.4) | histogram targets; CCT of highlight vs midtone regions |
| Color-temp discipline / kill mismatched fixtures | per-region CCT spread |
| 16 mm default, 14 mm floor (A8) | EXIF focal length |
| Stills ~120 cm use-plane height (A6) | keystone/tilt proxy + verticals |
| One look across the gallery (§5.1) | cross-image histogram/CCT distance |

## 6. Sources

dnglab SUPPORTED_CAMERAS.md (X-H2: "all" RAF variants) · rawler docs.rs
(Decoder::full_image/preview/thumbnail; imgop/develop.rs X-Trans dispatch) ·
nom-exif GitHub (format list) · exiv2 tags-fujifilm + ExifTool FujiFilm.pm
(makernote tags) · image CHANGES.md (zune-jpeg integration, 16-bit) ·
imageproc docs.rs (hough/edges/filter/integral_image/stats) ·
fast_image_resize arm64 benchmarks · opencv-rust issues #680/#428/#575
(macOS libclang) · HDRMerge (grouping prior art) · Apple/Fuji-X-Forum threads
(Tahoe 26.2 RAF support).
