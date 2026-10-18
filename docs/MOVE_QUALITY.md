# Move gesture quality in OBrush 0.1.7

This update addresses thin, sharply folded triangles produced by small, strong
Move drags. It is an independent OBrush correction, not a comparison to ZBrush's
Move algorithms or a feature-parity promotion. The reported settings were
radius **0.037**, strength **79%**, and X symmetry enabled.

## Reproduction

The frozen 0.1.6 browser build has WASM SHA-256
`9499dcea1ead33a196010b41ef38c83383b4b9858b748ad0ea42be752c9d9615`.
The isolated headless Chrome harness uses actual canvas controls, pointer
down/move/release, project download and WebGL buffer readback. It does not attach
to user browsers, native apps or accessibility services.

On the default 40,962-vertex sphere, a 180/-80-pixel Move drag increased the
longest edge from **0.02067 to 0.17852** world units. Minimum triangle quality
(`length(cross) / longest_edge_squared`) fell from **0.68823 to 0.02000018**;
34 adjacent edges exceeded 90 degrees, reaching **124.71 degrees**. The mesh
remained closed and consistently wound, and the displayed GPU data exactly
matched the saved mesh. Valid topology alone therefore missed the visual defect.

A crossing drag that returned to its exact starting pixel still left **0.15581**
world units of maximum displacement and a fold close to **150 degrees**. The
old brush recomputed influence and facing on the moving surface, progressively
losing the original grab. Move also skipped local refinement and selected one
mirrored displacement abruptly where symmetry footprints overlapped.

The original seven-case baseline receipt and saved meshes/PNGs remain under
`.artifacts/move-regression/baseline-2e0bdefc-5c0f-4e09-aedb-c97b8a21e24f`.
Its first partial-mask fixture accidentally protected the whole grab, so that
case was a valid no-op rather than a partial-mask movement check. The corrected
independent fixture and this failed setup are tracked separately. No result
overwrites the user's supplied project or OBJ.

## Shared geometry and input behavior

`Mesh::prepare_move_stroke` captures rest-space positions, mask protection,
front-facing influence, raster alpha and mirror symmetry. Its compact radial
field blends mirrored displacement vectors continuously. `MoveStroke::apply`
takes total displacement from the gesture origin. Input-event subdivision does
not repeatedly change the material selection or weaken the deformation guard.

Preparation adds bounded linear detail out to twice the undeformed brush radius,
using the active radius for its edge-length target, with at most four rounds /
2,048 new vertices. The work budget can still leave transition faces near the
active footprint. Preparation does not change the document
until a useful deformation is accepted. Move then retains fixed topology during
the gesture. A persistent subdivision stack disables this refinement to retain
level correspondence. Returning to a zero target, or an exactly cancelling
symmetry target, restores the original mesh and topology; an empty gesture
preserves redo.

Move preparation evaluates child quality as `length(cross) / longest_edge_squared`,
requiring `min(parent_quality, 0.15)` within a `1e-5` relative tolerance for each admitted split. Healthy generated
faces therefore have room above the deformation floor of 0.1. Ordinary Draw and
other dab refinement keep their preceding quality policy; the Move correction
does not silently change those algorithms.

The complete target is checked against the resting surface, including face
orientation, triangle quality and adjacent folds. Bounded refinement of the
accepted step avoids the sudden halving of an absolute target when a drag
crosses a safety limit. Existing bad geometry sets its own lower baseline; these
checks do not silently repair an old damaged sculpt.

The host captures a camera plane and depth, accumulates screen travel in double
precision, and applies the latest target synchronously. Move has no dab backlog.
Leaving the canvas pauses movement and discards outside travel; re-entry resumes
without a jump, and release outside commits the last displayed result. Shift or
Ctrl tool changes and Alt inversion changes start a new gesture on the committed
surface. They make separate undo entries within one held contact, preserving
intervening Smooth/Mask work. Release and command-frame endpoints use the captured
gesture kind/inversion, so a release-time Ctrl chord does not become a Mask dab.

Radius is fixed for each Move gesture. Current strength and pressure scale the
absolute target. The host defers zero-pressure preparation until positive
contact so optional pressure-radius captures that contact's radius. Mouse input
uses full pressure. The cursor remains the nominal-radius cursor; physical pen
hardware and native GUI interaction remain unverified.

## Final verification

The [comparison record](evidence/move-quality-comparison.json) binds the original
baseline, rejected candidate, final browser run and independent native diagnostic.
The [build record](evidence/move-quality-build.json) pins 52 source/helper files,
the required check logs and the actual generated artifacts. Final WASM SHA-256 is
`7632d4e47f03f69b6cd8428060e34071385db3aaf21291b9aaddce47831af4bb`.
Both current local artifacts match the frozen release copies; the user's supplied
OBJ and project hashes are unchanged.

The fresh-sphere comparison uses identical initial geometry, viewport, pixels,
radius 0.037, strength 0.79 and X symmetry. These are geometry comparisons, not
matched performance or ZBrush comparisons.

| Measurement | Frozen 0.1.6 | Final 0.1.7 |
| --- | ---: | ---: |
| Outbound drag: longest edge, world units | 0.17852 | 0.02422 |
| Outbound drag: minimum triangle quality | 0.02000018 | 0.112174 |
| Outbound drag: maximum adjacent fold | 124.71° | 68.82° |
| Outbound drag: edges folded beyond 90° | 34 | 0 |
| Outbound drag: maximum original-point displacement | 0.16211 | 0.01129 |
| Crossing drag returned to press: maximum displacement | 0.15581 | 0, exact original mesh |
| Symmetry-seam drag returned to press: maximum displacement | 0.13976 | 0, exact original mesh |
| Supplied sculpt after matched drag: edges folded beyond 90° | 98 | 78, equal to its initial count |

The final run passed **14 browser interaction scenarios** using canvas controls,
public file inputs/downloads and WebGL buffer readback. They cover outbound and
return paths, symmetry, partial/full masks, the unchanged supplied sculpture,
Move→Smooth→Move transitions, zero-pressure contact, pressure-radius capture,
release-time Ctrl, eight repeated grabs and canvas exit/re-entry/release outside.
Every saved mesh exactly matched the GPU position/normal/mask/color vertex-buffer
bytes, all measured releases
stayed quiet, and every performed undo/redo check restored the expected mesh
exactly. Return/full-mask scenarios intentionally end unchanged and do not imply
14 distinct movement or history passes.

Eight repeated contacts created seven effective history states and one exact
no-op. The final sphere had 45,802 vertices / 91,600 faces, maximum original-point
movement **0.01757**, minimum quality **0.099999885** (approximately 0.1 with
floating-point tolerance), maximum adjacent fold **53.67°**, and no folds above
60° or 90°. The captured image shows shallow local edits without protruding
slivers. The unchanged user sculpture moved **0.01217** at most across 144
original vertices; its pre-existing 78 edges above 90°, their total length
**1.01920648**, and maximum fold near 150° remained. This is prevention of further
damage in that fixture, not repair of the old geometry.

Final synthetic headless traces ran after the native diagnostics/builds exited.
Per-scenario 95th-percentile `requestAnimationFrame` callback gaps were
**16.7–16.8 ms**, the maximum gap was **33.4 ms**, and no task above 50 ms was
recorded. Those callbacks cover dragging plus post-release idle and do not measure
sculpt-render FPS. First quiet vertex-buffer probes **started 2.7–8.1 ms** after
the DOM release event; readback completion was not timed. Later probes remained
unchanged with zero pending dabs. These measure host scheduling in short local
traces, not physical input-to-display latency, GPU execution time or long-session
performance. Vertex-buffer equality does not independently check the GPU index
buffer. The baseline and focused probe do not establish a
matched timing improvement. The owned Chrome/server closed after the final run.

The workspace passed **156 tests** (138 core / 18 app), formatting, strict
native/WASM Clippy, pure-core WASM compilation, the browser build and ad-hoc-signed
macOS packaging. Twelve core Move tests include the recorded camera/nearby grabs,
event subdivision, protection, useful movement, exact return and history. Two
new host tests exercise screen-plane accumulation and exact return. Native GUI
and physical tablet interaction were not executed.

An independent 28-case native sweep passed all quality/fold, one-versus-twelve
event and exact-return gates. It passed **24/28** stronger movement-range gates;
four failures remain, with their original thresholds preserved:

| Press offset from the recorded point | Pull | Measured maximum movement | Required movement |
| --- | --- | ---: | ---: |
| −24, +24 pixels | Proportional camera-plane | 0.00855 | 0.00925 |
| −24, +24 pixels | Long camera-plane | 0.00861 | 0.00925 |
| −24, +24 pixels | Long normal | 0.02757 | 0.03700 |
| +24, +24 pixels | Long normal | 0.02785 | 0.03700 |

Those measurements include new vertices against the prepared resting surface.
The native diagnostic adds only read-only instrumentation to copied sources;
it does not establish bit-identical native/WASM internal picking or reference
equivalence. The remaining range limits are separate from the corrected
near-zero response of the rejected browser candidate.

## Limits

Small brushes cannot support arbitrary pulls without severe distortion; the
guard can limit them. Use a larger brush for broad shape changes or a finer
subdivision level for fixed-topology detail. Automatic refinement is split-only,
not dynamic remeshing, simplification or a global collision detector. Geometry
already damaged before the update is not guaranteed to improve.

Capturing a gesture keeps additional mesh/position snapshots and topology caches.
Whole-mesh comparisons, rendering snapshots and individual synchronous operations
remain work proportional to mesh size. Local headless timings do not establish
physical input-to-display latency or real-time performance on every device.

## Retained failed checks

The first candidate passed 154 workspace tests and built on both targets, but
the matched browser fixture exposed an unusable Move response: it added 1,024
vertices while moving original points by only **0.000001766** world units. Its
new points did not hide a useful pull: maximum radial deviation was only
**0.00005668**. Its build remains frozen in
`.artifacts/move-release-0.1.7-candidate-1`, with runtime observations in
`.artifacts/move-regression/candidate-1-fef965fb-b36e-4167-b9f6-023388bf5e90`.
The group was explicitly stopped after this failure; the recorded stop-related
CDP timeout is not an independent application exception. The served local
artifacts were restored to the preceding 0.1.6 build while correcting it.

The refinement child-quality test and Move guard originally used different
triangle-quality measures. Some generated fringe faces passed refinement but
were already below the Move threshold. Preventing any further deterioration of
those faces reduced the entire shared displacement almost to zero. A check on
valid topology or event-independent return-to-origin did not establish useful
Move behavior; the browser regression now requires useful displacement as well.

An earlier native candidate also failed its proposed requirement of more than
two brush radii of extension under an extreme tiny-radius pull. That failure is
retained in `.artifacts/move-surface-final-v1/failed-2radius-tests.log`. The
accepted requirements distinguish proportional small pulls, safely clamped
extreme small pulls and useful broad pulls; the original requirement has not
been relabeled as passed.
