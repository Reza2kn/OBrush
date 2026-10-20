# C289 prospective Source Drag camera and redo-normal repair

**Status: static draft only. No C289 product edit, Cargo build, browser run, or
acceptance result exists.** C286 setup and C288 geometry/GPU failures remain
unchanged. The read-only C288 attribution and independent replay are in
`SCULPTGL_SOURCE_DRAG_C288_CAUSAL.md`; the candidate Drag product code is
reverted. C289 is a distinct future attempt against pinned SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The ordinary OBrush browser baseline restored after C288 has WASM SHA-256
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`
and JS SHA-256
`03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.

## Supported mechanisms and candidate boundaries

C288's five-step live Drag replay explains the entire paired After position
error to at most `7.450580597e-9` component residual when each side uses its
own captured picked centers and radius. Replaying the Source Before mesh with
OBRush's centers/radius equals OBrush After bitwise. The browser camera/pick
inputs, rather than a measured kernel or sample-count difference, caused the
`2.060085535e-6` miss on this fixture. Source's post-fit X/Y projections
were `2.1905438899993896`/`2.776134729385376`; OBrush's were
`2.1905605792999268`/`2.776155948638916`. The next correction belongs in
the normal camera fit/picking host, not in a Drag-specific scale fudge or
weakened sculpt tolerance. It must derive the published frame from the pinned
reference's visible mesh bounds/fit and preserve other camera workflows.

Separately, Source's `Drag.js` refreshes selected normals after each dab, while
`StateGeometry.pushVertices` records the selected union and Undo/Redo calls
`mesh.updateGeometry` once on that union. C288 Source After/Redo positions are
identical, but 31 active normal rows differ; OBrush's rejected candidate kept
snapshot normals exactly. OBrush core already has bounded
`SourceHistoryEvent::Geometry` and
`SourceGeometryNormalState::refreshed_for_history` for selected-union history.
A C289 Drag candidate may opt into that shared-core history policy, provided
pre-write allocation/history budgets and failure rollback remain atomic;
normal recomputation belongs in `obrush-core` for native/WASM sharing. The
existing Legacy Drag route remains the default for old projects.

## Frozen diagnostic before product edits

After the C287 Cargo/browser lane is explicitly released, capture one fresh
normal-UI setup on the same CC0 fixture and controls below, **without a
stroke**. Record Source `getLocalBound`, `computeWorldBound`, camera projection,
translation, center and settled timers, and OBrush source-framing bounds,
projection/zoom, viewport and selected object. This read-only witness must
identify whether the fit difference is due to source loose/world bounds,
float32 operation order, animation completion or another concrete cause.
If it does not, stop without a camera/product edit. Keep the raw setup receipt
and exact diagnostic harness SHA before running it. Do not calibrate a fixed
constant from one projection pair.

The frozen diagnostic is
`scripts/check-sculptgl-source-drag-c289-bounds.mjs` (SHA-256
`77dbde73227c46443f4b4f7d289a79d339b2f6e7e1e5422fe83da1be7988303a`).
It reuses the C288 ordinary UI fixture import, 1024×808 viewport, front
orthographic selection and off-mesh fit gesture, but ends after a public
Before Save. It records Source local/world/visible-scene bounds directly from
the active mesh and camera, Source timer entries before and after fit, and
OBRush's public camera/source-framing snapshot and active object. Each receipt
records its exact reference, fixture, harness and served WASM/JS hashes. Run
Source and OBrush as separately owned browser processes using `--side=source`
and `--side=root`; never run them concurrently. The normal UI setup must
publish one 1,089-vertex active object, 1024×808, finite bounds, a settled
Source timer set, and matching public Before mesh identity. This is a causal
diagnostic, not a numerical parity pass. A missing witness, changed build or
fixture, unresolved fit discrepancy, or ambiguous timer/pivot state is a
**no-go for camera product changes**. Preserve raw failed receipts.

## Independent normal-UI acceptance fixture

Reuse independently authored CC0 33×33 curved-grid SGL fixture SHA-256
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`
(1,089 RGB-ID vertices, 2,048 triangles, protected ID544 and
half-unprotected ID545). Create fresh pinned Source, candidate Source Drag,
Legacy Drag compatibility and zero-travel documents. Both paired documents
use 1024×808 logical viewport, front orthographic normal-UI fit, mouse,
radius96px, pressure factors zero, Alpha None, symmetry/culling/Continuous/Lock
off. Deliver integer offsets `(-64,-20)` at press, then `(-28,-20)`, `(8,-20)`,
`(44,-20)` in three distinct held moves, release at `(44,-20)`. Each segment
has two 14.4px-spacing Source substeps. No Save/UI interaction during hold.

Capture public Before/After/Undo/Redo Save and active GPU position/display
normal words at Before, each held move, After, quiet After, Undo and Redo.
Before any geometry comparison, the candidate published X/Y projection
ratios relative to Source must each differ by <=`1e-8`; requested world rays
and radius by <=`1e-8`; picked press point and all six moving centers by
<=`1e-7` per component. These are new causal input gates, stricter than
C288, not retrospective changes to C288. Candidate XYZ and active GPU
display normals must differ from Source by <=`1e-6` at every held/history
stage; changed RGB-ID membership must match exactly. GPU position words at
public stages must equal each side's own public Save exactly. ID544 must stay
fixed; IDs541 and548 must both move, and held captures must progress.
Source/OBRush must also match the RGB-ID set of active normal rows whose
After-versus-Redo words change, and paired Redo normals must pass `1e-6`.
Each side's public Undo must restore exact own Before bytes; Redo exact own
After bytes; one gesture adds one undo step. Attributes/topology stay exact.
Quiet GPU matches After; one active VBO and correct active object are required.
The zero-travel control leaves geometry/history unchanged; invalid input
after an admitted preview restores exact Before and remains noncommittable.
Legacy Drag default behavior and old-project mode selection remain intact.

Before **any C289 product edit**, independently author and commit the new
normal-UI harness, numerical audit, source-bounds setup diagnostic, and their
SHA-256s here. One bounded pair follows a successful causal fix and full
`cargo fmt`, workspace tests, strict Clippy, WASM target/build checks. Preserve
all failed setup/numerical receipts. A scoped pass leaves dynamic topology,
alpha, symmetry, alternate cameras and long-session response open and does
not close full `SGL-BRUSH-DRAG` parity.
