# Mask extraction in OBrush

Mask extraction creates a separate active SubTool from the current part's dark
masked area. It keeps the original part, its mask, paint, subdivision levels,
sculpt layers and Morph Target. Creation is one whole-scene undo step.

## Using extraction

Paint with the Mask brush or hold Ctrl while brushing. In the inspector's
**EXTRACT MASK** section, set Threshold and Thickness (world), then press
**Create part**. The default threshold is 0.5; lowering it includes lighter mask
weights. Default thickness is 0.02 world units. The new part is unmasked and can
be sculpted, moved, hidden or isolated with Solo independently. Its source keeps
the mask until you clear it.

The **Whole incident faces** checkbox includes every triangle touching a vertex
whose mask reaches the threshold. It is off by default. With it off, extraction
clips triangles at the interpolated mask boundary. This selection option is
session-only and does not modify the saved source or its mask.
**Keep source selected** appends the extracted part while leaving the source
active, so its mask is still ready for further editing. It is off by default;
with it off, the new part becomes active. Both choices create one whole-scene
Undo step and can be changed between extractions.
With **Whole incident faces** enabled, **SculptGL surface offset** moves the
selected surface 0.01 world units along face-averaged source normals (−0.01 for
negative thickness), then places the second skin at the requested signed
thickness from that shifted surface. It also starts the new part with white
vertex color and default roughness/metalness (0.18/0.08), like the pinned
SculptGL extraction. The option is off by default and session-only. For
nonzero thickness on a closed triangle source, OBrush smooths the side-wall ring in
three passes using logical quad neighbors. A candidate with collapsed faces
is discarded, leaving the checked closed shell. Open sources also keep that
shell because the pinned SculptGL open-sheet output collapses triangles. An
imported source with original quads or UVs may now be extracted without
detaching its metadata; the source retains that metadata, while the new part
has fresh triangle topology and no UV channel. Quad sources keep the checked
shell without smoothing because the paired reference quad fixture leaves an
open ten-edge output. Arbitrary curved reference behavior remains unverified.
For a posed version 7 source, extraction keeps the source's editable local
mesh, affine pose, and SGL provenance unchanged. It bakes the selected patch
into world coordinates for the new plain part, applies the surface offset and
signed thickness in world units, and reverses copied face winding when the
source pose reflects space. A pose that rounds a selected face to zero area
is rejected before the document changes. Matched transformed-reference shape
and native GUI behavior remain unverified.

Threshold is finite in 0.01–0.99 and signed thickness in −5–5 world units.
Double-click a field to enter an exact value and press Enter. Thickness follows
the direction defined by the source's triangle winding. This is a direct
creation command; there is no extraction preview/Accept stage in this release.
The displayed last-extraction report is session history, including after undo
or part selection; successful file loading clears it.

## Geometry and document contract

In the default mode, the shared Rust core clips each triangle where its linearly
interpolated mask weight is at least the threshold. Edge intersections use f64 point and stored
numeric RGB interpolation before rounding to f32. Shared cuts reuse canonical
undirected source-edge IDs; exact threshold endpoints reuse original vertex IDs.
Distinct source IDs are not spatially welded even if their coordinates coincide.
Whole-incident-face mode keeps original source vertices and complete selected
faces: a selected original quad contributes both of its canonical triangles.
Both modes use the same validated patch, shell and history path. On a posed
source, the validated affine transforms each selected local point; the
inverse-transpose normal transform is applied to the optional face-averaged
source direction before offset or thickness.

The selected rounded patch supplies corner-angle-weighted geometric normals,
accumulated in f64. The optional SculptGL surface offset instead uses the
arithmetic mean of raw incident source-face normals, rounded to f32 before
normalization. Stored source normals do not determine extrusion. Positive
thickness reverses the original patch's winding and adds an outward skin;
negative thickness keeps the original patch facing outward and reverses the
inward skin. Boundary quads close either shell. Zero thickness creates only
the selected patch. Manifold holes and separate regions are supported. A fully
selected closed surface with nonzero thickness creates two nested,
disconnected skins representing a hollow layer. This uses the current level's
visible geometry; the new part has no retained levels, sculpt layers or target.

The complete source, selected patch and output must have consistent edge
orientation and connected vertex fans. Branching boundaries, collapsed faces,
cancelled normals, flipped offset faces, rounded-away thickness and invalid
coordinates are rejected. Signed displacement is checked on incident
faces. These are local and topological guards: they do not establish absence of
global self-intersections, overlaps with the source or collisions between parts.
The operation does not repair damaged input.

Source limits are 250,000 vertices and 500,000 triangles, checked before topology
scratch allocation. The shell also obeys the one-million vertex/two-million
triangle limits and the existing aggregate scene, retained metadata, output and
history budgets. A scene supports at most 32 parts. Large extraction and
serialization run synchronously; these caps are not latency or peak-memory
guarantees. Use a lower source level or reduce its surface if necessary.

The host disables extraction for a hidden active part, a running topology job,
a live transform drag or a reached input/object cap. Finish or cancel a transform
before extracting. An ordinary sculpt stroke is finished before creation; a
rejected pending edit aborts the dependent extraction. After pending valid edits
are finished, failed creation preserves the document and genuine redo. The
existing checked SubTool append transaction
validates the whole candidate before installing it.

No schema was added. Extraction promotes a plain single-object project to the
existing version 5 scene format; versions 1–7 remain readable. A posed
version 7 source retains its scene and provenance fields, while the new part
has no inherited pose. OBJ exports the active shell. Settings and the
last-extraction report are not saved project properties.

## Reference scope and remaining work

Maxon's [desktop Mesh Extract guide](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/creating-meshes/mesh-extract/mesh-extract.html)
describes mask/visibility selection, cleaned boundaries and preview acceptance
into a SubTool. Its [SubTool reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/tool/polymesh/subtool/subtool.html)
also covers smoothing, thickness direction and border/corner controls. No matched
ZBrush desktop 2026.2.1 extraction fixture was executed here. Preview/Accept,
surface or edge smoothing, visibility selection, symmetric extraction,
generated Polygroups, UV/material extraction and MeshFromMask brushes
remain gaps. T12's other base-mesh tools remain separately planned.
SculptGL's pinned source uses whole incident faces, the optional signed offset
and fresh channels described above. Its bridge policy and three nonzero-border
smoothing passes still differ. [The C74 reference comparison](SCULPTGL_EXTRACTION_REFERENCE_C74.md)
records ordinary browser behavior on open-sheet and closed-cube fixtures; it
does not establish full extraction parity.

## Verification

The historical 0.1.12 source passed **323 tests** (285 core, 38 app), formatting, strict
native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed
macOS packaging. The [build receipt](evidence/mask-extract-build.json) binds a
79-file source manifest and both artifacts. The [core receipt](evidence/mask-extract-core.json)
records six owner and 32 independent extraction contracts, including invalid
input, precision, topology, history, source metadata and capacity rejection.

Independent unit-scale fixtures use position/RGB/signed-volume gates of
2e-6 / 1e-6 / 2e-6. Their largest errors are respectively
1.1874361327546268e-8 / 2.6153060384004334e-8 / 3.235957782288246e-8.
These fixture errors are not global precision bounds. Earlier owner compile,
index-order/history oracle and helper lint failures are retained with diagnoses.

The [browser receipt](evidence/mask-extract-browser.json) records eight workflows
and 53 behavior gates on the frozen final WASM. All 52 saved snapshots match
145 visible vertex/solid-index buffer pairs, with 148 object contracts, 21 exact
f64 layer compositions and 21 target checks. The linear unit sheet uses threshold
0.5 and world thickness 0.125: five patch points/three faces become a ten-point,
16-face shell with signed volume 0.0625 and zero position/RGB/volume error.
The annular fixture has two boundary loops, Euler characteristic zero and volume
error 2.78e-17; disconnected patches become two closed components.

The ordinary painted-mask test uses two separately released identical default
strokes (radius 0.18, strength 0.35, X symmetry; 24 moves along a 60×15-pixel
trace). Each produces seven arc-length dabs. Maximum mask weight grows from
0.393386 to 0.78677195 before extraction at the unchanged 0.5 threshold.
The first, insufficient single-stroke preparation remains a failed fixture
record; no extraction or numerical gate was relaxed. Source geometry stays
exact and the result contains 452 points/896 faces in two closed components.

Rich source levels, hidden/faded layers and an unavailable origin-level target
survive extraction and version 5 reload. Existing inactive GPU buffers stay
unchanged. New-part transforms and Draw edits preserve the sources; rejected
empty/ambiguous operations preserve genuine redo. Hidden/32-part controls and
rejected imports preserve the document. Four short stroke traces remain quiet
after an explicit 100 ms settle and incremental 0/200/800 ms probes; this makes
no first-commit or physical input-latency claim. All 14 pages report actual 4×
MSAA and no runtime errors. Inspector checks cover 2,434 visible controls across
440 states, and the saved PNGs were visually reviewed. Native GUI, physical
devices, large/long-session performance and reference parity remain unverified.

## Signed-thickness update

[C70](SIGNED_MASK_EXTRACTION_C70.md) adds zero/inward output and the full −5..5
visible control. The historical verification figures above describe the
positive-only 0.1.12 implementation; C70 has separate source/build/browser
evidence. The clipped-boundary and non-smoothed border still differ from the
pinned SculptGL extraction policy.
