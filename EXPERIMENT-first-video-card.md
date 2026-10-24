# Experiment — the first real video card (2026-09-02, dusk test house)

The card from the first shoot on the video recipe, run through the tools
built the same day (PLAN-video-cut.md, steps 0–6). One card carried both
stills and video: 90 RAF (15 stations × 6), 24 MOV, 8.7 GB. Shot 18:41 to
20:07, in fading light and then darkness. No phone verticals yet. Job
folder: `jobs/2026-09-02_test-house/`.

## What ran

`retool run card out` end to end: 15 finals, scorecard, then clips (80 s),
plan, cut (147 s): master branded + unbranded, three reels. Then three
re-plans as the planner learned the card (below). `retool card` on the
card: 15 complete stations, CLEAN.

## What the card taught the tool (all fixed today)

| Finding | Fix |
|---|---|
| A station's second clip was 6–8 s, over the 5 s "detail" cutoff; and both clips at a hero station panned (wide first, then a tighter angle), no 18-50 static at all | Kind by motion, not length: the clip that pans is the station clip, an all-still one is the detail. The **first** panning clip is the wide; a second panning clip becomes the second angle when no same-type station follows |
| The walk-up and close were shot in the dark; the walk-up's last frame matched nothing (d 0.67) and was typed `entry` | The first clip of the pass whose **first** frame is exterior is the walk-up, whatever its landing looks like (origin typed too, at the gate as well) |
| The entry landed on living B (st07), which the living unit had absorbed as its second angle, and the foyer (typed kitchen, fit 13) had been collapsed as a duplicate kitchen, so two moves fell out of the sequence | A move lands on the station that stands for its destination: the same-type neighbour that kept it |
| Gimbal walks were 8–17 s of motion (16–35 s at 2×); the planner took a move's *first* motion segment as "the move" and sliced the middle of a walk, or gave a hallway six bars | A move spans all its motion; a walk longer than three bars keeps its **landing** (in mid-walk, out on the tail still), min two bars, so the runtime cap shortens walks before dropping rooms |
| Interiors 1.5–2 stops under: log frame means 63–79 (8-bit, mid grey is 102), graded means 45–62 against finals at 140–150; auto ISO at f/4 hit its ceiling at dusk | The exposure nudge the plan deferred to this card: the planner measures each camera row's log frame and writes a `gain` column (stops, −1 … +2); the renderer lifts the log signal before the LUT (`lutyuv`). Every interior row asked for +2.4 to +2.8 and got the +2.0 cap; the bath +1.0 |
| The patio (st15, table and umbrella) typed as `dining_room` at fit 13, so it played as an interior support room; the foyer (st03) typed `kitchen` at 13 (no hallway class in the corpus) | Corrected st15 to `patio` by hand in `out/scorecard.md` and re-planned: the human surface works. The foyer stays: the collapse rule dropped it, which is the right outcome for the wrong reason |
| Paradise's stamp `end_s` was past the file | Corrected in MUSIC.md; the planner clamps |

Unchanged and confirmed: timestamps placed all 19 station clips on their
stations; pan boundaries were clean (3 s hold, 3–4 s pan, 3–4 s hold); the
grid, the splice and the runtime behaved as on the synthetic card; a hand
edit of the scorecard type flowed through `plan`.

## What the card taught the recipe (for the operator)

1. **Daylight.** Exteriors, the walk-up and the close are black; the
   interiors are lamp-lit and warm under WB 5000 K. The plan said daylight
   for a reason: the window pull and the LUT's window handling are untested.
2. **Codec: H.265 4:2:2 10-bit, not H.264.** The card is H.264 8-bit; F-Log2
   in 8 bits bands in flat walls, and the +2-stop nudge makes it worse.
3. **Expose log properly.** f/4 with auto ISO under-exposed 1.5–2 stops at
   dusk. Raise the auto-ISO ceiling, or open to f/2.8 for video, or expose
   with zebras at 70 % on a white wall. The nudge is a safety net, not the
   exposure.
4. **The detail static.** Shoot it: 18-50, 3 s, no pan, in hero rooms.
   Today's second clips were a second wide with a pan; the planner now uses
   them as second angles, which is fine, but the unit is wide → pan →
   detail → angle.
5. **Walks of ~4 s between the two stills.** Today's were 8–17 s. At 2× a
   4 s walk is three bars; longer walks are cut into.
6. **The pass is six clips.** Today: walk-up, entry, hallway → bedroom,
   living → sunroom door, close. Missing: the primary look-back and a true
   "out to the grounds" (today's ended inside at the door). Shoot the
   walk-up and the close in light.
7. **A clip at every station.** The patio (st14) got none.
8. **Phone verticals.** Added after the first pass (below). ProRes 422 at
   4K 60p is 1 GB a clip; H.265 would do. Keep the grip out of the frame.
9. **Fill `job.md` before the render**: the cards printed the template
   placeholders.

## The phone clips, added the same night

15 clips from the Blackmagic Camera app on a 15 Pro Max: ProRes 422, 4K
59.94p, 10-bit 4:2:2, Apple Log (Rec.2020 primaries), stored landscape with
a 90° rotation tag, 4–11 s each, 13 GB. The phone's CreateDate is UTC, four
hours ahead of the camera. One clip is from 15:01, a test from elsewhere.

| Finding | Fix |
|---|---|
| No Apple Log cube on disk: phone rows would render flat | `luts/AppleLog_to_Rec709_approx.cube`, generated from Apple's published Apple Log curve (inverted), Rec.2020 → 709, a soft shoulder from 0.6, sRGB. The renderer and the room matcher prefer `luts/AppleLogToRec709.cube` the moment Apple's file is there |
| Picture matching alone: 11 of 15 confident (d ≤ 0.4); the tight feature verticals matched at 0.7–0.85 and two were wrong by time; the 15:01 test clip matched a living room | The phone clock runs on a constant offset from the camera's: the confident matches calibrate it (median of phone time − station start), and a doubtful match (d > 0.5 or margin < 0.05) falls back to the station whose window holds the clip's camera time; a clip outside the shoot is `stray`; the first station's window opens an hour early, since the facade verticals were shot 40 s before the first bracket. Replaces the station-order tie-break. The measured offset was 232 min, not the 240 the UTC stamp suggests: the camera clock is minutes off, which is why the offset is calibrated from the pictures rather than assumed |
| Three exterior verticals stacked at the open of a reel; a reel led with the foyer (typed kitchen, fit 13) | The facade opens, other exteriors stay out of the body; reel leads are hero rooms that have material, best fit first |
| The grip is in shot at the bottom of most verticals | A shoot note: hold the phone higher or tilt so the handle clears the frame; the tool does not crop native verticals |

The master took two phone verticals as hero-room details (kitchen, living),
centre-cropped to 16:9.

## The second pass, 2026-09-03 (tool changes from the debrief, recut on the same footage)

- Pops: every pop at a station is developed and composited by luma before
  the flambient blend (`composite_pops`); the gate shows the count.
- Tone target 0.30 → 0.38 linear (a third of a stop); the finals read dark.
- The vertical warp samples bicubic (Catmull-Rom) instead of bilinear; the
  finals read soft at 100 %. `retool sharp` measures blur but ranked the
  operator's four soft frames wrong, so it is not a scorecard flag.
- **The softness was the demosaic.** 1:1 crops of every final were mush, and
  rawler's developer turns out to demosaic X-Trans with a bilinear kernel.
  The crate ships a Markesteijn pass next to it; `develop_to_tiff` now runs
  the developer's rescale alone, demosaics the plane with Markesteijn, and
  hands the three channels back for white balance, calibration, crop and
  gamma. 7 s a frame; the painting in st07 resolves where it was a smear.
  Every final on every card so far was soft for this reason; the four you
  called out were the ones where it showed. Version 3 of the card renders
  with it.
- Music: the three vocal tracks left the rotation for the four CORE tracks
  that pass the screener. `retool music check` scores the stamped grid
  against the onsets and renders a click track: Paradise's grid was 282 ms
  late with the downbeat three beats off, which is the "not on the beat" of
  the first cut; A Mirage and es_xp had their accents corrected.
- Reels take the other tracks of the rotation (later sections of the
  master's track when the rotation is short).
- The planner follows a station clip's own order: hold then pan, or pan
  then hold; `hold_anchor` picks the still to cut on.
- `stabilise: on` in the cutlist header runs ffmpeg's `deshake` on gimbal
  rows after the scale (vidstab's transform cannot read its own detect
  file on this build, at any resolution).
- Gimbal clips are found by motion wherever they fall, so the pass can be
  shot first, in daylight.

## Open

- The warm cast: ambient-only video under tungsten at 5000 K. A per-clip
  grey-world white balance before the LUT is the next candidate nudge;
  daylight first.
- The foyer/hallway class in the realm (README known gap 3) now costs a
  move's landing; a `hallway` folder in the corpus would fix both.
- Apple's Log cube is still not on disk (`luts/AppleLogToRec709.cube`). The
  approximation stands in; the shadows look a touch lifted, which may be a
  full-range file decoded as limited. Apple's cube settles it.
