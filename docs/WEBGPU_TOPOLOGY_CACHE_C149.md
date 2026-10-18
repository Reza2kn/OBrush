# C149 resident topology reuse across browser strokes

The experimental browser WebGPU Inflate path still has a synchronous, checked
handoff to the shared Rust document. C148 measured about 127–135 ms in its
sparse fold guard on each million-vertex commit. The resident candidate was a
bounded full mesh copy that discarded derived topology, so the guard rebuilt
all incidence and face pairs even when a preceding stroke had already built
them for identical triangles.

This change stores the immutable derived adjacency behind `Arc` and copies its
handle only for a resident position candidate. Ordinary history/render
snapshots still discard the cache. The existing topology signature is checked
at each use, so a public vertex-count or triangle edit rebuilds the cache;
the cache is never serialized into `.obrush`. The first stroke still pays the
one-time build. A unit test checks pointer sharing, the unchanged-topology
position edit, snapshot omission, and invalidation after a triangle edit.

The owned browser replay imported the independently authored
`independent-id-sphere-1m.sgl` fixture (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`),
999,002 vertices and 1,998,000 triangles. It ran two three-contact mouse
strokes at the captured viewport center with an 8×5 CSS-pixel move, default
captured brush settings, then two visible Undo and two Redo commands. Its
record is `.artifacts/gpu-pointer-c146/c149-repeat-r0/result.json` (SHA-256
`05b6fc260da8862254aa81f1e9c2c8301e9eef9c3574b3fc1964af664501891e`).

| Consecutive stroke | Changed vertices | Source match | Candidate copy | Fold guard | Document replace | Release |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| First, builds full topology | 5,769 | 149.2 ms | 3.7 ms | 137.1 ms | 184.1 ms | 576.1 ms |
| Second, reuses full topology | 5,573 | 149.8 ms | 3.7 ms | 9.6 ms | 188.9 ms | 360.5 ms |

The before/first/second viewport SHA-256 values were respectively
`5398141e2042c211dcd70ad09f7796b918381a54c29269fe89fc43a3ddf7903b`,
`ae5446f55d34a3ac542c68812759df27ea12e933734e488ebb5265369e3b7ce7`,
and `bce3b015525556dbc8b406a130e3b1331093bd2bfefde3ef85cc62cd9282457d`.
Undo/Undo returned first/before; Redo/Redo returned first/second exactly.
The two strokes changed different vertex counts and source geometry, so these
one-run timings do not establish a controlled end-to-end speed ratio. The
measured fold-guard stage supports cache reuse; source matching, document
replacement, GPU picking, and sustained frame latency remain open. The
second release is still far above the 100 ms responsiveness target. This
does not establish normal-studio WebGPU integration or SculptGL parity.

The browser build hashes were
`51d1e87dab8dfc8be8bb22eecb4942b83db5d90e81365a20d608db3922d7dd4f`
for `obrush_app_bg.wasm`,
`b19f8213659944129c969027e1c53a0fc51828606af2630d647a374e5fb53cd6`
for `resident_bg.wasm`, and
`052b19db036d7fc32f377e683ad7c6b0a9c31ca77d9be2c344bb26c9ec03941d`
for `resident-pointer.js`.

Final-source checks passed: `cargo fmt --all -- --check`, 2,475 workspace
tests across 100 reported suites with zero failures, strict native all-target
Clippy, strict core/app/GPU WASM Clippy, `bash scripts/build-web.sh`, and
`bash scripts/package-macos.sh` with local app signature verification. The
two-stroke owned-browser replay passed with no recorded JavaScript exception,
console error, or external request. The first direct invocation of
`scripts/build-web.sh` was rejected by the shell because the script has no
executable bit; invoking the same script with `bash` succeeded. That failed
invocation made no source or artifact change.
