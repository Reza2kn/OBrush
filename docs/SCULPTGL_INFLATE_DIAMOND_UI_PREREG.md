# Prospective C270 ordinary-UI Inflate diamond pair

This protocol is committed before either ordinary-browser run. It adds a
normal-UI cohort to [C269](SCULPTGL_INFLATE_PAIR_C269.md); it does not replace
the previously frozen 10,114-vertex sphere UI cohort in
[the original preregistration](SCULPTGL_INFLATE_PAIR_PREREG.md).

Use the independently authored four-vertex, two-oriented-face OBJ at
`crates/obrush-core/tests/fixtures/independent-inflate-diamond.obj`, SHA-256
`9f1d457957a63a832cb229e66692e573da3e398d3ec256ce57272a3028a0cd35`,
through each application's ordinary file input. The pinned hosted SculptGL
bundle must hash to
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
record the exact OBrush Git revision and web WASM SHA-256 at execution. Run
each side in a separately owned Chrome process/profile. Do not assign tool,
mesh, camera or history state in the page; read-only diagnostics are allowed.

Disable source import auto-centering, clear each scene, import the OBJ, choose
ordinary Inflate through the normal UI, and frame the diamond in an
orthographic 1024×768 drawing viewport. Use radius **400 CSS viewport
points**, intensity **30%**, negative off for the first fresh-scene pair and
on for the second, no alpha, culling off, symmetry off, continuous off and
OBRush fold prevention off. Disable both pressure-to-radius and
pressure-to-intensity factors; CDP mouse events use neutral force 1. The OBJ
has no mask channel: every vertex is unmasked (effective factor 1). C269's
different `[1, .5, 0, 1]` mask vector belongs only to the direct-method
cohort. Record the actual published controls and reject any mismatched setup.

After ordinary frame/reset, measure each actual canvas rectangle and
projection. The first trusted mouse press is 8 CSS pixels right of the
drawing-viewport center; request one horizontal move another 16 CSS pixels
right, acknowledge a browser frame, then release. On OBrush, use measured
horizontal projection and viewport width to scale both radius and horizontal
offsets to the reference's initial world ray and brush radius. Record this
factor, all requested and delivered DOM events, and actual applied-dab
diagnostics where available; equal requests do not imply equal delivered
dabs or later picked contacts. If the diamond is not picked or neither app
changes a vertex, retain that failed admission. Do not change radius/path
after viewing outputs under this protocol.

Acquire actual public Before, After, Undo and Redo saves from each app. Check
byte-exact within-app Before→Undo and After→Redo recovery. Map the four vertex
IDs by their distinct original XY coordinates and audit all Float32 XYZ
words, oriented faces and available non-position channels. Require at least
one changed vertex per app; record changed-ID sets, maximum and RMS
component differences and each original/final Float32 word. On matched
initial rays/radii *and* matching changed-ID sets, use the prechosen
`3.0e-8` maximum absolute component difference as a narrow numerical gate;
otherwise report a diagnostic without declaring a matched UI response.

Capture a read-only active GPU vertex-buffer position/normal word digest at
After and Redo in OBrush and compare it with the independently packed public
save words. A missing or ambiguous active buffer is a failed GPU check, not
a pass inferred from the UI screenshot. Preserve raw failed receipts and
browser console/GL errors. This one flat four-vertex gesture does not cover
the earlier unmasked 10K sphere UI protocol, repeated normals, alpha,
nontrivial masks, pen pressure, native GUI, dense quality, sustained latency
or the full `SGL-BRUSH-INFLATE` gate.
