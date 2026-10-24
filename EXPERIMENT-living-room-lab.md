# Experiment — living room lab: 13 mm and Manual flash (2026-09-18)

Tests the two 2026-09-15 recipe changes from the Cool book on one real
living room: the **13 mm default** (his 20 mm full-frame sweet spot,
RECIPE.md §6) and the **Manual pop** (§3). Baseline: the 2026-09-02
test-house living stations st06/st07 were 10 mm, TTL 0 EV, fit 67/69.

Fill in the result sections after the card is processed.

## Flash power, in plain terms

1/1 is full power. Every halving is one stop less light: 1/2, 1/4, 1/8 …
1/128 (seven stops down). The V1 adds thirds between steps. Lower power
recycles faster; full power on a run of pops overheats the head.

## Shot list

Goal: prove the full bracket + pop workflow end to end with **nothing the
pipeline can't take blind**. A small quality drop is fine if no hand step
is needed. Every station below is the plain recipe station: C1 five-frame
bracket, then C2 one pop, tripod untouched between the six frames.

Setup once: tripod at counter height (~90 cm), bubble zeroed, doorway
two-wall composition, WB 5000 K, AF-S on the far wall. Fire one TTL pop,
press TCM, **write down the power it shows** — that is the "TCM power".

| # | Zoom | Pop | What it tests |
|---|---|---|---|
| 1 | 10 mm | Manual, TCM | baseline, matches the test-house living room |
| 2 | 13 mm | Manual, TCM | **the recipe** |
| 3 | 15 mm | Manual, TCM | past the sweet spot |
| 4 | 13 mm | TTL 0 EV | control for the Manual switch |
| 5 | 13 mm | Manual, TCM −1 stop | does the blend recover a dark pop on its own |
| 6 | 13 mm | Manual, TCM +1 stop | does a hot ceiling or near wall survive tone |
| 7 | 13 mm | Manual, TCM, no changes | repeat of #2 for consistency |
| 8 | 13 mm, opposite corner toward the windows | Manual, TCM | the window pull, hero room B angle |
| 9 | 13 mm, #8 position | Manual, TCM, second pop off-camera from the doorway | only if the X trigger arrived; interval timer 5 s |

Stations 1–7 are one tripod position; 8–9 are the second. Move nothing
else. Wait 5 s between stations so grouping keeps them apart. Under 15
minutes of shooting.

Pass criteria for the workflow: `retool card` says every station is
complete, `retool run` produces nine finals with no flag, and #2 at
1400 px looks at least as good as the test-house st06/st07.

## Processing

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
./target/release/retool card jobs/2026-09-18_living-lab/card        # gate: every variant should be a complete station
./target/release/retool run  jobs/2026-09-18_living-lab/card jobs/2026-09-18_living-lab/out
for f in jobs/2026-09-18_living-lab/out/st*.jpg; do sips -Z 1400 "$f" --out "${f%.jpg}_s.jpg"; done
```

Note: the gate will flag the B0 TTL station only if the pop is not at
ISO 320 / 1/125 mechanical; TTL vs Manual is not in EXIF the gate reads.
Aperture is not gated either (§7), so a C variant passes silently.

To see what the pop alone contributes per variant, `retool dev` the
`_DSF` frame and `retool tone` it; the blend with the same stack isolates
the flash difference.

## Results

### 2026-09-18 17:30 — one station, #2 of the list (13 mm, Manual pop)

Frames DSCF1392–1396 + _DSF1399. EXIF on recipe throughout: 13.1 mm,
f/6.4, stack ISO 125 electronic with metered frame 0.8 s (ladder 1/5 …
3 s), pop ISO 320 · 1/125 mechanical, 5000 K.

- **Gate:** split into two partial stations. The pop was fired 226 s after
  the bracket; the grouper absorbs a pop within 45 s. Process fault, not
  recipe: fire the pop straight after the burst. Processed by the stage
  commands instead (fuse → dev → blend 0.35 → level → tone).
- **Blend:** ambient match gain 2.92× (test house: 16×), tone gain 1.65×
  (test house: 4.3×). The Manual pop was well exposed; the pipeline had
  little lifting to do. Verticals rotated 0.68°, crop 0.7%.
- **By eye at 1400 px:** walls white, wood warm, lamps glow through from
  the stack, no halos, no ghosting, verticals straight. Clears the bar and
  beats the test-house living room. 13 mm reads as a room, no edge stretch
  on the near sofa arm.
- **Not tested:** the window pull. The french doors look onto a dark
  space at 17:30; nothing to pull.
- Outputs: `jobs/2026-09-18_living-lab/out/st01.jpg` plus `_ambient` and
  `_pop` for the components.

### 2026-09-18 17:45 — kitchen, two pops (on-camera + off-camera through the pass-through)

Frames DSCF1400–1404 + _DSF1405, _DSF1406. On recipe; pops 15 s and 21 s
after the burst.

- **Grouper bug, fixed.** `group_stations` absorbed a pop only when the
  station had no flash yet, so the second pop (6 s after the first, its
  own burst) became a partial station and `run` blended one pop. The
  recipe's multi-pop rule never reached `composite_pops`. Now any
  non-bracket burst within 45 s joins the station. Gate after the fix:
  `st03 [OK] 5 brk 2 pop`.
- **Composite by eye.** Pop 1 (on-camera, bounced) lit the kitchen; pop 2
  (off-camera, into the far room) lit almost nothing else and blew the
  pass-through on its own, median luma 0.009. The lighten composite took
  the far room from pop 2 and the kitchen from pop 1: the pass-through
  reads as a lit room with the TV visible, no white block, no fringe.
  Ambient match 0.65×, tone gain 3.47×. `st03.jpg` vs `st03_onepop.jpg`.
- Off-camera pop confirms: a hidden pop aimed at the dark part is enough
  even when it is a stop or more under the main pop.

### 2026-09-18 18:01 — living room B angle, three pops

Frames DSCF1407–1411 + _DSF1412–1414. On recipe; pops 24, 47, 70 s
after the burst (each within 45 s of the previous frame, so all absorbed).
Gate `st04 [OK] 5 brk 3 pop`; `run` composited three, ambient match
1.57×, tone gain 2.54×, realm living_room fit 66.

- **Pop 1** (on-camera, bounced): even, the whole room, ceiling clean.
- **Pops 2 and 3** (off-camera): lit the kitchen pass-through and the
  hall, but each also threw a hotspot on the near ceiling, upper left.
- **Composite by eye:** the pass-through and hall are now open and lit,
  which pop 1 alone did not do. But the lighten rule takes the brightest
  pixel wherever it comes from, so the near-ceiling hotspot from pops 2
  and 3 wins and prints as a bright wedge across the top left of the
  final. Usable, not clean. This is the hotspot risk RECIPE.md §3 warns
  about, now seen.
- **Two ways out.** Capture: keep the near ceiling and near wall out of
  the off-camera beam (feather away from the camera, flag with a hand).
  Code: the shadow-aware alpha already queued in the backlog; a first cut
  is "extra pops fill only where pop 1 is dark", which keeps pop 1's
  ceiling and still opens the pass-through.

### 2026-09-18 18:09 — same angle reshot, two pops, power up

Frames DSCF1415–1419 + _DSF1420, _DSF1421. Pops 15 s and 29 s after the
burst. Gate `st05 [OK] 5 brk 2 pop`; composited, ambient match 2.18×,
tone gain 1.99× (three-pop take: 2.54×). Pop 2 median luma 0.177, about a
stop over pop 1 (0.093).

- **By eye:** clean. Ceiling even, no hotspot, pass-through and hall lit,
  lamps glow, walls white, wood warm. The best living-room final of the
  session; `st05.jpg` supersedes `st04.jpg` for this angle.
- **Why it worked where st04 did not:** both pops were bounced up and
  behind from positions that kept the near ceiling out of the beam, and
  the higher power let two pops cover what three did before. The lighten
  composite had nothing hot to pick.
- The lower tone gain (2.0× vs 2.5×) is the higher power showing; the
  pipeline is passing the pop through rather than lifting it.

### 2026-09-19 15:08 — bedroom, midday, one window; four brackets to meter

Frames DSCF1423–1442 (four 5-frame brackets, metered at 1/10, 1/5, 1 s,
0.5 s, 18–20 s apart) + _DSF1443 (one pop, 13 s after the last).

- **Grouper: re-meters.** Before: three pop-less PART stations and one
  OK. Now a pop-less bracket burst followed by another bracket burst
  within 60 s is dropped as a re-meter, printed as such; the card shows
  one OK station. Only the bracket the pop follows counts.
- **Window, measured** on a 1200×1000 crop of the window (fraction of
  pixels ≥250/255 after tone):

  | version | clipped | mean |
  |---|---|---|
  | pop alone (1/125, ISO 320) | 0.0 % | 141 |
  | −2 frame (1/8, ISO 125) | 7.1 % | 141 |
  | fused stack (enfuse) | 23.4 % | 203 |
  | **final (blend)** | 1.2 % | 170 |

  The pop holds this window completely: a shaded, north-ish window at
  1/125 does not clip. The blend then makes it slightly *worse*, not
  better, because enfuse loses the window (the −2 frame's view does not
  win the fusion weights) and the 0.35 ambient luma mix lifts the pane
  toward that blown fused value. By eye: fence, greenery and umbrella
  read in the final, with a light haze the pop alone does not have.
- **What this says about the window pull.** Two separate problems:
  1. *Shaded windows* need no pull; they need the blend to stop lifting
     bright flash pixels toward a brighter ambient. Cheap: cap the target
     luma at the flash luma where the flash pixel is already bright.
  2. *Sunlit windows / sky* will clip the pop, and then colour has to come
     from the −2 frame (not the fused stack, which blows it too). That
     is the real mask, and this station cannot test it: the pop has no
     clipped pixel in the window.
- Verticals left as shot (weak consensus, 2272/11044 edges): a room with
  few straight verticals; fine by eye.

### Decision

Recipe holds as written. Runbook: every pop must follow the burst within
45 s or the tool sees two stations. Multi-pop stations now composite from
`run` with no hand step; the lighten rule lets an off-camera hotspot win,
so the next code step is fill-only-where-dark. Stations 1, 3–9 of the list still open. Window pull: next station must
face a sunlit window or open sky so the pop clips there.
