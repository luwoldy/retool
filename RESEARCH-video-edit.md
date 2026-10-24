# Can the tool cut the video? (station clips + gimbal clips → MLS cut)

Research pass 2026-09-02, prompted by the decision to roll video at every
station after the pop. Two questions: (1) if video is only rolled at
stations, what is missing from the cut? (2) how do gimbal shots cut into a
tripod-based edit? Then: can `retool` assemble the cut, and what does the
shoot have to give it?

Sources: the craft rulebook already in this repo (`RESEARCH-shoot-craft.md`,
`auto_v2/PLAN-shoot-craft.md`), five practitioner threads read this pass
(linked inline), the direct competitor's published pipeline, and ffmpeg tests
run on this machine.

## 0. Headline

1. **A station-only cut has exactly one gap, and it is the big one: layout.**
   Coverage is complete (every room gets a static and a pan), but viewers of
   tripod-and-slider tours say the same thing every time: "I can't remember
   what the house looked like", "I don't know the layout", "cuts between rooms
   were confusing." Pictures that move are still pictures. The gimbal exists
   to answer *where am I* between rooms, and only for that.
2. **So the gimbal pass shrinks to a connective set, four to seven clips:**
   walk-up, entry through the door, each hallway or stair head-on, the
   primary-suite look-back, out to the grounds, the close. Nothing inside a
   room needs a gimbal once the tripod has the wide and the pan.
3. **Gimbal clips cut in as transitions between rooms, never as coverage
   inside one.** The rulebook's grammar already says how: cut still to still,
   use a move whole, one walking speed for all moves, the head-on hallway is
   the neutral shot that lets the film change direction, and never show
   entering a room after its details.
4. **The tool can assemble the MLS cut deterministically**, because the shoot
   recipe fixes what every clip is: station clips inherit their room type
   from the stills scorecard, gimbal clips are typed by matching their first
   and last frames to the station finals with the realm's embedder, and the
   beat sheet fixes durations. Everything else is ffmpeg, verified below. The
   direct competitor (EstatePass Video Polish) runs the same architecture,
   guessing the room and the order with an LLM because phone footage has no
   recipe. Ours doesn't need to guess.
5. **What stays human:** the music choice, the one-minute watch before
   delivery, and edits to the cut list when a listing needs different
   emphasis. The cut list is a text file; the human edits it and re-renders.

## 1. Q1 — what's missing when video only rolls at stations

**Nothing in coverage.** Each station yields a locked-off wide and a pan
with a destination. The beat sheet's Establish (a static front wide is
sanctioned), Hero, Support, Grounds and Close beats can all be filled from
sticks. The only beat that literally needs a move is the Close pull-back,
and a held rear exterior is an acceptable substitute.

**Everything in orientation.** The evidence is unusually consistent:

- A slider-and-fluid-head tour, technically admired ("beautiful shots,
  camera movement, and colors"), drew as its top critiques: "I watched the
  video, but can't remember what the house looked like… stagger your shots
  in a way that follow the general layout of the house, so it's easier to
  comprehend where you are"; "not enough wide shots… don't even really know
  what it looks like from the curb"; "some of the cuts between rooms were
  confusing… I don't actually know the layout of the home from this video";
  "I have no real sense of the layout or size of the house… you'll have the
  sizzle and the steak"
  ([r/RealEstatePhotography, 21 comments](https://reddit.com/r/RealEstatePhotography/comments/knk1c3/)).
- Beginner critique, same year range: "start with a few wide establishing
  shots… when you enter a room start with a wide establishing shot panning
  in. You can hint the room with a shot behind the door frame and then walk
  in… It doesn't make sense to show entering the bedroom once you've already
  showed detailed shots of it"
  ([r/videography](https://reddit.com/r/videography/comments/1muouz7/)).
- The 693-point critique thread (gimbal-only tour): "your movement is too
  similar between shots… almost always a pan with a twist at the same speed…
  Let some shots hold without movement"; "pick one kitchen shot or the other,
  too similar visually, and you flip 180 which is disorienting… break it up
  with your countertop detail"; and the top craft comment: "A+ just for
  holding on shots more than 5 frames… You can actually get a feel of what
  the house looks like"
  ([r/videography](https://reddit.com/r/videography/comments/1lnwyzc/)).

Read together: statics carry the rooms, and the film needs something else
to carry the *between*. Two things do that. The cheap one, always
available from sticks, is tour order plus sightline pans (the pan that ends
on the doorway into the next room, so the cut to that room's wide reads as
"we went through there"). The strong one is the gimbal move that actually
goes through the door. Both are in the recipe below.

**Also missing without care: variety.** A tour that is all pans at one
speed reads as monotone even when every pan is good. Vary the unit: some
rooms get only the static, hero rooms get static + pan + a second-angle
static, and the moves sit at boundaries. The docs' pacing grid already says
this (room dwell 6–8 s built from 2.5–5 s shots, hero holds 4.8 s+).

**Verdict:** for a one-level home with a simple plan, a station-only cut is
an acceptable MLS video and honestly better than most gimbal-only tours.
For anything with a hallway, a stair or a second level, add the connective
gimbal set. It costs about ten minutes on site.

## 2. Q2 — how gimbal shots cut into the tripod edit

The rulebook's cut grammar (PLAN-shoot-craft.md §1, RESEARCH-shoot-craft.md
§3.4, A5), restated as rules a program can follow:

1. **A move is a transition, not coverage.** A gimbal clip goes *between*
   two rooms: it starts in room A (or outside) and ends on room B's feature
   or window. It is followed by room B's tripod wide, never by more of room
   B in motion.
2. **Cut still to still.** Every gimbal clip is bookended by 3 s of
   stillness on the shoot; the edit cuts on those stills. The move is used
   whole between its two stills; there is no cutting inside a move.
3. **One walking speed.** All gimbal clips are conformed by the same factor
   (60p → 30p at half speed) so consecutive moves match tempo. Moving-to-
   moving cuts happen only between clips at that one speed.
4. **Static to move to static, not shot for shot.** Group the statics (a
   room's wide, pan, second angle), then the move, then the next room's
   statics. Never alternate static / push / static within a room.
5. **The head-on hallway is the neutral shot.** Screen direction is kept
   inside a room; it may change across a hallway or stair clip because head-
   on / tail-away shots are neutral and license a reversal.
6. **Establish before detail; never enter a room after its details.** The
   entry move lands on the wide; details and the second angle come after.
7. **Match the landing.** The tripod wide that follows a move is shot from
   the side of the room the move lands on. In practice the station's tripod
   position is the doorway the entry move comes through, which the runbook
   already prescribes ("from the door").

The resulting pattern for one room boundary:

```
[A wide 4.8s] [A pan ~6s w/ 1s holds] [A second angle 3s]
   → [gimbal A→B, still 1s · move · still 1s]
      → [B wide 4.8s] [B pan] …
```

And the film-level skeleton (Establish first per §3.5, no hook):

```
walk-up (gimbal) or front static ─ title card
entry move (gimbal) → living wide → living pan → second angle
dining static → kitchen wide → kitchen pan → detail static
hallway/stair (gimbal, head-on) → primary wide → pan → look-back (gimbal)
secondary bed static · bath static · …
out to grounds (gimbal) → patio static → yard static
close (gimbal pull-back, or rear exterior held 8s) ─ end card
```

Runtime lands at 60–110 s for a 15-station house, under the 120 s Showcase
cap. When over, shorten support rooms first, then second angles, then pans;
never the hero wides (Murch: "sacrifice your way up").

## 3. What the shoot must give the machine

The point of the recipe is that every clip's identity is known from *when*
and *how* it was shot, so the tool never has to guess.

**Per station, after C2:** one video clip. Hold 5 s still (this becomes the
locked-off wide), pan three quarters of a frame to the destination over
5–7 s, hold 3 s still. Same tripod position and height as the stills. On a
station with no destination (bath, small bedroom), hold 5 s and stop; the
tool sees a clip with no pan and uses it as a static.

**Gimbal pass, fixed order, one clip per move, 3 s still at both ends:**
walk-up · entry (door → living feature) · each hallway or stair head-on ·
primary look-back · out to grounds · close. The order is the runbook's
order, so clip sequence alone types most of them; the frame matching in §4
confirms origin and destination.

**Details, hero rooms only.** After the video clip, swap to the 18-50, zoom
past 24 mm, one C2 press for the still and one 5 s static clip. The still is
recognised by focal length (longer than the room's bracket lens, within four
minutes of the station) and filed as `stNN_dK.jpg`; the clip is the extra
static in the station's window, and is the third shot of a hero room's unit
in the cut (wide → medium → detail) and the buffer between two similar
angles. The 18-50 rule in full is in `RUNBOOK-shoot.md`.

**Both cards share the body clock.** Stills go to CFexpress, video to SD,
timestamps are comparable, so a station's clip is "the MOV recorded between
this station's pop and the next station's first bracket." The gimbal pass
is "every MOV after the last station."

**Exposure.** Tripod clips manual, same as the stills' philosophy. The
gimbal walk crosses rooms of different brightness; practitioners run auto
ISO with fixed shutter and aperture for walks. Recipe candidate: video bank
manual for stations, auto-ISO for the gimbal pass. First card decides.

## 4. The assembly — `retool cut`

Inputs: the card (stills + MOVs), the finished `out/` (finals + scorecard,
so room types and fit are known), `realm.json`, a music file (optional), an
address string.

1. **Ingest.** `exiftool -json` over the MOVs: CreateDate, Duration,
   VideoFrameRate. Assign each clip to a station by the timestamp window
   above; the tail block is the gimbal pass, ordered by time.
2. **Analyse motion.** `ffmpeg -vf vidstabdetect` writes per-frame
   translation, rotation and zoom to a text file; the tool reads it and
   segments each clip into still / pan / push runs. Station clips yield
   `still_a`, `pan`, `still_b` boundaries; gimbal clips yield the two
   bookend stills and the move between them.
3. **Type clips.** Station clips inherit their station's room type and fit
   from the scorecard. For gimbal clips, extract the first and last frames
   (`ffmpeg -frames:v 1`), embed both with the realm's DINOv2 embedder, and
   take the nearest station final by cosine distance: origin room and
   destination room. Motion signature (forward push vs lateral) plus
   sequence position resolves the rest (walk-up has no interior origin;
   close ends on an exterior).
4. **Plan the cut** from the beat sheet: Establish, then rooms in station
   order, hero rooms (living, kitchen, primary, and any station with fit
   ≥ 60) get wide + pan + second angle, support rooms get one static,
   consecutive stations of the same room type collapse to the best two by
   fit, gimbal clips are inserted before the wide of their destination
   room, grounds, close. Emit `cutlist.md`: one line per clip with source,
   in, out, speed, duration, and the reason it was chosen. This is the
   human's control surface.
5. **Render.** Per clip: trim, `setpts` (gimbal 2×; tripod native by
   default, switchable per line), `lut3d` with Fuji's own X-H2 F-Log2 cube,
   scale 1920×1080, `libx264` CRF 18. Concatenate with the concat demuxer
   (straight cuts; a 12-frame fade in and out at the ends only). `drawtext`
   address card over the Establish, end card PNG (branded and unbranded
   variants), music mixed under with `afade` and back-timed so its end
   lands on the close. Output 1080p30 H.264 at 8–16 Mbps, AAC.
6. **Reels.** Three 9:16 cuts of 15–25 s from the 4K source: exterior,
   living, kitchen or primary. Centre crop 1215×2160 → 1080×1920. A crop,
   not a reframe; practitioners note vertical wants its own shooting, so v1
   reels are "good enough", not a second product.
7. **Outputs.** `out/video/mls_branded.mp4`, `mls_unbranded.mp4`,
   `reel_01..03.mp4`, `cutlist.md`. `retool cut --from cutlist.md`
   re-renders after a hand edit.

## 5. Feasibility, verified on this machine 2026-09-02

- ffmpeg 7.1 with `lut3d`, `setpts`, `fps`, `drawtext`, `concat`, `xfade`,
  `fade`, `vidstabdetect`, `minterpolate`, `libx264`, `h264_videotoolbox`,
  `aac`.
- Fujifilm's official F-Log2 LUT pack v1.06 is already on disk
  (`~/Downloads/F-Log2_LUT_E_Ver.1.06.zip` and installed under Resolve's LUT
  folder), with four X-H2 cubes: FLog2→WDR BT.709, →ETERNA BT.709,
  →ETERNA-BB, →FLog2 BT.709. `lut3d` loaded the ETERNA cube and rendered.
- Conform test: a 4 s 60p source through `setpts=2*PTS,fps=30` produced 240
  frames at 30 fps (8 s), with `drawtext` overlaid, in one pass.
- Cost estimate: 1080p x264 encodes near real time on Apple silicon; a 90 s
  cut renders in about two minutes; `h264_videotoolbox` is faster if needed.
  Embedding ~15 gimbal frames with DINOv2 is under 20 s.
- Not needed: stabilisation (sticks and gimbal), speed ramps, dissolves,
  AI anything.
- To verify on the first video card: X-H2 MOV tags via exiftool (CreateDate,
  VideoFrameRate), and that `vidstabdetect`'s zoom term separates a push
  from a pan cleanly.

## 6. The benchmark

EstatePass "Video Polish" (free, 2026): accepts MP4/MOV/WebM up to 500 MB and
3 minutes; "Gemini AI to analyze your raw footage — identifying rooms,
assessing quality, and suggesting optimal clip order"; ffmpeg for "color
correction, stabilization, noise reduction"; Remotion "composes the final
video with your chosen template, music, text overlays, and agent branding";
outputs 16:9, 9:16 and 1:1; 1–3 minutes end to end
([estatepass.ai](https://www.estatepass.ai/tools/ai-video/)). The wider
2026 field is mostly photo-to-video generators for agents (BetterSpace,
Reel-E, ReelTourStudio; see
[fotober's roundup](https://fotober.com/ai-real-estate-video-editor)).

Same shape as §4 with one difference that matters: their room typing and
clip order are inferred from arbitrary phone footage; ours are known from
the recipe and confirmed by the realm. That is the whole argument for a
shoot recipe: it turns editing into assembly.

## 7. Open questions the first video card answers

1. One clip per station (hold–pan–hold) or two (static, then pan)? One is
   fewer presses; the motion analysis splits it.
2. WDR vs ETERNA cube as the one LUT. WDR is flatter and holds windows;
   ETERNA is warmer and filmic. The bright-and-airy target and the "true
   whites, a tad more saturated" consensus point to WDR plus a small
   saturation lift; test both on the same room.
3. Tripod pans at native speed (frame-dropped 60 → 30) vs conformed 2×.
4. Reliability of gimbal end-frame → station matching (expected high: same
   room, same wide lens, seconds apart).
5. Auto-ISO on the gimbal pass: does exposure drift between the move and the
   station's manual clip show at the cut?

## 8. Decisions taken (recipe and backlog amendments)

*Superseded in part 2026-09-02, later the same day: the open questions in
§7 were decided (WDR LUT, everything conformed 2×, music folder rotation,
1080p30) and the build order written up in `PLAN-video-cut.md`. The live
pan is now ¾ frame in ~3 s with 3 s and 2 s holds, not 5–7 s.*

- **Per station:** one video clip after C2, hold 5 · pan · hold 3. Added to
  `RUNBOOK-shoot.md` and `SHOOT-SHEET.html`.
- **Gimbal pass:** reduced to the connective set (walk-up, entry, hallways
  and stairs, primary look-back, grounds, close). Runbook and sheet amended.
- **Tool:** `retool cut` per §4 is the video item in the machine backlog,
  replacing the earlier "conform" note. v1 = MLS cut + cutlist; reels
  follow. Build against the first card that has video; nothing on disk has
  any yet.
- **Details:** one per hero room on the 18-50 at the station (still + 5 s
  clip); the tool files them by focal length and timestamp. Built 2026-09-02
  for stills (`card`, `run`, `scorecard`, `deliver`); the clip side belongs
  to `retool cut`.
- **Delivery:** 1080p30, 8–16 Mbps, ≤ 120 s, branded + unbranded, one LUT.

## 9. Reels: purpose, pace and intrigue (decided 2026-09-06)

**What the evidence supports.** Cutting's Cornell work on 150 Hollywood
films: average shot length fell from about 12 s (1930) to about 2.5 s, and
shot-length fluctuations drifted toward the 1/f shape of human attention:
short shots cluster with short, long with long, and the run varies. Lang's
Indiana lab (the limited-capacity model): every cut triggers an involuntary
orienting response; arousal, attention and memory all rise with the edit
rate up to a point, and very fast pacing on top of dense content overloads
encoding so nothing is remembered. Smith's attentional theory of
continuity: a cut lands when the new shot gives the eye something to land
on; a cut to a similar frame wastes the grab. Platform data, secondary and
unsourced but consistent: the keep-watching decision is made inside 3 s,
and a first-3-seconds drop over 30 percent means the opening frame failed.
The "cuts every 0.5 to 1 s give 3.3× completion" claims trace to one
editing blog and are not used here.

In beats: at 102 BPM two beats are 1.18 s (action pacing), four are
2.35 s (modern feature pacing), eight are 4.7 s (the whole decision
window on one picture). Hence the social blocks in MUSIC.md.

**Each reel has a purpose** (three timelines in the Resolve project,
`retool plan` writes them):

1. `mls`: the listing condensed, in station order, opening on the facade
   vertical; the payoff is the best hero's pan.
2. `details`: details only, hero rooms first; verticals and 18-50 statics,
   no wides. The scroll-stopper.
3. `feature`: one station's material, the selling point: `Feature:` in
   `job.md` (a room type or `stNN`), else the hero with the best fit. Its
   verticals, its hold, its second angle, the move that arrives in it, its
   pan as the payoff. On set this wants the feature zone shot with intent:
   two extra verticals and the 18-50 detail there.

A thin pool serves second and third helpings, other windows of a clip
already used, and the cutlist says so; the on-set answer is more distinct
verticals.

**Build the intrigue.** The operator's rule for the shot list, quoted
2026-09-06: a movie trailer does not show everything; it shows enough to
get you into the theatre. The photographer's job is the same: intrigue
the buyer off the couch and into the home with the agent. Every closet,
laundry room and garage will not do that, and the point is made early and
often with agents and sellers so nobody calls back about the hallway bath.
What follows from it here: the reels never carry support rooms; the
`details` and `feature` reels are pure by construction; the master's
collapse rule (at most two of a type, best two by fit) is the same idea.
Open: whether the master should also leave out support statics below a
fit floor. The runbook's row 14 (office, laundry, garage, basement: only
if it sells) already carries the rule to the shoot.

**The master, decided 2026-09-06.** Same principle, fours and eights only.
The listing order stays, since a walkthrough that jumps around reads as
confusion; intrigue comes from omission and teasing. The shelf is a cold
open when the intro holds a phrase: two teases (the best details on the
card by hero fit, from different rooms), then the walk-up as the arrival
eight landing on the accent, the address card over it. Hero rooms are
8-4-4 reveals. A move closes a 4-4-8 arrival on the last hero's spare
teases, never on a support static. After every second hero a 4-4-4-4
montage of four teases: spare verticals, 18-50 details, and one glimpse
each of the support rooms that clear a fit floor of 30; under the floor a
room is not shown and the skipped table says so. Grounds are a glimpse in
the last montage. Runtime is what the material gives, capped at 110 s and
the track's usable phrases; nothing is padded. On the 2026-09-02 card this
prunes the dining pair (fit 10 and 13) and the hallway move into it.

**Measure it.** Per reel, 3 s retention and completion from the platform
insights go into `jobs.tsv` at +7 days. The cutlist header names each
phrase's pattern, so two reels of one listing on different grammars are a
test that reads in a week.

