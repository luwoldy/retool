# Plan — `retool cut`: the video, assembled from the card

Decided 2026-09-02 with the operator, on top of `RESEARCH-video-edit.md`.
This is the build plan. Each step has a verify line; a step is done when
its verify line is true, not when its code compiles.

## 1. Decisions

| # | Decision | Choice | Why |
|---|---|---|---|
| D1 | The one look | Fuji's own `XH2_FLog2_FGamut_to_WDR_BT.709` cube, plus a small saturation lift (`eq=saturation=1.08`) | Bright-and-airy target; whites stay white; holds windows. ETERNA reserved for a future "signature" variant |
| D2 | Timing | **Every clip conformed 2×**: 60p → 30p at half speed | One motion feel, natural 180°-equivalent blur, one rule on set (pan ¾ frame in ~3 s live) |
| D3 | Delivery | 1080p30 H.264, CRF 18 capped at 16 Mbps, AAC −14 LUFS, ≤ 120 s, branded + unbranded | MLS/Showcase-safe; portals re-encode anyway. Note: the six past deliveries were 24p at 55–60 Mbps; 24p is a one-line change (`setpts=2.5*PTS`, `fps=24`) if the cadence is missed |
| D4 | Music | From `~/Documents/ShowReady/Music/ROTATION` (three screened YouTube Audio Library tracks today; Epidemic set aside), rotated by job, overridable per job. **The track's stamped grid is the cut grid**: accent hit = grid origin, bar = 240/bpm (~2.4 s at 100 BPM), 16-beat phrases; establish rides the pre-accent shelf, the cut into the first interior lands on the accent, every cut sits on a beat, the composed ending is spliced in at a bar line so the last real bar lands on the close. Loudness −14 LUFS | The operator's six music essentials, in `MUSIC.md`. Licensing stays the operator's call; the tool only reads the folder and the per-track stamp table |
| D5 | Cards | Open: address + city over the Establish shot. End, branded: agent name, brokerage, phone from `job.md`. End, unbranded: address only. Plain type on a colour block, no logo art in v1 | Canopy allows branded tours; the unbranded cut exists for Showcase/portal rules |
| D6 | Cuts | Straight cuts only; 12-frame fade from black at the open and to black at the close | The MLS-wide idiom; dissolves read as amateur |
| D7 | Video exposure | Movie mode 4K 60p, F-Log2, 1/125, f/4, ISO auto (F-Log2 floor ISO 1250), WB 5000 K, for stations and gimbal alike | Auto ISO carries the walk across rooms; f/4 keeps wides deep; one bank |
| D8 | Details in video | The 3 s static on the 18-50 is the third shot of a hero room's unit (wide → pan → detail) and the buffer between two similar angles | Grammar: never wide straight to detail; critique: break up similar angles with a countertop detail |
| D9 | The human surface | `cutlist.md`, one row per clip; edit and re-render with `retool cut --from` | Every other decision the tool makes is visible and reversible there |
| D10 | Reels | Three 9:16 cuts of 15–25 s, part of the package. Accent is frame zero; details one bar each; the vertical pool is drawn first, cropped gimbal moves and pans fill the rest; hard out on a phrase boundary | The old package's social cuts lived on detail shots; the crop-only version would be the weak tier |
| D11 | Vertical pass | Verticals are shot **on the phone**, portrait, 4 s with a small tilt or slide: two per hero room, one in every other room with a feature, one of the facade. 10–12 clips, ~8 min. Blackmagic Camera app: 4K 60p, 1/125, WB 5000 K, Apple Log on an iPhone 15 Pro or newer (else standard profile, HDR off). Phone clips arrive in their own folder; the tool matches each to its room by picture, since the phone's clock is not the camera's. Decided 2026-09-03 over an L-bracket, which needs an Arca clamp on the fluid head and a re-rig per clip | A 9:16 crop of a horizontal wide keeps a third of it; practitioners shoot listing video on phones outright and vertical as its own thing. Gimbal doorway moves and tripod pans survive the crop and fill the rest |

### Corrections, 2026-09-05 (`RESEARCH-resolve-handoff.md`)

- **D1 superseded:** no LUT. The cut is colour-managed in Resolve (DaVinci
  YRGB Color Managed, DWG/Intermediate, Rec.709 Gamma 2.4, per-clip input
  spaces `Fujifilm F-Log2` and `Apple Log`). The ffmpeg fallback keeps the
  WDR cube and now looks slightly different.
- **D3 amended:** the deliverables are rendered by Resolve from the project
  `tools/resolve_build.py` builds; `retool cut`'s ffmpeg render is the
  fallback for a machine without Resolve.
- **D9 extended:** the cutlist is also what the Resolve project is built
  from; every row lands as a named, coloured clip on the grid.
- **D10 amended:** one song per job; the reels ride the master's track with
  the accent at frame zero (the same section three times). *2026-09-06:*
  each reel has a purpose (`mls`, `details`, `feature`) and cuts in the
  social blocks, 2-2-2-2 | 2-2-4 then 4-2-2 | 8 (RESEARCH-video-edit.md §9,
  MUSIC.md "Social blocks").
- **Step 2 rules 2 and 6 superseded by the beat/phrase grammar** (MUSIC.md
  "The grammar"): shots are fours or eights, a hero room is one 8-4-4
  phrase (its pan or hold, a medium, a detail), a move closes a 4-4-8
  arrival phrase, support statics and reserve verticals complete phrases.
  Over the cap the planner drops whole phrases, arrivals first from the
  end, never the first. The cutlist header's `patterns:` line shows the
  tiling; the 2026-09-02 card tiles as seven phrases, 87 s.
- **The master builds the intrigue (2026-09-06),** RESEARCH-video-edit.md §9:
  D4's 12 s shelf becomes a cold open when the intro has a phrase in it,
  tease · tease · walk-up landing on the accent (S 4-4-8), the address card
  over the walk-up; hero rooms stay 8-4-4 in listing order; moves close
  4-4-8 arrivals on the last hero's spare teases only; after every second
  hero a 4-4-4-4 montage of spare verticals, 18-50 details and one glimpse
  each of the support rooms over a fit floor of 30; support rooms under the
  floor are not shown and say so in the skipped table; grounds are a glimpse
  in the last montage; no padding to fill time. Over the cap: montages go
  first, then arrivals, then the last phrase.

## 2. What the card gives the tool (already in the runbook)

- **Per station, after C2:** one MOV: 3 s still, ¾-frame pan in ~3 s, 2 s still. In hero rooms an 18-50 detail still (C2 once). Then the verticals on the phone: two 4 s portrait clips in a hero room, one in any other room with a feature, none in a bare room, shot in station order.
- **Gimbal pass, in order, one MOV each, 2 s still at both ends:** walk-up · entry · each hallway/stair · primary look-back · out to grounds · close.
- Stills on CFexpress, video on SD, one body clock. Phone clips on the phone, its own clock; exported into the job's `phone/` folder.

## 3. Build order

Each step is a subcommand or a function that can be run alone (repo rule).
Rust orchestrates; ffmpeg/ffprobe/exiftool do the media work, like enfuse
does for fusion. Estimated total: ~700 lines of Rust across
`src/video.rs`, three or four sessions, most of it steps 2 and 4.

### Step 0 — a test card without a shoot

Synthesise clips from the finals already on disk so every step below can
be verified today:

- Tripod clip: `ffmpeg -loop 1 -i st05.jpg -vf "zoompan=..."` producing
  3 s hold, 3 s lateral pan, 2 s hold at 3840×2160 60p, 8 s total, encoded
  H.264 with a fake CreateDate 40 s after the station's pop.
- Phone vertical: a 4 s portrait clip (2160×3840, 60p) synthesised from a
  centre crop of `st02_d1.jpg` with a slow tilt, in a `phone/` folder,
  timestamped on a different clock (an hour off) to prove the picture match.
- Gimbal clip: a 6 s push synthesised as a slow zoom from a living-room
  final into its centre, with 2 s stills at both ends, timestamped after
  the last station.
- Music: any file from `ROTATION`.

**Verify:** the synthetic card plays; `exiftool` reads CreateDate and
VideoFrameRate from the synthesised MOVs (if it does not, the fixture is
wrong, not the tool).

*Built 2026-09-02:* `test/synth-card.sh` writes `test/synth/` as a job folder
(13 stations across seven room types from the finals in `out/`, `test/` and
`photocorpus/`, hand-set type + fit in its scorecard; 22 camera clips, 12
phone clips, `truth.tsv`). Two departures from the sketch: the camera clips
are encoded in F-Log2 / F-Gamut (an inverse cube the script generates from
the published F-Log2 curve) so the WDR LUT round-trips them like real
footage, and `.meta/stNN.json` carries `first_at` and `pop_at` so the
ingest can place clips in time without the RAFs. Verified: every clip
decodes; exiftool reads CreateDate (camera wall clock; phone an hour
behind), Duration, VideoFrameRate 60, dimensions, Rotation; a WDR-graded
frame looks like the photo, the log frame is flat.

### Step 1 — `retool clips <video_dir> <out_dir>`: ingest, analyse, type

1. `exiftool -json` over the MOVs in `video/` and `phone/`: CreateDate,
   Duration, VideoFrameRate, ImageWidth/Height, Rotation. Fail loudly on
   any clip that is not 60p. Phone clips are the vertical pool by folder;
   their rotation tag settles orientation.
2. Assign each clip to a station by timestamp: between this station's pop
   and the next station's first bracket. Clips after the last station are
   the gimbal pass, in time order. Station meta comes from `out/.meta`.
3. Motion: `ffmpeg -i clip -vf "scale=480:-2,vidstabdetect=result=…"`;
   parse per-frame dx, dy, rotation, zoom; smooth over 10 frames; label runs
   as `still` (|motion| under threshold), `pan` (lateral dominant), `push`
   (zoom dominant). Emit segment boundaries in seconds.
4. Type: station clips inherit room + fit from the scorecard. Phone clips:
   embed the middle frame with the realm's `Embedder` and take the nearest
   station final by cosine as the room (the phone's clock is ignored);
   sequence order breaks ties. Gimbal clips: extract first and last frames,
   embed, nearest station final → origin and destination room; sequence
   position + motion kind resolves walk-up, entry, hallway, look-back,
   grounds, close.
5. Write `out/video/clips.tsv`: file, source (fuji/phone), station, kind,
   orientation, segments, origin, destination, confidence.

**Music stamp:** read the per-track table in `MUSIC.md` (bpm, accent_s,
phrases_after_accent, ending_start_s, end_s). A track without a complete
stamp is skipped with a warning; the planner needs the grid. **Built
2026-09-02:** `retool music stamp <dir|track>` proposes the stamp from the
audio (onset autocorrelation for tempo, refined over eight multiples; the
largest sustained level step in 8–22 s for the accent; the last strong beat
for the last bar; a fade only when the level falls while beats continue)
and prints the verdict per track. Calibrated on the seven CORE tracks
against their waveforms.

**Verify:** on the synthetic card every clip is assigned to the right
station, the hold/pan/hold boundaries are within 0.3 s of the truth, the
gimbal clip's destination is the room it was synthesised from, and the
synthesised phone clip is matched to its room despite the hour-off clock.
`clips` runs in under a minute for 20 clips.

*Built 2026-09-02:* `src/video.rs`, `retool clips <video_dir> <out_dir>`
(phone clips from the sibling `phone/`). Three departures from the sketch:
the motion estimate is ours (vid.stab's detect file is binary and its own
transform filter cannot read it back on this ffmpeg; four quadrant block
matches on 320-px grey frames at 10 fps give translation, and opposite
quadrants moving apart give push/pull); a station is "the last station
whose brackets started before the clip"; and the gimbal pass starts at the
first clip after the last station that pushes or pulls (the walk-up), so a
re-take at the last station stays a station clip. Motion runs four ffmpeg
decodes in parallel. On the synthetic card: 22/22 camera clips on the right
station with the right kind and role, gimbal destinations 5/5, pan
boundaries within 0.2 s, phone clips 11/12 (the miss is a 0.40 vs 0.41
tie between the two angles of one kitchen; the row shows both), 34 clips
in 67 s.

### Step 2 — the planner: `cutlist.md`

Input: `clips.tsv`, scorecard, `job.md` (address, agent). Output:
`out/video/cutlist.md`, a pipe table the human can edit:

```
| # | role      | clip            | in   | out  | speed | room        | why |
|---|-----------|-----------------|------|------|-------|-------------|-----|
| 1 | establish | gimbal_01.MOV   | 1.0  | 9.0  | 2x    | exterior    | walk-up |
| 2 | wide      | st04_clip.MOV   | 0.4  | 2.8  | 2x    | living_room | hero, fit 74 |
| 3 | pan       | st04_clip.MOV   | 2.8  | 7.6  | 2x    | living_room | pan to windows |
…
```

The grid comes from the track (`MUSIC.md`): `bar = 240/bpm`, phrase = 4
bars, origin = the accent. Every cut lands on a beat; wides take 2 bars,
details 1 bar, pans 2 or 3 bars, moves are fitted to a bar multiple by
choosing in/out inside their bookend stills. At 100 BPM that is the
2.4 / 4.8 s grid; at other tempos the same bar counts.

Rules, in this order:

1. Establish rides the pre-accent shelf: the track starts at `accent_s −
   shelf`, `shelf = min(12 s, accent_s)`; walk-up if present, else the
   front-exterior station's still segment, exactly `shelf` long on screen
   (`shelf:` in the cutlist header changes the 12 s). Address card over it. The cut into the first interior lands on the
   accent.
2. Rooms in station order, on the grid. Hero = living, kitchen, primary
   bedroom, primary bath, plus any station with fit ≥ 60. Hero unit: wide
   (2 bars) → pan (2–3 bars, whole pan plus slack from the holds) → detail
   (1 bar; a vertical detail is centre-cropped to 16:9 for the master, or
   skipped if it does not crop well) → second angle if a same-type station
   follows (1 bar). Support rooms: the still segment, 1 bar. Room units
   are packed into whole phrases; a phrase never ends mid-room.
3. Duplicates: consecutive same-type stations beyond two collapse to the
   two with the best fit; dropped ones are listed at the bottom of the
   cutlist with `skipped` so the human can revive them.
4. Gimbal clips are inserted before the wide of their destination room,
   whole, from first still to last still. Hallway clips go where the
   destination room's station follows a different room type.
5. Grounds: patio/yard/rear stations after the interior. Close: the close
   gimbal clip, else the last exterior still held 6 s. End card over it.
6. Runtime: 12 s + N phrases + the composed ending, in 60–110 s,
   hard cap 120 s, with N ≤ `phrases_after_accent`. Over the cap, shorten
   in this order: support statics out, second angles out, details out, pans
   to 2 bars. Never the hero wides or the close.
7. Music: track = `ROTATION` sorted, index = job number mod count, unless
   `music:` is set in the cutlist header. The track plays from `accent_s −
   12 s`; at the bar line after phrase N the audio splices to
   `ending_start_s` with a 100 ms crossfade on the bar; the film ends at
   `end_s`. The close (the
   pull-back or last exterior, held) occupies the last real bar. No fade
   out.

**Verify:** on the synthetic card the cutlist reproduces the film skeleton
in `RESEARCH-video-edit.md` §2, every cut time is a multiple of the bar
from the accent, the first interior cut lands 12 s in, the film ends at
`end_s`, runtime lands inside 60–110 s, and editing a row (say, swapping
the pan for the second angle) survives `--from`.

*Built 2026-09-02:* `src/cut.rs`, `retool plan <job_dir> <out_dir>`. The
cutlist gains an `at` column (film time where the row starts) so the grid
is visible; the renderer holds or trims each row to its slot, so a hand
edit cannot move a cut off the beat. Interpretations: "packed into whole
phrases" = the room sequence totals whole phrases (a hero unit of six or
seven bars straddles phrase lines by nature); a move first shortens to its
tightest bar fit around the move (the bookend stills give, never the move)
before rule 6 drops any room; the primary bedroom and bath are the first
of their type in station order; the job number is the jobs.tsv row with
the job.md address (job 0 when absent). On the synthetic card (Paradise,
95 BPM): the entry cut lands on the accent at 11.4 s (shelf = accent), all
18 cuts sit on bar lines, splice 92.2 s, end 102.4 s = `end_s`. **Tension
found:** at 95 BPM four hero units (wide 2 + pan 3 + detail 1 [+ angle 1])
plus three moves fill eight phrases on their own, so rule 6 drops every
support static (dining, second bedroom, second bath, patio, yard) before
it touches a pan; the §2 skeleton's support rooms survive only if the pan
fits two bars, which on set means a ¾-frame pan in ~2.4 s rather than
~3 s. Operator's call (the order in rule 6, or the pan on set); the
skipped rows are one edit away.

### Step 3 — the renderer: `retool cut`

Per row, one ffmpeg pass to an intermediate. Fuji rows take the WDR cube;
phone rows take Apple's Log-to-Rec709 LUT when the clip is Apple Log
(exiftool reports the colour profile), otherwise no LUT. Fetch Apple's
cube from Apple's "Apple Log Profile" developer page once and keep it next
to the Fuji cubes.

```
ffmpeg -ss IN -to OUT -i CLIP \
  -vf "setpts=2*PTS,fps=30,lut3d=file=XH2_FLog2_FGamut_to_WDR_BT.709_33grid_V.1.00.cube,eq=saturation=1.08,scale=1920:1080:flags=lanczos" \
  -an -c:v libx264 -preset slow -crf 18 -maxrate 16M -bufsize 32M -pix_fmt yuv420p seg_NN.mp4
```

Then the concat demuxer joins the segments; `drawtext` writes the address
card over segment 1 and the end card over the last (two renders: branded,
unbranded); `fade=in:0:12` at the open, none at the close. Music: two
`atrim`s (0 → splice bar, `ending_start_s` → `end_s`) joined with
`acrossfade=d=0.1`, then `loudnorm=I=-14:TP=-1.5`; `-movflags +faststart`.

Outputs: `out/video/mls_branded.mp4`, `out/video/mls_unbranded.mp4`, and
`render.log` with the exact commands.

**Verify:** `ffprobe` says 1920×1080, 30/1, duration = Σ rows ± 0.1 s
and equals the music's `end_s` after the splice; a frame from the rendered
file is not log-flat (mean luma of a white wall above 0.7); both cards are
legible at 1080p on a phone; the audio splice is inaudible on headphones;
total render under 3 minutes on this machine for a 90 s cut.

*Built 2026-09-02:* `retool cut <job_dir> <out_dir> [cutlist.md]` in
`src/cut.rs`: one segment per row, held or trimmed to its slot on the grid,
three ffmpeg processes at a time, scaled to 1080p before the LUT (what
keeps the render in budget); close rows rendered twice for the two cards;
concat demuxer; music from `accent_s − shelf` with the ending spliced at
the bar line over a 100 ms crossfade, two-pass `loudnorm`, AAC 192k,
`+faststart`. Cards: Helvetica lower-left on a translucent box, in a
second after the open and out before the accent; the end card in half a
second into the close, held to the end. No fade to black at the close (D6
says one, this step and MUSIC.md say none; the composed ending wins).
Phone rows take `luts/AppleLogToRec709.cube` when it is on disk, else no
LUT. On the synthetic card: 1920×1080, 30/1, 3043 frames = the cutlist's
end to the frame, audio ends at `end_s`, −14.1 LUFS / −1.6 dBTP, a kitchen
cabinet region reads Y 0.70, both cards legible, 88 s for a 101 s cut;
swapping row 4's pan for the second angle in a copy and rendering from it
kept the grid and used the new clip. Found on the way: Paradise's stamp
ran past the file (corrected in MUSIC.md; the planner now clamps). Not
verified: the splice by ear. *2026-09-05:* superseded as the delivery renderer
by the Resolve project (`RESEARCH-resolve-handoff.md`); this ffmpeg path
stays as the fallback.

### Step 4 — the gate learns video

`retool card` reports, per station, whether a clip exists, and lists the
gimbal clips found against the expected set. A station without a clip is
`PART` for video; a missing walk-up or close is a `REVIEW`. This runs at
the door like the stills gate.

**Verify:** removing one synthetic station clip flips that station to
`PART`; removing the close produces `REVIEW: no close`.

*Built 2026-09-02:* `retool card` runs the video gate when a `video/`
folder sits next to the card; `retool gate <video_dir> <out_dir>` runs it
alone (stations from `out/.meta`, which is how the synthetic card is
checked). It probes, segments motion and places clips like `clips`, but
since no finals exist at the door a move's destination is typed by the
realm's nearest reference type (the synthetic facade types as `porch`,
which counts as exterior). Per station: the clip and its segments, or
`NO CLIP -> PART`; the gimbal pass with its roles; `gimbal set missing:`
for entry/hallway/look-back/grounds; `REVIEW: no walkup` / `no close`.
About 40 s on the synthetic card, after the stills report. Verified:
baseline CLEAN; st04's clip removed → `st04 video: NO CLIP -> PART`; the
close removed → `REVIEW: no close`.

### Step 5 — reels

Three 9:16 cuts of 15–25 s, each leading with a different hero room. The
track's head is trimmed so the accent is frame zero; details run one bar
each from the phone pool first (native 9:16, scaled 2160 → 1920 tall),
then cropped gimbal moves and pans from the Fuji (centre crop 1215×2160 →
1080×1920, WDR cube); hard out on a phrase boundary. The phone facade
vertical opens each reel. Expect the two sources to differ in colour; v1
does not match them.

**Verify:** three files, 1080×1920, 15–25 s, every cut on a beat from the
accent, at least 60 percent of each reel's duration from vertical-native
clips on the synthetic card, playable on a phone.

*Built 2026-09-02:* `retool plan` writes `reel_01..03.md` next to the
cutlist (same row format; `end` and `native` in the header) and `retool
cut` renders them after the master unless a cutlist path was given. Two
phrases each (the largest whole-phrase count under 25 s): the facade
vertical, the lead hero room's verticals (one bar each) and its pan
(two bars, centre crop 1215×2160), then the vertical pool in station
order, then cropped moves and other pans; hard out on the phrase line,
audio from the accent with a 100 ms fade at the out, −14 LUFS. On the
synthetic card: three files, 1080×1920, 30/1, 20.2 s each (95 BPM), leads
living / kitchen / bedroom, 7/7 cuts per reel on the beat, 75 percent
vertical-native, master plus reels rendered in 98 s; the facade vertical
and a cropped graded pan looked right by eye, and differ in colour as
expected. *2026-09-05:* the reels take later sections of the master's
track (two phrases further in per reel), one song per job; and they are
built as 1080×1920 timelines in the Resolve project, see
`RESEARCH-resolve-handoff.md`.

### Step 6 — into the process

`retool run` on a card that has a `video/` sibling calls `clips` → plan →
`cut` after the scorecard, and `deliver` copies the two MLS files and the
reels into the delivery folder. The scorecard gains a "video" section:
runtime, clips used / skipped, gimbal set complete or not.

**Verify:** one command from card to delivery folder on the first real
video card; the human's only touches are the cutlist (optional) and the
watch.

*Built 2026-09-02:* `retool run` calls `clips` → `plan` → `cut` after the
scorecard when the job folder has a `video/` next to the card; `deliver`
copies the two MLS files and the three reels; `retool cut` appends (or
refreshes) a `## video` section on the scorecard: runtime and track, rows
used and skipped, the gimbal set, the reels, the files. Verified on the
synthetic card for `cut` → scorecard section → `deliver` (13 images and 5
videos in the delivery folder); the `run` chain itself waits for the
first real card, since the synthetic job has no RAFs.

## 4. Risks and how each is caught

- **`vidstabdetect` cannot separate a push from a pan** on some clip →
  the gimbal typing falls back to sequence order; the cutlist `why` column
  says `by order`. Step 1 verify covers it.
- **Frame matching lands on the wrong room** (two similar rooms) → the
  cutlist row shows both candidates when the margin is small; the human
  fixes the row. Cheap; rare in practice.
- **Auto ISO makes the gimbal clip visibly brighter or darker than the
  room's tripod clip** → the planner compares mean luma of the move's last
  still to the wide and warns in the cutlist; v1 does not correct it.
  First real card decides whether v1.1 needs a per-clip exposure nudge.
- **A pan across a bright window** slipped through on set → the clip's
  luma spikes mid-pan; the planner flags `window` and uses the still only.
- **Music licence** → `MUSIC.md` records, per track, where it came from and
  under what terms; the render log names the track used.
- **A track's stamp is wrong** (accent off by a beat) → every cut is off by
  a beat and the film feels late. The stamp is checked once per track by
  rendering a 20 s test cut with a click on the grid; the cut list carries
  the stamp so the human can correct it per job.
- **Too few verticals on a card** → the reel planner fills with crops and
  the cutlist says how much of each reel is cropped; under 40 percent
  native is flagged.

## 5. What the first real video card must contain

The recipe on the sheet, plus, for this plan's sake: at least one hallway
clip, one detail still, at least eight phone verticals, and both a walk-up
and a close, so every branch in the planner gets exercised once. Music is
ready: three stamped tracks in `ROTATION`.

**Where the cards go when the shoot is back.** One folder per job, per
`PROCESS-media-machine.md`:

```
jobs/2026-09-03_test-house/
  job.md        from jobs/TEMPLATE/job.md
  card/         every RAF from the CFexpress card (DCIM/1xx_FUJI)
  video/        every MOV from the SD card
  phone/        the phone's clips, exported at full quality (AirDrop keeps them)
```

Then, from the repo root: `retool card jobs/<job>/card` (the gate),
`retool run jobs/<job>/card jobs/<job>/out` (finals + scorecard),
`retool deliver realm.json jobs/<job>/out jobs/<job>/delivery`. The video
steps follow as they are built; until step 6 they are run by hand in the
order clips → plan → cut. Shoot it in daylight; the window pull and
the LUT's window handling get tested on the same card.

## 5b. The first real card, 2026-09-02

Shot the same evening the tools were built. `retool run` went card to
delivery folder in one command. What the card corrected in the tool (clip
typing by motion, first panning clip as the wide, the walk-up by its
origin, moves landing on the station that stands for their destination,
whole-walk spans capped at three bars, the exposure nudge as a `gain`
column) and what it corrected in the recipe (daylight, H.265 10-bit, log
exposure, the detail static, 4 s walks, six moves, a clip at every
station, the verticals) is in `EXPERIMENT-first-video-card.md`.

## 6. Recipe deltas already applied today

`RUNBOOK-shoot.md` and `SHOOT-SHEET.html`: video bank (f/4, ISO auto,
"slowed 2× in the edit"), station clip timing (3 · ~3 · 2), the vertical
pass (two 4 s vertical clips per hero room, one per other room with a
feature, one of the facade, L-bracket in the gear list), gimbal bookends
2 s, connective gimbal set, when to swap to the 18-50. Time budget 100
minutes.

## 7. Handoff — starting the build in a fresh session

Read `CLAUDE.md`, then `README.md`, then this file. The state on
2026-09-03:

- **Built and verified:** `retool card`, `run` (flat finals + details +
  `scorecard.md`), `deliver`, `level` (undistort + verticals), `realm`,
  `music stamp`. All in `src/main.rs`, `src/verticals.rs`, `src/realm.rs`,
  `src/music.rs`. Build with `cargo build --release` (PATH quirk in
  `CLAUDE.md`); `retool` is symlinked into `/opt/homebrew/bin`.
- **Built 2026-09-02:** §3 steps 0–6, each with a dated note under its
  verify line (`src/video.rs`, `src/cut.rs`, `test/synth-card.sh`,
  `test/synth/` from `test/synth-card.sh`). Open: the first real card (§5);
  the rule-6 runtime tension (step 2 note); Apple's Log cube; the audio
  splice by ear; the stamp tool proposing `end_s` past the file.
- **Test material on disk:** finals in `test/st*/final.jpg` (old layout, 9
  kitchen/living frames) and `out/st*/final.jpg` (19). The flat layout
  `run` writes today is `out/stNN.jpg` + `out/stNN_dK.jpg` + `.meta/`. For
  the synthetic card, any JPEG works; `test/` is the stable set.
- **Music:** `~/Documents/ShowReady/Music/ROTATION`, three tracks, stamped in
  `MUSIC.md`. The planner reads that table.
- **LUTs:** Fuji cubes in `~/Library/Application Support/Blackmagic
  Design/DaVinci Resolve/.LUT/F-Log2_LUT_E_Ver.1.06/X-H2/` (the zip is in
  `~/Downloads`). Apple Log cube: not yet on disk. *2026-09-02:* the Resolve
  folder holds only `.cube_shaperlut` conversions, which ffmpeg cannot load;
  the plain X-H2 cubes are unzipped from the download into the repo's
  `luts/` (gitignored), which is where the renderer looks.
- **Verified tooling:** ffmpeg 7.1 with lut3d, setpts, drawtext, concat,
  vidstabdetect, libx264, h264_videotoolbox, aac; exiftool; DINOv2 weights
  cached under `~/.cache/retool`.
- **Conventions that bind the build:** one binary, subcommands, no CLI
  framework; stages are plain functions, individually runnable; shell out
  to ffmpeg/exiftool the way `fuse` shells out to enfuse; verify by eye
  (render, look at frames), not by exit code; amend docs with dated notes
  when a decision changes.
- **What is happening meanwhile:** the operator is shooting a test house
  with the new recipe (station clips, 18-50 details, phone verticals,
  connective gimbal set), then a real listing. Those cards are the first
  real video input; the build should be at step 3 by the time they land.
