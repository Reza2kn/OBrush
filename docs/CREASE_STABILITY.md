# Crease stability — OBrush 0.1.2

This is historical 0.1.2 evidence. The subsequent overlapping-Draw and local-refinement update is recorded in [DRAW_QUALITY.md](DRAW_QUALITY.md).

The reported damaged Crease screenshot prompted this investigation. The user
confirmed radius **0.180** and strength **35%**. X symmetry is the app-default
assumption used by the fixtures, not a user-confirmed setting. The screenshot
shows Move selected at capture and supplies no saved mesh or exact stroke trace.
The tests below use new, independently generated sphere fixtures and reproduce
an engine failure; they do not reconstruct that session or establish a visual
result on the final native/browser app.

## Failure and change

The old Crease operation attracted vertices toward the dab center in its tangent
plane, with coefficient `0.2 * strength * falloff`. Near the center, a 35% dab
could contract lateral offsets by approximately 7%. Overlapping dabs compounded
the contraction without checking triangle orientation or shape. Even the first
default stroke reduced the minimum edge length to 25.6% of its original length.
Repeated strokes produced triangle reversals and adjacent faces folded nearly
back to back. Higher mesh density did not cure the problem: the level-7 fixture
first reversed a face at dab 110, versus dab 150 on level 6.

The new Crease response reduces tangent contraction and blends local vertex
normals with the hit normal. Geometry dabs now backtrack or reject displacement
that would reverse/collapse affected faces or worsen adjacent folds. The shared
guard also applies to Move. It prevents newly exceeding a 150° adjacent-normal
angle, while preserving an already sharper imported surface's previous bound.
Geometry edits touching edges with more than two incident faces are rejected.
These checks preserve existing damaged surfaces; they do not repair them.

The first candidate removed per-dab reversals but left 11 extreme-angle edges
after sixteen default passes and four after Move→Crease. That failed intermediate
check is retained. The second candidate added adjacent-face protection. Final
0.1.2 also tracks the actual committed surface point for Move anchors and avoids
copying topology caches into snapshots; those app/API changes are outside this
probe's direct core-dab comparison.

## Reproduction and metrics

Run the same CPU probe against the current source:

```sh
cargo run --locked -p obrush-core --release --example crease_stability_probe
```

The [comparison record](evidence/crease-stability-comparison.json) contains all
14 fixtures, exact parameters, source/probe/result hashes and paired baseline,
first-candidate, second-candidate and final-0.1.2 measurements. The original core
and failed outputs are retained in ignored `.artifacts/crease-stability-baseline`;
candidate outputs/metadata are in `.artifacts/crease-stability-first-fix` and
`.artifacts/crease-stability-second-fix`. Final output is in
`.artifacts/crease-stability-current`. The [build record](evidence/crease-stability-build.json)
records final build hashes, **60 passing tests**, strict native/WASM Clippy,
the pure-core WASM check, web/native builds and macOS signature verification.
The fixture comparison itself contains no build-artifact hashes.

Default fixtures start with a unit-radius level-6 icosphere: **40,962 vertices /
81,920 triangles**. Every dab raycasts the current surface from `(x, 0.09, 3)`
toward `-Z`. A pass has 41 points from `x=-0.4` to `0.4`, step `0.02`, with
alternating direction. Lifted passes repeat the endpoint as a new initial dab;
continuous turnarounds omit that duplicate. Radius is 0.18, strength 0.35,
symmetry is on and inversion is off unless the record explicitly varies it.
Move→Crease starts with one Move dab at ray `(-0.23,0.09,3)`, using delta
`(0.6,0,0)` or `(1.2,0.4,0)`, then four default Crease passes. Other fixtures
vary density, symmetry, strength, inversion or repeat one point.

- **Per-dab reversal events:** a triangle's new cross product has nonpositive dot
  product with its cross product immediately before that dab. This counts events,
  not unique final faces. Gradual legitimate rotation is not counted as a step flip.
- **Extreme-angle edges:** final neighboring unit face normals have an angle
  above **160°**. This diagnostic threshold differs from the engine's **150°**
  prevention bound. It is not a general triangle-intersection detector.
- **Triangle quality:** `4√3 * area / sum(edge_length²)`, with equilateral quality
  1. Near-zero faces have final/initial area at most `1e-4`; the low-quality
  counter uses quality at most `1e-3`.
- **Inward displacement:** position change projected inward along the initial
  sphere's radial direction, in world units. It establishes that sculpting still
  changes the surface, not matching ZBrush's response or preserving volume.

## Same-input results

| Fixture | Baseline reversals / extreme edges | Final reversals / extreme edges | Final minimum quality |
| --- | --- | --- | --- |
| Default, one pass / 41 dabs | 0 / 0 | 0 / 0 | 0.67415 |
| Default, four lifted passes / 164 dabs | 26 / 22 | 0 / 0 | 0.33047 |
| Default, sixteen passes / 656 dabs | 2,108 / 606 | 0 / 0 | 0.12707 |
| Continuous turnaround / 161 dabs | 20 / 12 | 0 / 0 | 0.33045 |
| Move `(0.6,0,0)` → four passes | 36 / 26 | 0 / 0 | 0.05411 |
| Move `(1.2,0.4,0)` → four passes | 52 / 76 | 0 / 0 | 0.17972 |

All 14 final fixtures have zero per-dab reversals, zero extreme-angle edges,
zero near-zero/low-quality faces and valid mesh structure. Four default passes
retain inward displacement **0.16245**, versus **0.15054** before the fix; minimum
edge ratio improves **0.01229 → 0.40114**. Sixteen default passes reach inward
displacement **0.57134**. The worst quality among all fixtures is **0.03662**;
the strength-1.0 sixteen-pass fixture remains a deep deformation (**1.3098**
inward displacement), so the local checks do not establish thickness preservation.

One final native Apple M2 / 24 GiB release run measured approximately **402 ms**
for the 164-dab default fixture, **1,526 ms** for 656 default dabs and **6,089 ms**
for 656 level-7 dabs. These times include snapshots and full-mesh double-precision
diagnostic scans after every dab, exclude rendering/UI/final metric scans, and
are neither application FPS nor input-to-display latency.

Final native/browser runtime visuals have not been verified. Global self-intersection
safety, automatic repair of pre-damaged imports, all topology/input combinations,
and ZBrush brush/appearance parity remain unverified. The numerical improvements
establish these named OBrush fixtures only.
