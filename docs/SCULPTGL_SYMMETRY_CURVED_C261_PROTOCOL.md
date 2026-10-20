# C261: baseline-corresponded curved signed-offset Draw pair

C260's preregistered direct-ID gate failed because pinned SculptGL's
`Mesh.optimizePreTransform` renumbers vertices during import. Before looking
at a new held stroke, C261 freezes the correspondence rule below. C260's
failed receipts and posthoc diagnostic remain intact; C261 is a new,
independent browser gate, not a relabelled C260 result.

Run two fresh 1600×1000/DPR1 owned Chrome cases on the pinned SculptGL
commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and independently authored irregular curved fixture SHA-256
`6d5557d2650d5176c5565415e5d94937f41345f48bdb0f3cdc841b6480dcc9ad`
(4,514 vertices, 4,608 faces). Use normal visible import, Solo, normalized
active-mesh SymOffset `+0.25` and `-0.25` in separate sessions, OBrush Source
mask conversion, Draw/Brush nominal radius 120, intensity 0.5, symmetry on,
mouse force 0.5, no alpha and no continuous mode. For sign `s`, project
`(s×0.5,-0.13,0.86)`, `(s×0.5,0,0.8660254)`, and
`(s×0.5,+0.13,0.86)` through each actual camera. All three direct and
mirrored rays must hit the active imported mesh. Keep the visible camera path
span within 1% and both paths longer than 18 CSS pixels. Require actual
trusted held mouse pressure 0.5, pressure factor 1, paired 120 CSS-pixel
radius, primary/mirror local radii within 1% or `1e-5`, matching original
makeStroke/completed dab counts, at least one admitted primary/mirror pass,
and zero WebGL errors.

Before either stroke, decode the complete ordered authored XYZ array directly
from the frozen independent SGL v3 fixture. Capture the complete SculptGL
active mesh baseline, also before its stroke. Each must have exactly 4,514
finite positions. Build a **one-to-one immutable map** from each SculptGL
baseline vertex to the unique authored SGL vertex with maximum per-axis
coordinate difference `<1e-6`. Use four-decimal coordinate bins for lookup,
with a full unused-point fallback only where a bin has no unique candidate.
Reject any missing or multiple candidate, duplicate target, or less than
4,514 matches. Require maximum baseline error `<1e-6`. Freeze the map before
any pointerdown; never use after-stroke coordinates to establish or repair a
match. After the OBrush import and before its stroke, require the normal
OBRush Save baseline to have all authored positions in the authored order
within `<1e-6`. Self-check that identical, permuted baseline triples map
uniquely and that a duplicate coordinate or missing coordinate is rejected.

Collect both full after arrays and compare per-vertex XYZ deltas through
this frozen map. A scoped **geometry candidate** requires both sides to
change ≥10 vertices at absolute component threshold `1e-6`, changed-ID IoU
≥0.90, maximum component error ≤0.01 local units, and RMS component error
≤0.002 local units. Report each axis and ten worst mapped IDs. Capture
parked-cursor before/after 1600×1000 screenshots, count RGB changes >4 in
a radius-90 CSS crop at the first point, and require ≥100 changed pixels per
app; report pixel count, bounding box and ratio descriptively without a
visual-equivalence threshold. Use normal Ctrl+Z/Y and verify all ordered
positions exactly in each app; OBrush Save hashes must match before/undo and
after/redo; undo depth rises by one and peer IDs remain fixed.

Preserve all receipts, exact fixture/reference/product/harness hashes,
controls, camera, picker and dab observations, and failed setup attempts.
A failed sign fails this two-sign gate. Success remains a scoped witness:
full Draw, symmetry, pressure, image, topology and ZBrush rows stay open.
