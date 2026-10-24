# Research — the Resolve handoff: the cut as a project, not as files

Decided 2026-09-05 with the operator, on top of `PLAN-video-cut.md`, and
spiked the same day on the 2026-09-02 card. The question: instead of
`retool cut` rendering the MLS files, can the tool hand over a DaVinci
Resolve project with the finished cut, so the operator opens it, checks
that the right portion of each clip is playing, tweaks, and renders?

Answer: yes. `tools/resolve_build.py <job_dir> <out_dir> [--replace]
[--render none|check|all]` builds it from the cutlists in 49 s and Resolve
renders all five files in 66 s. What follows is what was decided, what the
API can and cannot do, and what the spike measured.

## 1. Decisions

| # | Decision | Choice | Why |
|---|---|---|---|
| R1 | The renderer | **Resolve renders the deliverables**, from the project the tool builds; the render jobs are queued in the project. `retool cut`'s ffmpeg render stays as the no-Resolve fallback and its look now differs | Two renderers means two looks that drift. The project is the QC surface *and* the output |
| R2 | Colour | **No LUTs.** DaVinci YRGB Color Managed, Custom, timeline DaVinci WG/Intermediate, output Rec.709 Gamma 2.4, input DRT None, output DRT Luminance Mapping, white point adaptation on (the frame.io cheat sheet). Input colour space per clip: `Fujifilm F-Log2` as the project default, `Apple Log` on the phone clips, both native in Resolve 21. Supersedes D1 (the WDR cube) | Two log formats in one project are two per-clip input spaces, nothing more; Apple's cube stops being an open item; the look lives in Resolve where the operator can move it |
| R3 | Music | **One song per job**: the reels ride the master's track with the accent at frame zero, the same section three times (`retool plan`, 2026-09-05; first built as later sections, changed the same day when the grammar came in). Supersedes the step-5 note in PLAN-video-cut.md | The operator's call for now, and the grammar's rule that verticals place the accent at frame zero |
| R4 | Who plans, who translates | Rust plans (`retool clips` → `plan`, the cutlists), Python translates (`tools/resolve_build.py`, the scripting API is Python or Lua only). The translator reads the cutlists, `clips.tsv` and `job.md`; it decides nothing | The split `handoff.py` chose in the first-generation pipeline; every choice stays visible in the cutlist |
| R5 | The 2× conform | The clip's frame-rate attribute: 59.94 → 29.97, 60 → 30, set by script before the clip is placed | The API has no retime; this is what D2 calls "conform". Fallback if ever needed: a lossless remux with `-itsscale 2` |
| R6 | The music bed | The tool renders one WAV per cut (splice at the bar line, 100 ms crossfade, two-pass loudnorm to −14 LUFS), which lands on A1 as a single clip | No transitions and no loudness normalisation in the API; the bed is deterministic from the cutlist, so it is the tool's job |
| R7 | The cards | ProRes 4444 clips with alpha on V2, drawn by ffmpeg the way `retool cut` draws them; the open card carries the 0.4 s fade from black in its alpha | Inserted titles cannot be trimmed by script; a clip with an explicit frame range can. The operator can retype a card by dropping a Text+ over it |
| R8 | The exposure nudge | The cutlist's `gain` column becomes a CDL offset of `gain × 0.07329` on the clip's node | One stop in DaVinci Intermediate, the node's working space under RCM; the F-Log2 constant was 0.0738. No add-node call, so one node |
| R9 | Stabilisation | `TimelineItem.Stabilize()` on gimbal rows when the cutlist says `stabilise: on`; 2–3 s per clip | Better than ffmpeg's `deshake`, and the operator can turn it off per clip |
| R10 | The grid in the project | SR1's ruler from the accent: every beat Cream, bars Sky, phrase downbeats Green named by the cutlist's `patterns:` line (`P2 4-4-8 entry move → living_room`); each row a named, coloured clip (`07 wide living_room`) with its `why`, gain and source window in a clip marker; the whole card in bins `camera`/`phone` with station, kind and segments in Comments, unused clips in Tan | "Make sure the right portion of each clip is playing" needs the beat visible and the why at hand; a swap is a drag from the same bin |

Considered and rejected: writing FCPXML/OTIO and importing it. Resolve's
import of speed changes and grades is lossy, and the API route was already
proven by the operator's SR2 script.

## 2. What the scripting API cannot do, and the workaround used

| Missing | Workaround |
|---|---|
| Retime / speed | R5, the frame-rate attribute |
| Trim an inserted title or generator | R7, pre-rendered cards with an explicit frame range |
| Transitions, audio crossfades, loudness | R6, the bed WAV; straight cuts were D6 anyway |
| Add a node | R8, one node: CDL offset for the gain. Saturation lift dropped (RCM's own rendering, the operator grades) |
| Keyframes (fades, opacity) | The open's fade is in the card's alpha; nothing else fades (composed ending) |
| Read the placed source in-point exactly | `GetSourceStartFrame` floor-rounds a float and reads one low on some rows; `GetLeftOffset` reads the frame that was asked for. Record positions read exact |

## 3. The spike, 2026-09-02 card (measured)

Build: 49 s from cutlists to a saved project (`re/2026-09-02_test-house`,
`.drp` exported next to the beds and cards in `out/video/resolve/`).
Five timelines: MASTER branded, MASTER unbranded (duplicate, end card
swapped), REEL 01–03 at 1080×1920. Render, all five in Resolve: 66 s
(master 24 s each, reels 5 s each). Outputs 1920×1080 / 1080×1920, 30/1,
H.264 16 Mb/s, AAC; master 106.10 s = the cutlist's end; reels 18.83 s.

Placement: 22/22 master rows and 7/7 per reel placed; every record
position equals `round(at × 30)` to the frame; every duration equals its
slot; the bed on A1 runs to the end; cards at 0–12 s and 96.7–106.1 s.

Colour, Resolve frame vs the ffmpeg render at the same time, mean Y
(8-bit): 5 s 58/56 · 14 s 110/107 · 37.5 s 96/92 · 72 s 112/108 ·
88 s 126/123 · 100 s 48/47. Fuji rows sit within 4 %, Resolve a touch
brighter and more saturated, the WDR cube a touch more contrast. The
phone row at 24.5 s reads 53/66: Resolve's native Apple Log transform is
darker and more contrasty than the approximate cube; the native one is
the reference now. Frames viewed side by side: kitchen, bedroom, both
cards, the walk-up, the close, the facade vertical and a cropped pan on
the reel. Blacks at the dark close match, so `Data Level: Auto` reads the
8-bit F-Log2 files correctly.

The five unknowns from the assessment, settled:

1. The `FPS` clip property is settable and reads back (60 → 30, 59.94 → 29.97).
2. A timeline takes its own 1080×1920 through `useCustomSettings`. **But a
   custom-settings timeline carries its own colour-management block and
   it defaults to unmanaged** (`davinciYRGB`, Rec.709 (Scene)): the first
   reel render was flat and milky on both sources. The build copies the
   project's keys onto each such timeline, in this order: colorScienceMode,
   isAutoColorManage, rcmPresetMode, separateColorSpaceAndGamma,
   colorSpaceTimeline, colorSpaceOutput, inputDRT, outputDRT,
   useCATransform. `colorSpaceOutput` is refused until `rcmPresetMode` is
   Custom.
3. Portrait phone clips arrive upright (Resolve honours the rotation tag);
   `Scaling = fill` on a mismatched item is the centre crop the ffmpeg
   path used, in both directions.
4. Data levels: Auto is right for these files.
5. Look parity: within 4 % on Fuji rows, see above.

Also found: Resolve auto-detects `Apple Log` on the phone clips from
metadata, before the script sets it; the Fuji clips carry no such tag and
take the project default. `VideoQuality` in the render settings is
ignored (reads None); the bitrate follows the `H.264 Master` preset.
**The playback frame rate is not settable by script**: `timelinePlaybackFrameRate`
refuses every value at project and timeline level, does not follow
`timelineFrameRate` or `videoMonitorFormat`, and reads the default's 24
on a project the API created. The renders are unaffected (30 fps to the
frame); the viewer may play the 30 fps timeline at 24. The route around it
is `--template <file.drp>`: a project the operator set to 30 once in the
UI, exported, imported per job and stripped of its timelines and media.

## 4. What the operator does in QC

Open Resolve, project folder `re`, project named after the job folder.
Five timelines; the grid is the markers; each clip is named by row, role
and room, its why on a clip marker; the whole card is in the bins with
comments. Slip any clip to the portion that should play, or drag a
replacement from its bin; the slot on the grid stays. Render Queue holds
the five jobs: Render All writes `out/video/resolve/*.mp4`. Without a QC
pass, `--render all` does the same from the command line.

Not yet on the human surface: `job.md` still prints the template
placeholders on the cards (fill it before the build, as before), and the
warm cast of tungsten under 5000 K is the recipe's, not the tool's.

## 4b. The grammar, 2026-09-05 (later the same day)

The operator's beat/phrase grammar went into the planner after the spike:
fours and eights only, a hero room one 8-4-4 phrase, moves closing 4-4-8
arrival phrases, reels opening on a 4-4-4-4 hook with the accent at frame
zero (MUSIC.md "The grammar", PLAN-video-cut.md corrections). On the
2026-09-02 card the master tiles as seven phrases, 87 s, down from nine and
106 s; every row in Resolve is now a four or an eight and the phrase markers
read the pattern. The catalog columns `track_id`, `band` and `shelf_end_s`
are read by name from MUSIC.md; `band` and `shelf_end_s` are carried but
not yet acted on, their meaning is to be confirmed.

## 5. The shot-list step

The operator asked for a step that plans the shot list from what is on
the card and produces it for the horizontal and the social cuts. That step
exists: `retool clips` types the card, `retool plan` writes the master
cutlist and the three reels, and the spike shows the cutlists *are* the
shot list, row for row, in the project. What the exploration found it
should become:

- **One document per job**, `shotlist.md`, with the four cuts as sections
  on one song (R3), instead of four files with repeated headers. The
  header carries the track and grid once; each section its lead, bars and
  end. The build reads it the same way.
- **A pool section**: every clip on the card with station, kind, segments
  and where it is used (or why not), which is what the spike wrote into
  the bins' Comments. The skipped table becomes part of the pool.
- **The still next to the row**: each station row names its final
  (`out/st04.jpg`), so the QC compares the moving frame to the photo it
  will sit beside. The spike's colour check was exactly that comparison.
- **Before the shoot it is the runbook's station list; after the card it
  is `plan`'s output.** The two should use the same room names and roles
  so a missing move or vertical reads the same at the door and in the
  cutlist (the gate already reports the gimbal set).

None of that changes the planner's rules; it changes the shape of what it
writes and what the translator reads. Build order: merge the four files
into `shotlist.md` with the pool, teach `read_cutlist` and the translator
the sections, then wire the build into `retool cut --resolve` so the beds
and cards come from one place.

## 6. Environment notes (also in CLAUDE.md)

Resolve Studio 21 must be running; external scripting is on. The script
appends the API's `Modules` folder to `sys.path`; no env vars needed on
this machine. The project is created under a database folder `re`;
`--replace` deletes and rebuilds a project of the same name, and the
currently open project is saved before Resolve switches away from it.
Plain `python3` under a non-interactive shell has an ASCII locale: write
files with `encoding="utf-8"`. Timeline markers need media on the timeline
first; one marker per frame. Switching a timeline to custom settings
resets its colour management to unmanaged, also when done later by hand.
