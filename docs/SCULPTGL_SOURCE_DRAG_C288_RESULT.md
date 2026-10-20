# C286/C288 Source Drag result: not promoted

The independently authored C286 fixture and normal-UI harness are described
in `SCULPTGL_SOURCE_DRAG_C286_PREREG.md`. Its first Source setup failed before
Before Save because the generic live-state snapshot omits Drag's absent
`_lockPosition` field; the failed receipt is preserved. The corrected Source
run completed. C286's OBrush run then failed its frozen X/Y projection-ratio
setup gate (`7.618793e-6`, `7.643452e-6` versus `5e-6`) before geometry
comparison. Its raw receipt remains at
`.artifacts/sculptgl-source-drag-c286-ui/root-63e856f4-b4b5-4e81-a2a5-d66c63be3d36/result.json`.

C288 was separately preregistered with the identical fixture, 96px radius,
integer four-point gesture, Source/OBRush controls, `1e-6` XYZ and active
GPU normal tolerances, and exact history/GPU/attribute/topology checks. Only
the published projection-ratio setup bound changed to `1e-5`; measured world
radius difference was `6.521270e-7`, and maximum requested-ray difference
was `4.347513e-7`, both within the unchanged `1e-6` bounds. The complete
paired normal-UI audit is:
`.artifacts/sculptgl-source-drag-c288-ui/source-99a8ccab-9991-43ea-a525-d02bd6a231de/paired-audit-root-43905303-9d47-4fb8-b1a4-50ae7c644f91.json`.

**C288 failed.** Paired after/redo XYZ maximum was `2.060085535e-6`;
held move 1 was `1.054257154e-6`, and held move 2 was `2.060085535e-6`,
all above the frozen `1e-6` gate. Redo GPU display-normal maximum was
`1.615210203e-4`, also above its `1e-6` gate. The paired Before and Undo
positions/normals were exact; held move 0 XYZ maximum was `3.129243851e-7`.
The Source and OBrush each changed the same 95 RGB IDs, kept protected ID544
fixed, passed their own active GPU-position/public-Save checks, passed
attribute/topology checks and restored exact Before/After bytes with one
Undo step. Quiet post-release GPU matched After on each side. These scoped
passes do not repair the numerical failures.

Candidate Source Drag core/app commit `8cfcf40` was built from this isolated
worktree with web WASM SHA-256
`fef05973a75a7ad5667a7ba5f261368bea509fcff9e40c8adcec9691e52e70c4`
and JS SHA-256
`f5be35803fed50aa7f8a8e46153c4fe26b2cdb833d692db5f94770c112d889e5`.
Pinned SculptGL bundle SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
`cargo fmt --all -- --check`, full `cargo test --workspace`, strict
`cargo clippy --workspace --all-targets -- -D warnings`, and
`scripts/build-web.sh` passed after the named projection-result refactor.
The initial strict-Clippy `type_complexity` failure is retained in task
output. No product code from the failed trial is promoted; `SGL-BRUSH-DRAG`
remains open. A later attempt needs a new preregistered causal fix, not a
changed tolerance under C288.
