# First drone card — does the bracket pipeline take DJI DNGs?

2026-09-29. Mavic Air 2 (FC3170), 16 DNG + JPEG pairs from a ground-level
test at the operator's house: one single frame, then three AEB sets of five
(indoor 0004–0008, backyard 0009–0013, patio 0014–0018). Card facts and the
manual-exposure mistake are in `RECIPE.md` §5a (correction 2026-09-29).

## Result

`retool fuse <out> DJI_00NN.DNG ×5` ran with **no code change**: rawler 0.8
decodes the DNG as ordinary RGGB Bayer and `develop_to_tiff` takes its
non-X-Trans branch; enfuse fuses the five 12 MP TIFFs. About 10–13 s a set.

Looked at (`sips -Z 1400` and 100 % crops of fine edges):

- **Range.** The backyard fuse holds the fence, AC unit and shade that the
  camera's own JPEG crushes, and the sky gaps do not clip.
- **No ghosting** at 100 % (mortar, roof edge, chair weave) — the burst is
  under a second and the frames register without alignment. A hovering
  drone at height in wind is the case still untested.
- **Lens correction missing.** The DJI JPEG is wider-corrected and cropped;
  our develop ignores the DNG's `OpcodeList3` (GainMap, WarpRectilinear), so
  barrel distortion and the dark corners stay. `level` gets its lens from
  the RAF and has no DJI path.
- **Flat and cool** before tone: shade at 5000 K, untoned fusion. Expected.
- Coloured specks at bright points through leaves (demosaic / CA). Minor.
- The indoor set is black at every step: shot in manual 1/120, not a
  pipeline result.

## Next

1. Apply WarpRectilinear + GainMap from the DNG before fusion (or in
   `level` for a DNG reference) — the one code gap for aerial stills.
2. A real flight card with photo exposure on Auto: A1–A3 at height, V1.
3. `card` reading `DJI_*.DNG` sets by timestamp and bracket bias.
