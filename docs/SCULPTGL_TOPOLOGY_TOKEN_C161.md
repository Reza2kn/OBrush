# C161: reuse exact Source topology in the normal viewport

This scoped browser follow-up keeps the current shared Rust/WASM Source brush and WebGL2 viewport. A Source normal refresh retains the exact immutable face and triangle-row allocations. The renderer now uses that retained allocation identity, together with unchanged vertex and triangle counts, to skip reuploading the index buffer for that one path. A new independently built topology, a changed count, or a path without this Source certificate still uploads its indices. The check does not change geometry, file IO, history, material upload, or packed vertex upload.

The independently authored fixture has 999,002 vertices and 1,998,000 triangles (SGL SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`). The frozen 1M/60-second protocol is SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`: 400 requested mouse moves per second, Source Ordinary Brush radius 64 pixels and intensity 0.25, with no clay, accumulate, alpha, symmetry, or fold guard. The pinned SculptGL bundle is SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`. Its separate C156 browser trace observed 21.5 ms RAF P95 and 2.11 ms mean dab; this is context, not a matched end-to-end speed ratio.

| Normal OBrush browser observation | C160 | C161 |
| --- | ---: | ---: |
| WASM SHA-256 | `12fe2bdf73bf42309821220c237e9db582eece1903beb1b3a660effbcda6f220` | `9e873d5ab072acb9bf74af7d8fe1bc8b4ce7fdbca99572f6148f657fec6fce18` |
| Delivered moves / applied dabs | 851 / 778 | 864 / 789 |
| Full 23,976,000-byte triangle-index uploads | 778 | 1 |
| Full 7,992,016-byte material uploads | 778 | 790 |
| Full 39,960,080-byte packed vertex uploads | 778 | 790 |
| Mean queued Source geometry dab | 25.043 ms | 25.076 ms |
| RAF P95 | 79.5 ms | 78.1 ms |
| Release acknowledgement | 265.4 ms | 278.4 ms |
| Independent whole-project/history audit | pass | pass |

The full-index upload reduction is directly observed in the scalar WebGL recording; one initial index upload remains. The runs received different delivered events and changed different vertices, so the similar dab and frame timings do not establish a causal speed gain or regression. C161 changed 40,803 original vertices. Its complete independent saved-mesh/history audit has zero failed data checks. All four frozen responsiveness gates still fail: RAF P95 ≤33.34 ms, RAF maximum ≤250 ms, app CPU P95 ≤16.667 ms, and release idle by 100 ms. The C161 RAF maximum is 413.9 ms, app CPU P95 is 77.6 ms, and release acknowledgement is 278.4 ms.

The final-browser result and independent audit are in `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-535fc288-9ec3-44a1-b22b-1578d85e93b9/`; the scalar recorder contains the upload counts. The older C72 mask/extraction harness was also run against C161 and exited nonzero on its mask-only no-index-upload gate. That gate is outside this Source-only change; the failed record is preserved at `.artifacts/mask-filter-browser/c161-topology-fallback-1654ef85-3df6-4808-a50e-4dced2fe47dc/result.json` and `.artifacts/gpu-pointer-c146/c161-mask-extract-browser.log`. Two preceding analytic mask cases passed. This auxiliary failure is not counted as a passing browser check.

The current normal browser still deforms Source brushes in WASM and draws with WebGL2. Avoiding repeated index transfer alone leaves full vertex/material repacking and transfer, CPU deformation, retained-state work, and release latency. The separate WebGPU prototype has not replaced this path. This result does not promote a SculptGL or ZBrush feature-parity claim.

Validation: `cargo fmt --all -- --check`, workspace tests (2,480 passed, zero failed, one ignored across 101 groups), strict native and WASM-target Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, `git diff --check`, the complete final-build browser trace, and the independent whole-project/history audit pass. The first focused unit test failed because its fixture used the protected-mask convention; the fixture was corrected to the intended unprotected Source convention and the repeated focused test passed. Both logs are retained under `.artifacts/gpu-pointer-c146/`.
