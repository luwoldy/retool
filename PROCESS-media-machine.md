# The media machine — one stale listing in, one relaunch out, and a loop that compounds

Decided 2026-09-01 with the operator. Three calls set the shape:

1. **Deliverable: the full re-media package** — replace the listing's photos, add
   the MLS video + reels. (Photos sell the house; agents want one vendor; Zillow
   Showcase only plays video from the listing's own photographer.) This supersedes
   the video-only product in `auto_v2/realestate/business.md` — noted there.
   *Correction 2026-09-29:* the standard package is **12–20 photos (aerials on top), one MLS
   video, drone shots, and a floor plan**. The operator already holds Part 107
   and owns a drone. The pipeline handles neither drone files nor floor plans
   yet: `card` reads `-ext RAF` only, so a drone card is ignored, not rejected.
2. **Operator: the artist shoots now, designed for handoff.** Every step is
   written so a hired shooter's card is accepted or rejected by the tool, not by
   taste. Leaving the field is a planned milestone, not a hope.
3. **Loop signal: listing outcome per job.** Portal saves/views before vs after,
   status, days to contract. It becomes the proof line in the next cold call and
   the only honest test of "sells faster".

The machine carries the work. The human carries four judgments: which listing
to call, what to say, whether the gallery is good enough, and what the relaunch
note should say. Everything else is a command.

## The unit: a job

One stale listing (30+ days on market, at least one price cut, an exposure gap
or a named objection — never a price-only problem) → shoot → package → relaunch →
measured outcome → one row in `jobs.tsv`.

## The loop

```
        proof line + sample gallery (MEASURE → CALL)
   ┌────────────────────────────────────────────────────────────────────┐
   │                                                                    │
 FIND ─▶ CALL ─▶ BOOK ─▶ SHOOT ─▶ GATE ─▶ EDIT ─▶ QA ─▶ DELIVER ─▶ MEASURE ┘
  m       M       M       M       m       m      M       M          M→m
                          ▲       │              │
                          │       └─ deviations ─┤   approved finals → own realm
                          └── runbook edits ◀────┘   manual fixes → pipeline backlog
```
M = human judgment, m = machine. Each step has an owner, an input → output, and
a gate that must pass before the next step starts.

| Step | Owner | Input → output | Gate | Tool |
|---|---|---|---|---|
| **FIND** | machine | MLS/portal feed → today's call list of stale listings with a visible gap (weak photos, no video, cut-and-nothing-else) | filter: 30+ DOM, ≥1 cut, not price-only (A10-06) | dial-page briefs (separate repo) |
| **CALL** | human | brief → booking or a dated no + reason | A9 script: price-drop opener → "another cut, or a marketing push?" → listing diagnostic (saves, zip DOM) → sample link → "yes, photos and video, one visit" | dial page |
| **BOOK** | human | booking → `jobs/<date>_<slug>/job.md` with **before** numbers captured the same day | before row written in `jobs.tsv`; seller prep list sent | `jobs/TEMPLATE/job.md` |
| **SHOOT** | human (→ hired shooter) | house → one CFexpress card of stations + one SD card of gimbal clips, in ≤ 90 min | `RUNBOOK-shoot.md` followed; leave gate below | X-H2, 10-18, V1, gimbal |
| **GATE** | machine | card → per-station compliance report + verdict, in seconds, **at the door** | `CLEAN` and complete stations ≥ the station list; any `PART` station is reshot before leaving | `retool card <card>` |
| **EDIT** | machine | card → one folder: `out/stNN.jpg` finals (fuse → pop → blend → undistort+verticals → tone) + `scorecard.md`, a card per image | every complete station produces a final; failures listed | `retool run <card> <out>` |
| **QA** | human + machine | finals → ordered 2048px gallery + fit score + room coverage | `deliver` reports coverage complete; eye pass at 1400px on the competence bar (white walls white, no halos/ghosts, verticals plumb, windows not black, TVs off) | `retool deliver realm.json <out> <delivery>` then `sips -Z 1400` |
| **VIDEO** | machine plans, human approves | station clips + connective gimbal clips → 16:9 MLS cut (branded + unbranded, 1080p30 H.264 8–16 Mbps, ≤120 s) + 3 × 9:16 | planned per `RESEARCH-video-edit.md` on the music grid; the plan is looked at as a page with thumbnails and signed off (`approved:`) before anything is built (`PLAN-review.md`); QC in Resolve | `retool run` plans → `retool review` (swap/slip/reorder/drop/revive, Approve builds the Resolve project) → Render All in Resolve; `retool resolve … --render all` skips the QC pass; `retool cut` is the no-Resolve fallback |
| **DELIVER** | human | gallery + video + hosted link + **relaunch note** → agent, within 48 h | hosted link renders on Realtor.com from Canopy's Virtual Tour URL field; payment on download | zip + host + email |
| **MEASURE** | human → machine | listing at +7, +14, +30 days → row updates | after numbers in `jobs.tsv` | portal pages, `jobs.tsv` |

**Rejects at QA are logged, never silently fixed.** Every manual touch
(Capture One, Photoshop) is a line in the pipeline backlog at the bottom of
this file. That is how the machine gets better instead of the human getting
busier.

## The feedback edges (why this is positive feedback)

1. **Outcome → call.** Each measured job adds to the proof line. Until five
   jobs exist, the pitch is the listing-level diagnostic plus the sample
   gallery (A9-04, A9-05). From five on, it is "of the N stale listings I
   re-shot, K went under contract inside 30 days" — a claim the research says
   nobody in the market can currently make with controls.
2. **Approved finals → own realm.** Every gallery the agent accepts goes into
   `photocorpus-own/`; a realm built from it answers "is this shoot at my
   standard?" independent of the market corpus. Culling gets sharper with
   every job at zero cost.
3. **Deviations → runbook.** `retool card` prints the recipe deviations; a
   deviation that recurs is either a runbook bug or a recipe bug, and the doc
   is amended with a dated note (already happened: pops at 1/60 vs 1/125).
4. **Manual fixes → backlog.** The next thing to build is whatever the human
   touched most last week. No speculative features.
5. **Rehire → FIND.** An agent who rehires has other stale listings; their
   inventory goes to the top of the call list.

## The ledger — `jobs.tsv`

One row per job, tab-separated, edited by hand until it earns automation.

| column | when | what |
|---|---|---|
| `job` | book | `YYYY-MM-DD_slug` — also the folder name under `jobs/` |
| `address`, `agent`, `listing_url` | book | identity; the URL is the measurement page |
| `list_price`, `dom`, `cuts`, `photos`, `has_video` | book | the listing as found |
| `views_0`, `saves_0` | book | portal counters the day of booking |
| `shot`, `delivered` | shoot / deliver | dates — the 48 h promise is checked here |
| `paid` | deliver | amount actually collected (0 until download) |
| `views_7`, `saves_7`, `views_30`, `saves_30` | measure | same counters at +7 and +30 days after delivery |
| `status_30` | measure | active / pending / contract / sold / withdrawn / relisted |
| `days_to_contract` | measure | from delivery; blank until it happens |
| `rehire` | +90 d | y/n |
| `notes` | any | what was wrong with the listing, what changed, agent quotes |

Per-job folder: `jobs/<job>/{job.md, card/, video/, out/, delivery/}`. Only
`job.md` is committed.

## Handoff plan (operator → shooter)

- **Jobs 1–5: operator solo.** Runbook edited after every job; station list and
  time budget converge. `retool card` is run at the door every time, even
  though the operator does not need it yet — it is being calibrated.
- **Jobs 6–10: operator with a stopwatch.** Shoot must fit 90 min with the
  runbook as the only reference. Anything the operator does that the runbook
  doesn't say gets written down.
- **Shooter candidate shadows two jobs, shoots one with the operator present
  for the gate, then solo.** Their card is accepted by `retool card` + the
  coverage check in `retool deliver`, or it is reshot. The operator keeps CALL,
  QA, the relaunch note, and MEASURE.
- **QA leaves the operator last**, once the own-work realm flags what the
  operator's eye flags.

## Honest constraints (from the research, so the process doesn't lie to itself)

- "Sells faster" is unsupported, not refuted. The ledger is how it becomes a
  claim; on the call it stays a listing diagnostic until then.
- Zillow plays video beside the photos only on Showcase listings, and only from
  the listing's own photographer. Being the photographer now opens that door;
  the agent still has to be paying for Showcase. Realtor.com and the MLS show
  the tour link for everyone.
- A listing whose only problem is a falling-submarket price is not saveable by
  media and is not worth the call. The FIND filter is part of the machine.
- Delivery is not done until the hosted link renders on Realtor.com.
- Price of the full package is not settled: the research puts full packages
  at $200–500 in most markets, $600 at the top of the standard tier. Decide
  before job 1; the package must look like the reason on contact.

## Machine backlog, in loop order

Done 2026-09-01: lens undistortion from the RAF's own table + verticals
correction (`level`), the door-side card gate (`card`), the ordered delivery
export with room coverage (`deliver`).

1. **Ghosting detection** between bracket frames → single-frame fallback for
   that station (QA catch st13; the realm found it, the pipeline should).
2. **Window pull** from the darkest bracket — stale listings get shot in
   daylight, so this shows up on job 1. Needs a daytime card to build against.
3. **TV-on / clutter flags** at QA (the most visible tell in the first shoot).
4. **`retool cut`**: assemble the MLS cut from station clips + the
   connective gimbal set. Decisions and the step-by-step build with verify
   lines: `PLAN-video-cut.md` (research in `RESEARCH-video-edit.md`). Steps
   0–3 can be built today on synthetic clips; step 6 waits for the first
   card with video.
5. **Own-work realm** from `photocorpus-own/` (edge 2 above).
6. **`retool ledger`**: prints the proof line and the 48 h hit rate from
   `jobs.tsv`.
7. **Speed**: native Mertens fusion + rayon; a 19-station card currently takes
   ~30 min sequential, target under 10.
