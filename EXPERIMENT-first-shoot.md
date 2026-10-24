# Experiment log — first shoot through the Rust pipeline (2026-08-31)

Input: 117 RAFs in `RAW/` (X-H2, Sigma 10-18/2.8, shot 2026-08-30 evening
through 01:36). Grouping found 45 stations; **10 complete 5-bracket+pop
stations**, 5 pop-less stacks, ~28 flash-only singles, 2 flash misfires.

## Result

All 10 complete stations ran the full ladder successfully. Outputs in
`out/stNN/`: `ambient_toned.jpg` (fusion only), `pop_toned.jpg` (flash only),
`final.jpg` (blend of both). Verdict at web size: **final.jpg clears the
cheap-editor bar** on the frames spot-checked (st01 exterior, st05 hallway,
st12 living, st15 living, st36 bedroom) — white walls stay white, wood stays
warm, no halos/ghosting/HDR-grunge.

Pipeline: `retool fuse` (rawler develop → 16-bit TIFF → enfuse) →
`retool dev` (pop) → `retool blend` (Cool-stack luminosity blend, linear
light, ambient exposure-matched, luma weight 0.35) → `retool tone`
(linear exposure targeting median 0.30 + Reinhard shoulder above 0.7).
~60–90 s per station unoptimized, sequential.

## What the shoot data taught

- **Night ambient stacks contribute almost nothing** (fused median 0.002
  linear at ISO 125). The pop carries night stations; ambient donates lamp
  mood via the luma mix. Recipe works, but is effectively flash-first after
  dark.
- **This card cannot test the window pull** — every station is dusk/night,
  windows are dark. Need a daytime interior set for that stage.
- Reproduce: `python3 <scratchpad>/run_all_stations.py` (grouping in
  `group_stations.py`; both to be ported into retool as `retool ingest`).

## Recipe deviations found (recipe doc vs. actual EXIF)

1. All 38 pops at **1/60**, recipe says 1/125. Decide: recipe wrong or habit?
2. Bracket ISO drifted 200–1600 on 8/15 stacks (recipe: 125). Recipe tension:
   fixed f/6.4 + "middle ≤ 1/30" forces ISO up OR long shutters after dark.
3. St01 metered frame 1/60 — faster than the "1/30 or slower" rule (harmless
   outdoors, no LED risk).
4. C1 writes sRGB, C2 writes Adobe RGB (`DSCF*` vs `_DSF*` filenames).
   Invisible to raw develop; undocumented bank difference worth aligning.
5. Held everywhere: ±2EV ladders exact, ES brackets / MS pops, WB Kelvin
   5000, DR100, f/6.4, pop ISO 320.

## Realm v1 validation (same day, later session)

`retool realm build photocorpus realm.json` (180 refs, 10 types, k=9,
~0.9 s/img on Metal) then `retool realm score realm.json out`:

- **Room-typing: 10/10 correct** (exterior→porch, bedroom→bedroom, …) —
  the embedding's implicit conditioning works.
- **Blend > ambient in every station** — the realm prefers the flambient
  finals over dark ambient-only versions; sensitivity is real.
- **Caught a real defect spot-checks missed**: st13 (fit 3/100) has severe
  fusion GHOSTING — sheer curtains moved between bracket frames — plus a
  cluttered table. First confirmed catch by the culling tool. Mitigation
  to design: motion detection between brackets, or single-frame fallback.
- **Coverage gap confirmed as predicted**: st05 (clean hallway, fit 13)
  has no hallway/entry reference class — scored against living rooms.
  Fix: add an entry/hallway type to photocorpus, or tune the
  "no similar reference" threshold (currently 1.5×p99 of LOO 1-NN — too
  lenient; st05 should have been flagged rather than scored low).
- Tightness profile: bedrooms/kitchens tightest (0.36) — most
  conventionalized compositions; aerials/patios loosest (0.47).

## Known gaps, in build order

1. **Verticals correction** — visible lean in most frames; the #1
   post-outsourcing fix per research (stage e: hough tilt → warp).
2. Port grouping + compliance checks from Python scratch scripts into
   `retool ingest` (note: python check had an inverted middle-shutter
   comparison; port the corrected rule: metered frame duration ≥ 1/30 s).
3. TV blackout, fan-shadow, clutter — staging/edit extras; TV-on is the
   most visible tell in st12/st15.
4. Window pull (stage c) — blocked on daytime material.
5. Native Mertens fusion to replace enfuse (IPOL spec, ~500 LOC); also
   removes the Hugin-app dependency and its broken-symlink/Gatekeeper
   quirks (enfuse binary really lives in HuginStitchProject.app).
6. Rayon-parallelize develop/tone (currently single-threaded, ~5 s/40MP).

## Update 2026-09-01 — geometry stage built, gaps re-ranked

- **Gap #1 (verticals) closed**, and it pulled lens correction with it: the
  RAF's `GeometricDistortionParams` (first value = half-diagonal / 9, then 9
  radii as fractions of the half-diagonal and 9 distortion percentages) is
  written for the Sigma 10-18 too, −9.6% barrel at the corner at 10 mm. Uncorrected,
  each off-center vertical bows and votes for two vanishing points; the
  verticals fit was 41% consensus and under-rotated. Undistorting first (same
  table, one resampling) took consensus to 65% and made the stage idempotent
  (second pass 0.13°, third pass "already plumb") on `_DSF0725` (hallway).
  Stage cost ≈ 4 s per 40 MP frame, single-threaded.
- `card` (report-only gate) and `deliver` (ordered gallery + coverage) added
  for the process in `PROCESS-media-machine.md`. The current card gates as
  19 complete / 35 partial / 16 with deviations → `REVIEW` (the 1/60 pops).
- Ghosting is now the top pipeline gap; window pull remains blocked on
  daytime material.
