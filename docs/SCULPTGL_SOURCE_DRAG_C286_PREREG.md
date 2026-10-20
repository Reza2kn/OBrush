# C286 prospective Source Drag workflow

## Concrete gap and boundary

Pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
exposes Drag as tool index 7. Its `Drag.js` starts with a 150-pixel radius,
samples a moving brush center at `0.15 × radius` spacing, projects the
prior picked center onto each sample's view ray, and adds the resulting
displacement to **live** vertices under the quartic falloff, multiplied by
raw unprotected material and optional raster alpha. This is a distinct
workflow from anchored Move and OBrush's older Drag queue. Current OBrush
source commit `f8768b4e93fb8dd21d9fcdbfc451c56454ff6b14` has no
`SourceGeometryTool::Drag`, no Source Drag UI mode, and no shared-core
moving-center kernel. The old Drag path remains the compatibility default.

The first implementation is opt-in **Source Drag** on an existing single
visible static mesh, no retained hierarchy/layer, symmetry off, Alpha None,
no dynamic topology or Continuous. It may reject broader settings visibly;
it must not silently enter Legacy. Geometry and history remain in
`obrush-core`, shared by desktop and WASM. Bounded failure cancels the
transaction to exact Before. Dynamic topology, alpha, symmetry, other
cameras and long-session response stay open.

## Independent fixture and fixed input

`scripts/fixtures/author-source-drag-c286.py` independently authors a CC0
SGL v3 fixture without either product's encoder. It is a 33×33 curved,
asymmetric grid (1,089 RGB-ID points and 2,048 triangles); ID544 is fully
protected and ID545 half unprotected. Fixture SHA-256:
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`.
The pinned hosted Source bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The unchanged OBrush web baseline at this freeze has WASM SHA-256
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`
and JS SHA-256
`03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.
Candidate build/source hashes must be recorded separately at run time.

Four fresh normal-UI documents: pinned Source Drag, candidate Source Drag,
and one each for the untouched OBrush Legacy Drag compatibility check and
a zero-travel Source Drag check. Both paired documents use 1024×808 logical
viewport, front orthographic fit, mouse, radius 96 pixels, pressure factors
zero, Alpha None, symmetry off, no culling, no Continuous, no Lock. Start at
integer viewport-center offset `(-64,-20)` and move while held to
`(-28,-20)`, `(8,-20)`, `(44,-20)` in three separately observed deliveries,
then release at `(44,-20)`. Each 36-pixel segment yields two Source Drag
substeps at the frozen 96-pixel radius (`floor(36 / 14.4) = 2`), so the
second substep projects a distinct ray instead of a zero-travel first
substep. No public Save or UI interaction during hold.
The published Source and OBrush world brush radius and each requested ray
may differ by at most `1e-6`; X/Y projection ratios may differ by at most
`5e-6`. No adjusted pointer path or mesh is allowed after seeing output.

## Frozen acceptance and rejection

Capture public Save at Before, After, Undo and Redo; capture active GPU
position and **display normal** words at Before, all three held moves,
After, quiet After, Undo and Redo. Verify the active object and exactly one
VBO candidate; active GPU positions must match each own public Save bitwise.
Source and OBrush must have the same changed RGB-ID set and mapped position
components within `1e-6` at each held move and After/Undo/Redo; active GPU
display normals must be within `1e-6` at the same stages. Public Undo must
recover exact own Before bytes, Redo exact own After bytes, and one released
gesture adds exactly one undo step. RGB, raw mask, material and topology must
remain identical to Before. ID544 must stay bitwise fixed; a vertex near
the first segment and a different vertex near the last segment must each
change, proving successive moving neighborhoods. The three held captures
must differ from one another and the final quiet GPU must equal After.

The zero-travel Source control must leave the public file and history
unchanged. A bounded invalid/stale input after an admitted preview must
restore exact Before and leave the gesture noncommittable on release. The
Legacy Drag route must remain available and unchanged by default; a saved
older project cannot silently select Source Drag. Keep any failed setup,
geometry, GPU, history or stress receipt. A scoped pass does not close
`SGL-BRUSH-DRAG` or its alpha/radius rows.
