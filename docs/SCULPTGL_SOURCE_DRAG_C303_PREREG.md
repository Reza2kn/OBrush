# C303 Source Drag symmetry inspector-scroll correction

Frozen before any C303 browser run. Preserve the failed C300 and C301
OBRush symmetry-on setup receipts unchanged. C300's unchanged-product
symmetry-off pair passed its scored geometry/GPU/history checks, and both
C300/C301 Source symmetry-on captures completed. C301 OBrush symmetry-on still
stopped before pressing on the mesh: `Normal control offscreen
source_geometry.mirror`. It is a setup failure, not a geometry result.

Static cause: the shared normal-control helper probes an offscreen inspector
control but sends `deltaY:-350` when the control lies below the window; this
scrolls upward and repeats 20 times. C303 keeps that shared helper unchanged.
Before setting the same visible `source_geometry.mirror` checkbox, the capture
measures its published rectangle and sends at most 16 ordinary CDP mouse-wheel
events over x=50 in the inspector. A below-fold target gets positive deltaY;
an above-fold target gets negative deltaY. Each event and rectangle is retained.
There is no DOM property mutation or direct model setting. The original normal
control click and publication acknowledgment still follow. Only the on-case
OBRush setup uses this pre-scroll.

The independently authored fixture, pinned reference commit and bundle SHA,
1024×808 settled camera, 96 px brush, Alpha None, pressure factors zero,
trusted integer path, symmetry off/on settings, and all `1e-8` projection,
`1e-6` XYZ/active-GPU-normal, exact membership, mask, public history, GPU,
quiet-state, matrix gain and C295 regression gates are unchanged from
`SCULPTGL_SOURCE_DRAG_C300_PREREG.md` and
`SCULPTGL_SOURCE_DRAG_C301_PREREG.md`. Candidate product code remains unbuilt;
run one unchanged integrated-root baseline quartet first. Record actual served
WASM/JS and root HEAD. If setup still fails, preserve it and stop before
candidate build or product promotion.

Frozen SHA-256:

| Script | SHA-256 |
| --- | --- |
| `scripts/check-sculptgl-source-drag-c303-ui.mjs` | `908e06b0846db2850d4be33f45d089c79514c9b271270650a99120548763a6b4` |
| `scripts/audit-sculptgl-source-drag-c303-ui.mjs` | `fd2a3a5e1c4e503a008cfcb17c6d9522891a7287cca1af6d9dc3db1ff05206e1` |
| `scripts/audit-sculptgl-source-drag-c303-matrix.mjs` | `03a8a79a7adf65d3fdf51b3920b5eb70be31cae039dadc819dd76fb078814c4b` |

All three passed `node --check` before capture. A C303 candidate can ship only
after this fixed matrix, C295 regression, focused and full Rust tests, strict
native/WASM Clippy, WASM check, normal web build and macOS package succeed.
Full SculptGL Drag and ZBrush parity remain open.
