# Region masks in OBrush 0.1.14

Rectangle and Lasso select mask regions in the viewport. They change weights
on the active part without changing geometry, normals, paint, layers, stored
shapes or triangle order. A changed selection is one undo step.

## Using region masks

Choose **Rectangle** or **Lasso** in the left **MASK** section. This selects the
Mask tool and leaves Transform mode. Drag from any point in the viewport,
including empty background, then release to apply. The outline is a preview;
mask weights stay unchanged until release. Hold **Alt when pressing** to erase
the selected weights instead. The captured erase choice stays fixed during the
gesture. Choose **Brush** for ordinary painting with the Mask tool or Ctrl.
Changing mask mode to Brush preserves any other selected sculpt brush.

Other brushes keep their ordinary gestures. Ctrl temporarily uses the selected
mask mode while sculpting; Transform mode retains its existing priority. Region
masks select through the active part's depth, including occluded and back-facing
points. They do not select another part under the pointer. Radius, strength,
pressure, alpha and X symmetry apply to brush painting and do not alter these
hard region selections. Use Blur or the other mask filters afterward to refine
the result, then sculpt or extract the region.

Escape cancels a preview. Focus loss, viewport resizing, lost contact without a
release, and document/history/file/part/settings/transform commands also cancel
unfinished region gestures. A command delivered with a release cancels rather
than implicitly applying the selection. Camera movement is paused during the
gesture; a camera-navigation button cancels the preview first. Releasing outside
the viewport uses an endpoint clamped to the captured viewport. A rectangle with
zero width or height selects nothing. The host also skips clicks whose maximum
observed clamped distance from the press is at most one logical screen unit.
This skips projection, preserves redo and marks the report `skipped_tiny_click`;
a large closed lasso remains a selection even when it ends at its start.

The active part must be visible, with no running topology job or live transform.
Pending valid ordinary brush edits finish before a region starts; a rejected
pending edit aborts the dependent gesture. After pending valid edits finish,
unchanged selections preserve genuine redo, document revision and dirty state.
The last-region report is historical session data, including after undo or part
selection. Successful Open/Append clears it. Mode, preview and reports are not
project properties.

## Projection and selection contract

The pure Rust core receives a frozen column-major f32 OpenGL view-projection
matrix, f64 top-left viewport `[x, y, width, height]`, a rectangle or polygon,
and an erase flag. It widens source f32 positions and matrix entries to f64
before homogeneous multiplication. A point is eligible only when `w > 0` and
`-w <= x, y, z <= w`, including near/far and side boundaries. Screen X maps left
to right and screen Y maps top to bottom. No depth buffer or face normal is used.

Rectangles include their exact bounds, independent of drag direction. Lasso
uses even-odd inclusion and supports self-crossing outlines. A segment boundary
includes points within 1e-7 screen units of the segment with an expanded endpoint
bounding box; zero-length edges use distance to the point. Consecutive repeated
points and a repeated closing point are allowed. A lasso requires three
non-collinear distinct points; entirely collinear or shorter input is rejected.
Signed polygon area is not used to reject a self-crossing lasso.

Selected weights become 1 for masking or 0 for erasing. Outside weights retain
their bits, and numerically unchanged selected weights also retain their old
bits, including signed zero. Selection is per vertex: it does not cut faces or
add points at the drawn boundary. Mesh density limits boundary detail.

Matrix entries must be finite. Screen coordinates, viewport origin and corners
must be finite with absolute values at most 1,000,000; width and height must be
positive and at most 1,000,000. The existing one-million vertex/two-million
triangle and aggregate document limits apply. Lasso accepts at most 512 input
points and rejects a conservative `vertex count × input point count` estimate
above 32,000,000 before scratch allocation. The report's point-edge work is that
estimate, rather than a measured number of comparisons. These synchronous work
and allocation checks do not guarantee latency or peak memory on large meshes.

Validation and candidate document budgets run before mask installation. Geometry
and retained-level geometry, exact layer/target metadata and inactive payloads
stay intact. Retained mask transfer follows the existing subdivision
reconciliation/navigation policy. No schema is added: existing project versions
remain readable, and version 2/3/4/5 mask fields store the result. OBJ does not
preserve masks.

## Reference scope

Maxon's [desktop masking guide](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/masking/masking.html)
describes Ctrl gestures for rectangles and a Lasso Mask brush, including starts
outside the model and alpha-driven rectangle selection. OBrush uses explicit
mode controls and its independently defined projection/inclusion rules.
Reference gesture conventions, alphas, soft edges, occlusion/front-face modes,
symmetry and the exact ZBrush 2026.2.1 region response remain unverified.

Feature, topology, color/group/UV/draft masks, mask adjustment curves, visibility
selection and broader combined masking workflows remain gaps. This is an S06
foundation, without a full-family parity claim.

## Verification

The final source passes **422 tests** (365 core, 57 app), formatting, strict
native/WASM Clippy, pure-core WASM compilation, the browser build and
ad-hoc-signed macOS packaging. The core checks include eight owner and 37
independent region contracts. Sixteen focused host contracts cover gesture capture,
caps, cancellation, same-frame navigation taps and the bounded DOM-button latch.
A listener check also covers the production browser wiring. Thirty-five existing filter
contracts also pass through the shared mask transaction helper.

Selected and unaffected masks are checked as exact f32 bits; geometry, normals,
numeric RGB, ordered faces, precise layer/target data, inactive state and
document history are also exact. Analytic identity, asymmetric column-major and
perspective fixtures cover inclusive clipping and top-left screen coordinates.
No projected-coordinate error bound is claimed because the API exposes selection
weights and counts rather than point coordinates. Existing subdivision mask
transport has independent stencil and staged f32 tests.

Two genuine initial core failures are retained: a diagonal point beyond an
endpoint at distance 1.2727922046807977e-7 was included despite the fixed 1e-7
boundary rule, and a valid outline with 1e-200-scale coordinates was rejected
when its cross product underflowed. Finite-segment endpoint distances and
normalized direction checks correct those cases without changing the tolerance,
caps or minimum-extent policy. Tests cover tiny truly-collinear and mixed-scale
controls; they do not establish exact-real arithmetic at every floating-point
dynamic range.

The first broad source/build run passed 413 tests before a separate review found
a host navigation gap. Same-frame right/middle press-and-release was missed by
checking only final button state. Both regressions failed on the retained old
predicate and pass after raw navigation press events were included. The
415-test run included that native/raw-event correction. The browser then
exposed another path: pressing right while left was held produced a DOM
`pointermove` with updated button bits rather than a new `pointerdown`, so the
framework did not emit the expected secondary press. That failed browser check
and its superseded build are retained. The host correction uses current
navigation bits and one cancellation latch across frames; final browser checks
exercise actual right and middle chords. Review also found that primary-only
DOM `pointercancel` needs an explicit cancellation signal: clearing navigation
bits alone does not cancel a primary contact. The interrupted pre-correction
check run is retained and excluded from final evidence. A prior host Clippy
failure is retained too.

An initial browser helper wrongly expected part selection to create undo
history. That failed check and its source are retained. The corrected test
verifies the established policy with exact Undo to a prior import and Redo
to the selected scene.

Eight workflows and 59 behavior gates pass on the frozen browser build. They cover forward/reversed/Alt rectangles; closed, repeated, concave and self-crossing lassos; preview/Escape/navigation/resize cancellation; frozen camera and clamped release; command barriers and captured modifiers; active-part/layer/target isolation; tiny/empty/saturated redo-preserving no-ops, hidden controls, Transform priority and the 512-point rejection; project versions 2–5 and historical reports; and a default-sphere region/filter/extraction workflow. All 115 saved snapshots match visible GPU vertex and solid index buffers (127 exact visible pairs). Exact f32 mask bits use an independent projection/inclusion oracle bound to actual dispatched press/move/release events. The report preserves the frozen matrix and viewport.

[Core](evidence/mask-region-core.json), [browser](evidence/mask-region-browser.json) and [build](evidence/mask-region-build.json) receipts retain independently created fixtures, final source/build/helper hashes, numerical differences and failed attempts. Browser checks use an isolated, owned headless Chrome session. Release traces sample after a 150 ms settling interval, with subsequent 0/200/800 ms quiet probes; they do not measure first-commit or physical input latency. Actual DOM pointercancel/blur signal wiring has a separate mock check; physical contact cancellation and focus loss were not exercised in this browser run. Horizontal toolbox/inspector containment and full enabled gizmo containment are checked in the observed states. Browser frame grouping, physical focus loss, the 32-million work limit in a browser and large/long-session performance remain unverified.

Native GUI, physical devices and reference equivalence remain unverified.

The default-sphere workflow projects 40,962 vertices and selects 1,667. One Grow pass at 0.5 followed by one Blur pass at 0.5 has maximum mask error 0 against the independent scalar oracle. Extraction at threshold 0.5 and thickness 0.02 produces 4,084 points and 8,160 triangles in 2 closed components; the filtered source remains exact. Exact captured matrices, screen endpoints and f32 parameters are recorded in the browser receipt.
