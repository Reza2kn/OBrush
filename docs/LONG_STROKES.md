# Long strokes on the supplied project: 0.1.5

The reported delayed brushing is reproduced on the supplied 61,580-vertex /
123,156-triangle project. In 0.1.4, a 20-second Draw gesture continues uploading
changed mesh data for **12.775 seconds after mouseup**. There are no main-thread
tasks over 50 ms in that run: successive redraws look responsive while obsolete
sculpt work accumulates. The earlier short fresh-sphere tests did not detect
this prolonged-input failure.

Final 0.1.5 ends visible mesh work 19.2 ms after the same synthetic Draw release
and 20.4 ms after Crease release. These are actual mesh-upload timestamps after
DOM pointerup handling, not physical mouse-to-display latency or a guaranteed
execution deadline. Pre-save GPU readback matches all serialized mesh fields,
two saves 1.5 seconds apart agree, and whole-stroke Undo/exact Redo pass.

## Input policy and measured paths

Normal brush processing uses up to 64 dabs per redraw and checks an 8 ms budget
between atomic dabs. Pending movement is limited to 32 dabs; obsolete movement
can be coalesced when work is old or capacity is exhausted. Mouse release commits
visible work and at most the newest pending endpoint, then closes the stroke.
This prioritizes current input under overload and deliberately changes the old
FIFO replay policy. An atomic dab can exceed the budget. The 50 ms age threshold
is checked between work, not a hard physical-input latency limit.

Each build loads the exact original user project through the public browser
file-input handler. Draw and Crease are independent fresh imports, radius 0.180,
strength 35%, X symmetry on, no inversion. Each trace contains 500 pointer moves
plus initial move/down/up over 20.08 seconds, following the same repeating
figure-eight. Move spacing varies between 50 ms, 20 ms and 66.7 ms to include
both sustained movement and a faster burst. Node schedules sends independently
of browser acknowledgements.

| Original-project path | 0.1.4 last mesh upload after release | 0.1.5 after release | Mesh submission interval p95, old → new |
| --- | ---: | ---: | ---: |
| Draw, 20 seconds | 12,774.8 ms | 19.2 ms | 30.1 → 27.6 ms |
| Crease, 20 seconds | 0 ms | 20.4 ms | 25.5 → 27.1 ms |

Crease already keeps up with this baseline input; its newer geometry safeguards
add work. The result is not a universal frame-rate improvement. Both final paths
have zero tasks over 50 ms, but their maximum mesh-submission gaps are 62.3 ms
and 53.6 ms. Independent rAF scheduling and GPU submission/presentation are
different measurements.

| Final queue counters | Draw | Crease |
| --- | ---: | ---: |
| Processed dabs, including misses/rejections | 1,366 | 1,464 |
| Dabs that actually change geometry | 772 | 815 |
| Coalesced dabs | 362 | 264 |
| Recorded peak pending dabs | 14 | 11 |
| Pending dabs after release/quiet | 0 | 0 |
| DOM pointer events delivered / sent | 502 / 503 | 498 / 503 |

Oldest-pending-age samples peak at 59.3 ms and 56.7 ms. They can exceed the nominal
threshold before the next processing check. Browser event coalescing also occurs;
these measurements do not establish lossless reconstruction of every hardware
sample. The two saved final sculpts contain 76,995 vertices for Draw and 71,206
for Crease; their geometry need not equal the overloaded old FIFO output.

## Actual surface and existing folds

The supplied mesh is closed, manifold and consistently wound, but already
contains thin folded regions. All four saved prolonged sculpts retain finite
fields, valid indices, closed topology and consistent winding. That structural
check does not establish freedom from self-intersection.

Counts alone can grow when adaptive refinement splits an existing folded edge.
The table therefore includes total edge length at adjacent face angles over 90°,
using the project's world units. Triangle quality is
`2√3 × cross_length / sum(edge_length²)`, with 1 for an equilateral triangle.
These are descriptive diagnostics; no global acceptance threshold was
preregistered for this supplied sculpt.

| Mesh | Edges over 90° | Their total length | Faces with quality below 0.15 |
| --- | ---: | ---: | ---: |
| Original supplied project | 78 | 1.01921 | 64 |
| 0.1.4 prolonged Draw | 90 | 1.01907 | 72 |
| 0.1.5 prolonged Draw | 90 | 1.01901 | 64 |
| 0.1.4 prolonged Crease | 146 | 1.72529 | 100 |
| 0.1.5 prolonged Crease | 80 | 1.01916 | 64 |

The new Crease path avoids the substantial extra severe folding seen in the old
path. Existing folds remain; importing the file does not automatically heal the
stored sculpt. The unchanged shader and actual 4× multisampling render the real
saved geometry, rather than concealing it through an image filter.

A separate one-Loop smoothed duplicate is a **preview**, independent of these
runtime cases. It contains 246,314 vertices / 492,624 triangles, retains two edges
over 90°, and reduces their total length to 0.009879. Its minimum triangle quality
worsens from 0.04055 to 0.02850. Matched windowless CGL renders show softer ridges
and broadly retained form. A tighter frontal pair—yaw 0, pitch 0, distance 1.6,
target (0,0,1), 1153×912—still reveals thin forehead slits and corrugation along the
right/lower folded rim. It is not a fully repaired replacement. The original
project and screenshot remain untouched, and no prolonged performance claim is
made for this roughly four-times-face-count preview.

## Evidence and limits

[The browser record](evidence/long-stroke-browser.json) binds shipping WASM
`deee04f729ed17e04a0955e7d221adea1934cba49aafcdee91464ba7dffee4cb`, source and
renderer hashes, the original project hash, complete scheduled traces, actual
runtime data, GPU/serialized-mesh hashes, stable saves, history checks and matched
CGL images. [Native core measurements](evidence/long-stroke-core.json) and
[geometry tests](evidence/long-stroke-geometry.json) have their own contracts.
[The build record](evidence/long-stroke-build.json) binds the 97 passing workspace
tests, strict native/WASM checks and packaged browser/macOS artifacts.
Failed import-readiness and harness syntax checks are retained; the successful
baseline used a harmless headless pointer repaint after public import and then
verified every imported mesh field. Final import explicitly requests repaint.

This is one 20-second case per brush/build on Apple M2, Chrome 155, ANGLE Metal,
1600×1000, DPR 1, actual 4×AA. Final CPU-heavy audit jobs completed before browser
sampling; background/thermal variation is not otherwise isolated. Final
instrumentation adds public queue counters every 200 ms to the otherwise matched
input/render measurements, so full harness bytes differ and the extra overhead
is not separately measured.

Only owned headless Chrome processes/profiles and loopback listeners are used;
Chrome exits and owned servers close. There is no desktop, accessibility,
VoiceOver or user-tab/process control. Windowless CGL renders actual mesh data
with matched camera/material/shaders; they do not certify native GUI behavior or
pixel-identical browser presentation. Tablet pressure, alternate browsers/GPUs,
large imported-model capacity, long sessions and physical input-to-display
latency remain outside this check. ZBrush parity remains unverified.
