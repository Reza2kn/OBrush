# C148 resident commit fingerprint and release profile

The opt-in WebGPU Inflate path already accepted a checked 999,002-vertex
browser stroke, but synchronous release took roughly 1.6 seconds. This report
uses the same independently authored `independent-id-sphere-1m.sgl` fixture
(SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
and the same three-contact CDP pointer trace to locate and reduce that cost.
The normal studio remains WASM CPU geometry with WebGL2; this is not a
SculptGL brush-response or responsiveness pass.

The first timing attempt (`profile-r0`) failed: `std::time::Instant` panicked
at runtime on the browser WASM target. Its browser result and console stack
are preserved. The measured path now uses the existing `web-time` dependency.
Five core stages are reported separately after a successful, one-use sparse
commit; timing does not authorize or bypass any validation.

| Browser build, same three contacts | Source match | Sparse decode | Candidate copy | Fold guard | Document replace | Total JS commit |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| JSON-formatted SHA-256 (`profile-r1`) | 1,294.7 ms | 0.1 ms | 2.9 ms | 126.7 ms | 161.2 ms | 1,593.1 ms |
| Canonical binary SHA-256 (`binary-hash-r0`) | 336.0 ms | 0.2 ms | 3.8 ms | 125.9 ms | 172.9 ms | 647.4 ms |
| Canonical binary portable BLAKE3 (`blake3-portable-r0`) | 141.3 ms | 0.1 ms | 2.9 ms | 126.5 ms | 172.9 ms | 451.8 ms |
| Canonical binary BLAKE3 WASM SIMD trial (`blake3-simd-r0`) | 87.9 ms | 0.1 ms | 2.8 ms | 126.7 ms | 175.7 ms | 400.1 ms |

All four accepted runs changed the same 5,769 vertices and produced identical
before/after viewport hashes:
`5398141e2042c211dcd70ad09f7796b918381a54c29269fe89fc43a3ddf7903b`
and `ae5446f55d34a3ac542c68812759df27ea12e933734e488ebb5265369e3b7ce7`.
In each run Undo returned to the before hash and Redo returned to the after
hash. These are one-run local timings, not a confidence interval or a
controlled SculptGL speed ratio. The portable BLAKE3 implementation is the
final configuration. The SIMD trial was removed because its crate feature
assumes SIMD on every WASM target, while its total-release improvement here
was about 52 ms.
The feature behavior follows the [official BLAKE3 crate documentation](https://docs.rs/blake3/latest/blake3/),
which says `wasm32_simd` assumes SIMD support on all WASM targets when enabled.

The internal stale-result key now hashes every serialized mesh channel in a
versioned, canonical little-endian binary stream: positions, normals,
triangles, masks and convention, colors, optional materials, original faces,
and optional corner UV coordinates and indices. It includes lengths and
presence tags. Derived adjacency is omitted, as it is from project
serialization. A core test mutates every channel and option individually and
requires a different digest; existing stale-source and Undo/Redo tests still
apply. This remains a cryptographic comparison, with no trust in the GPU's
returned vertex IDs or its visual canvas.

The final portable longer run (`blake3-long-r0`) delivered 225 contacts over
10.0 seconds, applied 113 dabs, coalesced 112 contacts, changed 25,483
vertices, and passed visible before/after/Undo/Redo recovery. Release took
561.8 ms, including 469.3 ms of sparse readback plus shared-core commit.
Source matching took 141.0 ms, the fold guard 134.5 ms, and document
replacement 173.7 ms. GPU pick await time was 10,268.6 ms across the run;
requestAnimationFrame P95 was 50.1 ms. The prior longer run delivered a
different number of contacts, so these numbers do not establish a controlled
speed ratio. They still fail the million-vertex responsiveness targets,
including RAF P95 at most 33.34 ms and release within 100 ms. The remaining
full-triangle GPU pick, fold guard and document replacement work are open.

The browser records and screenshots are preserved under
`.artifacts/gpu-pointer-c146/{profile-r0,profile-r1,binary-hash-r0,blake3-portable-r0,blake3-simd-r0,blake3-long-r0}`.

Final-source validation passed: `cargo fmt --check`; 2,476 workspace tests
across 80 reported suites with zero failures; strict native all-target and
core/app/GPU WASM Clippy; `scripts/build-web.sh`; `scripts/package-macos.sh`;
and the normal owned-browser three-contact and 10-second runs above. The
final web artifact SHA-256 values are
`e0ef1fbd208df2cb02d277cdefee2da50f72265020a352d5eda6a3e5fc36e912`
for `obrush_app_bg.wasm`,
`e11d81df199321e2fad1b531a28a0fe56f68b8cd99b310fd4373d61d982b34c0`
for `resident_bg.wasm`, and
`052b19db036d7fc32f377e683ad7c6b0a9c31ca77d9be2c344bb26c9ec03941d`
for `resident-pointer.js`. The macOS package passed local signature checks.
