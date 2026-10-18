# Draw surface quality — OBrush 0.1.3

This is the historical 0.1.3 quality record. Version 0.1.4 optimizes the same
geometry operations and queues input work between redraws. Its same-ray checks
preserve all 12 final OBJ fixtures exactly; [performance evidence](BROWSER_PERFORMANCE.md)
records the new implementation and its runtime scope separately.

The user reported jagged raised Draw strokes on a fresh 0.1.2 sphere at radius
**0.180**, strength **35%**, with X symmetry enabled. No saved mesh or exact input
trace was available. The fixtures below are independently generated comparisons
at those settings, not reconstructions of the screenshots or ZBrush reference
runs. Higher density and screen-edge antialiasing alone did not resolve the bug.

## Cause and implemented change

The 0.1.2 Draw operation used 70% evolving vertex normal and 30% hit normal.
Overlaps fed changing surface slopes back into lateral displacement. Its geometry
safety checks prevented step reversals while still allowing adjacent faces to
reach 150°. One fresh-sphere line already reached 70° and four edges above 60°,
with **zero reversals**. Zero flips therefore did not establish surface quality.

An intermediate coherent-patch normal reduced many artifacts but still failed:
all nine remaining edges above 60° in its four-line fixture lay exactly on the
**X-symmetry seam**. Selecting the stronger mirrored dab's direction caused a
jump across the seam; zeroing X motion only on exact seam vertices did not make
the surrounding direction field continuous. Its failed outputs are preserved.

Final 0.1.3 combines a coherent patch normal, direction blending where mirrored
footprints overlap, and local one-ring surface fairing. Scalar strength retains
maximum influence, so mirrored overlap is not doubled. Existing exactly mirrored
pairs with equal protection retain exact symmetry before refinement. Vertex
normals now accumulate corner-angle-weighted unit face normals in double
precision, avoiding area-weighting bias when triangles are subdivided.

After an accepted Draw dab, the shared Rust core refines long local edges toward
**radius × 0.12** (0.0216 world units here), on both sides of shared edges.
Refinement is bounded to two rounds and at most 1,024 added vertices per call.
Symmetry mode uses up to 256 selected edges per round and reserves budget for
symmetry-preserving interior face samples. New geometry interpolates masks/RGB;
mesh limits and unsupported selected topology reject refinement atomically.
Geometry guards, attribute/history tests and detail-limit feedback are separate
from the surface measurements here. This is local triangle refinement, not
persistent subdivision levels, decimation, remeshing or Sculptris Pro parity.

## Reproduction and local gates

```sh
OBRUSH_DRAW_PROBE_EXPORT_DIR=.artifacts/draw-overlap-final/obj \
  cargo run --locked -p obrush-core --release --example draw_overlap_probe
```

The [comparison record](evidence/draw-overlap-comparison.json) contains exact ray
inputs, all threshold counts, source/probe/OBJ hashes, and four stages: frozen
0.1.2, the failed coherent-normal candidate, the adaptive candidate and final
0.1.3. Initial sphere vertex/face record hashes match across stages. The two
repeated-point fixtures were added later and are explicitly **not run** for the
coherent candidate. Full failed/intermediate outputs remain under ignored
`.artifacts/draw-overlap-*` directories.

Most fixtures start with a unit-radius level-6 sphere, 40,962 vertices / 81,920
triangles. Every dab raycasts the current surface. Lines use 41 points across
`x=-0.4..0.4`, `y=0.09`, from `z=3` toward `-Z`, reversing on alternate passes.
Crossings add a vertical line at `x=0.09`. Loops use 65 points centered at
`(0.21,0.06)` with radius 0.19. Tilted rays use view direction proportional to
`(0.6,0.25,1)`, offsets `x=-0.35..0.35`, `y=0.86` in its screen basis.
Repeated tips pick `(0.23,0.09,3)` toward `-Z` for 64 or 128 dabs. They represent
repeated applications, not a claim that a stationary held mouse emits dabs.

Angles below are between **geometric face normals**, not shading normals. Quality
is `4√3 × area / sum(edge_length²)`; an equilateral triangle has quality 1.
Roughness is the normal projection of a vertex's one-ring position residual,
normalized by mean edge length; the table reports its 95th percentile on edited
or appended vertices. It includes intended curvature and is not a universal
artifact detector. Height is maximum radial displacement from the initial sphere.
Per-dab reversal checks use retained original vertex indices on pre-dab faces;
new child winding is covered separately by the refinement tests.

Local thresholds were selected after inspecting baseline/candidate results,
before the final run. They are not preregistered ZBrush tolerances: zero step
reversals and edges above 60°/90°; roughness p95 ≤0.12; quality ≥0.15; maximum
edge ≤0.0216 plus 1e-6; height ≥0.05 and ≥80% of baseline; finite valid structure.
These limits apply to these fresh-sphere fixtures, not arbitrary imported meshes.

## Same-input results

The edge-count column lists **above 30° / 60° / 90°**. Every baseline and final
case has zero per-dab reversals; the angular/roughness measures distinguish them.

| Fixture | Edge counts, before → final | Maximum angle | Roughness p95 | Height, world units |
| --- | --- | --- | --- | --- |
| One line, 41 dabs | 10/4/0 → 8/0/0 | 70.0° → 39.3° | 0.085 → 0.067 | 0.089 → 0.087 |
| Four lines, 164 dabs | 124/28/5 → 15/0/0 | 150.0° → 60.0° | 0.190 → 0.096 | 0.322 → 0.303 |
| Crossing ×2, 164 dabs | 56/44/24 → 52/0/0 | 150.0° → 59.9° | 0.128 → 0.077 | 0.163 → 0.155 |
| Crossing ×4, 328 dabs | 197/73/41 → 62/0/0 | 150.0° → 60.0° | 0.147 → 0.087 | 0.321 → 0.298 |
| Crossing ×8, 656 dabs | 199/95/65 → 75/0/0 | 150.0° → 60.0° | 0.128 → 0.087 | 0.532 → 0.502 |
| Loops ×4, 260 dabs | 287/37/17 → 43/0/0 | 150.0° → 60.0° | 0.229 → 0.095 | 0.215 → 0.207 |
| Loops ×8, 520 dabs | 443/213/101 → 59/0/0 | 150.0° → 60.0° | 0.300 → 0.098 | 0.314 → 0.337 |
| Tilted silhouette ×4, 164 dabs | 72/0/0 → 12/0/0 | 49.8° → 56.7° | 0.167 → 0.076 | 0.189 → 0.189 |
| Tilted silhouette ×8, 328 dabs | 294/148/76 → 14/0/0 | 137.7° → 55.2° | 0.203 → 0.093 | 0.326 → 0.266 |
| Level-7 crossing ×4, 328 dabs | 220/89/55 → 101/0/0 | 150.0° → 60.0° | 0.061 → 0.053 | 0.319 → 0.300 |
| Repeated point, 64 dabs | 14/0/0 → 0/0/0 | 39.2° → 26.6° | 0.130 → 0.084 | 0.281 → 0.277 |
| Repeated point, 128 dabs | 24/6/0 → 0/0/0 | 76.2° → 26.5° | 0.085 → 0.065 | 0.420 → 0.380 |

All 12 final fixtures pass the named local gates. Worst roughness p95 is
**0.09770**, minimum quality **0.19868**, and maximum edge length **0.02159975**.
The lowest retained height is **81.7%** of baseline. The 128-dab raised tip retains
**90.5%** of height while reducing its maximum adjacent angle **76.2° → 26.5°**.
Nonzero edges above 30° remain: the change does not promise an everywhere-flat
surface or eliminate intentional sculpted curvature.

## Visual, host and performance evidence

The [visual comparison](evidence/draw-visual-comparison.json) records final
source-bound, matched-camera CGL renders using the actual viewport shaders and
4× multisampling. Final crossings, loops, tilted silhouettes and repeated tips
show rounded boundaries without the baseline's large teeth; the record binds
these observations to final OBJ/source hashes. Intermediate outputs are retained.
This is additional visual evidence, not a claim based only on the enforced
60° limit. The [isolated browser record](evidence/draw-browser-runtime.json)
records WebGL2 with actual 4× multisampling, three Draw strokes, saved project
bytes, undo and exact redo. The saved mesh has valid closed topology and no
adjacent face angle above 60°. Its exact input trace and limited scope are recorded
separately.
The [build record](evidence/draw-quality-build.json) owns release checks and hashes.
These records do not establish the user's interactive native/browser experience
or ZBrush appearance parity.

Performance remains a limitation. One native Apple M2 / 24 GiB release run of
the final core algorithms, without external per-dab diagnostic scans/snapshots,
measured **32.62 ms/dab** for four lines, **31.15 ms/dab** for eight loops and
**28.53 ms/dab** for the repeated tip. This includes picking, deformation and
refinement; it excludes app history, GPU/UI work and input-to-display latency.
It is not application FPS. Multiple dabs in one frame can delay interaction;
spatial acceleration, local normal updates and work scheduling remain needed.

Global intersection safety, automatic repair of previously damaged imports,
pressure/input-device coverage, exhaustive paths/cameras and reference brush
response remain unverified. The captured comparisons establish these named
OBrush fixtures only.
