# Geometry sculpt layers in OBrush 0.1.8

This milestone adds an independently written geometry-layer foundation to the
shared Rust document. It addresses part of feature family S07.1; it does not
establish ZBrush 3D Layers parity.

## Workflow

Choose **New layer** in the Object inspector to start recording shape edits.
Draw, Move and the other geometry brushes use the existing shared brush engine.
**Stop recording** makes subsequent shape edits affect the base while preserving
the saved layer changes. Selecting a row chooses its controls; **Record layer**
records on that visible layer at 100% intensity.

Use **Intensity** to fade a layer from 100% to 0%. The number previews during a
slider drag; releasing commits one undoable change. Hiding a layer or lowering
the recorded layer below 100% stops recording. Show it and return it to 100%
before recording again. Visibility, rename, delete, record state and intensity
are part of document history. Empty strokes preserve redo.

**Bake visible** keeps the currently visible composite and removes the complete
layer stack, including hidden layers. Undo restores it. Exported OBJ contains
the current visible geometry; save an `.obrush` project to retain layers.

## Correspondence and persistence

Layers retain a fixed ordered triangle/vertex correspondence on the level where
they are created. Adaptive brush refinement, primitive replacement, subdivision
changes, level navigation, remeshing and decimation are blocked while layers
exist. An existing subdivision stack remains stored; baking enables its normal
navigation and detail-transport rules. Importing another validated document is
a separate undoable whole-document replacement.

Geometry offsets and the base use double precision. Composition adds visible
weighted offsets in stored order, skips zero component contributions to preserve
signed zero, and converts the result to the mesh's float32 positions. Intensity
is float32 and is promoted to float64 for that sum. Stroke capture must reproduce
the accepted visible position bits exactly;
unrepresentable edits are rejected with complete stroke rollback. Normals are
derived from the resulting geometry. Mask and RGB changes remain ordinary
document channels and do not disappear when hiding a geometry layer.

JSON decoding uses `serde_json`'s `float_roundtrip` parser feature. The default
parser changed some stored f64 offsets by one ULP in the first independent file
tests, failing four exact Save→Load→Save checks. Those failed logs are retained;
the fix preserves the original tolerances and data contract.

Layer-bearing projects use version 3 JSON. Projects without layers continue to
use version 2, and versions 1 and 2 remain readable. Imports validate the layer
scheme, topology, dimensions, finite values, names, intensities, recording state
and the exact stored composite before replacing the document. Encoded input and
output remain limited to 128 MiB.

There are at most 16 layers, subject to a shared limit of 1,000,000 base-plus-delta
vectors and 64 MiB of layer storage. Density therefore limits the available
layer count. A name is at most 256 UTF-8 bytes. Deserialization applies the shared
vector budget while reading arrays. History retains the existing combined
128 MiB / 64-entry budget; layer snapshots count toward it. These are allocation
limits, not total process-memory or real-time performance guarantees.

## Reference scope and remaining gaps

Maxon's [3D Layers guide](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/3d-layers/3d-layers.html)
and [Layers reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/tool/polymesh/layers/layers.html)
describe geometry, PolyPaint and mask recording, intensity inversion/exaggeration,
additional stack operations and subdivision/timeline interactions. OBrush's
current geometry-only stack uses intensity 0–1 and fixed creation-level
correspondence. Negative intensity, exaggeration above 1, layered paint/masks,
reorder/duplicate/split/merge, per-level recording, MDD/timeline/pose workflows
and reference-equivalent response remain separate gaps. Morph Targets,
HistoryRecall and Thick Skin are separate S07 gates.

Recomposition rejects collapsed/nonfinite geometry but does not establish global
self-intersection prevention. Layer capture/serialization, recomposition,
normals and GPU uploads remain proportional to mesh/layer size. Native GUI,
physical tablet behavior and ZBrush fixture equivalence require separate checks.

## Verification

The [build record](evidence/sculpt-layers-build.json) binds **179 passing tests**
(161 core, 18 app), formatting, strict native and WASM Clippy, pure-core WASM
compilation, `scripts/build-web.sh` and ad-hoc-signed `scripts/package-macos.sh`.
The repository has no Git commit yet, so a 63-file source manifest identifies
the checked implementation. The final browser artifact SHA-256 is
`da6ab1eb12cd3702d57f88ce8feae2f3123c1fed8722f207c91f169ee53220a3`;
the native executable is
`2f604ebf043d304bf07f44a400927ead3db1e7f6dc590140f1c55e4c7b3ab0f7`.
Native GUI interaction was not exercised.

The [shared-core evidence](evidence/sculpt-layers-core.json) records 18 independent
layer contracts, three signed-zero checks and four document bounds/memory tests.
They cover exact visibility/intensity endpoints, independent additive blending,
coarse-level bake/detail retention, rejected strokes/imports, undo/redo and
allocation gates. The additive and multiresolution tests check tolerances of
`2e-7` and `3e-6` world units respectively; they do not emit observed maxima.
The first run's four failed f64 metadata round trips remain recorded: 74 differing
scalars, each one ULP, maximum absolute difference `2.7755575615628914e-17`.
Enabling `float_roundtrip` fixed them without changing those contracts.

The [browser evidence](evidence/sculpt-layers-browser.json) records seven actual
UI workflows in isolated headless Chrome 155.0.8059.39 on Apple M2, at
1600×1000 CSS pixels and scale 1, with WebGL2 and allocated 4× multisampling.
Public UI clicks, file inputs and captured Save bytes exercise Draw/Move layer
independence, 0/50/37/100% intensity, visibility, base edits with recording off,
delete/bake with history, fresh project load, malformed import preserving redo,
historical version 2 import, and retained subdivision navigation after bake.
The historical import check compares editable fields/mesh bits, not the older
encoder's canonical bytes.

All 52 saved snapshots match the packed GPU vertex data, and all 42 layered
snapshots match the independent ordered f64-to-f32 composition oracle exactly.
This GPU check covers positions, normals, masks and RGB, not an independent
index-buffer readback. Default-radius/strength Draw changes 850 vertices, with
maximum displacement `0.02551279856227842` world units; the independent inferred
half-intensity response has maximum component error `7.000000001866624e-8`,
below the declared `3e-7` gate. The separately recorded Move changes 626 vertices,
with maximum displacement `0.022776244626112097`. A deliberately coarse closed
latitude/longitude sphere checks signed zero, actual subdivision, useful fine
layer edits and exact bake/history/navigation retention. Its purpose is
correspondence preservation, not production sculpt quality.

Five short brush traces have no pending dabs and unchanged geometry at three
release probes, with incremental waits of 0/200/800 ms. The first probes begin
4.4–9.7 ms after trusted DOM pointer release, before their GPU readback completes.
Their rAF callback-gap p95 is 16.7–16.8 ms, with no observed tasks above 50 ms in
the short stroke/idle intervals. These measure scheduling and quiet release;
they do not measure rendered frame time, physical input-to-photon latency,
long-session behavior or large-layer performance.

Two earlier failed browser attempts remain retained. The first found that a
floating scrollbar covered the 100% slider endpoint; scrollbars now reserve
space and the required build checks were rerun. The second clicked a fractional
coordinate intended to select 37%; eframe's integer `client_x` binding selected
36%. Rounding the harness's intended coordinate to the nearest pixel fixed that
setup failure without changing production code or thresholds. The final helper,
runtime, fixture and screenshot hashes are recorded. No failed record was
rewritten; the frozen build record's original pending-runtime statement is
superseded by the later public receipt.
