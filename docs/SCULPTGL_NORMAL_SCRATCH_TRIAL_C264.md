# C264 preregistration: bounded normal-refresh scratch trial

This preregistration precedes the post-change browser run. The [current unmodified baseline](SCULPTGL_MILLION_CURRENT_C264.md) is commit `28fabe2`, portable WASM SHA-256 `926a0962235d74343aae9c7019e2335d1c8611ef3a10e4aa67849cbb8efad086`, and result SHA-256 `9cc22bcc0f2989a2d1e53576a13bda5db2528dd51ebbb217beaf3ff1ef9ff3e5`. Its 957 dabs averaged 1.987 ms in Legacy normal refresh and 22.943 ms in Source geometry. Active RAF P95 was 66.7 ms and release acknowledgment 159.0 ms; all four responsiveness gates failed.

The one proposed code change replaces two full-vertex scratch allocations in `Mesh::recompute_normals_near_vertices_cached` with one affected-vertex accumulator. The saved pre-change algorithm is the test oracle. The production function must preserve ascending incident face order, corner order, every normal f32 word, and unrelated mesh/project words across repeated local dabs. It cannot change file/history or Source selection rules.

Admission before accepting this candidate:

1. `cargo fmt --all -- --check`, the focused original-algorithm bitwise regression, full workspace tests, strict native and WASM Clippy, and `scripts/build-web.sh` pass.
2. Re-run the exact frozen 999,002-vertex, 60-second, 400-requested-moves/s normal-browser protocol on the candidate build. Keep the same fixture/protocol hashes and use a fresh owned Chrome profile. Independently audit complete public before/after/stable/Undo/Redo saves, unchanged non-position channels and faces, finite geometry and affected-face shape; record browser errors and clean closure. A focused actual WebGL vertex-buffer/public-save word comparison is required before claiming displayed-output equivalence.
3. Compare candidate per-dab normal-refresh and Source-geometry means to the baseline, plus RAF P95/max, app CPU P95, release acknowledgment, dabs/coalescing, renderer admission/upload, and vertex-upload bytes. Different delivered contacts make the browser runs observational rather than a causal paired speed test. Reject this optimization if the direct normal-refresh stage does not improve or Source geometry worsens materially; preserve the failed receipt. Even success leaves the existing million-vertex responsiveness gates open unless the original thresholds actually pass.

This trial has at most about 1.987 ms per dab of directly measured normal-refresh work to remove. It cannot by itself bridge the much larger frame gap; renderer and state-advance costs are separate targets.
