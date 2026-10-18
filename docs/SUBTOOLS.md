# SubTools foundation

Version 0.1.10 adds separate sculptable objects to the shared Rust document.
The desktop package and browser use the same geometry, object state, history and
project codec. This is a foundation toward T06, not ZBrush SubTool parity.

## Using parts

The SubTools inspector offers Add sphere, Add cube and Append OBJ. Added primitives
are placed to the right of the visible scene and selected; Append retains the
OBJ's coordinates. Open replaces the whole scene. Duplicate creates a separate
copy at the same coordinates, including its masks, color, subdivision levels,
layers and Morph Target. Rename, reorder, hide, solo and delete are undoable.
At least one part remains. Select a row, or click a visible part to select and
start sculpting. Exact overlapping duplicates prefer the active part when picked
and drawn. Other visible parts occlude strokes; hidden parts cannot be picked.

Each part retains independent geometry, masks, paint, subdivision levels, sculpt
layers and a Morph Target. Switching parts commits a pending stroke first and
cancels outstanding topology jobs. Selection does not add an undo entry; an undo
of an edit restores its scene snapshot, including the selected part. History
covers the entire scene, so undo can restore a deleted part and its editable state.
Selection, list order, names, visibility and Solo are saved in the project. Camera,
brush settings and history are session state.

Solo temporarily filters drawing and picking without rewriting visibility flags.
The active part can be hidden; show it to resume sculpting. This differs from
ZBrush's selected-object visibility rules. Symmetry still uses the scene X axis;
local object pivots, transforms and local symmetry are not implemented. Duplicates
remain coincident until their geometry is edited. There are no folders, merge,
split, transform gizmos, booleans, visibility sets or folder operations yet.

Save project retains every part. Export active OBJ writes only the active mesh;
OBJ grouping and whole-scene OBJ interchange are not implemented. Append accepts
OBJ, not a second project. Native ZPR/ZTL compatibility remains a separate gap.

## Persistence and bounds

A scene-bearing project uses version 5 with `scene.scheme = "subtools_v1"`,
`active_id`, `next_id`, `solo`, and ordered `objects`. Objects store stable uint32
IDs, UTF-8 names and visibility flags. The active object's `payload` is null:
its mesh and optional levels/layers/target remain in the main project fields.
Inactive objects store those fields in their own payload. Active subdivision
meshes retain the existing null-slot convention. Versions 1–4 remain readable;
projects that have not used scene features keep their prior single-object format.

There are at most 32 objects and 256 UTF-8 bytes per nonempty object name. IDs are
unique and allocation rejects overflow. Geometry across all objects and retained
levels is bounded to 1 million points and 2 million triangles. Layer base-plus-delta
vectors, ordered layer faces and Morph Target arrays have aggregate scene limits;
layer and target storage each retain a 64 MiB capacity cap. Retained scene storage,
project input/output and combined history each have separate 128 MiB limits;
history retains at most 64 snapshots. A shared decode scope charges array elements
and capacity before reservation across active and inactive payloads. A rejected
import preserves the live scene and history. These caps are not a peak-memory or
interactive-performance guarantee.

The viewport shares one depth/MSAA pass among visible parts. GPU buffers are
cached by stable ID and immutable mesh identity. Active edits reuse unchanged
inactive buffers; hidden/deleted GPU resources are freed on the next paint.
The renderer preflights at most 32 objects and 256 MiB of geometry storage,
including possible wire edges, separately from its existing MSAA allocation cap.
Full-mesh ray scans, geometry uploads for the edited part and serialization remain
performance work. GL allocation/upload errors are displayed and leave document
state intact.

## Reference scope and verification

Maxon's [SubTools guide](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/subtools/subtools.html)
and [SubTool reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/tool/polymesh/subtool/subtool.html)
provide the public workflow vocabulary. No matched ZBrush 2026.2.1 SubTool fixture
has been executed for this milestone. Native GUI, physical tablets, broad host
coverage, large scenes and long sessions remain unverified.

The [core receipt](evidence/subtools-core.json) binds 22 independent contracts,
six owner integration tests, three private decode-scope tests and a bounded
read-only host/renderer review. The final [build receipt](evidence/subtools-build.json)
binds **247 passing tests** (219 core, 28 app), formatting, strict native/WASM
Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging.
Native GUI interaction is unverified.

The allocation fixtures accept two 500,000-point objects (1,000,000 total) and
reject two 550,000-point objects (1,100,000 total), in both top-level field orders.
A raw topology release with 1,000,000 active points plus 42 inactive points
restores the exact scene, target, attributes and redo. Twenty malformed scene
variants and ten malformed inactive-payload variants reject in two field orders.
Duplicate/signed-zero checks require exact float32 bits and history checks require
canonical state; the ray-distance tolerance is 1e-6 world units, with no measured
maximum emitted. These are correctness fixtures, not capacity benchmarks.

The [browser receipt](evidence/subtools-browser.json) records eight workflows and
42 explicit behavior gates in isolated headless Chrome 155.0.8059.39 on Apple M2,
at 1600×1000 CSS pixels and scale 1. All 71 saved scene snapshots matched the
complete visible packed vertex and solid triangle-index buffers (175 solid draws),
with actual active shader flags and active-last ordering. All seven pages
allocated 4× MSAA and reported no GL/JS/CDP errors. A separate clipping gate
checks 1,390 visible scene controls across 100 states against the actual inspector
clip, within 0.01 CSS pixel tolerance. Final screenshots show readable movement
labels and intact object counters.

On an independently authored coarse two-part scene, the Draw trace at radius
0.18/strength 0.35 changes one original point by up to 0.0078300414 world units
and adds 981 points (266 to 1,247). The inactive 266-point/528-face part, its packed
vertex/index bytes, buffer identities and upload count (2 before and 2 after) stay
exact. This tests editing/cache isolation, not general sculpting quality. Actual
Mask/Paint, layers, target storage and level addition remain independent across
selection. Fresh version 5 and legacy version 2/3/4 public file inputs, malformed
inactive rejection preserving redo, rich duplication/deletion and nearest visible
masked picking are exercised with complete scene histories.

Six short gestures stay quiet with zero pending dabs at release probes after
incremental 0/200/800 ms waits. First probes start 3.0–4.7 ms after trusted DOM
release, before asynchronous buffer readback completes; rAF callback-gap p95 is
16.7–16.8 ms, maximum 16.8 ms, with no observed tasks above 50 ms in these
intervals. These are synthetic browser scheduling checks, not rendered frame time,
input-to-photon or long-session performance measurements.

The first browser run passed its 42 behavior gates, but manual PNG inspection
found a wide action row clipping the inspector and unsupported arrow glyphs.
That artifact remains intermediate. The final layout separates lifecycle and
movement rows, uses text labels and adds the clipping gate. All required source
checks/builds and eight browser workflows were rerun on the corrected bytes.
The core borrow error, intermediate renderer compile/oracle failures, receipt
path-helper failure and interrupted owned validation remain preserved in receipts.

Final source manifest: 70 source/helper files, SHA-256
`c04409a5eb05032f896d1d9547b17c6f8b68fd3bfbf40442f9b3e2111d059a6d`.
WASM: `0dbc84227b922654c8f608ca0dc4941581922ae4f69982c95a7b6aa0acf1f136`;
ad-hoc-signed native executable:
`b9d029ea99aaaa95626076d26ad0c420d558b7ab56f357a7ef6b89c4602dc8bd`.
Frozen build records retain their original pending-runtime statement; later public
receipts bind the completed execution without rewriting the frozen records.

Version 0.1.11 adds [whole-part transforms](TRANSFORMS.md). Coordinates are baked
across the active part and its levels/layers/target; the historical 0.1.10 gaps
above remain unchanged for that release. Persistent transforms, local pivots,
folders, merge/split and booleans are still unfinished.
