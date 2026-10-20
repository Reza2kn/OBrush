# CR01: repeated default Crease on a fresh curved sphere

This is a user-visible quality and mesh-safety comparison prompted by the
reported jagged, tear-like slivers after overlapping Crease strokes. It is
**not** a complete Crease feature-parity gate. OBrush's ordinary default
Crease uses world radius `0.180`, strength `0.35`, X symmetry on and its
legacy geometry response. Pinned SculptGL's Crease starts with screen radius
25 px, intensity `0.75`, negative on and a different response. CR01 keeps
OBRush's user defaults untouched, then sets the reference intensity to
`0.35` and its visible integer screen radius to the nearest pixel projection
of the OBrush `0.180` local radius after matching camera scale. Record the
actual pixel and local radii and negative settings; residual kernel, shading
and UI differences preclude an exact geometry/pixel parity threshold.

The independently authored fresh closed sphere is
`scripts/fixtures/independent-crease-overlap-sphere.sgl`, SHA-256
`572bed8539b5fb2606f9362e12e94964aa24b2b232f404197b8ab930346aaa08`,
951,784 bytes, 18,242 vertices and 18,432 source faces (96 latitude rings,
192 sectors). Its only geometric input is the unit-sphere equation. Use the
same public SGL in separate clean 1600×1000, DPR1 normal-browser sessions for
pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and the current official OBrush web build. Import with normal visible controls
and isolate the active imported object. Require unchanged count/identity of
the hidden default sphere. Use normal visible Crease selection, brush numeric
settings, OBrush camera zoom and Save controls. Keep alpha None, no masks,
continuous off, and pressure effects off. Verify actual published controls.

Freeze eight overlapping held strokes on the front surface at local
`x=0.25`, `y=-0.18...+0.18` with 13 evenly spaced points each, reverse the
point order on alternating strokes. Project these local points through each
app's measured camera; the front `z` coordinate is
`sqrt(1 - 0.25^2 - y^2)`. Every path point and its mirrored ray must hit the
active imported surface before deformation. Require the two camera path spans
within 1%, actual trusted primary pointerdown/move/up events, no unexpected
pen pressure or continuous timer, and at least one admitted primary and
mirrored Crease dab per held stroke. Later paths use the same fixed screen
coordinates, as real repeated brushing does; record actual delivered dabs.
There is no adaptive recentering after a stroke.

Capture a normal OBrush `.obrush` Save and parked-cursor 1600×1000 screenshot
at baseline and after each stroke, then after eight sequential Undo steps and eight sequential Redo steps.
Compare every intermediate state against the saved checkpoint after the
corresponding stroke, including exact baseline/final restoration. In the reference,
read the active mesh's ordered XYZ arrays at the same checkpoints and record
screenshots. Require OBrush's undo depth to rise by one per stroke; exact
within-app XYZ restoration for every stage. OBrush Save hashes for the
reconstructed baseline/final must match the originals. Keep the hidden peer
object ID and content hash unchanged. Require zero browser WebGL errors and
no after-release geometry change after the host reports completed work and
at both 200 ms and 800 ms parked-cursor probes.

For every OBrush saved mesh, require exactly 18,242 finite XYZ positions,
constant triangle/index count, in-range indices, finite coordinates bounded
by `1e6`, no new area <`1e-10` triangles among baseline triangles above
`1e-10`, zero orientation reversals (baseline/current area-normal dot ≤0)
among baseline triangles above `1e-7` with any moved corner, and maximum
corresponding edge-length stretch ≤4.0. Record the worst ten face/edge IDs,
changed vertices, max local displacement and area ratio; retain any failure.
These are conservative tear/fold alarms, not a proof of aesthetic quality.
The independent reference mesh gets the same finite/index/fold diagnostics,
but no shared vertex-ID assumption or pass/fail safety threshold because its
import optimizer renumbers vertices. Do not silently attribute differences
between the renderers to mesh tearing.

For visual evidence, crop a fixed radius-180 CSS-pixel square centered on
the middle path point from baseline, stroke 4 and stroke 8 screenshots.
Count RGB-changed pixels at channel threshold >4, require at least 500 at
stroke 8 in each app to exclude a no-op, and retain the full screenshots plus
crops. Compute baseline-to-final ROI changed count, maximum channel change,
RMS RGB difference and silhouette/background leak count within a radius-150
interior disk. The last is descriptive because different renderers and
materials have different backgrounds and contours. Human close-up review
remains necessary before calling the surface clean.

Preserve source/build/harness/fixture hashes, exact UI values, projected
paths, admitted input, ordered positions, safety metrics, Save hashes,
screenshots and failed setup receipts. A failed safety gate is not tuned away;
follow-up experiments require a separately frozen gate. The SculptGL Crease,
radius, intensity, negative, culling, lock, alpha and ZBrush rows remain open.
Chrome execution waits until Q03 and MRP10 release the shared browser lane.
