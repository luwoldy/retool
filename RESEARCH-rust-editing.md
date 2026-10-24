# Can Rust replace the outsourced editor? (brackets + pop → delivery)

Research 2026-08-31, two passes: (A) ground truth on what outsourced editors
and AI auto-editors actually do to a bracket+flash set (HDRsoft/Photomatix
docs, PhotoUp/BoxBrownie published specs, PFRE, Nathan Cool method writeups,
~30 r/RealEstatePhotography threads); (B) Rust feasibility per pipeline stage
(crates.io/docs.rs/GitHub verified same day). Companion: decode/EXIF/analysis
stack in RESEARCH-rust-imaging.md; input spec is the C1/C2 recipe.

## 0. Headline verdict

**The assumption survives: a competent Rust auto-edit of brackets + pop is
buildable, and the recipe was unknowingly designed for it.**

1. **The volume-market merge engine is exposure FUSION (Mertens), not
   tone-mapped HDR** — Photomatix's RE presets, LR/Enfuse, and the AI tools
   all converge there; tone-mapping is the amateur tell. Mertens is ~500 LOC
   from the IPOL 2018 reference, or a v1 shell-out to `enfuse` — the exact
   binary part of the industry already uses (PFRE sold a 145-page
   "Enfuse for RE" course; PhotoUp teaches LR/Enfuse).
2. **The recipe removes fusion's known failure modes operationally.** Fixed
   5000K WB across frames = the documented case where fusion is safe and
   color work is deterministic. Tripod + 2s timer + static scenes = no
   ghosting (Photomatix's own RE guidance: deghosting OFF; Fusion/Interior
   has no deghosting at all). Fixed ladder = deterministic grouping.
3. **The flash pop is the moat.** NO AI auto-editor accepts a flash frame
   (verified across Autoenhance/AutoHDR/RealtyEdit/Fotello/Imagen), yet the
   flash frame is the market's only robust answer to mixed-WB casts — the
   single hardest editing skill and the #1 reason shooters keep human
   editors. The blend is codified (Nathan Cool stack, §1.5) and is plain
   per-pixel math in Rust.
4. **Where humans still earn their $1–2:** judgment-consistency QC — uniform
   window pulls across a frame, mirror reflections matching, room-to-room
   coherence. 100% of AI-tool adopters still hand-check output. This is
   where the *analysis* half of this tool loops back in: the §5 metric map
   (CCT spread, cross-image histogram distance, verticals) becomes the
   automated QC layer over the editing output.
5. **Economic bar:** humans $0.65–1.50/img (volume sweet spot ~$0.80, plus
   the photographer's own 15–30 min/shoot QC anyway); AI $0.18–0.65 with
   documented failure modes (bleached walls, poster windows, resynthesized
   pixels, model churn changing your look without consent). The Rust tool
   competes on: deterministic look (no model churn), flash-frame color
   truth, zero marginal cost, same-hour delivery (delivery speed is a craft
   feature per the old-geezer finding).

## 1. Ground truth: the delivery pipeline (ordered, consensus)

1. **Group brackets** by EXIF capture-time gap; flag the flash frame.
2. **Lens correction + CA removal identically on every frame pre-merge**
   (double-application is a documented editor bug — do it exactly once).
3. **WB by the numbers, synced across ambient frames** — the recipe's fixed
   5000K does this in-camera. Flash and window-pull frames deliberately get
   independent color treatment (do not sync).
4. **Fuse the ambient 5-stack.** Engines in the market: LR/Enfuse (volume
   workhorse), Photomatix Fusion/Interior ("Interior 2" preset, align on,
   deghost off), LR Merge-to-HDR (gaining share since ~2025). Enfuse
   defaults ≈ pure exposure weighting: exposure 1.0, saturation 0.2,
   contrast 0, optimum 0.5→~0.6 for bright/airy bias, `--depth 16`.
5. **Flambient blend (the codified Cool stack):** flash frame = BASE layer
   (color authority); fused ambient on top in LUMINOSITY blend mode at ~50%
   opacity — ambient luminance, flash color. Volume-editor variant: flash
   used only as color reference. The live debate is just the opacity (how
   much ambient mood survives, 50% vs 70%+).
6. **Window pull from the darkest bracket:** darkest frame on top, Darken
   blend mode + soft feathered mask around glass (~5px feather, low-opacity
   brush edges). Dissent worth keeping (Hargis): not every window needs one.
7. **Local color-cast cleanup:** Color-mode layer painting with sampled
   neutrals at 10–20% opacity; hue/sat corrections placed below the
   window-pull layer so exterior color is untouched.
8. **Sky replacement** (base-price standard at services), TV blackout, lawn
   greening, dust/reflection removal. ← generative/inpainting territory,
   NOT v1 scope for the Rust tool; skies matter for exteriors only.
9. **Verticals/keystone late** (after flatten), identical geometry across
   blended frames. Note: the thing photographers most often still fix after
   outsourcing — an automated Hough-based pass has a real quality edge here.
10. **Bright/airy tone recipe:** Shadows +20–50, Whites +10–30, Highlights
    held down, gentle S-curve, Vibrance over Saturation, slight cool until
    white walls read white; Clarity/Dehaze sparingly.
11. NR + output sharpening.
12. **Export sRGB JPEG q80–100**, 2048px long edge = MLS-safe, 3000px
    common; native aspect.

## 2. The competence bar

From a blind editor audition (r/RealEstatePhotography, $1.25 vs $0.90
editors), the crowd judged on: **correct wall color with lamp warmth
retained** (losers: gray/green), **uniform window pulls across every window
in one frame**, restrained cool temperature, and mirror reflections whose
brightness matches the real window. Counterpoint from the same thread:
agents mostly can't tell past "good enough" — turnaround wins.

Documented failure modes to test against (each is a checkable metric):
gray/muddy desaturated whites (#1 cheap-editor complaint) · poster-like
over-dark window pulls · halos / "HDR gray" bloom at windows · ghosting /
warped alignment · mixed WB casts · crooked verticals · double lens
correction · flash shadows/hotspots · inconsistency across the shoot.

## 3. AI auto-editor benchmark (the direct competitor, 2025–26)

| Tool | Price/img | Notable | Fails at |
|---|---|---|---|
| Autoenhance.ai | ~$0.29 | Window pull as API param; C2PA "AI Modified" tags | texture regressions, melted leaded glass, hallucinated text |
| AutoHDR | $0.45–0.65 | brackets + sky + twilight | bleached walls/trim, blown ceilings, model churn |
| RealtyEdit | $0.18–0.40 | community pick for natural color | weak exteriors, camera left in mirrors |
| Fotello | ~$0.21–0.50 | per-listing pricing, 6-hr human revisions | warm casts, altered wall text, billing complaints |
| Imagen | ~$0.32/HDR | non-generative (real pixels), LR-native | RE profile training "not worth the squeeze" |
| Photomatix CL/API | $19/mo | deterministic fusion only | no pull/verticals/color — not a delivery |

Three structural facts: no tool accepts a flash frame; every vendor sells or
admits a human-QC layer (the $0.10–0.25 hybrid premium is the market pricing
"unattended AI isn't delivery-grade"); vendors retire models and the look
changes without consent. Several tools resynthesize pixels ("a coloring
book") — passable at MLS res, exposed on large screens.

## 4. Rust feasibility per stage (verified 2026-08-31)

| Stage | Verdict |
|---|---|
| RAF develop, 16-bit | `rawler` 0.8: **Markesteijn X-Trans demosaic native** (`imgop::sensor::xtrans`), develop to RGB. The ~13MP embedded preview is NOT full-res — delivery requires demosaic, and rawler has it |
| Mertens fusion | **No crate.** v1: shell to `enfuse` (`brew install enblend-enfuse`, 16-bit TIFF in/out). v2: native from IPOL 2018 pseudocode, ~400–700 LOC + rayon, 1–2 wks. Pitfalls documented: out-of-range after pyramid collapse (window clipping), halo-vs-depth tradeoff, weight epsilon, odd-dimension pyramid bookkeeping; fuse in gamma space, f32 |
| Flash luminosity blend | Native, ~100–200 LOC f32 (Cool stack: flash base, ambient luminosity ~50%) |
| Window pull | Native, ~200–400 LOC: luminance threshold + `imageproc` morphology + Gaussian feather; guided-filter refinement ~100 LOC if needed |
| Alignment | Skip by default (tripod); `--align` flag shells to `align_image_stack` as insurance |
| Keystone/verticals | Native: canny+hough tilt → 3×3 → `imageproc::warp` (f32, bicubic, rayon); compose lens+keystone remaps, resample once |
| Lens correction | RAF makernotes carry Fuji's own spline params (GeometricDistortionParams 0xf00b, CA 0xf00f, exiftool-documented); rawler does NOT parse them — small own parser, or `lensfun` crate (pre-alpha) fallback |
| Global color | Native f32 math (WB gain, S-curve, vibrance, whites-neutral); ICC optional (`moxcms` pure-Rust if ever needed) |
| Resize + encode | `fast_image_resize` (NEON Lanczos3) + `jpeg-encoder` 0.7 (quality, **4:4:4** — `image`'s encoder is hard-coded 4:2:2; window muntins deserve 4:4:4), sRGB profile embedded |
| Sky replacement | Out of scope (generative). Exteriors: defer, or price as the one outsourced/AI item |

Bit-depth ruling: fusion tolerates 8-bit sources (HDRsoft itself says JPEGs
often fuse *better*), but post-fusion pushes (tone curve, window pull) band
on 8-bit walls. Fixed-WB tripod work is the safest 8-bit case — moot for us:
rawler develops to 16-bit, work f32 end-to-end, quantize once at export.

Prior art: exactly one open project ever did this product — Homedeck
(Python/DL, archived 2021), decomposed identically (fusion + flash/color +
scene-conditional edits). The commercial tools are closed.

## 5. The experiment (test the assumption before building the product)

Goal: see how brackets + pop actually shape the final photo, and whether the
Rust path clears "competent."

1. Shoot (or pull from a card) ~5 real stations: mixed rooms, at least one
   hard window room, one mixed-lighting room.
2. Build the thinnest possible harness: rawler develop → 16-bit TIFFs →
   `enfuse` (defaults; then optimum ~0.6) → native flash blend at 30/50/70%
   → darkest-frame window pull → verticals → tone recipe → JPEG.
3. Produce a ladder per station: (a) fused only, (b) + flash blend,
   (c) + window pull, (d) + verticals + tone. This isolates each
   ingredient's contribution — the stated assumption to test.
4. Benchmark the same stations against one outsourced editor (~$1 trial,
   Pixlmob/FB group) and one AI tool (RealtyEdit or Autoenhance free tier).
5. Judge on §2's bar, on a phone and a big screen. The analysis metrics
   (whites-neutrality, CCT spread, verticals, window clip) score all three
   outputs — the QC layer bootstraps itself.

Setup still missing for this: `brew install enblend-enfuse exiftool`.

## 6. Feedback into the shoot recipe

- Fixed 5000K WB: **validated** — it is precisely what makes automated
  fusion + deterministic color safe (and what 8-bit-vs-RAW debates say you
  can skip worrying about).
- Tripod + timer + deghost-off philosophy: **validated** (Photomatix RE
  guidance is deghost off; fusion's Interior mode can't deghost at all).
- 5×1EV = ±2EV total: **flagged as wide** — fusion halo risk concentrates at
  windows with big EV gaps; literature mitigation is tighter spacing/more
  frames. Counterweight: the −2 frame is the window-pull source and the
  1/125 flicker anchor. Keep, but revisit if window halos show in the
  experiment.
- Pop at ISO 320 vs brackets at 125: fine — the flash frame contributes
  color, not luminance, in the Cool stack.

## 7. Open questions the experiment answers

1. Enfuse defaults vs exposure-optimum ~0.6 vs native-Mertens tuning for
   the bright/airy target.
2. Flash-blend opacity (30/50/70%) for this flash recipe (bounced TTL 0EV).
3. Whether every window needs a pull (Hargis dissent) or only money shots.
4. Whether Fuji makernote lens params need parsing at all — the embedded
   preview is corrected in-camera, but rawler's raw develop output is NOT
   lens-corrected; measure how visible 16mm distortion is on interiors.
   *Answered 2026-09-01:* yes, and it's cheap. The RAF carries
   `GeometricDistortionParams` (radial table, −9.6% at the corner for the
   Sigma 10-18 at 10 mm); distortion is visible on every interior (bowed
   jambs, curved baseboards) and it breaks the verticals fit. Implemented in
   `src/verticals.rs` as one warp with the vanishing-point rotation (stage 9
   above); stage 2's "exactly once" holds because it runs on the flattened
   blend, not per bracket frame.
5. Does the assumption "agents/buyers can't tell past good-enough" hold for
   YOUR market (ties to the A1 finding: photos sell the house, delivery
   speed sells the rehire).

## Sources

Pass A: hdrsoft.com RE tutorials + FAQ + Pro 6.2 manual §8.6 · photoup.net
flambient + LR/Enfuse tutorials · boxbrownie.com image-enhancement spec ·
fstoppers Nathan Cool color system (901073) · rephotojournal flambient ·
richbaumphotography Darken-mode pull · PFRE window-pull roundup + Enfuse
e-book · r/RealEstatePhotography threads incl. 1s17gw4 (blind audition),
1ifommw, 195ot6i, 1jo516r, 1d5qm2g, 1r2wm48, 1ktda3w, 1r8nr8h, 1tmsmr0,
1nzm0gh · vendor sites/APIs (autoenhance.ai, autohdr.ai, realtyedit,
fotello, imagen-ai, photomatix CL). Pass B: IPOL 2018 exposure-fusion
implementation (ipol.im/pub/art/2018/230) · Mertens–Kautz–Van Reeth 2007 ·
enfuse man page + PanoTools wiki + hugin HDR workflow docs · docs.rs:
rawler 0.8 (xtrans module), imageproc 0.27 (geometric_transformations,
hough), fast_image_resize 6.1, jpeg-encoder 0.7, lensfun, moxcms ·
dpreview 4688241 (13MP embedded preview) · exiftool FujiFilm.pm (0xf00b/
0xf00f) · github: dnglab, Homedeck (archived), Hugin-Enfuse-HDR-Toolkit.
