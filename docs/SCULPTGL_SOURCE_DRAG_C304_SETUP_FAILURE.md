# C304 Source Drag visible-symmetry candidate: setup failure

The C304 trial added the previously absent ordinary Source Drag X-symmetry
checkbox and retained the isolated shared-core mirrored Drag candidate. The
full workspace test suite, `cargo fmt --all -- --check`, strict native and
WASM Clippy, `cargo check -p obrush-core --target wasm32-unknown-unknown`,
`scripts/build-web.sh`, and `scripts/package-macos.sh` all passed. The macOS
bundle was ad-hoc signed and verified. These checks establish compilation and
local test correctness only; the frozen paired browser gate did **not** pass.

The frozen C304 normal-UI Source off/on and OBrush off captures completed.
The OBrush symmetry-on capture failed before fixture import or pointer stroke:
`Normal OBrush logical viewport1024x808 not reached`. The raw viewport trace
shows the inherited normal driver reached exactly 1024×768; then a fixed +40
window-height adjustment produced a stable 1025×808 logical viewport due to
one-pixel inspector-width rounding. The harness's eight-frame exact-size
barrier correctly rejected it. No geometry, GPU, history or matrix score is
claimed from this run. The `source_geometry.mirror` control was not yet
reached, so C304 also does not prove the UI repair in a browser.

| C304 raw receipt | SHA-256 |
| --- | --- |
| Source off `off-source-2f70ecf7-f941-48ca-b7ff-c19baecf8054/result.json` | `58db61b75d7db95a62f106d8ed1e67c241cea1aefc2df28dc5d2ce16f6d2401b` |
| OBrush off `off-root-19a1ae93-12a1-431b-b4a0-5d549f775ac7/result.json` | `25df48a488a9f59add050e1996bb5f3a052f574e3cce93c5ba3635ecd83a0b42` |
| Source on `on-source-50e09531-0c34-4a56-9640-543a91cdbf23/result.json` | `595197f19e08b769e21758ec5caa3a4468af15f11d9c2b01511cb6093991463f` |
| OBrush on `on-root-e5561c4c-90d7-4638-95a2-f16e95146240/result.json` | `44a64b07953f5ca6857910e38e7a8f476cfcf2e2afbe0f10572a9e9fccb2f4e4` |

The active portable OBrush WASM/JS SHA-256 were
`7bda17d3c4f92ce0f3d5a29261f60d57de459daf0dcc2447b1917aa47c618bce` /
`bb07c4da377b12963fb12c77444d95a37df00560aa33291e5ab1d10ee0aae830`.
All owned browser/server/build processes were closed and the shared lane
released after the failure. C300/C301/C303 setup failures remain separate;
none is converted into a C304 pass. A later correction must preregister its
bounded viewport adjustment under a new ID while preserving the fixture,
pointer trace, settings and all numerical gates. Product candidate remains
isolated and unpromoted.
