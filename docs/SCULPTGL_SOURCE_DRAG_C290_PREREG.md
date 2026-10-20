# C290 Source Drag reset-fit causality, read-only

**Frozen intent before browser or product edits.** C288's geometry and Redo
normal gates remain failed. C289's identical mesh bounds, shifted Source
center, and ambiguous pre-fit near remain unchanged. This diagnostic uses the
same independently authored 1,089-vertex fixture (SHA-256
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`),
pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
and bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
Record OBrush HEAD and served WASM/JS SHA-256 in each receipt. One owned
browser per side, no root port 8792, no Cargo build, no sculpt stroke.
The frozen normal-UI harness is
`scripts/check-sculptgl-source-drag-c290-causality.mjs` (SHA-256
`b6295e6c1c3942ae38789db957080d7016b3c22b54fd04096ba384c51cee40f9`).

Use the C289 ordinary UI 1024×808 viewport, SGL Open with Scale and center
off, front orthographic camera, Drag selected on Source, radius 96 px,
pressure factors zero, Alpha None, symmetry/Continuous off. Do Source
`Reset (bar)` and the two-click off-mesh fit at `(screen.left+16,
screen.top+pixelHeight-16)`. On OBrush, use its visible Reset view and Center
controls after the same import, recording after each step. Do not substitute
a direct API camera setter for either normal UI action. Both sides retain
public Before Save with exactly equal 1,089 RGB-ID positions and no history
growth.

Before Source reset, install observational wrappers around the already
instantiated camera's `resetView`, `computeFrustumFit`,
`setAndFocusOnPivot`, and `_centerDelta`, and the scene's
`resetCameraMeshes`. Each delegates once to the exact original function and
records finite arguments, return value, camera center/offset/translation,
near/projection, world bounds, timer entries and a monotonic timestamp
immediately before/after. No kernel, geometry, camera parameter, timer,
pointer or return value may be changed by the observer. Record UI state
after import, after Reset, immediately before double-tap, after each tap,
and after all timers settle. The reset-vs-picked-pivot branch is determined
from actual `resetCameraMeshes` and `setAndFocusOnPivot` calls, not inferred
from the intended click coordinates. Record every `_centerDelta` after the
fit call to test whether their component sum exactly explains final center
drift. If observer binding or timing is ambiguous, stop.

At settled final view, deliver two trusted, integer-coordinate mouse hover
positions at viewport-center offsets `(-64,-20)` and `(44,-20)`, with no
button pressed. On Source, invoke its read-only picking intersection query
after the normal hover and record returned face, point, normal, and local
radius; the query may update picking state but must not edit mesh/history.
On OBrush, record the corresponding published camera frame and compute the
front orthographic ray/triangle intersection offline from its public Before
Save and the independently authored fixture. Compare actual Source point to
independent Source/OBRush ray predictions per coordinate; any unavailable
or stale pick is diagnostic failure, not a substituted value. Radius uses
the published 96 px and each published projection. Capture public Save and
history again after hover and require exact own Before bytes and unchanged
undo/redo depth. Source and OBrush active objects, viewport, fixture and
projection mode must be verified.

The causal gate requires: at least one observed `computeFrustumFit` call in
the normal UI sequence; captured pre-fit near/projection and returned factor
must reconstruct the pinned Source formula within `1e-12`; an observed
reset-or-pick branch; center-delta callbacks must reconstruct the final
Source center within `1e-12`; and the controlled ray/pick comparison must
account for the published camera difference within `1e-7` world units.
Retain raw failures and stop before a camera product edit if any gate is
ambiguous or fails. Even a causal pass only identifies the reference's
timing-sensitive camera path; C288's frozen `1e-6` geometry and GPU/history
gates require a separately preregistered product candidate and fresh pair.
