# Mask filters in OBrush 0.1.13

Grow, Shrink, Blur and Sharpen refine the active part's mask before sculpting
or extracting it. They change mask weights without changing the
current geometry, normals, paint, triangle order, sculpt layers or Morph Target.
Each changed result is one undo step.

## Using the controls

Paint with the Mask brush or hold Ctrl while brushing. In the left **MASK**
section, choose **Passes** and **Amount**, then press a filter button. Defaults
are one pass and full amount. Double-click either value, type it and press Enter
for exact entry. Clear and Invert retain their existing behavior.

Grow spreads darker weights to connected neighbors. Shrink contracts them. Blur
averages the mask across connected points. Sharpen increases contrast around
the half-masked weight, 0.5. More passes repeat the chosen operation; Amount
reduces the change per pass. After pending valid edits finish, zero amount and
other unchanged results leave genuine redo, document revision and the dirty
state untouched.

The active part must be visible. Filter controls are disabled during a topology
job or a live transform drag. A pending sculpt stroke finishes before filtering; a
rejected pending edit aborts the filter. After pending valid edits finish,
invalid settings or a failed candidate preserve the document and redo. The
last-filter report describes the last successful attempt, including no-ops.
It remains visible after selection or undo and clears on successful Open or
Append. Settings and reports are session state.

## Scalar and document contract

The shared pure Rust core validates the current mesh before scratch allocation.
Passes must be 1–16; Amount must be finite in 0–1. All points and faces obey the
existing one-million vertex/two-million triangle and aggregate document limits.
Repeated-index faces remain invalid. Valid duplicate faces, disconnected regions
and nonmanifold indexed graphs are supported by these mask-only operations.

Grow, Shrink and Blur use unique undirected triangle-edge neighbors. Each point
includes itself once, and neighbors are processed in ascending vertex-ID order.
Duplicate faces do not increase neighbor weights; coincident coordinates with
different IDs are not spatially welded. Each pass reads only the preceding
pass's weights. For an old weight `m`, neighbor set including self `N`, and
Amount `a`, the policies are:

| Filter | One pass |
| --- | --- |
| Grow | `m + a * (max(N) - m)` |
| Shrink | `m + a * (min(N) - m)` |
| Blur | `m + a * (mean(N) - m)` |
| Sharpen | `clamp(0.5 + (m - 0.5) * (1 + a), 0, 1)` |

Calculations use f64 from the previous f32 weights, then round to f32 after
each pass. Grow, Shrink and Blur preserve isolated points exactly. Sharpen is
pointwise and also affects isolated points. Fixed or rounded-unchanged values
retain their original bits, including signed zero. Reports count final changed
points and the largest absolute weight change relative to the input.

Filtering uses the current subdivision level's mask. Retained level geometry
stays intact; mask propagation follows the existing reconciliation and level
navigation policy. It does not promise byte preservation of every hidden
level's mask. Layers and stored targets retain their coordinates and bindings,
and inactive parts retain their payloads. No schema is added: existing mask
fields in project versions 1–5 persist the result. OBJ does not preserve masks.

These are synchronous whole-mesh operations. Checked scratch allocation and
pass limits bound accepted work, without establishing a peak-memory or latency
guarantee for large meshes or repeated UI commands.

## Reference scope

Maxon's [desktop Masking reference](https://help.maxon.net/zbr/en-us/Content/html/reference-guide/tool/polymesh/masking/masking.html)
describes Grow as adding a blurred edge and Shrink as softening the edge to
reduce it. OBrush's max/min graph policies above are independently defined and
have not been matched to that response. Blur and Sharpen are also unverified
against ZBrush desktop 2026.2.1. Blur and Sharpen are not inverse operations;
use undo to restore the prior mask.

Rectangle/lasso/region selection, mask adjustment curves, reference-matched
filter kernels, feature/backface/topology and color/group/UV/draft masks,
visibility selection and full combined masking workflows remain gaps. This
release supplies a limited S06 foundation and does not establish family parity.

## Verification

The final source passes **361 tests** (320 core, 41 app), formatting, strict
native/WASM Clippy, pure-core WASM compilation, the browser build and ad-hoc-signed
macOS packaging. The [build receipt](evidence/mask-filter-build.json) binds the
final source and artifacts. The [core receipt](evidence/mask-filter-core.json)
includes seven owner contracts, 28 independent
contracts and three focused host contracts. The numerical gate is an absolute
mask-weight error of 1e-7, with geometry, normals, numeric RGB, triangle order,
layer/target data and whole-document undo/redo checked exactly.

Four analytic square/filter cases differ from nominal mathematical weights by
at most 4.768371586472142e-8 after f32 rounding. All 138 independent numerical
observations agree exactly with the declared scalar oracle, including staged
f32 subdivision mask transport. These results are fixture measurements, not
global precision or reference behavior guarantees. An initial retained-level
test assumed a constant fine mask survived transport bit-exactly; its 1.1920928955078125e-7
error is retained. The test oracle was corrected to model the existing Loop
stencils and staged f32 delta arithmetic, without changing production or the
1e-7 acceptance gate.

The [browser receipt](evidence/mask-filter-browser.json) records eight workflows
and 45 case gates against the frozen final browser artifact. All
99 saved snapshots match 112 actual visible vertex/solid-index buffer
pairs, with 115 scene-object contracts, 11 exact f64 layer compositions and
11 target-binding checks. Recorded filter measurements agree exactly with
the independent scalar oracle under the unchanged 1e-7 gate. An irregular
indexed graph exercises duplicate faces, coincident distinct IDs, isolated
points and three synchronous passes at amount 0.37.

The ordinary Mask workflow uses exactly two separately released default
contacts: radius 0.18, strength 0.35, X symmetry and 24 moves along a
60×15-pixel trace. It then applies Grow for two passes at amount 0.5, Blur for
one pass at 0.5, and Sharpen for one pass at 0.5. Extraction at the unchanged
threshold 0.5 and world thickness 0.02 produces 620 points/1,232 faces in
2 closed components, while retaining the filtered source exactly.

Rich layers, an origin-level target and retained geometry survive filtering.
Inactive payloads and actual GPU cache identities/upload counts stay exact.
Version 2/3/4/5 imports keep their schemas and roundtrip through fresh file
loading. Hidden and live-transform controls are disabled; cancellation enables
them again. Last reports retain their original object/revision through undo
and selection, then clear on load. No-op dirty/revision tests exercise the
dirty state after Undo, with genuine redo preserved; a separate clean-state
browser no-op trial was not performed. Retained-level mask bytes are excluded
from browser geometry hashes; the independent core contracts establish their
existing transfer math.

All 26 pages report actual 4× MSAA and no runtime errors. Horizontal
toolbox/inspector containment checks cover 6,212 visible controls across
1,060 states; gizmo checks additionally require full viewport containment.
Three short stroke traces remain quiet after an explicit 100 ms settle and
incremental 0/200/800 ms probes, with no first-commit or input-latency claim.
The saved controls, disabled-transform state and extracted shell PNGs were
visually reviewed. An initial helper run failed when a schema-loop payload
binding shadowed its read-only state function. The failed run and helper are
retained; only that binding was renamed, without changing product code,
fixtures or acceptance limits.

Native GUI, physical devices, source-cap/large or long-session performance and
reference equivalence remain unverified.
