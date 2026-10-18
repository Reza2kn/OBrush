# Visual quality — OBrush 0.1.1

This is the historical density/rendering update. The later 0.1.2 Crease/Move deformation fix is recorded in [CREASE_STABILITY.md](CREASE_STABILITY.md).

Version 0.1.1 addresses three separate causes of the original rough appearance:
insufficient sculpt geometry, input-dependent stroke sampling and jagged viewport
edges. Baseline failures and measurements captured on 2026-10-12 are retained
below. Visual parity with ZBrush 2026.2.1 remains unverified. Read
[FEATURE_PARITY.md](FEATURE_PARITY.md) for the reference and status contract.

## Implemented changes

- The default sphere is now level 6: **40,962 vertices / 81,920 triangles**, four
  times the original triangle count. New spheres offer Standard (20,480), High
  (81,920) and Fine (327,680) triangle choices. Existing loaded meshes keep their
  topology. Even this higher default has limited geometry for the smallest
  possible brush marks; the density measurements below explain the limit.
- **Smooth subdivide** performs one Loop subdivision step on triangle meshes,
  interpolating positions, masks and RGB and rebuilding normals. Boundary and
  interior stencils are tested. It rejects unsupported topology and allocation
  limits without replacing the mesh, and undo restores the previous surface.
  It rounds corners and can shrink the shape or soften sculpted details.
  **Refine without smoothing** retains the original linear midpoint operation.
  Neither operation creates persistent subdivision levels or establishes
  ZBrush's subdivision behavior.
- Dab-based strokes now carry their arc-length spacing remainder across input
  updates, with spacing fixed at stroke start. The regression test splits the
  same straight path into 1-, 3-, 10-, 37- and 160-pixel input updates and checks
  dab count, positions and resulting sculpt geometry. Additional checks cover
  short segments, corners, stationary updates and nonfinite input. This is
  consistent OBrush sampling, not calibrated ZBrush brush response or Lazy Mouse.
  Path breaks prevent backfilling Move drags when temporarily smoothing and
  prevent interpolated strokes across exits from the canvas. A regression checks
  that starting a new sampling segment discards the preceding path remainder.
  This does not establish a fixed bound on per-frame dab work or add spatial
  acceleration/worker scheduling.
- **Smooth viewport edges** renders actual mesh geometry to an offscreen
  multisample target before resolving and compositing it into the viewport. It
  requests 4× samples, then tries 2×, then uses the original direct path if device,
  format or memory limits prevent allocation. The inspector reports the allocated
  sample count rather than only the requested setting. The render target has a
  256 MiB storage budget; this is not a total application/GPU memory limit.
  Antialiasing changes screen edges without smoothing the mesh geometry.

## Verification for 0.1.1

The source passes **47 tests** (39 core, 8 app) and formatting after the final
input-discontinuity changes. Strict native and WASM Clippy and static web plus
ad-hoc-signed macOS builds passed during this update. The
[visual-quality build record](evidence/visual-quality-build.json) binds the final
source/build hashes and records each completed check and its limits. The new
core tests include Loop stencils, sphere
convergence, attribute propagation, undo/redo and unchanged state after rejected
subdivision. App tests cover stroke segmentation and antialias storage limits.

Before the runtime checks were interrupted and before the final input/state
fixes, Chrome startup displayed **0.1.1**
and **4× antialiasing active**, with no reported startup errors. That observation
establishes that earlier build's browser startup/sample-count check only. Runtime
checks were not repeated after the final fixes. Current native visual quality,
viewport resize and antialias-toggle behavior, and a full editing/save/load
workflow on the changed renderer have **not been verified** for 0.1.1. No
reference sculpt-stroke or appearance comparison was completed.

[VALIDATION.md](VALIDATION.md), the REA binary inspections and the OBJ reference
exchange remain historical 0.1.0 evidence. Their completed workflows should not
be treated as verification of the changed 0.1.1 renderer or smooth subdivision.

## What the original 0.1.0 implementation did

The original startup sphere had 10,242 vertices and 20,480 triangles (`Mesh::sphere(5)`).
It already uses area-weighted vertex normals, recomputes them after each geometry
dab, and interpolates them in the fragment shader. The original appearance is
therefore not explained by an absence of smooth normals. Smooth shading cannot
add missing geometric detail or remove a polygonal silhouette.

The smallest allowed brush radius is 0.015 world units. The sphere's mean unique
edge length is 0.03776595. A small dab can hit no editable vertex, or deform just
one vertex into a spike. Deformed patches consequently show real coarse geometry,
even though the initial sphere looks smooth. The original subdivision operation
only splits edges at their midpoints: it adds editable points while preserving
the angular surface already present.

The original app chooses `ceil(segment_length / spacing)` dabs per input update
and then resets spacing at that update's endpoint. This makes the amount of
sculpting depend on input-event delivery. Near-minimum cursor sizes also use a
two-logical-pixel minimum spacing, a large fraction of the brush radius. The
64-dab limit can increase spacing further during long jumps. These are separate
causes of uneven strokes; denser geometry alone cannot resolve them.

Viewport edge antialiasing was another independent issue. It smooths screen-space
stair steps, not underlying geometry. The new runtime behavior still needs native
and browser screenshots and resize/toggle checks, rather than a CPU mesh experiment.

## Independently created numerical fixtures

A release-mode Rust harness used a copy of the original core, so later changes
do not alter the baseline. Local supporting files are in the ignored
`.artifacts/visual-quality-baseline/` directory: `src/main.rs`, `core/`,
`Cargo.toml`, `Cargo.lock`, `output.txt`, and `metadata.json`. Reproduce with
`cargo run --release --manifest-path .artifacts/visual-quality-baseline/Cargo.toml`.
The source capture had no resolved Git HEAD; SHA-256 binds it instead:

| Source | SHA-256 |
| --- | --- |
| `obrush-core/src/brush.rs` | `f11ebc444340c5fb78d49decf6e14eb8e980c6c8626fb49b75933246c781b0cb` |
| `obrush-core/src/mesh.rs` | `9ac26d11e53fbd825f0f806f45636b27fb8c35b40ceb1d9c8cbb4d334086a5f7` |
| `obrush-app/src/app.rs` | `6ce78e4efa576cf6a7233f6c247be1a2723918b6efea48b2ac600dff0be0e61a` |

Density fixtures are unit-radius icospheres. Counts below average 64 front-facing
ray hits from `(x, y, 3)` toward `-Z`, with `t=i/63`, `x=-0.4+0.8t`, and
`y=0.11+0.13 sin(2πt)`. A vertex is influenced when its distance to the hit is
less than the world radius and its normal faces the hit normal. Symmetry is off.
These sampled counts are not global guarantees for arbitrary imported meshes.

| Sphere level | Vertices / triangles | Mean edge length | Radius 0.015: affected min / mean / max | Radius 0.18: affected mean |
| --- | --- | --- | --- | --- |
| 5, original default | 10,242 / 20,480 | 0.03776595 | 0 / 0.625 / 1 | 79.812 |
| 6 | 40,962 / 81,920 | 0.01888387 | 1 / 2.062 / 3 | 319.297 |
| 7 | 163,842 / 327,680 | 0.00944763 | 6 / 8.688 / 11 | 1,276.828 |

At radius 0.015, 24 of the 64 original level-5 hits affect no vertex. Maximum
triangle-centroid radial deficit falls from 0.00028479 at level 5 to 0.00007129
at level 6 and 0.00001788 at level 7. A flat subdivision of level 5 leaves every
existing vertex unchanged; added midpoints have minimum radius 0.99978632. That
operation preserves the original triangles' geometric planes.

For the stroke fixture, the harness reproduces the original app sampling rule
at spacing 5.76 logical pixels (`48 * 0.12`) on one straight 160-pixel path. It
maps pixel coordinate `s` to world `x=(s-80)*0.004`, `y=0.07`, then raycasts the
current mesh before each dab. The brush is Draw, radius 0.18, strength 0.35,
normal direction, and symmetry off. Each run starts from the same level-5 sphere.

| Input segmentation | Dabs including initial click | Maximum vertex displacement |
| --- | --- | --- |
| One 160-pixel update | 29 | 0.04012015 |
| 160 one-pixel updates | 53 | 0.07951421 |
| 16 ten-pixel updates | 33 | 0.04604861 |

The one-pixel input stream produces about 98% greater peak displacement than the
single update. The two final meshes differ by RMS position distance 0.00262867.
This is a failed consistency check, retained as baseline evidence.

Smooth also depends on topology scale: the original one-ring neighbor average
has maximum radial shrink 0.00018215, 0.00004679, and 0.00001186 on levels 5,
6, and 7 for one Smooth dab at radius 0.5, strength 0.5, hit `(0.07,0.13)`.
Total displacement includes tangential drift and is a different measure. Merely
increasing mesh density makes this smoothing operation weaker in world units.

One 64-sample native CPU pass on an Apple M2 with 24 GiB, Rust 1.96.0,
release `opt-level=2`, measured mean Draw/pick costs of approximately
0.584/0.361 ms at level 6 and 2.344/1.415 ms at level 7. This excludes rendering,
history snapshots, upload, UI overhead, and WASM. These exploratory timings
support a conservative level-6 default with optional higher density; they are
not a browser capacity or application frame-rate claim.

## Documented reference behavior

Maxon's [desktop beginner workflow](https://help.maxon.net/zbr/en-us/Content/html/getting-started/your-first-creature/your-first-creature.html)
connects small sculpting detail to polygon density and recommends Divide when
more detail is needed. Its [Geometry reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/tool/polymesh/geometry/geometry.html)
distinguishes flat subdivision from subdivision with smoothing enabled. OBrush's
triangle-only topology and any independently implemented smoothing algorithm
must be compared separately; no matching subdivision mathematics is claimed.

The [Draw reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/draw/draw.html)
documents control over radius, depth intensity, and Focal Shift's influence
curve. OBrush's original falloff is fixed at `(1-d²/r²)²`.
The [Stroke reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/stroke/stroke.html)
documents input averaging, and [Lazy Mouse](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/stroke/lazy-mouse/lazy-mouse.html)
documents stabilization and discrete brush-size-relative stepping. Those sources
describe available controls, not measured ZBrush default settings or a formula
for its brush engine.

Maxon's [Antialiasing reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/render/antialiasing/antialiasing.html)
describes edge smoothing and supersampling for the Best Renderer. It does not
establish ZBrush's live viewport sample count. A local controlled viewport
comparison remains necessary before claiming an equivalent appearance.

## Remaining validation and development

Native and browser resize/toggle/editing checks remain outstanding. Capture actual sculpted surfaces, not only an unmodified sphere.
Check physical-pixel dimensions, silhouette edges, fallback behavior and resource
cleanup; keep geometry tests separate from framebuffer quality checks. Long
input jumps and differences in world-space Smooth response across mesh densities
also need further performance and behavior work. The new straight-path sampling
test does not establish every input-device or curved-stroke behavior.

Adaptive topology, persistent multiresolution, reference brush calibration,
pressure, and full MatCap/material behavior remain additional requirements for
ZBrush-quality sculpting workflows. This investigation does not close their
parity gates.
