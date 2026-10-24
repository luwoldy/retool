# The recipe — every camera setting, and why it is that number

`RUNBOOK-shoot.md` is the procedure; this is the settings behind it and the
reasoning. The recipe is the operator's, recorded 2026-08-31. The reasoning
was written down 2026-09-06 after "why not f/8 like every book?" turned out
to have no answer anywhere in the repo. When a number changes, change it
here with a dated note, then in the runbook and `SHOOT-SHEET.html`, then in
the gate (`run_card` in `src/main.rs`) if it is one the gate checks.

## 1. The rig, and the facts the numbers hang on

| Part | What matters |
|---|---|
| X-H2 | 40 MP APS-C, 7728 × 5152 on 23.5 × 15.6 mm: **3.04 µm pixels**. Base ISO 125. Mechanical sync 1/250; **flash will not fire under the electronic shutter**. Crop factor 1.53 |
| Sigma 10-18 f/2.8 DC DN | the wide: every room, the gimbal pass. 15–27 mm full-frame equivalent |
| Sigma 18-50 f/2.8 DC DN | details past 24 mm, the exterior from the street |
| Godox V1 | round head, about 5600 K, TTL on the X-H2 |
| Tripod with a bubble level, 2 s timer | the camera never moves between the six frames of a station |
| DJI Mavic Air 2 | 1/2" 48 MP Quad Bayer (12 MP native binning), **fixed f/2.8**, 24 mm equivalent. 570 g: Part 107, registered, no flight over people. ND kit 16/64/256. Its own microSD card (§5a) |

## 2. C1 "RE-BRKT" — the bracket stack

| Setting | Value | Why |
|---|---|---|
| Mode | M | The ladder varies one thing, the shutter. Aperture and ISO are fixed so the five frames differ in exposure time only; fusion assumes it and the gate checks it |
| Aperture | **f/6.4** | See §8. It is the book's f/8 translated to a 40 MP crop sensor, not a softer choice. *2026-09-15:* in a **small room** with something near the lens that must be sharp, go to **f/8**, and move C2 with it: the pop and the stack must share an aperture (§3). The gate does not check aperture (§7), so this is on the operator |
| Focus | **AF-S**, single point on the **far point** of the room; on the **subject** instead when something nearer the camera has to be sharp | *2026-09-15, from the book.* Far-point focus at 13 mm puts the hyperfocal behind the camera and everything past a metre is sharp; a near subject (island edge, bed foot, chair) is the one case the far point loses |
| ISO | **125** | The X-H2's base ISO, and the only ISO DR100 allows. On a tripod with a timer there is no reason to leave base; the book's ISO 400 kept a 2012 sensor's shutters fast |
| Bracket | 5 × 1 EV, −2 … +2, order 0 → − → + | The −2 frame holds windows and bulbs (it is the window-pull source); +2 opens shadows. 1 EV steps keep the fusion weights smooth; wide gaps band at windows (`RESEARCH-rust-editing.md`). Five fixed frames make every station the same shape on the card, which is what lets the gate check a card in seconds. Metered frame first so the burst leads with the composition frame; the tool matches the set, not the order |
| Shutter rule | metered frame **1/30 s or slower** indoors | Puts the −2 frame at 1/125 or slower: about one full mains-flicker cycle (120 Hz on 60 Hz mains) per sensor row, so LED banding cannot print under the electronic shutter. The gate reports a faster metered frame. *2026-09-06:* a daylit room meters faster and that is fine; see §10 |
| Shutter type | electronic | No shutter shock across five frames, silent, no wear. The costs, rolling shutter and flicker, are covered by the shutter rule and by "nothing moves in the frame" |
| IS | off | On a tripod, stabilisation can drift between frames and misregister the stack |
| Timer | 2 s | Lets the press settle; also the shooter's step out of a mirror |
| File | RAF lossless compressed, 3:2 | All the data at about half the size of uncompressed; rawler decodes it. 3:2 is sensor-native and `level` crops back to it after the verticals |
| WB | **5000 K, fixed per house, never AWB** | One white balance across the set: processing consistency is the top-decile separator (`RESEARCH-shoot-craft.md` §5). 5000 K sits between daylight and the V1, so the flash frame, which supplies the colour of every final, is near neutral. AWB is a per-frame guess that changes room to room; the book's AWB-then-eyedropper is a Lightroom workflow and the pipeline has no eyedropper step |
| DR | **DR100** | DR200 and DR400 expose the sensor one or two stops darker than the reported ISO and force ISO floors of 250 and 500. They would break the ladder and the base-ISO rule; highlight range comes from the −2 frame instead |

Height and level are in the runbook: stills at **counter or furniture
height**, about 90 cm, in most rooms, higher only under a very high
ceiling; exteriors at door or eye level, roll and pitch at zero
(`RESEARCH-shoot-craft.md` A6). *2026-09-15, from the book:* was about
120 cm keyed to the use-plane with kitchens 50 cm above the counter
(2026-09-06); the book's counter-height rule supersedes it.

## 3. C2 "RE-FLASH" — the pop

| Setting | Value | Why |
|---|---|---|
| Aperture | **f/6.4**, same as C1 (**f/8** when C1 moved to f/8 for a small room, *2026-09-15*) | The blend takes colour from this frame and luminance from the stack. A different aperture changes the lens's vignetting field, and the mismatch would print as a ring in the final |
| ISO | **320** | 1⅓ stops of flash efficiency at base-level noise: more bounce reach, the V1 stays off full power, faster recycle. Noise at 320 is invisible in a blended frame |
| Shutter | **1/125, mechanical** | Flash needs the mechanical shutter on Fuji. 1/125 is under the 1/250 sync and holds the ambient to a fraction of the flash, so the pop's colour is the flash's. Lamp mood reaches the final through the stack via the blend, not through the pop. *2026-08-31:* every pop on the first card was 1/60; that was habit, not recipe, and the gate flags it (`EXPERIMENT-first-shoot.md`) |
| Flash | V1 **Manual**, bounced 45–90° up and behind, never at the wall being shot. Starting powers at f/6.4 · ISO 320: bedroom/bath bounced ~1/4, living/kitchen bounced 1/2–1/1, direct feathered at a far wall from a doorway 1/8–1/16. At the first station of the house fire one TTL shot, press TCM to lock its manual equivalent, and leave it | A large soft source from the ceiling behind the camera. *2026-09-15: was TTL 0 EV.* TTL meters the whole frame off a pre-flash: fine for one bounced pop from the tripod (the blend exposure-matches the ambient to this frame either way), wrong for a hidden pop at a dark corner (it meters the bright half of the room and starves the corner) and inconsistent across several pops in one room (the lighten composite then picks the brightest, not the right one). Manual makes every pop the same known light. TTL stays the fallback for a room where nothing bounces. Bounce keeps the flash out of mirrors and glass |
| Count | one; two or three in a big or L-shaped room, within seconds | The tool composites multiple pops by lighten. *2026-09-15:* the extra pops may be **off-camera** (Godox X trigger on the shoe, V1 in hand) fired from a doorway or behind a wall at the part of the room the tripod pop cannot reach, camera on the interval timer (one frame every 5 s, 3–4 frames, re-aim between frames). Bounce or feather off the nearest wall or ceiling that faces the dark area, not a direct blast; keep the near wall out of the beam (a hotspot wins the lighten composite). Keep the flash out of reflective surfaces and yourself out of frame |

## 4. The video bank

`PLAN-video-cut.md` D7, corrected 2026-09-03 after the first video card.

| Setting | Value | Why |
|---|---|---|
| Format | 4K 60p, **H.265 4:2:2 10-bit** | Every clip is conformed 2× to 30p in the edit (D2). Log in 8 bits bands in flat walls; the first card was H.264 8-bit and showed it |
| Profile | F-Log2 | One grade, colour-managed in Resolve (`RESEARCH-resolve-handoff.md`). F-Log2's floor is ISO 1250 |
| Shutter | 1/125 | 180° at 60p |
| Aperture | f/2.8–4 | At 10–12 mm depth of field is deep at any aperture; the stop buys light. The first card was 1.5–2 stops under at f/4 with the default auto-ISO ceiling |
| ISO | auto, ceiling raised; zebras at 70 % on a white wall | Auto ISO carries the walk between rooms; the zebra is the exposure check, since log looks dark on the screen |
| WB | 5000 K | Matches the stills. Lamp-lit rooms go warm under it; the grade handles that per clip |
| Outdoors | variable ND, or f/8 at 1/500 | Daylight at ISO 1250 needs the ND to hold 1/125; the fast-shutter fallback trades motion blur for not carrying the filter |

## 5. The phone

Blackmagic Camera app, 4K 60p, H.265, Apple Log, 1/125, WB 5000 K, ISO auto,
portrait, HDR off. Why: a 9:16 crop of a horizontal wide keeps a third of
it, so the reels need footage shot vertical, and the phone is native
portrait with no re-rig. Apple Log grades beside F-Log2 in the
colour-managed Resolve project. ProRes at 4K 60p records only to an
external drive; HDR would fight the grade. Older than an iPhone 15 Pro:
the standard profile with HDR off. Decided 2026-09-03 over an L-bracket on
the Fuji (D11).

## 5a. The drone (*2026-09-29*)

Aerials are part of the standard package, on top of the 12–20 photos.
Settings are from the Air 2's published specs; the first drone card
confirms them (AEB writing DNG is the one to check).
*Correction 2026-09-29, first test card (16 DNG + JPEG, ground-level):*
AEB does write DNG: 4000 × 3000, 16-bit uncompressed RGGB, with DJI's
lens correction as DNG opcodes (GainMap, WarpRectilinear), which rawler
0.8 does not apply. The ladder is 0, −0.7, +0.7, −1.3, +1.3 (2.7 EV), the
five frames within a second. WB held one AsShotNeutral across every
bracket. The card was shot in **manual** exposure (1/120, ISO 100 then
400, the video numbers), so the ladder hung off a fixed shutter and the
indoor sets came out black even at +1.3. Photo exposure must be **Auto**:
AEB centres on the meter only then.

**Photo**

| Setting | Value | Why |
|---|---|---|
| Mode | **AEB 5 frames**, 0.7 EV steps | A 2.8-stop ladder: enough outdoors, and the merge only runs when the middle frame clips. Castle shot every aerial as one auto frame; his sensor was not a 1/2" one, and midday sun against shade is where this one clips. It costs a second in the air |
| Resolution | **12 MP** | 48 MP is a Quad Bayer remosaic, noisier per pixel, and disables AEB |
| Format | JPEG + RAW (DNG), 4:3 | The pipeline develops the DNG; 4:3 is sensor-native |
| Exposure | Auto, EV 0 | Daylight lands at ISO 100, about 1/1000–1/2000, Castle's numbers |
| WB | **Manual 5000 K** | The house's white balance (§2), so aerials sit beside the ground exteriors |

**Video**

| Setting | Value | Why |
|---|---|---|
| Format | **4K 60p, H.265**, 120 Mbps | Conformed 2× to 30p like the bank (§4) |
| Profile | **D-Cinelike** | The Air 2's only flat profile. 8-bit, so expose right and grade gently; it bands sooner than F-Log2 10-bit |
| Exposure | Manual, **ISO 100**, shutter **1/120** | 180° at 60p |
| ND | **Sun: ND16**, shutter about 1/200. **Overcast: none** | ND32 would hold 1/120 in sun; the kit has 16/64/256 and ND64 is too dark. Slow moves and the 2× conform forgive the faster shutter |
| WB | Manual 5000 K | Matches the stills |
| Flight | **Cine mode**, gimbal pitch speed low | Speed caps keep moves smooth |
| Video subtitles | **On** | An `.SRT` beside each clip with altitude and GPS, for typing aerials without looking |

**When not to fly** (never delays delivery; note it in `job.md`): heavy
tree canopy over the house (Castle: "I wouldn't have normally recommended
it"), sustained wind over about 15 mph, rain, LAANC denied, or no way to
keep off people and moving cars. Check airspace at booking, not on site.

## 6. Lenses and focal lengths

- **10-18 for every room wide.** Default **13 mm** (about 20 mm full-frame);
  a little wider, 11–12 mm, when the room calls for it; **never 10 mm** for
  stills (`RESEARCH-shoot-craft.md` A8: over-wide framing is a trust
  problem, not a style choice). *2026-09-15, from the book:* was default
  11 mm with 10 mm allowed for a tiny room; 13 mm is the book's sweet spot
  and sits on the conservative side of A8's 16 mm-equivalent. The gimbal
  pass still runs at 10–12 mm.
- **18-50 past 24 mm for details**, one per hero room, a front-door or
  porch detail at station 1 when the build quality shows (*2026-09-06*, from
  the book), and from the street for the front exterior. The tool reads any frame longer than the room's
  bracket lens within four minutes of the station as that station's detail;
  18 mm is ambiguous, so zoom past 24.
- The book's 17 mm full-frame floor and its "11 mm stretches the edges"
  captions agree with A8.

## 6a. Composition (*2026-09-15, from the book*)

- **Two-wall** composition is the default: the camera angled so two walls
  and their corner carry the frame.
- **One-point** perspective when the room calls for it: a symmetrical room,
  a hallway, a view straight through to a window.
- **One wall** is an option, not a default: a feature wall, a vanity, a
  headboard.
- **Rule of thirds** for anything being highlighted, and for almost every
  detail shot: the subject on a third, not centred.
- Tension with `RESEARCH-composition.md` §3, which reads the realm as
  "hero rooms: go one-point": the book makes two-wall the default and
  one-point the exception. Runbook follows the book; the realm profile
  will say which the references actually favour.

## 7. What the gate checks

Per station, from EXIF alone (`retool card`): six frames; the ladder set
{0, −1, +1, −2, +2}; bracket ISO 125; metered frame 1/30 s or slower; pop
ISO 320 with the flash fired; electronic shutter on the stack, mechanical on
the pop; DR100; WB mode fixed. **Aperture is not checked** (2026-09-06), so
a move to f/8 would pass the gate unnoticed. Add the check before relying on
it.

## 8. Why f/6.4 and not f/8

Every f/8 in the literature is a full-frame number. Castle shoots a 5D Mark
III: 22 MP on 36 × 24 mm, 6.25 µm pixels. Depth of field and diffraction
scale with the crop factor, so:

| | Full frame f/8 | X-H2 f/6.4 | X-H2 f/8 |
|---|---|---|---|
| Full-frame equivalent | f/8 | about f/9.6 | about f/12 |
| Airy disk at 550 nm | 10.7 µm | 8.6 µm | 10.7 µm |
| Airy disk in pixels | 1.7 | 2.8 | 3.5 |
| Hyperfocal at 10 mm, CoC 0.02 mm | | 0.78 m | 0.63 m |

- **Diffraction.** With 3 µm pixels the Airy disk passes two pixels at about
  f/4.5. f/6.4 is already past that; f/8 is another two thirds of a stop into
  the hole. Reviews of the body put peak sharpness at f/4 to f/5.6, f/8
  acceptable, f/11 visibly soft. f/6.4 takes most of the corner gain the
  10-18 gets from stopping down and stops there.
- **Depth of field is not the constraint.** At 10 mm the hyperfocal distance
  is under a metre at either aperture; a room is sharp front to back.
- **The bracket budget.** f/8 costs two thirds of a stop. With ISO at base
  and the metered frame at 1/30, the +2 frame moves from about 1/8 s to
  about 1/5 s; after dark, from seconds to longer seconds. Longer brackets
  are more ghosting from curtains and fans, the top pipeline gap.
- **The flash budget.** The bounced V1 keeps two thirds of a stop
  more headroom before full power, which is the difference in a big room
  and in recycle time between pops. (*2026-09-15:* Manual now, not TTL;
  the headroom argument is unchanged.)
- **Equivalence.** In full-frame terms the recipe already runs deeper than
  the book's f/8. The sources and the recipe agree; the recipe is the same
  choice on a different sensor.

## 9. Sources that say otherwise

| Source says | Where | Why the recipe differs |
|---|---|---|
| f/8 always | Castle 2018, most full-frame guides | §8: full-frame number; on 3 µm pixels the equivalent is f/5.3 |
| ISO 400 to keep shutters fast | Castle | Tripod and timer make shutter length free; base ISO is cleaner and DR100 needs it |
| Auto white balance, fix by eyedropper | Castle | Set consistency and a pipeline without a hand step; RAW makes the tag free either way |
| Bracket "until no blown highlights", four to five frames | Castle | A variable count cannot be gated; five fixed frames with −2 cover everything short of direct sun |
| Never deghost | Castle | Agreed on the tool; the recipe's answer is nothing moves in the frame, plus ghost detection in the backlog |
| 20–25 photos, the MLS cap | Castle | A 2018 limit; check Canopy's before capping `deliver` |
| Wider is better, 12–14 mm | volume-market agents | A8: buyers call it deception; 16 mm equivalent default, 14 only for tiny rooms |

## 10. Open tensions

1. **After dark** the fixed aperture and base ISO run the +2 frame to
   seconds; the first card drifted bracket ISO to 200–1600 on eight of
   fifteen stacks. Accept the long frames (tripod, electronic shutter, the
   pop carries night stations anyway), or allow ISO 250 after dark as a
   documented exception. Undecided.
2. **Bright daylight rooms** meter faster than 1/30. *Decided 2026-09-06:*
   accept it. The rule exists for LED banding under the electronic shutter,
   which needs LED light to dominate the frame, and a room where it does is
   dim enough to meter at 1/30 anyway. ISO and aperture stay put; the gate
   notes the fast frame and the note is not a deviation on a daytime card.
   Runbook step 5 says so.
3. **Shower door**: *decided 2026-09-06:* open when the tile is the feature
   and the pop can be bounced away from the glass, closed otherwise. The
   runbook's station 11 carries it.

## Change log

- 2026-08-31 — recipe recorded from the operator; first card found the
  1/60 pops and the after-dark ISO drift.
- 2026-09-03 — video bank corrected: H.265 10-bit, raised auto-ISO ceiling,
  f/2.8–4; verticals moved to the phone.
- 2026-09-06 — this document; the f/6.4 reasoning written down; aperture
  found unchecked by the gate. Same day, the v2 capture refinements from
  v1 and the book went into the runbook: daylight by facade bearing, the
  horizontal rule, kitchen height as a number, the edge check, the camera in
  mirrors, three fronts, the stairs still, the exterior detail, the prep-list
  additions; tensions 2 and 3 above decided.
- 2026-09-15 — from the book: pop to Manual with TCM, off-camera pops via
  the X trigger (§3); 13 mm default and never 10 mm (§6); counter height,
  superseding 120 cm and kitchen counter+50 (§2 note); AF-S far point (§2);
  f/8 allowed in small rooms, both banks together (§2, §3); composition
  defaults (§6a).
- 2026-09-29 — the drone (§5a): Mavic Air 2 photo and video settings, the
  no-fly rules. Aerial shot list in the runbook. The gate does not read the
  drone card yet (`card` takes `-ext RAF` only).
