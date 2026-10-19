# C278 prospective noninterrupting normal-UI Brush Clay pair

C277's four normal-browser captures and exact audit are retained as a failed
gate. The C277 protocol called public Save after the first held move; that UI
action clicked the SculptGL file menu during the gesture and stopped the brush.
Both applications then kept their first geometry through the sixth held move.
The failure identifies a measurement interference, not a Clay geometry gap.
It is not rewritten or called a pass.

C278 keeps the same pinned SculptGL commit/JS, authored 10,114-vertex RGB-ID
sphere, four fresh Clay-on/off Source and OBrush documents, visible control
settings, 1024×808 logical viewport, integer press at center+`(80,0)` and six
held moves to X offsets `92,80,92,80,92,80`, radius 64, intensity 25%,
Accumulate on, and all other C277 settings. It keeps the same `1e-6` paired
position/display-normal/effect tolerances and exact changed-ID/attribute/face,
history and active-object gates. This is a new prospective protocol, not a
post-hoc relaxation of C277.

The only capture change is to **not call public Save while the mouse is down**.
Capture active GPU position and display-normal Float32 words at every held
move, especially first and sixth; require the first and sixth held position
streams to differ in both applications, and all six paired streams to remain
within `1e-6`. The DOM trace must contain only the expected trusted down,
six held moves and up on the sculpt viewport, with no intervening UI pointer
down/up. After release, use normal public Save, Undo and Redo; active GPU
position words at Before/After/Undo/Redo must match their corresponding public
Save Float32 words exactly and active GPU display normals must pair within
`1e-6`. Require one sculpt history step and an unchanged quiet frame after
release. The on/off final effect must be nonzero in each app, involve the same
RGB IDs and pair within `1e-6` per component.

Preserve setup/numerical failures. Run the unchanged product baseline before
considering a shared-core/host edit. Passing this one static gesture is a
scoped witness and cannot close the whole Clay row or SculptGL/ZBrush parity.
