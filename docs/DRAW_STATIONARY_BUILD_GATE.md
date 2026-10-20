# Stationary Draw build: scoped release gate

The user reported that repeated default Draw contacts at one cursor point stop
building outward. This gate uses the shared Rust core, without viewport/GPU
timing. It does not establish SculptGL or ZBrush brush parity.

## Frozen input and thresholds

Start from `Mesh::sphere(6)` (40,962 vertices), use independent current-surface
ray picks from `(0.23, 0.09, 3)` along `-Z`, and apply 256 Draw dabs. Each dab
represents a separate mouse contact at the same screen point. Settings are
radius `0.180`, strength `0.35`, X symmetry on, pressure `1`, pressure-affects-
strength on, adaptive detail on, backface culling on, and all other
`BrushSettings::default()` values. Record the picked Z after contacts 0, 1,
16, 64, 128 and 256, first clipped dab, first unchanged dab, added topology,
minimum equilateral-normalized triangle quality, and maximum adjacent face
angle. Recheck exact stroke Undo/Redo on a smaller independent sequence.

A product candidate passes only if `Z(256) - Z(128) >= 0.05`, every picked Z is
finite and nondecreasing at the recorded milestones, all geometry validates,
minimum triangle quality is at least `0.15`, maximum adjacent face angle is at
most `60.001°`, and Undo/Redo exactly restores positions, triangles, masks and
colors. The final result must preserve the existing deformation guard; a
height gain obtained solely by weakening its face/fold limits fails this gate.
This numerical criterion was frozen after the baseline and simple tangent-only
diagnostic below, before a remedy was selected. It is a local user-intent
threshold, not a reference-product tolerance.

## Baseline and failed simple candidate

Baseline product source `brush.rs` SHA-256
`e3b2fda2cb503d4f3bc6e94de774100e7510726e4cb0062873f4894c93a674c9`:
picked Z was `[0.96895695, 0.9737718, 1.0440274, 1.252614,
1.3572633, 1.3572633]` at the six milestones. The first unchanged dab was
96, with 161 unchanged dabs through 256. Final minimum quality was
`0.2254066220`, maximum adjacent face angle `26.502622°`, and topology
43,750 vertices / 87,496 triangles.

A diagnostic change removing only the normal part of Draw's one-ring fairing
still nearly stopped: picked Z was `[0.96895695, 0.973793, 1.0457488,
1.2711881, 1.3747365, 1.3750261]`. The guard first clipped at dab 87 and
reached an accepted scale of `1/2048`; all 256 dabs changed some vertices, but
`Z(256) - Z(128) = 0.0002896`. Final minimum quality was `0.222228891`,
maximum adjacent angle `30.24766°`, topology 43,572 / 87,140. Removing
normal fairing alone is therefore a failed control. A second supporting-only
normal fairing diagnostic gave the same height trajectory and clipping scale.

The first causal question is which face/fold constraint clips at dab 87. A
bounded repair should improve the offending local geometry or displacement
profile while retaining the guard and the quality thresholds above.

## Causal result and negative controls

In the tangent-only trial, the first clipped dab was 87. The first rejected
full-step constraint was face `[20957, 43448, 43020]`: its signed projection
remained `1.01493` times the old face, but its `cross.length / longest_edge²`
quality decreased from `0.09850155` to `0.09839990`. Because that old quality
was already below the per-face `0.1` floor, the guard correctly prohibits any
further decline. Its normals rotated only slightly; a folded or inverted face
was not the first problem. At dab 128, 421 vertices were within radius, so the
stall was not a missed ray or an empty brush footprint. The baseline first
clipped at dab 90, then became a no-op at dab 96 with 505 vertices still in
radius. The fairing adds an opposing force, but removing it alone does not
solve the guard-limited plateau.

One temporary local guard-salvage prototype halved displacement only on the
vertices of offending faces/folds for up to 16 rounds before falling back to
the original global guard. It retained the original guard and increased height
briefly, but also failed the frozen growth gate: with original fairing,
`Z(128) = Z(256) = 1.4251163`, first no-op dab 106, final normalized minimum
quality `0.1638516`, maximum adjacent angle `57.8472°`. Combined with tangent-
only fairing, `Z(128) = Z(256) = 1.5205014`, first no-op dab 118, quality
`0.1660501`, angle `59.9991°`. The latter nearly exhausted the 60° guard. Both
experimental source changes were discarded; no weaker guard or visible
sharpness regression was shipped. The persistent defect needs a bounded
local remeshing/quality-repair strategy or a different Draw displacement profile
validated against this gate and the earlier overlapping-stroke fixtures.

The ignored test `draw_stationary_build` retains the failing numerical gate and
its live-pick trajectory. Run it explicitly with
`cargo test --locked -p obrush-core --test draw_stationary_build -- --ignored --nocapture`.
The expected assertion failure is preserved until a real fix passes. A passing
workspace suite alone must not be read as a stationary Draw fix.
