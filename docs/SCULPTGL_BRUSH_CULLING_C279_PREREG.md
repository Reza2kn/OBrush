# C279 prospective opposed-sheet normal-UI Brush Culling pair

This is a prospective test of active inventory row `SGL-BRUSH-BRUSH-CULLING`.
[C262](SCULPTGL_BRUSH_CULLING_C262.md) checked the visible controls, pinned
reference front-vertex method and an OBrush core fixture, but did not run a
paired normal-browser stroke on the same double sheet. C279 keeps all earlier
C274/C276/C277/C278 evidence unchanged and does not claim full parity.

The pinned reference is SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independent CC0 SGL v3 fixture was authored by
`scripts/fixtures/author-opposed-culling-c279.py`, not by either product codec.
Its public binary `tests/fixtures/independent-opposed-sheets-c279.sgl` has
882 RGB-ID vertices, 1,600 triangles, 57,512 bytes and SHA-256
`80b6430b152aad76fface01e9bf4890fdd525b2d9f11c60cd74e6c1b021d876b`.
It contains two disconnected coincident-XY 21×21 square grids, front IDs
0–440 at Z `0.02` with +Z wound normals and back IDs 441–881 at Z `0.0`
with -Z wound normals. An independent SGL cursor checked its counts and each
triangle winding before browser admission. No fixture face is copied from
SculptGL or ZBrush.

Run four fresh normal-browser documents: pinned SculptGL and OBrush Source
Draw, each with the visible Thin surface/Culling checkbox **off** and **on**.
Import exactly the same SGL bytes through the visible file input. Use
orthographic front fit, a 1024×808 logical sculpt viewport, DPR 1, UI zoom 1,
radius 64 screen points, intensity 25%, Clay off, Accumulate on, Alpha None,
zero pressure factors, no Negative, X symmetry, Continuous, Prevent folds or
Lock, static topology, one visible object. Match published projection X/Y
scales and world brush radius to `1e-6` before touching the mesh. All pointer
coordinates and delivered DOM events must be trusted integers on the sculpt
viewport, with no intervening UI click.

Press at logical viewport center +`(40,0)` screen pixels, move held to X
offsets `52` and `64` with one acknowledged frame at each, then release at
`64`. Capture active GPU position and display-normal words before, at both
held stages, after release, after public Undo and after public Redo. Public
Save occurs before the stroke and only after release/Undo/Redo, never during
the hold. Capture published control state and one-step history depth. Repeat
with a fresh document for the other culling state; do not toggle mid-stroke.

The frozen acceptance gate is: before states and oriented faces/attributes
map exactly by RGB ID across all four imports; culling **off** must move at
least one front and one back ID, while culling **on** must move at least one
front ID and preserve *every* back ID bit-exact through After and Redo.
Both apps must have identical changed-ID membership per state. At every
matched held/public state the maximum paired XYZ component and active GPU
display-normal component differences must be `<=1e-6`. Active GPU position
words at each public state must match the corresponding public Save exactly;
held GPU words must come from the unique active visible object and be
error-free. Untouched IDs, masks/materials/colors, face order and topology
stay exact. Before=Undo and After=Redo are exact within each application;
each stroke adds exactly one history step and its quiet post-release GPU
state is unchanged. Any mismatch or non-useful off/on response fails the
scoped gate.

Preserve every raw setup and numerical failure. Test the unchanged product
build first and change shared core/host only if a concrete paired defect is
proved. Passing this one double sheet and front camera would still leave
zero-normal behavior (only direct-method/core checked in C262), alternate
meshes/cameras, Clay on, masks, pressure, mirror and dense-mesh performance
unpaired. Keep the full culling row open.
