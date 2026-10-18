# Morph targets in OBrush 0.1.9

This milestone adds a geometry-only Morph Target foundation to the shared Rust
document. It addresses part of S07.2. Reference-equivalent Morph response,
Project Morph, difference-mesh creation and the full ZBrush workflow remain
separate gates.

## Workflow

Choose **Store target** in the Object inspector before sculpting. The saved
shape belongs to the active subdivision level. Ordinary brushes keep fixed
point correspondence while a target exists. **Morph** (shortcut **0**) brushes
points toward their saved positions with radius, strength, masks, pressure,
X symmetry and raster alpha. Alt/invert moves away from the stored positions.
The existing local deformation checks can reduce or reject a step; they do not
prevent all global intersections or repair old damage.

**Restore shape** recalls the entire saved geometry, keeping paint and masks.
It restores all points, including masked ones. **Swap** exchanges the visible
shape and the stored shape; **Replace target** stores the current visible shape
instead. **Delete** removes the target. Each control is undoable. A local brush
stroke is one undo step, and empty strokes/actions preserve redo.
Full Restore and Swap validate their endpoint geometry; they do not check the
continuous path between the shapes or prevent every self-intersection.

Only one target is stored. Adding or navigating retained subdivision levels
keeps it, but Restore, Swap and the Morph brush require its creation level.
Refine without smoothing, remeshing, successful triangle reduction and object
replacement remove it as one undoable operation. An explicit object/hierarchy
replacement removes the target even when the replacement mesh is identical.
Undo restores both the previous shape and target. Failed operations, a reduction
with zero collapses, and no-op Morph actions preserve the target. Core callers that
enable adaptive topology changes invalidate correspondence at the stroke
barrier; the application disables adaptive detail while a target is stored.

Targets can coexist with sculpt layers. Geometry restoration follows ordinary
layer recording: it changes the recorded layer when recording is active, or
the base otherwise. Target state and shape changes belong to the same undo
transaction. Paint and masks remain ordinary document channels.

## Persistence and bounds

A target-bearing `.obrush` project uses version 4 and stores
`morph_target: {scheme: "positions_f32_v1", creation_level, positions, triangles}`.
The target keeps exact float32 position bits and ordered triangle correspondence;
it does not store paint, masks or normals. Layers without a target remain
version 3, and plain projects remain version 2. Earlier version 1–3 files remain
readable. OBJ exports the currently visible mesh and does not carry targets.
As in version 2, the hierarchy's active mesh is stored in the main `mesh` field;
`multiresolution.levels[active_level]` is null, with inactive meshes in other slots.
A target on the active level binds to the main mesh; an inactive target binds to
its saved level mesh.

Imports must validate the target version, scheme, point/triangle counts,
coordinates and creation-level correspondence before replacing a document.
Targets are bounded to 1 million positions, 2 million triangles and 64 MiB of
stored capacity. The existing project input/output and combined history budgets
remain 128 MiB, with at most 64 history entries. These caps do not guarantee
peak process memory or interactive performance at the limit.

## Reference scope

Maxon's [Morph Targets guide](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/morph-targets/morph-targets.html)
describes one fixed-point target per SubTool, linked to its creation subdivision
level, and local restoration through the Morph brush. Its
[Morph Target reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/tool/polymesh/morph-target/morph-target.html)
also lists further controls. The historical 0.1.9 implementation had one document object. Version 0.1.10
[SubTools](SUBTOOLS.md) retain one target per part. Project Morph, Create Difference Mesh, MorphDist and matched reference fixtures
remain unfinished. No ZBrush response or family parity is claimed.

## Verification

The [build receipt](evidence/morph-target-build.json) binds **208 passing tests**
(188 core, 20 app), formatting, strict native/WASM Clippy, pure-core WASM
compilation, browser build and ad-hoc-signed macOS packaging. The final manifest
pins 67 source/helper files; the repository has no Git commit yet. WASM SHA-256:
`9391e86690f0320224f789626f56043a757dc8b111008635ff17297aa6d5de86`.
Native executable:
`87a3e2a0818dd63b85d0b9fdb36bdff0ec40768b08abedfe3c4588062237662e`.
Native GUI behavior was not exercised.

The [core receipt](evidence/morph-target-core.json) records 21 independent
contracts and five owned Morph tests. The independent standalone run used
separately recorded interim bytes; the final workspace run executes those same
contracts on the final formatted implementation. Checks cover exact f32
endpoints/signed zero, canonical save/history, protected local restoration,
level correspondence, layers, malformed metadata and bounded decoding. The
independent planar pressure and coarse-detail tests check `2e-7` and `3e-6`
world-unit error gates; observed maxima were not emitted. Array rows total at
most 36,000,000 bytes (about 34.33 MiB) plus small metadata, below the conservative
36 MiB upper bound and separate 64 MiB storage cap. Peak process memory was not
measured.

The [browser receipt](evidence/morph-target-browser.json) records eight workflows
and 48 explicit gates in isolated headless Chrome 155.0.8059.39 on Apple M2, at
1600×1000 CSS pixels and scale 1. All nine pages reported WebGL2 with allocated
4× multisampling and no GL, JS or CDP errors. All 66 saved snapshots matched the
packed GPU vertex data. These cover positions, normals, masks and RGB; index
buffers were not read back. All 58 target snapshots passed creation-level
binding/availability checks, and all 15 layered snapshots matched independent
composition exactly.

Actual UI actions exercise Store/Restore/Swap/Delete with history, local Morph,
masked synthetic pen pressure/radius, fresh version 4 load, malformed input and
zero-contact preserving redo, historical version 2/3 imports, explicit refinement
invalidation, signed-zero subdivision navigation and combined layer recording,
restoration, swap and bake. Older imports compare editable fields/mesh bits,
not the older encoder's canonical byte spelling.

On the independently half-masked default sphere, the local Morph stroke changes
375 vertices and reduces total squared distance to the target from
`0.09280905963156345` to `0.05373601847029082` (42.10046%). All 20,353 fully masked
vertices remain exact, with no observed per-vertex error increase. The matched
half-force trial with pressure radius enabled changes 129 vertices and reduces
error by 20.70737%. On the deliberately coarse retained fine level, a recorded
layer's Morph changes 12 vertices and reduces its target error by 78.43909%;
full Restore/Swap/Bake and their undo/redo retain exact accepted state. These
are fixture results, not reference response or a general quality guarantee.

Eight short stroke/idle intervals remain quiet with zero pending dabs at release
probes using incremental 0/200/800 ms waits. First probes start 1.8–7.0 ms after
trusted DOM release, before asynchronous readback completes; rAF callback-gap
p95 is 16.7–16.8 ms, maximum 16.8 ms, with no observed tasks above 50 ms in these
intervals. They describe scheduling and quiet release, not rendered frame time,
physical input-to-photon latency, large-target performance or long sessions.

Three earlier browser failures remain recorded with hashes and diagnoses: a
contact trace missed most of the displaced patch on the angled view; the test
oracle incorrectly read an active hierarchy slot instead of the main mesh; and
synthetic wheel events failed to move the pointer into the intended inspector.
The final helper corrects those inputs/oracles and verifies actual scrolling.
No acceptance threshold was relaxed and no compiled application input changed.
Only the helper and a core README format clarification changed after source
checks. Frozen build receipts retain their original pending-runtime statements;
the later public receipt binds the completed runtime without rewriting them.
