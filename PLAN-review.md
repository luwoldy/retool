# Plan — `retool review`: see, tweak and approve the plan before it is built

Decided 2026-09-06 with the operator, on top of `PLAN-video-cut.md`,
`RESEARCH-resolve-handoff.md` and `RESEARCH-video-edit.md` §9. The
principle: **the planner comes technically correct, the operator adds the
feeling.** The grid, the durations, the colour and the loudness are the
machine's and stay so; which picture sits in which slot, which tease opens,
which room is the feature, what is not shown at all, is the operator's, made
on a page with thumbnails before anything is built, and signed off with one
line in the cutlist.

Each step has a verify line; a step is done when its verify line is true.

## 1. Decisions

| # | Decision | Choice | Why |
|---|---|---|---|
| V1 | Where the record lives | The cutlists stay the plan: `out/video/cutlist.md`, `reel_01..03.md`. The review page is a view of them, never a second source of truth | D9: every decision visible and reversible in one text file; the Resolve build and `retool cut` already read only these |
| V2 | What the operator edits | Slot-preserving edits only: **swap** the clip in a slot, **slip** its window, **reorder** slots of equal beats within a phrase, **drop** a whole phrase, **revive** a skipped shot into a slot. Structure changes (music, shelf, floor, feature, the tease pair) are header overrides that `plan` honours on a re-plan | The grid is what makes the cut technically correct; a slot's beats never change by hand. Heavy edits happen in Resolve after the build, which is already the QC surface |
| V3 | The gate | `approved: <date>` in the cutlist header, written by the page or by hand; one approval per job covers the reels. `plan` refuses to overwrite an approved cutlist (`--replan` discards the approval and says so); the Resolve build and `retool cut` refuse a cutlist without it (`--unapproved` bypasses, for the synthetic card and tests) | Nothing gets built on a plan nobody looked at, and an edited plan cannot be lost to an accidental re-plan |
| V4 | `retool run` stops at the plan | `run` ends after `plan` and prints the review command. The build follows approval (V8), never the plan | The plan is that step's deliverable |
| V5 | The page | `out/video/review.html`, written by `plan` every time, self-contained HTML with inline CSS and JS, thumbnails as files in `out/video/thumbs/`. Opened directly it is read-only (thumbnails, patterns, skipped, pool). Served by `retool review` it edits and approves | A static file costs nothing and already answers "what is in the cut"; edits need a writer |
| V6 | The writer | `retool review <job_dir> <out_dir>`: a std-only HTTP server on localhost (`TcpListener`, no framework), `GET /` the page, `POST /save` the edits as JSON, `POST /approve`, `GET /thumbs/…`; opens the browser with `open`. Rust validates every edit against the grid, rewrites the cutlists through the same writer `plan` uses, re-lays windows for swapped clips, re-measures gains, regenerates thumbnails for changed rows, rewrites the page | One binary, subcommands, no CLI framework (CLAUDE.md); the writer is ~200 lines of std |
| V7 | Thumbnails | Per row the in frame and the out frame; per pool clip the mid frame; per hero phrase the station's still (`out/stNN.jpg`) beside it. Extracted by ffmpeg with the fallback cubes (WDR for Fuji, the Apple approximation for the phone) so log footage reads as a picture, 320 px wide, cached by clip and time in `out/video/thumbs/` | Recognition, not colour: the Resolve render is the colour reference. The cache keeps a re-plan under 10 s |
| V8 | After approval | **Approve builds the project.** The page's Approve writes the line and then runs the build (`tools/resolve_build.py`, no render): the project is in Resolve for the QC pass, the five render jobs queued, Render All the operator's. If Resolve is not running the page says so and leaves the command. `retool resolve <job_dir> <out_dir> [--render all]` is the same build from the shell, gate checked, `--render all` for the no-QC path; `retool cut` stays the no-Resolve fallback and honours the gate too | Approve → open Resolve → QC → render is the whole loop, and the translator stays Python because the API is |

Header overrides `plan` honours (existing: `music:`, `shelf:`; new: `floor:`,
`feature:`, `teases:` as two clip names, `cold_open: off`). They survive a
re-plan; rows do not, which is why V3 protects an approved cutlist.

## 2. The page, what it shows

- **Top:** address, track (name, BPM, accent), runtime, the `patterns:`
  line, the approval state and button. Notes from the header in a callout.
- **Master:** one band per phrase, labelled as the Resolve markers are
  (`S 4-4-8 cold open`, `P3 4-4-8 hallway move → bedroom`). Slots are cards
  whose width is their beats (a four is one unit, an eight two). A card:
  the in frame large, the out frame small, role badge in the Resolve clip
  colour, room, clip name, window in source seconds, the why in small
  type, and the gain for camera rows. Hero phrases carry the station still
  at the band's end.
- **Reels:** the same bands, twos at half width, purpose and lead in the
  band header, the native percentage.
- **Skipped:** every row of the skipped tables with a thumbnail and the
  reason, so "not shown: build the intrigue" is a picture the operator can
  disagree with.
- **Pool:** every clip on the card, mid frame, station, room, kind,
  segments, and where it is used. The swap picker draws from here, same
  room first.
- **Edits (served only):** swap (picker), slip (±0.25 s buttons and a
  number field, bounded by the clip), reorder (drag between equal-beat
  slots in the same phrase), drop phrase, revive (a skipped shot into a
  slot of its beats). Save writes; Approve writes `approved:`, locks the
  page, and builds the Resolve project (V8), showing the build log as it
  goes and the project's name when it is done. Every edit re-renders its
  card's thumbnails after Save.

## 3. Build order

### Step 1 — thumbnails and the static page

`src/review.rs`: `thumbs(job_dir, out_dir, cutlists, clips)` extracts the
in/out frames per row and the mid frame per pool clip, cached; `page(...)`
writes `review.html` from the cutlists, `clips.tsv`, the scorecard's rooms
and fits, and the thumbnails. `plan` calls both at its end. The page shows
everything in §2 and is read-only.

**Verify:** on the 2026-09-02 card `retool plan` writes `review.html` in
under 30 s cold and under 5 s warm; opening it shows the cold open's two
teases as pictures, the five master phrases with their labels, the three
reels, the four "not shown" rooms with thumbnails, and the pool with 39
clips; the station still sits beside each hero phrase.

*Built 2026-09-06:* `src/review.rs` (`refresh`: the grid model, the
thumbnail cache, the page's data) and `src/review.html` (the page: CSS and
the renderer, `include_str!`'d into the binary; the data block is what
`plan` or a save wrote). On the 2026-09-02 card: 135 thumbnails, 28.7 s
cold (24 s of it thumbnails, six ffmpeg workers with `-hwaccel
videotoolbox`), 3.7 s warm, and the four cutlists byte-identical to the
ones planned on 09-05. The page shows everything in §2. One correction on
the way: the cutlist's `at` is printed to 0.01 s, so a row is placed in
its band with 0.02 s of slack (under a frame) and the master's true shelf
is read back from `splice` (three decimals), not `shelf` (two).

### Step 2 — the gate

`approved:` in the header: `read_cutlist` exposes it; `plan` refuses to
overwrite an approved cutlist unless `--replan`; `cut` and
`tools/resolve_build.py` refuse without it unless `--unapproved`; `run`
stops after `plan` and prints `retool review …`. The synthetic card's test
path passes `--unapproved`.

**Verify:** with `approved:` absent, `retool cut` and the Resolve build
exit with a one-line message naming the cutlist; with it present, both run;
`retool plan` on an approved cutlist exits without writing and names the
date; `--replan` writes and the line is gone.

*Built 2026-09-06:* `Cutlist::approved()`, `cut::approved(out_dir)`,
`cut::require_approved()`; the flags are parsed in `main` (positionals
first, `--flags` anywhere after the subcommand). All five checks pass on
the 2026-09-02 card. `run` now ends after `plan` and prints the three
commands (review, resolve, cut); on an already-approved job it skips the
plan and says so instead of refusing.

### Step 3 — `retool review`, the writer

The localhost server of V6 and the page's edit controls of §2. The edit
JSON is `{cutlist, rows: [{n, clip, in, out}], drop: [phrase labels],
header: {…}}`; the server validates (every `at` and every slot's beats
unchanged; `in`/`out` inside the clip; a swapped clip's window re-laid by
the planner's anchor for its role), rewrites the cutlists, re-measures
gains, regenerates changed thumbnails, rewrites the page, and answers with
the new page. `POST /approve` writes the line.

**Verify:** swap the cold open's second tease for another vertical in the
picker, save: the cutlist's row 2 names the new clip, its window sits in the
clip, its thumbnails changed, the `at` column is unchanged to the digit;
drop P5, save: the close moves up one phrase and `splice`/`end` follow;
revive the second bedroom into a montage slot on a card that has one;
approve: the header carries today's date, the page locks, and within two
minutes Resolve holds the project with five timelines and five queued
render jobs, nothing rendered; with Resolve closed, approve still writes
the line and the page names the command to run. A hand edit of the
markdown while the server runs is picked up on reload.

*Built 2026-09-06:* the server is 80 lines of `std::net` in
`src/review.rs` (`review`, `handle`, `apply`, `approve`, `build`); port
7877, `GET /` re-renders the page from the cutlists on disk (which is how
a hand edit shows on reload), `POST /save` validates one cutlist's edit
against the grid, `POST /approve` streams the build log (chunked) into
the page. Verified on the 2026-09-02 card, driven through the browser:
swap the second tease for the porch vertical → row 2 names it, window
centred in the clip (1.767–2.933 s), both thumbnails new, the `at` column
identical to the digit; drop P5 → the close moves from 56.47 to 47.06,
`splice`/`end`/`phrases`/`patterns`/`runtime` follow, the three rows go
to `skipped` with the operator's reason; the card has no montage phrase,
so st13 was revived into P4's detail four instead → row 15 names
`DSCF1124.MOV`, centred, gain measured (+2.0), st13 gone from `skipped`;
a drag swaps the two fours of P1 and the slot times stay; a slip of +¼ s
keeps the window's length; approve → `approved: 2026-09-06` in the
header, `/save` refused with the date, the Resolve project rebuilt in
38 s with five timelines and five queued jobs, nothing rendered. The
"Resolve closed" branch was not exercised (Resolve was up): the code
path prints the failure and the `retool resolve` command into the log.
Two choices the plan left open: Approve passes `--replace` (a
re-approval always follows an explicit `--replan` or a hand edit, so the
project in Resolve is the old plan's), and a save answers "ok" and the
page reloads rather than carrying the new page in the response. Revive
is the swap picker's first group ("skipped — revive"), so a skipped
station offers its clips, station clip first. The `header` field of the
edit JSON is merged into the cutlist header but the page has no control
for it yet; the new overrides of §1 (`floor:`, `feature:`, `teases:`,
`cold_open:`) are not read by `plan` yet — only `music:` and `shelf:`
are.

### Step 4 — `retool resolve` and the run chain

`retool resolve <job_dir> <out_dir> [--render none|check|all] [--template
<drp>]` runs `tools/resolve_build.py` with the gate checked and the same
flags; `run` prints the sequence: review, approve, resolve. README's
workflow block and the process table's VIDEO row say the same.

**Verify:** on the approved 2026-09-02 cutlists, `retool resolve … --render
all` rebuilds the project and the five files as `tools/resolve_build.py`
does today, in the same time; without approval it refuses.

*Built 2026-09-06:* `review::resolve` checks the gate and runs
`python3 -u tools/resolve_build.py` from the working directory with the
flags passed through (`--unapproved` reaches the script too, which
honours it). On the approved 2026-09-02 cutlists, `retool resolve …
--render all --replace`: build 39 s, five renders 41 s, 80 s wall-clock,
the five files in `out/video/resolve/`; without approval it refuses
naming the cutlist. README's workflow block and the process table's
VIDEO row say the sequence: run → review → (approve builds) → Render All
in Resolve, `resolve --render all` for the no-QC path, `cut` the
fallback.

## 4. Risks and how each is caught

- **Thumbnails of log footage read wrong** (too dark, wrong colours) → the
  cubes are the same the ffmpeg fallback uses; a thumbnail is for
  recognition, and the page says so under the first one.
- **An edit breaks the grid** → the server rejects any change to `at` or
  beats and says which row; the page never offers a control that could.
- **A re-plan after edits** → V3; `plan` refuses, `--replan` is explicit.
- **The server and Resolve disagree** after a Resolve-side edit → they
  never share state: the cutlist is the plan, the project is the cut. A
  change made in Resolve is a QC change and stays there; a change that
  should recur goes into the cutlist and the project is rebuilt.
- **Port in use / browser blocked** → the static page still opens; the
  message names the file.

## 5. Handoff — starting the build in a fresh session

Read `CLAUDE.md`, then `README.md`, then this file. The state on 2026-09-06
(evening: all four steps built and verified, notes under each step):

- **Built and verified:** `retool clips` → `plan` → `review` → `resolve`
  (`cut` the fallback) in `src/video.rs`, `src/cut.rs`, `src/review.rs`;
  `tools/resolve_build.py` builds the Resolve project from
  the cutlists (RESEARCH-resolve-handoff.md); the grammar in `src/cut.rs`:
  master in fours and eights with the cold open, 8-4-4 reveals, 4-4-8
  arrivals, 4-4-4-4 montages, the fit floor (RESEARCH-video-edit.md §9,
  MUSIC.md "The grammar"); reels in twos with purposes `mls`, `details`,
  `feature`.
- **Where things are in `src/cut.rs`:** `plan()` (entry; header overrides
  read from the existing cutlist), `build()` (the master: units, tiling,
  cap, shelf), `build_reels()`, `lay()` (slot → window, the anchors),
  `measure_gains()`, `read_cutlist()` / `write_cutlist()`, `locate()`,
  `read_job()` (`Feature:`), `read_stamps()` (MUSIC.md by column name),
  `anchor_for()` / `window()` (the slot → window rule, shared with the
  review's swap), `approved()` / `require_approved()` (the gate).
  `Item` is a shot in beats with an `Anchor`; `Phrase` carries items and
  its label; the header `patterns:` line is the labels joined by ` · `.
  `src/main.rs` dispatches subcommands in a `match` near line 49 and
  `run()` calls the chain near line 345.
- **Test material:** `jobs/2026-09-02_test-house/` (card, video, phone,
  out with cutlists, `out/video/resolve/` with beds, cards, renders and the
  `.drp`); `test/synth/` for a card without a shoot. `out/video/frames/`
  already holds mid-frame JPEGs from `retool clips` (room typing); the
  thumbnail cache is a sibling, `thumbs/`.
- **Conventions that bind the build:** one binary, subcommands, no CLI
  framework; stages are plain functions, individually runnable; shell out
  to ffmpeg the way `fuse` shells out to enfuse; verify by eye; amend the
  doc that made a decision with a dated note. Environment quirks in
  CLAUDE.md (cargo PATH, ffmpeg hwaccel, Resolve scripting).
- **Not in scope here:** the Resolve playback-rate template (`--template`,
  documented), the shot-list merge into one file (RESEARCH-resolve-handoff
  §5, deferred), the master's support floor value (30, in `FIT_FLOOR`).
- **Left for next:** the §1 header overrides `plan` does not read yet
  (`floor:`, `feature:`, `teases:`, `cold_open: off`) and a page control
  for them; the read-only page opened from `file://` was checked only by
  its data this session (the browser driver blocks `file:`), the served
  page by eye.
