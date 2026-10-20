# C260: curved-mesh Draw with signed symmetry offsets

C259 matched one held Draw dab on a three-vertex triangle at normalized
SymOffset `+0.5`. C260 broadens the independent browser comparison to an
irregular closed curved mesh and **both signs** of the offset. The fixture is
the independently Python-authored SGL v3 analytic sphere
`scripts/fixtures/independent-dense-shift-smooth.sgl`: SHA-256
`6d5557d2650d5176c5565415e5d94937f41345f48bdb0f3cdc841b6480dcc9ad`,
`236392` bytes, `4514` vertices, `4608` faces, 48 latitude rings and 96
sectors. Its authored front mound makes the geometry asymmetric; it contains
no commercial model or copied source mesh. Preserve the full vertex order
and all imported peer objects. The pinned SculptGL bundle remains SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
at commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`.

Run two independent fresh Chrome 1600×1000/DPR1 cases, normalized offset
`+0.25` and `-0.25`, using normal visible import, active-object Solo, numeric
SymOffset, Draw radius and OBrush Source/mask-conversion controls. Keep X
symmetry on, ordinary Draw (clay and accumulate), alpha None, nominal radius
`120`, intensity `0.5`, no continuous mode. For sign `s ∈ {+1,-1}`, freeze
three local front-surface points `(s×0.5,-0.13,0.86)`,
`(s×0.5,0,0.8660254)`, `(s×0.5,+0.13,0.86)`, projected through each
app's normal current camera. Each point must ray-hit the imported active
surface and its mirrored ray must ray-hit the same part; otherwise fail the
setup and preserve the receipt. Require a projected path longer than
`0.15×120=18` CSS px and inside both canvases; match OBrush's visible
Source projection zoom to SculptGL's path span within 1% relative.

Use the separately validated C259 `Input.dispatchMouseEvent` host route:
`pointerType: "mouse"`, `force: 0.5` for press and each held move, force `0`
on release. Require actual trusted held PointerEvents at raw pressure `0.5`
on both pages, and admitted Source pressure factor `1` with primary/mirror
local radii agreeing within 1% or `1e-5` local units, whichever is larger.
If an app admits a different number of dabs, report both counts and mark
the geometry pair **unmatched delivery**, not a brush mismatch. Record full
Source original-call camera/project/unproject/pose and completed OBrush dab
observations; do not infer admission from requested events.

Take exact ordered XYZ arrays before and after on each side: SculptGL's
active mesh in the pinned instance, OBrush's normal visible Save downloads.
Require finite coordinates and equal `4514×3` lengths. Report total changed
vertices using an absolute component threshold `1e-6`, changed-vertex ID
intersection/union, per-axis maximum and RMS paired delta errors, and the
worst ten IDs. A **geometry candidate** requires max absolute component
error ≤`0.01` local units, RMS component error ≤`0.002`, and changed-ID
intersection/union ≥`0.90`. These thresholds screen this one fixture; a
failed candidate is retained, not tuned away. Require each app to change at
least 10 vertices and produce at least one admitted primary and mirror dab.

For each sign, capture parked-cursor before/after 1600×1000 screenshots.
Count pixels with any RGB channel change `>4` in a radius-90 CSS crop about
the first stroke point. Require ≥`100` changed pixels on each side to catch
a no-op, and report count, bounding box and OBrush/reference ratio without
a visual-parity pass threshold. Require zero WebGL errors. After each stroke,
use normal Ctrl+Z then Ctrl+Y and compare **all** ordered positions exactly
to the before and after snapshots on each side; OBrush's undo depth must rise
by exactly one for the stroke and peer object IDs must remain unchanged.
Record normal Save file hashes for before, after, undo and redo.

Each case retains a separate result, source/build/harness/fixture hashes,
actual events, camera, symmetry plane, input/radius/dab trace and all failed
setup attempts. A combined report must show both signs and preserve any
failure; one success cannot stand in for the other. C260 does not close the
full symmetry or Draw rows, pen hardware support, or visual equivalence.
Browser execution waits for explicit ownership of the shared Chrome lane.
