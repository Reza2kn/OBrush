# Held-pointer Source Worker diagnostic (unpromoted)

This branch tests an experimental, query-gated (`?source-worker-held`) route from real Chrome viewport PointerEvents to one Worker-owned Source Draw transaction. The trial button is off by default. Only unmodified, asymmetric, static Source Draw with an eligible fixed-topology mesh is admitted; other brush settings reject the gesture. This is a correctness and latency diagnostic, **not** evidence of SculptGL or ZBrush parity and **not** a responsiveness improvement.

## Build and checks

- Base integrated revision: `5da840e`; the checkout changes represented by this document were built with `scripts/build-web.sh` and the lockfile-matched wasm-bindgen CLI 0.2.129.
- Scalar app WASM SHA-256: `925bc066107dd5c01b75bd9a25df6b28576df1aa33defc5f7483c68d2b4a75b2`.
- SIMD app WASM SHA-256, used by Chrome: `236d154c608a1bbb6f7dec66b07c606423714da7f83ba2cd59ce4930ee137286`.
- Worker JS SHA-256: `dfc1f140cd6872e6c2fa4fc3e809260802679710467ae70b3a0d787a13ba7221`.
- Instrumented held-pointer input JS SHA-256: `13df213158394cf8c66e0b60c46d6c898e9f3d0a1c2d6af947aced45423a2bc7`.
- `cargo fmt --all --check`, `cargo test --workspace --locked --quiet`, native workspace/all-targets strict Clippy, WASM core/app/gpu strict Clippy, and `scripts/build-web.sh` passed. Focused core cold/warm capture tests were 8/8 and the app preview/publication test passed. The separate Worker-realm script passed against an independently created triangle; receipt: `.artifacts/source-worker-realm/cf2a0bd8-8692-42f4-b77a-1246619bff39/result.json`.

## Ordinary Chrome path

Chrome opened the built app at `http://127.0.0.1:8840/?source-worker-held`. A fresh 40,962-vertex, 81,920-face sphere was explicitly converted to raw Source mask representation; X symmetry was turned off; Source Draw (the UI's fixed-topology response) was selected. Draw used its default 50-viewport-point radius, 0.75 radius factor, and default Source Brush options. The experimental toggle was enabled. Two browser-driven, trusted mouse drags crossed the visible sphere: approximately `(770,395)→(828,410)` and `(816,390)→(850,411)` in the 1633×891 Chrome viewport.

| Observed counter | First cold stroke | Second warm stroke |
| --- | ---: | ---: |
| Cumulative committed edits | 1 | 2 |
| Cumulative rejected gestures | 0 | 0 |
| Cumulative acknowledged dabs | 1 | 2 |
| Cumulative queued pointer samples discarded on release | 1 | 2 |
| Pointerdown through Worker begin reply | 365 ms | 834 ms |
| Pointerdown through first checked preview | 521 ms | 1,045 ms |
| Pointerup through public commit | 1,086 ms | 1,418 ms |
| Maximum main-thread preview decode during run | 58 ms | 159 ms |
| Maximum Worker call during run | 97 ms | 97 ms |

The second stroke confirms that the active retained Source producer can be transferred to the Worker and returned for another ordinary stroke. It does not prove responsive sculpting: even this 40k mesh stalls visibly. Release deliberately discards queued path samples rather than drawing several seconds after pointerup, so the published sculpt can end before the user's release position. At most one already-posted apply completes after pointerup. This fidelity limit must be removed before enabling this path by default.

The normal **Save project** button produced a versioned `.obrush` file after two strokes with SHA-256 `2df7c1abf38487ed12c335e7f92245064ed58283ba87efeb786bfe78fd7f9734`. The normal Undo shortcut changed 200 position rows and 255 normal rows without changing topology, masks, or colors. Redo followed by Save produced a byte-identical file with the same hash. The saved files are in `.artifacts/held-source-worker-chrome/`. A Save after only the first stroke was not captured, so exact Undo-to-first-stroke byte identity is **unverified**. GPU buffer position/normal words were not read back in this run; renderer-selected retained display normals can intentionally differ from saved mesh normals.

The per-dab path still serializes and hashes the complete public scene, validates and clones full meshes, and snapshots the Worker mesh. This is an O(vertex-count) cost on both threads for each dab and is the likely blocker for 100k and million-vertex responsiveness. The frozen sustained admission in `SCULPTGL_WORKER_SUSTAINED_ADMISSION.md` remains open. A separately checked sparse preview shadow and renderer row-patch route, plus exact browser GPU and cancellation checks, are required before any performance or parity promotion.
