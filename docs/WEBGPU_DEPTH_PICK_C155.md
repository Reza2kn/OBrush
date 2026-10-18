# C155: browser depth-pick experiment

The browser prototype now has a separate opt-in `?webgpu-preview=1&webgpu-depth-pick=1` path. It samples the WebGPU viewport's retained depth texture, unprojects the pointer pixel in a compute pass, and applies the existing resident Inflate kernel. The Rust/WASM document remains authoritative for the checked sparse commit and Undo/Redo. Normal OBrush sculpting still uses shared Rust geometry in WASM and WebGL2 drawing.

This answers a narrow performance question: moving picking into WebGPU is feasible, but this depth method did **not** make the million-vertex browser stroke real-time. The first trial failed WebGPU validation because the sampled depth texture lacked `TEXTURE_BINDING`; its aborted stroke and unchanged document are preserved in `.artifacts/gpu-pointer-c146/c155-depth-short-r0/result.json`. The corrected build passed short and 10-second browser pointer runs, with visible history recovery.

| Frozen local browser run | Earlier GPU-owned ray pick (C153) | Depth pick (C155) |
| --- | ---: | ---: |
| Independent SGL sphere | 999,002 vertices / 1,998,000 triangles | Same fixture |
| Input | center mouse press, move +8/+5 CSS pixels, release; radius 0.18, strength 0.35 | Same |
| Three-dab changed vertices | 5,769 | 5,765 |
| Three-dab release | 443.9 ms | 439.1 ms |
| Three-dab after-image SHA-256 | `ae5446f55d34a3ac542c68812759df27ea12e933734e488ebb5265369e3b7ce7` | `82b9fff7145e9ac0a930a21539cc1573d675ed97d2e92377cb8a0dab990c6d5b` |
| Profiled pick + sculpt for three dabs | 97.4 + 9.5 = 106.9 ms | 103.8 ms combined |
| Profiled draw for three dabs | 103.7 ms | 104.2 ms |
| Ten-second dabs | 112 | 113 |
| Ten-second RAF P95 | 50.1 ms | 50.1 ms |
| Ten-second release | 547.0 ms | 540.9 ms |

The short images and changed-vertex counts differ, so these are scoped latency observations, not identical-output speedup claims. Depth is sampled at a raster pixel center; ray picking intersects triangles continuously. The 6.1 ms long-release difference is smaller than the uncontrolled run-to-run variation and does not close the responsiveness gate. The native/browser parity and broad brush-response gates remain open.

The independently authored input is `.artifacts/sculptgl-large-sustained-c58-stage/fixtures/independent-id-sphere-1m.sgl` (SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`). The final browser GPU WASM is `web/dist/gpu-pkg/resident_bg.wasm` (SHA-256 `3c326e8bfc296eb2297ac2965c25c9ade856052568ab48ca05dcab26523b5443`) and the pointer host is `web/dist/resident-pointer.js` (SHA-256 `e02660f04b7c34ec461b0e324f1e29bd48409228edc2781a070840b07ac40a7a`). Raw short, long and profile records are `.artifacts/gpu-pointer-c146/c155-depth-{short-r1,long-r1,profile-r2}/result.json`; C153 counterparts are in sibling directories. These ignored artifacts are local evidence, not distributable fixtures.

The remaining latency includes a full resident draw and a shared-core document release that validates geometry, refreshes normals, and records history. Merely selecting WebGPU as the browser renderer cannot eliminate those CPU/WASM tasks. Production integration requires a safe persistent GPU mesh, broader brush coverage, a lower-cost document handoff, and matched visual/performance checks.

Final source checks passed: `cargo fmt --all -- --check`, `cargo test --workspace --all-targets` (2,478 passed, zero failed, one ignored across 101 groups), strict workspace Clippy, strict WASM-target Clippy for the core/app/GPU crates, `scripts/build-web.sh`, `scripts/package-macos.sh`, and `git diff --check`. Browser short, long and fenced-profile records each passed visible commit and Undo/Redo. The failed first browser run remains in the raw evidence above.
