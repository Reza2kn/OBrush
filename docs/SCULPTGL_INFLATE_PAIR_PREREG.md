# Preregistered Source Inflate comparison (SGL-BRUSH-INFLATE)

This is a **prospective** test protocol. No reference or OBrush output has been
examined for this fixture when the limits below were chosen. The target is the
pinned SculptGL hosted bundle at SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
and source revision `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, specifically its
ordinary `Inflate.inflate` method and normal browser controls. The OBrush
build's Git and WASM hashes must be recorded at execution time; no unchecked
current build is assumed equivalent.

The independently authored planar diamond is
`crates/obrush-core/tests/fixtures/independent-inflate-diamond.obj`, SHA-256
`9f1d457957a63a832cb229e66692e573da3e398d3ec256ce57272a3028a0cd35`.
It has four indexed positions and two oriented triangles. Its face and vertex
normals point along +Z. Direct-kernel tests must use original vertex IDs, fixed
center `[0,0,0]`, radius squared `1`, selected IDs `[0,1,2,3]`, alpha `None`,
and material mask factors `[1,0.5,0,1]` in this order. Positive intensity is
`0.3`, then negative true from a fresh fixture at the same intensity, then a
separate zero-intensity control. Disable OBrush's optional fold guard for the
reference comparison. Screen pressure is neutral (`1`). No dynamic topology,
symmetry, culling or camera motion is in this direct-kernel cohort.

The analytic positive Z displacements are `[0.020625,0.0103125,0,0.020625]`;
negative reverses the sign. This is an *independent arithmetic check*, not
the reference output. Before the run, accept at most `3.0e-8` absolute
position-component difference between paired Float32 arrays (including cast
order), exact unchanged X/Y words and exact unchanged fully masked vertex,
and exact face order/indices. Report both maximum and RMS error across all 12
position words; retain failed values instead of changing the tolerance.

The normal-UI cohort must import the same independently authored, ID-colored
10,114-vertex SGL sphere in both browsers with auto-centering disabled. Its
fixture SHA-256 is
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.
It must select ordinary Inflate, radius 50 viewport points,
intensity 30%, Negative off, no alpha, culling off, symmetry off and the same
orthographic framing. Its first actual gesture uses one 16-point mouse move
from the viewport center after a press; a separate fresh run uses Negative on.
Audit all input and output vertex IDs, faces and materials from public exports.
The SGL sphere is unmasked; this UI cohort does not substitute for the
four-vertex mask-kernel cohort. Require at least
one changed vertex in both applications, source and OBrush public Save/Undo/Redo
to recover their own exact before/after geometry, and a final visible GPU mesh
that agrees with the OBrush saved geometry. Compare changed-vertex sets and
Float32 components, but do not impose the direct-kernel `3e-8` tolerance on
unmatched screen-space contacts. Record canvas dimensions, projection,
contact/radius calibration and delivered dab counts; if these differ, report a
diagnostic instead of declaring matched UI response.

The direct method does not exercise picking, stroke spacing, Undo, browser
rendering or host queues. A successful small case cannot close the full
SGL-BRUSH-INFLATE acceptance gate, native stylus behavior, million-vertex
responsiveness, SculptGL parity or ZBrush parity. Preserve every failed or
incomplete run and report exact source/build hashes and numerical differences.
