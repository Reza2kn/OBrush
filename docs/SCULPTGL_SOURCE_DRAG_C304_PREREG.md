# C304 Source Drag visible-symmetry control and mirrored kernel

Frozen before C304 product edit or browser capture. C300, C301 and C303 are
separate failed OBrush symmetry-on **setup** attempts. Their raw receipts and
script hashes remain unchanged. C303 proves the control is absent: the
`SourceGeometryMode::SourceOrdinary` panel excludes `Drag` from
`source_geometry_alpha_panel`, the only renderer of
`source_geometry.mirror`. C300's symmetry-off unchanged-product pair passed;
Source symmetry-on captures succeeded. None of those attempts proves OBrush
symmetry-on geometry. The C298 Drag draft was superseded before execution and
belongs to a different feature track.

The C304 trial adds a visible `X mirror symmetry` checkbox in the ordinary
Source Drag panel, reusing the existing global `brush.symmetry_x` and
`commit_brush_view` state; it does not expose an unsupported alpha picker.
It retains the isolated shared-core/host candidate for a mirrored drag center
and one atomic preview/history transaction. Normal UI clicks and trusted
pointer events remain the only input route. There is no direct model mutation
by the harness. The prior C295 single-gesture Drag path must regress cleanly.

Fixture, reference commit/bundle hash, 1024×808 settled camera, 96 px radius,
Alpha None, zero pressure factors, Lock/Continuous off, four integer pointer
offsets, off/on matrix, 1e-8 projection, 1e-6 paired XYZ/active GPU display
normal, exact changed-ID membership, ID541/544/545/548 mask and mirror
witnesses, one-step public Save/Undo/Redo, active GPU words/identity and quiet
state gates are exactly those in `SCULPTGL_SOURCE_DRAG_C300_PREREG.md`.
Independent fixture SHA-256 is
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`;
pinned SculptGL commit is `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
and bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The integrated product base is root `0643619afdd2d303b0a37f276648ff54b6eb2305`.
Record actual served JS/WASM SHA-256 for each capture.

C304 harness is C301 with only protocol/result names changed. The visible
`source_geometry.mirror` selector is the same as C301. No C303 inspector
scroll is used. Reuse the Source/off-root captures only as setup provenance;
run a fresh normal-UI C304 quartet on the candidate with all fixed gates.
If setup or a numerical/history/GPU gate fails, preserve the receipt and do
not promote the trial. Required product checks: cargo fmt, workspace tests,
strict native and WASM Clippy, WASM check, scripts/build-web.sh,
scripts/package-macos.sh, the C304 matrix and independent C295 regression.
Full Drag, SculptGL and ZBrush parity remain open.

Frozen SHA-256 (all three passed `node --check`):

| Script | SHA-256 |
| --- | --- |
| `scripts/check-sculptgl-source-drag-c304-ui.mjs` | `900bad98e485c5b064f04a7b74540bbf6a05eaae740988ef85a5c75f26d77012` |
| `scripts/audit-sculptgl-source-drag-c304-ui.mjs` | `2c362b8e723b0dea5e1c6dbf42b8f556a2c50bcbc8dae082cd73a23c621a7156` |
| `scripts/audit-sculptgl-source-drag-c304-matrix.mjs` | `b8120ed5747c85f5eb6adc17d3f81eaf82292ae0cc9ed58546071b37465e27f3` |
