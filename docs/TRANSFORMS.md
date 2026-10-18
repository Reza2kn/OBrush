# Whole-part transforms in OBrush 0.1.11

This foundation adds active-part placement to the shared Rust document. It covers
a subset of T11; reference Gizmo/Transpose behavior remains unverified.

## Using transforms

The Transform inspector offers translation on X/Y/Z, rotations in degrees and
positive uniform scale. Double-click a numeric value to type it, then Apply.
Choose the current part's bounding-box center or the world origin as the pivot.
Apply changes all points in the active part, including fully masked points;
paint and mask weights remain attached. This is whole-part placement. Masks do
not define a partial transformation in this release.

Press T, or enable the viewport movement control, for translation handles.
Drag a world-axis arrow to constrain movement, or the center to move on the
camera plane. The drag uses the camera and plane captured at pointer-down and
total displacement from that point. A shader offset previews the active part;
its stored geometry, history and GPU buffers stay unchanged during movement.
Release applies one shared-core operation and clears the preview. Returning
exactly to the starting point is a no-op. View-aligned axes and handles outside
the viewport cannot start a drag.

Save, selection, import, history and topology commands finish the pending visible
translation before proceeding. Late mouse release cannot apply it a second time.
T/mode changes, Escape, Reset and focus loss cancel a preview. Rotation and scale
use numeric Apply; viewport rotation/scale handles are not implemented. A hidden
active part cannot be edited through the host controls.

## Stored geometry and math

A transform changes the current mesh, all retained levels, every layer's base and
offsets (including hidden/faded layers), and the Morph Target at its creation
level. It keeps point/triangle correspondence, masks, RGB, layer names,
visibility/intensity/recording, level selection, object identity and other parts.
Undo/redo restores the whole scene and metadata. Inactive meshes retain their
shared snapshots and GPU buffers.

For column vectors, the operation is:

`p' = pivot + scale * (Rz * Ry * Rx) * (p - pivot) + translation`.

Rotation applies fixed world X, then Y, then Z. Full turns are normalized. Layer
offsets use only `scale * (Rz * Ry * Rx) * offset`; translation belongs to the
layer base. Arithmetic uses f64 before casting stored point/target coordinates
to f32. The displayed layered mesh is composed from the transformed f64 layer
state, which can differ from transforming its prior rounded f32 display. Rounding
depends on coordinate magnitude and scale; fixture error bounds are not global
bounds for every allowed transform. Normals are rebuilt for deterministic saved-file loading.

Coordinates and pivots are finite and bounded to ±1,000,000 per component;
rotations are bounded to ±36,000 degrees and scale to 0.001–1,000. Invalid input,
coordinate/metadata/output overflow and newly collapsed or reversed faces from
f32 rounding reject the entire operation. A pending rejected stroke also prevents
the dependent transform. With no pending edit, identity and representable no-ops preserve history and
redo. These guards do not repair existing damaged geometry or prevent overlaps
between separate objects.

Transforms are baked into coordinates. There is no persistent object matrix,
orientation or pivot property, and the project schema does not change. Plain,
layered, target-bearing and scene projects remain versions 2, 3, 4 and 5;
versions 1–5 remain readable. OBJ exports the active transformed mesh. History,
pivot choice, numeric drafts and viewport mode are session state.

## Reference and remaining scope

Maxon's [Gizmo guide](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/gizmo-3d/gizmo-3d.html)
and [Transform reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/transform/transform.html)
describe a broader selection, pivot, deformation and multi-SubTool workflow.
No matched ZBrush desktop 2026.2.1 transform fixture was executed here. Masked
transforms, nonuniform/negative scale, local axes, multi-selection, persistent
pivots, Stager, Align/Distribute, Transpose lines and deformers remain separate
gaps. Native GUI and physical device behavior are unverified.

## Verification

The final source passes **282 tests** (247 core, 35 app), formatting, strict
native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed
macOS packaging. The [build receipt](evidence/transforms-build.json) binds the
75-file source manifest and both artifacts. The [core receipt](evidence/transforms-core.json)
includes 28 focused transform contracts, scalar world-axis oracles, invalid-input
and hidden-state guards, exact history and version 2/3/4/5 roundtrips.

A retained failing regression showed that reconciling a coarse-level baseline
could consume redo even when saved geometry did not change. The correction
compares against the effective input before installation. The regression now
preserves exact state and genuine redo; a separate tiny-rotation check confirms
that real hidden f64 layer changes still commit when visible f32 points stay fixed.

The [browser receipt](evidence/transforms-browser.json) records eight workflows
and 43 behavior gates against the frozen final WASM. All 57 saved snapshots match
102 visible vertex/solid-index buffer pairs, with 20 exact layer compositions and
23 target checks. Fully masked translation uses `[0.25, -0.5, 0.75]`; the rich
rotation/scale fixture uses world degrees `[20, -35, 50]`, scale `1.25` and the
active bounding-box center. Measured maximum component errors across the browser
fixtures are:

| Quantity | Maximum error | Gate |
| --- | --- | --- |
| Point / target coordinates | 1.1920928955078125e-7 | 2e-6 |
| Recomputed normals versus rotated normals | 3.838761104268951e-6 | 2e-5 |
| f64 layer bases / offsets | 4.440892098500626e-16 | 2e-13 |

Shader-only previews preserve buffers and revisions. Three release commits and
two no-op releases on the authored pair of 266-point parts reach an observed
final rendered state in 2.3–20.5 ms, below their 100 ms gate, followed by stable
probes with incremental 0/200/800 ms delays. An earlier immediate-sampling gate
failed by pairing a preceding GPU frame with newer CPU state; both failures and
the atomic-frame diagnosis remain hash-bound. The corrected measurement observes
the rendered release boundary before checking quiet behavior. These small,
synthetic headless traces are not physical input-to-photon or large-scene latency
bounds. All 12 pages report actual 4× MSAA and no runtime errors. The inspector
check covers 3,290 visible controls across 420 states; saved PNGs verify readable
fields, pivot controls and translation handles. Native GUI, physical devices and
reference behavior remain unverified.
