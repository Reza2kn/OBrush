# OBrush core

Portable CPU mesh and sculpting engine shared by native and WebAssembly builds.
This is an initial sculpting foundation; it does not provide ZBrush feature parity.

`Mesh` supports icospheres, a subdivided cube, corner-angle-weighted normals,
triangle raycasting, linear refinement, Loop smooth subdivision, and OBJ geometry
with per-vertex RGB colors.
OBJ color import accepts inline vertex RGB and ZBrush's `#MRGB` comments. Each
MRGB record contains eight hexadecimal digits, `MMRRGGBB`; RGB channels decode to
byte / 255. Multiple rows form one complete stream matching the vertex count.
A complete MRGB stream overrides inline RGB. Invalid hex, partial records, empty
rows, excessive counts, and count mismatches are rejected. The leading MM byte is
validated but ignored: OBrush does not claim ZBrush OBJ mask compatibility.
Brushes are Draw, Clay, Inflate, Smooth, Flatten, Crease, Move, Mask, Paint, and
Morph. Morph requires the Document target API; the generic Mesh entry point
has no target and safely makes no change.
Brush dabs use front-facing vertices, mask protection, and optional X symmetry;
overlapping mirrored dabs use their maximum influence. Smooth reads the complete
previous vertex state. Inversion reverses geometric dabs, erases masks, and restores
the default surface color when painting.

Draw uses a coherent patch direction, blends mirrored directions, and fairs local
surface bumps. Accepted Draw, Clay, Inflate, Flatten and Crease dabs refine long nearby edges without changing the
existing piecewise-linear surface. Refinement interpolates masks/colors, preserves
exact mirrored geometry, and is bounded to two rounds and 1,024 added vertices
per call. The entire stroke, including added topology, remains one undo step.
Smooth, Mask and Paint retain their topology. `adaptive_detail=false` keeps topology fixed for persistent levels. Pressure and immutable raster alphas weight the same shared geometry brushes. See [Draw quality evidence](../../docs/DRAW_QUALITY.md)
for measured overlap behavior and limitations.

Use `Mesh::prepare_move_stroke` and `MoveStroke::apply` for a Move gesture: supply
total displacement from pointer-down, rather than applying incremental Move dabs
to an evolving selection. Preparation captures material influence, masks, facing,
alpha and symmetry, with at most four local refinement rounds / 2,048 new points.
It leaves the mesh unchanged until a useful displacement is accepted. Exact
return-to-origin restores the original topology and attributes. Pressure and
strength scale the current absolute target; radius is captured at preparation.
Prepare a fresh gesture after semantic changes or external mesh edits. The legacy
single-dab Move entry point remains available, but does not provide this gesture
contract. These bounds and local deformation guards do not detect every global
self-intersection or repair an already damaged mesh.

`Document` owns up to 32 separate SubTools with stable IDs, selection, names, visibility and Solo. Each keeps its own mesh, masks, colors, levels, layers and target. Whole-scene history restores deleted parts and their state. Version 5 stores the scene; versions 1–4 remain readable. Inactive meshes use immutable shared snapshots. See [SubTools](../../docs/SUBTOOLS.md) for the schema, aggregate decode/storage limits and gaps.

`Document` groups dabs into undoable strokes and makes topology changes, imports,
and mask operations separate history entries. Geometry layers record fixed-topology
world-space offsets with intensity 0–1, visibility, rename/delete and baking;
paint and masks remain document channels. Layers without a Morph Target use
version 3.
One geometry-only Morph Target stores exact positions and ordered triangles on
its creation subdivision level. Store/delete/restore/swap and local masked Morph
restoration are undoable; explicit topology replacement invalidates the target
with undo, while retained subdivision navigation keeps it on its origin level.
Target-bearing projects use version 4. Version 2 JSON preserves the mesh, masks,
vertex colors, document name and optional persistent Loop levels without those
features; versions 1–3 remain readable. Normals are rebuilt on load. Low-level geometry edits
propagate upward while retaining fine detail; new source-static hierarchies use
normal-frame transport, while Loop and older static hierarchies retain world-space
residuals. Fine edits do not reverse-project down.
Project files do not retain history or application settings. OBJ has no mask
channel. The bounded studio importer retains face-corner UVs and supported
vertex color/material data; grouping semantics and external textures remain gaps.

`Mesh::remeshed` and `Mesh::decimated` return validated new meshes plus reports,
without changing the source. Progress callbacks allow cancellation. Voxel
remeshing takes closed solids; it is not guaranteed to repair existing folds.
Decimation pins protected masks, boundaries and sharp features and may stop
above the requested count. Neither is quad retopology or reference parity.

Bounds are one million vertices, two million triangles, 128 MiB of project/OBJ
input, and 128 MiB of project output. Persistent subdivision is capped at eight
levels with aggregate geometry limits across all objects and levels. Topology operations have additional work
and input caps; brush alphas are bounded to 1024 × 1024 grayscale samples. OBJ faces are limited to 512 vertices;
concave polygons are triangulated and intersecting edges rejected. History keeps
at most 64 snapshots and 128 MiB of reserved document snapshots, including stored
levels, layer/target data and names. Layer stacks cap base-plus-delta vectors at
1 million, names at 256 UTF-8 bytes and stored capacity at 64 MiB, with at most
16 layers. Morph Target arrays are independently bounded to 1 million positions,
2 million ordered faces and 64 MiB of stored capacity. Active geometry, the
open stroke snapshot, temporary subdivision storage, smoothing adjacency, and
renderer allocations are additional memory. Project serialization borrows the
mesh and stops before writing beyond its output limit.

Run correctness checks with:

```sh
cargo test -p obrush-core
cargo clippy -p obrush-core --all-targets -- -D warnings
```

The core tests include two public-API integration workflows: picking and sculpting
a visible sphere, grouped undo/redo, fully masked paint protection, polypaint
project and OBJ restoration, subdivision save/reload, import history, and derived
normal reconstruction. Unit tests also exercise malformed inputs, symmetry seams,
stable smoothing, brush inversion, allocation limits, and history eviction.
Five MRGB integration tests cover nonuniform colors split across rows, precedence,
malformed data, ignored mask bytes, and import limits. The small fixture is
synthetic and follows the format observed in a ZBrush-exported OBJ.
Draw and patch-refinement regressions additionally cover overlapping strokes,
exact symmetry, mask/no-op protection, shading invariance, conforming topology,
attribute interpolation, allocation limits and exact undo/redo.

Inspect a real OBJ through the importer with:

```sh
cargo run -p obrush-core --example inspect_obj -- path/to/model.obj
```

## CPU timing fixture

```sh
cargo run -p obrush-core --release --example core_timing
```

Historical **0.1.0** run on October 12, 2026, Apple M2, rustc 1.96.0, release
build. These values do not describe the changed 0.1.3 Draw algorithm. A separate
0.1.3 release timing run measured about 29–33 ms per Draw dab, including picking
and local refinement; see [the scoped timing report](../../docs/DRAW_QUALITY.md).
Paired 0.1.4 fixtures reduce median sculpt cost from 33–36 ms to about 9 ms,
preserving output fields exactly. These are native CPU measurements, not browser
frame rates; see [performance evidence](../../docs/BROWSER_PERFORMANCE.md).

| Mesh | Operation | Median | p95 |
| --- | --- | ---: | ---: |
| Sphere 5: 10,242 vertices / 20,480 triangles | Raycast | 0.099 ms | 0.193 ms |
| Sphere 5 | Draw dab | 0.168 ms | 0.363 ms |
| Sphere 5 | Smooth dab | 0.298 ms | 0.956 ms |
| Sphere 5 | Mask dab | 0.078 ms | 0.178 ms |
| Sphere 5 | Paint dab | 0.082 ms | 0.246 ms |
| Sphere 6: 40,962 vertices / 81,920 triangles | Raycast | 0.388 ms | 0.644 ms |
| Sphere 6 | Draw dab | 0.672 ms | 0.865 ms |
| Sphere 6 | Smooth dab | 1.377 ms | 2.653 ms |
| Sphere 6 | Mask dab | 0.308 ms | 0.418 ms |
| Sphere 6 | Paint dab | 0.315 ms | 0.458 ms |

Each operation has 128 single-threaded wall-clock samples. Brush radius is 0.2,
strength 0.5, X symmetry enabled, and successive dabs alternate inversion.
Sphere 5 brushes affect roughly 171 vertices per dab; Sphere 6 roughly 680.
Smoothing adjacency is warmed before timing. Mesh creation, history snapshots,
file IO, rendering, UI, and browser overhead are excluded. These are CPU costs,
not application frame rates. Raycasting and normal updates currently scan the
full topology; timings do not establish million-vertex interactive performance.

`Document::transform_active_subtool` applies a baked `ObjectTransform` to the
complete active state: world translation, fixed world X then Y then Z rotation,
and positive uniform scale about an explicit pivot. All levels, layers and the
Morph Target follow, while mask/RGB/topology and inactive objects stay intact.
The transaction is bounded and undoable; schema versions stay unchanged. See
[transform scope](../../docs/TRANSFORMS.md).

`Mesh::extract_mask` clips a linear mask field and creates a bounded outward
closed shell. `Document::extract_mask_as_subtool` installs it as a new plain,
unmasked active object with whole-scene undo, retaining the source's complete
editable state. The operation shares the existing checked SubTool append and
version 5 format; no display or GPU is required. See
[mask extraction scope](../../docs/MASK_EXTRACTION.md).

`Mesh::filtered_masks` returns mask weights and a `MaskFilterReport` without
mutating the mesh. `Document::filter_mask` finishes pending valid edits and
commits a validated changed result as one history entry; subsequent bitwise
no-ops preserve redo. `MaskFilterSettings` chooses
Grow, Shrink, Blur or Sharpen, 1–16 synchronous passes and finite strength 0–1.
Geometry, paint, layers and target bindings remain intact. Graph operations use
unique indexed neighbors, while Sharpen is pointwise. See the exact scalar and
level-channel contract in [mask filter scope](../../docs/MASK_FILTERS.md).

`Mesh::region_masks` returns a rectangle/lasso mask candidate and report using
a supplied column-major OpenGL view-projection matrix and top-left viewport.
`Document::mask_region` validates and commits changed masks as one history entry
after pending valid edits finish; no-ops preserve redo. Projection and polygon
selection run in the pure Rust core, without a display or depth buffer.
Selected active-part weights become 1, or 0 for erasing. See the exact projection,
boundary and work rules in [region mask scope](../../docs/MASK_REGIONS.md).
