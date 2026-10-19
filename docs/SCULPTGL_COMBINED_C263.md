# C263 integrated SculptGL checkpoint

This checkout combines the C261/C262 Source Move, scene-selection, worker
safety, culling and renderer changes. It is a measured checkpoint, not
SculptGL or ZBrush parity. The frozen SculptGL reference is commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` with hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The 317-ID [current crosswalk](SCULPTGL_CURRENT_LEDGER.md) now links scoped
evidence for 58 of 309 active workflows; every full active acceptance gate
remains open.

## Product behavior and checked limits

- [Source Move](SCULPTGL_MOVE_DEFAULT_C261.md) now starts with connected-patch
  selection on in the SculptGL profile, as the pinned reference does. An
  independent disconnected-sheet fixture keeps the nearby unconnected sheet
  exact, and public Save/Open/Undo/Redo pass. Ordinary OBrush starts off;
  the user can toggle either setting. Matched reference displacement and
  edited multires behavior remain open.
- [Viewport Shift-click](SCULPTGL_SCENE_SHIFT_C261.md) now toggles selected
  parts on an independent three-box scene without a sculpt dab or new Undo
  step. The pinned selection sequence and OBrush sequence correspond by part
  identity. [A denser follow-up](SCULPTGL_SCENE_SHIFT_DENSE_FOLLOWUP.md)
  confirms Shift-drag still enters Smooth and has exact public Undo/Redo.
  SculptGL and OBrush displacements differ; brush-response parity remains open.
- [Stationary Continuous Draw](SCULPTGL_SOURCE_CONTINUOUS_PAIR_C261.md) and
  [Brush/Clay culling](SCULPTGL_BRUSH_CULLING_C262.md) were already present in
  the current product despite older frozen inventory gap labels. Paired
  browser/independent core checks now record their scoped behavior. Neither
  check closes the broader brush-family gates.
- The [cold Worker capture](SOURCE_WORKER_COLD_CAPTURE_C261.md) and
  [retained seed witness](SOURCE_WORKER_RETAINED_SEED_SEAM.md) validate bounded
  one-use candidates and exact generation-zero Source normal/tree
  reconstruction across a public-project round trip. They reject warmed
  retained state and do not publish a Worker sculpt or history event. Ordinary
  Worker sculpting and million-vertex responsiveness remain open.
- The [renderer channel scan](SCULPTGL_RENDER_CHANNEL_FUSION_C261.md) combines
  exact raw-mask/material verification with a controlled geometry attribute
  walk. A 10,114-vertex normal-browser stroke passed active GPU position
  readback, public Save and exact Undo/Redo. In separate preregistered 99,602-
  vertex, 60-second runs, renderer upload mean was 0.5112 ms before and
  0.4336 ms after; both passed all four 100k responsiveness gates. Different
  delivered dabs and scheduling prevent a causal speedup claim. The last
  [million-vertex paired baseline](SCULPTGL_MILLION_PAIRED_C226.md) still
  failed all four OBrush responsiveness gates.

## Integrated validation

`cargo fmt --all -- --check`, `cargo test --workspace --locked --quiet`,
strict native workspace/all-target Clippy and strict WASM workspace/library
Clippy passed. The inventory generator passes `--check`. Fresh
`scripts/build-web.sh` and `scripts/package-macos.sh` runs passed; the local
app also passed `codesign --verify --deep --strict`. Portable, SIMD and GPU
WASM SHA-256 hashes are respectively
`926a0962235d74343aae9c7019e2335d1c8611ef3a10e4aa67849cbb8efad086`,
`9b840cb8770595268463499bd0784cf0a42528686da7c5c62f62ec5b39eef038`,
and `172087711ccbe9fe500c34a1938f3b5d78992354fc2b15d2a7e57bfe1d97539c`.
The packaged macOS executable SHA-256 is
`c30da452b326375727ff60090fedd243b40d93580f108fa6d6fb1d6a0a5895b1`.
The fresh local browser route at `http://127.0.0.1:8792/` loaded the
integrated build; its SculptGL profile showed Move's connected-patch control
checked and Along Normal unchecked. This is a local UI smoke check, not a
native GUI or long-stroke comparison.

The linked reports preserve failed harness and verifier checks, including the
coarse Shift-drag no-op in both C260 and C261, the cold Worker historical
100k verifier failure, the retained-witness digest-offset test failure before
its correction, and browser harness setup failures. They were not counted as
passing reference parity.
