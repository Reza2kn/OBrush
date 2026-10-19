# Source Brush Negative: prospective normal-UI pair

This protocol is frozen before executing the new browser runs. It tests the
active `SGL-BRUSH-BRUSH-NEGATIVE` inventory row on one ordinary, unmasked Brush
gesture. It does not test Clay, held Alt, the N shortcut, other tools, or
long-running response. The pinned reference is SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independent ID-colored 10,114-vertex, 20,224-triangle SGL sphere is the
unchanged C172 fixture, SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.
Record the OBrush source commit, WASM/JS SHA-256, fixture hash and actual
published options at execution time; do not assume the previous C172 build.

Run four fresh, separately owned Chrome sessions in this order: reference
Negative off, reference Negative on, OBrush Source Negative off, OBrush Source
Negative on. Import the same SGL through the ordinary file control. Select
Brush with Clay off, Accumulate off, no alpha, Culling off, X symmetry off,
Continuous off, Prevent folds off, pressure-to-size and pressure-to-intensity
zero, orthographic/front framing, default material and no mask. Use the visible
per-tool Negative checkbox, not an internal setter. Request radius 64 screen
points and intensity 25%, retain the reference slider's actual published value,
and calibrate OBrush radius and horizontal offsets from both actual projection
matrices and viewport widths. Neutral force is one. The gesture is one press
at viewport center plus 80 CSS points X, one held move to plus 92, one release
there, with one RAF after press and move. Capture trusted DOM events, actual
camera/viewport/rays/radius, active options, diagnostics and release state.

Save complete public Before and After files, then one Undo and Redo file in
each session. The independent audit maps all 10,114 IDs via original RGB,
checks all positions, oriented faces and attributes, exact per-app
Before/Undo and After/Redo bytes, nonzero changed-vertex sets, and opposite
signed normal displacement for the two polarities. For each polarity, require
same changed IDs and at most `1.0e-7` maximum absolute paired XYZ component
error across all IDs. Preserve raw values even if the bound fails. Do not
silently adjust the gesture or tolerance after seeing results.

Capture active viewport GPU position and normal Float32 words independently
from the application Save parser. OBrush uses a unique active WebGL2 VBO
readback. SculptGL WebGL1 requires observing actual upload bytes for its
active draw buffer; a saved mesh normal array is **not** a GPU display-normal
oracle. Compare GPU position words to each side's public geometry, and
display-normal words between the two active renderers only after confirming
their buffer layout, active-draw identity and index/ID mapping. If that
cannot be proven, report the normal comparison as unqualified rather than
comparing OBrush's GPU normals to public Save normals. Retain failed setup,
numeric, GPU and history checks. This small, single-move witness cannot close
the complete Negative row or SculptGL/ZBrush parity.
