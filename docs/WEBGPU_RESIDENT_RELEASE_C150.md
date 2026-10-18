# C150 resident release profiling and repeated-stroke normals

The opt-in browser WebGPU Inflate preview still returns sparse positions to
the shared Rust document for source identity, fold, scene, normal and history
checks. C149 removed repeated full-topology construction, but the second
million-vertex release remained about 360 ms. This report separates document
replacement stages and records two checked follow-ups. The normal studio
still uses WASM CPU sculpting and WebGL2 rendering.

All browser replays imported the independently authored
`independent-id-sphere-1m.sgl` fixture (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`),
999,002 vertices and 1,998,000 triangles. Each replay sent two three-contact
mouse strokes at the captured viewport center with the same 8×5 CSS-pixel
move, then two Undo and two Redo commands. These are local one-run timings,
not a confidence interval or a matched SculptGL speed ratio.

| Build / second stroke | Source match | Fold guard | Normal refresh | Scene check | Replacement | Release |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Staged profile, existing implementation | 146.3 ms | 8.9 ms | 95.5 ms | 37.5 ms | 174.0 ms | 343.5 ms |
| Direct-array fingerprint | 130.2 ms | 8.9 ms | 94.1 ms | 36.6 ms | 171.0 ms | 321.1 ms |
| Direct fingerprint + proved local normals | 122.7 ms | 8.8 ms | 11.5 ms | 37.7 ms | 91.6 ms | 253.3 ms |

The first staged profile separated its 174.5 ms replacement into 12.1 ms
mesh validation, 95.0 ms full normal recomputation, 37.5 ms scene checking,
and 4.8 ms history work; other metadata/copy overhead remained outside those
four timers. Direct-array hashing feeds the same canonical little-endian
stream into BLAKE3 without per-word packing on little-endian targets. A rich
fixture with every serialized channel asserts the exact prior 32-byte digest,
and independent mutations of each field still change it. Big-endian targets
keep the explicit little-endian word path. The browser's visible outputs and
history hashes were unchanged.

For successive resident strokes, the complete normal field can be refreshed
locally only after an exact witness comparison of prior positions, triangles
and normal words. A full rebuild remains the first-stroke and failed-witness
path. The witness is derived, excluded from `.obrush` and ordinary history
snapshots, and capped at 48 MiB; the million-vertex fixture retains
47,952,048 data bytes when available. Failed allocation disables this
optimization without rejecting a valid commit. The shared-core test compares
the cached second commit against a freshly loaded document that must do the
full normal rebuild; their complete project JSON matches. A public triangle
edit or even a normal sign-bit edit invalidates the witness and forces the
full path. The browser measured 1.3 ms to refresh the witness after each
successful commit; on its second stroke `sparse_normals=true` and all visible
Undo/Redo hashes matched the full-normal builds.

Across all three initial runs and the final-build replay, the before,
first-stroke and second-stroke viewport
SHA-256 values were respectively
`5398141e2042c211dcd70ad09f7796b918381a54c29269fe89fc43a3ddf7903b`,
`ae5446f55d34a3ac542c68812759df27ea12e933734e488ebb5265369e3b7ce7`,
and `bce3b015525556dbc8b406a130e3b1331093bd2bfefde3ef85cc62cd9282457d`.
Undo/Undo recovered first/before; Redo/Redo recovered first/second. The
final-build replay changed 5,769 then 5,573 vertices. Its second release took
255.1 ms, with 11.5 ms local normal refresh and 1.3 ms witness refresh. It still
exceeds the 100 ms target, and this two-stroke replay does not establish
sustained frame-time or source brush-response parity. Source matching, scene
validation, GPU picking, normal-studio WebGPU integration and the complete
SculptGL inventory remain open.

The separate final-build 10-second first-stroke replay (`c150-final-long-r0`)
delivered 224 contacts, applied 113 dabs, coalesced 111 contacts and changed
25,483 vertices. Its requestAnimationFrame P95 was 50.1 ms, cumulative wait
for pick readbacks was 10,293.4 ms (including earlier queued GPU work), and
mouse release took 549.0 ms, including a
444.1 ms readback/checked commit. The first-stroke normal witness was
correctly absent, so the full normal rebuild took 94.2 ms. The
viewport-before/after hashes
were `5398141e2042c211dcd70ad09f7796b918381a54c29269fe89fc43a3ddf7903b`
and `97591625966ab9ae0b2daa07c059990a838ce9c01f6601f554dc466f8a7b3a2e`;
Undo/Redo recovered them exactly. The trace has different delivered contacts
from C148, so it cannot support a controlled release-speed ratio. It still
fails the P95 ≤33.34 ms and release ≤100 ms responsiveness gates. Its result
record is `.artifacts/gpu-pointer-c146/c150-final-long-r0/result.json`
(SHA-256 `9c9a7fc7ab46f55aeddd78033a3e4a2ec3edd019584c7933c66e9b6495be021f`).
The earlier `c150-long-r0` trace remains preserved; it delivered a different
222 contacts and is not used as a controlled speed comparator.

The preserved browser result records are
`.artifacts/gpu-pointer-c146/c150-replace-profile-r0/result.json` (SHA-256
`f62b0b9125fb63fbdfa17bd8643680f33364115c5b526bc142955e1a73eaec00`),
`c150-direct-r0/result.json`
(`3b51fd83919853a19f689af5cb61047fab3acfc44c497c322892c02a051a62bc`),
and `c150-witness-r0/result.json`
(`eeea8464ca4eb85469023cf62338b546c1756a67e2a8ff192eb150108de10f84`).
The final-build two-stroke result is
`.artifacts/gpu-pointer-c146/c150-final-repeat-r0/result.json` (SHA-256
`7ba52c18c46d8570268d87030522315411337cc284d7431ef3ca05093e6be64a`).
The final browser build SHA-256 values are
`6d231576f06b2b60cf0621375886695d6ff41633da1479fe88b96a46a7d5c733`
for `obrush_app_bg.wasm`,
`0694439cfb8b1eeee386e394014d9c9bc3a522511cc4000fdc747576beb5e8db`
for `resident_bg.wasm`, and
`052b19db036d7fc32f377e683ad7c6b0a9c31ca77d9be2c344bb26c9ec03941d`
for `resident-pointer.js`.

Final-source checks passed: `cargo fmt --all -- --check`, 2,476 workspace
tests across 100 reported suites with zero failures, strict native all-target
and core/app/GPU WASM Clippy, `bash scripts/build-web.sh`, and
`bash scripts/package-macos.sh` with valid local signature. Both owned-browser
replays passed with no recorded JavaScript exception, console error or
external request. The external `block` dependency emitted Cargo's future-Rust
compatibility notice during native Clippy; it did not fail the strict check.
