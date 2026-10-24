# Composition tool — "realm of good compositions" design

Research 2026-08-31, two verified passes: (A) geometry/algorithms for
interior composition analysis; (B) embedding-based reference-set scoring.
Everything below is commercial-safe-licensed and runnable locally in Rust on
Apple Silicon. Companion docs: RESEARCH-rust-imaging.md (decode/metrics),
RESEARCH-shoot-craft.md (the rulebook this operationalizes).

## 0. Verdict: two engines, one tool

**Engine R — the Realm (culling/ranking).** DINOv2-with-registers ViT-B/14
embeddings + cosine k-NN distance against your reference set. This exact
recipe is published SOTA for few-shot "score newcomers against exemplars"
(AnomalyDINO, WACV 2025; kNN-OOD, ICML 2022). No training. Works from ~30
reference images; scales to 300+. Runs in `candle` (ships dinov2reg4 +
Metal fp16; ~30–60 s to embed 300 refs at 518px, ms per query after).
Apache-2.0 model, pure-Rust static binary.

**Engine G — the Geometry (explanation/guidance).** Explicit, nameable
measurements that map 1:1 to the shoot-craft rulebook: verticals/level,
one-point vs two-point perspective, floor/wall/ceiling thirds, wall-junction
count (2- vs 3-wall), symmetry, clutter, thirds placement. This is what
powers "tell me what to look for during the shoot" — and it explains WHY a
photo fell out of the realm when Engine R flags it.

They share everything upstream (decode, resize) and cover each other's
blind spots: R catches gestalt problems features miss; G names causes R
can't articulate.

**Market check (verified):** nobody does this. Commercial cullers
(Aftershoot, Narrative, Lightroom Assisted Culling) are people-photography
tools — blur/eyes/duplicates, zero composition grading. RE-specific vendors
fix verticals (Autoenhance) or check MLS compliance (Restb.ai); none grade
composition. A tool grading tilt, camera height, 2-vs-3-wall framing, and
perspective type sits in a verified empty intersection.

## 1. Engine R — the realm recipe

1. Embed references: DINOv2-with-registers ViT-B/14 @ 518px; concatenate
   CLS token + mean-pooled patch tokens (structure lives in patch tokens —
   Splice-ViT; registers keep patch maps artifact-free). L2-normalize.
2. Score a new photo: mean cosine distance to its k nearest references,
   k = clamp(n/20, 3, 20). Non-parametric — handles the multimodal
   reference set (kitchens/baths/exteriors) implicitly; a kitchen query's
   nearest refs ARE the reference kitchens. No room classifier needed.
3. Calibrate 0–100 via leave-one-out conformal percentile on the refs.
   At n=30 the granularity is ~3 points — present bands, not digits.
4. Rank alternates of the same room by raw distance (continuous).
5. Surface "no similar reference" (nearest-distance threshold) instead of
   a misleading low score for an unrepresented room type.
6. Dedupe references; curate for style consistency — the tool measures
   "fits this distribution," so the distribution must mean what you intend.

Math: hand-rolled kNN (300×768 f32 = a matvec, microseconds) — every ANN
crate is overhead at this n (instant-distance archived, hora dead). PCA +
Ledoit-Wolf Mahalanobis as optional secondary signal (`ndarray` + `faer`;
avoid ndarray-linalg on Apple Silicon).

**Honest limit (evidence-backed):** embeddings measure content first,
composition second (CSD paper: SSL/contrastive training treats style as a
nuisance). R will reliably catch off-realm photos — wrong angle, snapshot
look, clutter, bad height — but won't arbitrate rule-of-thirds subtleties
between two well-shot frames. Mitigations already in the design: DINO
family not CLIP (probe3d: DINOv2 features carry depth/normals/camera
geometry far better), patch-token scoring, curated refs. Escape hatch if
needed later: small head trained on CADB (9.5K composition-scored images,
MIT) over frozen DINO features.

Runtime: candle 0.11+ (MUST be ≥0.10 — Metal GEMM fix PR #3313), hf-hub +
safetensors, bincode embedding cache. ort 2.0.0-rc.13 (pinned) only if we
later want DINOv3/SigLIP2 (custom license / candle gap respectively).

## 2. Engine G — the geometry stack

| Capability | Method | License | Effort |
|---|---|---|---|
| Line segments | M-LSD-tiny ONNX (~2.5MB) via ort + ~100-line Rust decode | Apache-2.0 | 2–4 d |
| Level / keystone | median tilt of near-verticals; L/R convergence split (free from lines) | — | 1 d |
| Vanishing points | port Lu WACV'17 2-line exhaustive method — from the BSD-2 repo, NOT the GPL main repo; focal from EXIF (15mm-eq → f_px ≈ 0.42·w) | BSD-2 | ~1 wk |
| One- vs two-point ("Shulman L" vs 45°) | threshold on VP positions in focal units — geometry is trivial once VPs exist; NO published prior art does this classification (novel in practice) | — | 1 d |
| Floor/wall/ceiling thirds | TopFormer-ADE20K seg ONNX (wall=0, floor=3, ceiling=5, rug∪floor) → pixel fractions; the A6 "third-third-third / use-plane height" check | Apache-2.0 | 1–2 d |
| Wall junctions (2- vs 3-wall, A7) | near-vertical segments ∩ wall mask, cluster by x | — | 2–4 d |
| Symmetry (one-point heroes) | HOG left-half vs mirrored right-half — 2026 benchmark: within 0.03 of best deep method at ~300× CPU speed (`imageproc::hog`) | MIT | 2 d |
| Clutter / near-lens foreground (A4) | Canny edge density + Rosenholtz subband entropy port; foreground band weighting | MIT | 3 d |
| Thirds/saliency placement | spectral-residual saliency (~50 lines, rustfft) → centroid vs power points; weight LOW (weak evidence; SOD models fail on empty rooms) | MIT | 2 d |
| Leading lines | SKIP as such — the diagonal-convergence score from VP work IS the one-point hero signal | — | free |

Licensing traps verified: original LSD source AGPL (don't port), Perspective
Fields + NVIDIA SegFormer weights + uLayout non-commercial, dssim AGPL
(use image-compare), semantic-line models CC-BY-NC, SwiftVP unlicensed.
Skip: learned VP nets (no ONNX, GPU-bound), all panorama layout models
(wrong input type), RoomNet-era dead code.

## 3. Rulebook → measurement map

| Shoot-craft rule (validated) | Engine G feature | In-shoot guidance it yields |
|---|---|---|
| Level + plumb (table stakes) | tilt°, keystone convergence | "roll 0.8° left; tilt down 2°" |
| Height keyed to use-plane (A6) | floor/ceiling fraction + keystone | "too much ceiling → drop 15cm" |
| Shulman L vs 45° workhorse | VP-based perspective type | "hero rooms: go one-point" |
| Third wall a sliver at most (A7) | junction count/position | "3rd wall is 28% of frame" |
| Show the door (A11) | door via seg classes (later) | — |
| No near-lens foreground (A4) | foreground-band clutter | "clear the bottom third" |
| Doorway vantage, crop past jamb (3.8) | edge-density at frame borders | "jamb visible left edge" |
| One-point symmetry prized | HOG symmetry score | "center on the axis" |
| Focal restraint 16mm default (A8) | EXIF focal | "you're at 15mm-eq again" |

The "realm profile" doubles as guidance: after `realm build`, report the
reference set's stats in these terms (e.g. "your refs: tilt ≤0.4°, ceiling
28–35% of frame, 80% of heroes one-point, symmetry ≥0.7 on one-points") —
that IS the "what to look for" card, derived from your own exemplars.

## 4. CLI design

```
retool realm build <refs_dir>        # embed + profile; writes realm.bin + profile report
retool realm score <img|dir>         # 0–100 fit + nearest refs + Engine G reasons
retool cull <shoot_dir>              # rank stations/frames; flag out-of-realm + why
retool comp <img>                    # Engine G report alone (no realm needed)
```

Culling integrates with the station pipeline: score the pop/final per
station, rank alternates of the same room, output keep/review/reject tiers.

## 5. Build order

1. **v1 (small):** Engine R via candle (embed, kNN, conformal calibration,
   cache) + `realm build/score/cull`. The published recipe, no training.
   Plus free G features already in reach: EXIF focal check; verticals tilt
   via imageproc Hough — SHARED with the editor's stage-e verticals
   correction (build once, use in both).
2. **v2:** M-LSD + Lu VP port → perspective type, keystone, junctions;
   TopFormer thirds; symmetry + clutter. Full "what to look for" card.
3. **v3 (only if R+G insufficient):** CADB-trained composition head over
   frozen DINO features; CACNet/SAMP-Net as references (MIT).

Key sources: AnomalyDINO arXiv:2405.14529 · kNN-OOD arXiv:2204.06507 ·
registers arXiv:2309.16588 · probe3d arXiv:2404.08636 · CSD arXiv:2404.01292 ·
symmetry benchmark arXiv:2607.08379 · CADB/SAMP-Net github.com/bcmi (MIT) ·
M-LSD github.com/navervision/mlsd · Lu VP github.com/xiaohulugo/
VanishingPointDetection_BSD2 · TopFormer github.com/hustvl/TopFormer ·
candle PR #3313 · ort 2.0.0-rc.13 · Airbnb photo study (12-attribute spec) ·
culling market: aftershoot.com/selects, narrative.so/changelog, Lightroom
Assisted Culling docs, filterpixel DeepCull.
