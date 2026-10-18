# C160: carry exact-domain proof through prepared brush apply

This scoped continuation uses the same independently authored 999,002-vertex, 1,998,000-triangle SGL fixture (SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`) and frozen 1M/60-second browser protocol (SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`). It requests 400 mouse moves per second with Source Ordinary Brush at 64-pixel radius and 0.25 intensity, without clay, accumulate, alpha, symmetry or fold prevention. The pinned SculptGL bundle remains `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`; its separate C156 browser trace observed 21.5 ms RAF P95 and 2.11 ms mean dab.

C159 verified the retained geometry and captured nongeometry domain during contact preparation, but the same prepared call scanned the domain again before deformation. The prepared value now carries a private exact-domain admission bit until its consuming `apply` call. It owns the exclusive `Document` and stroke borrow throughout, so no public mesh mutation can occur between the check and apply. Only that private prepared path skips the duplicate domain comparison. The direct controlled-dab API and cold/mismatched prepared paths keep their independent capture validation. All geometry, file and history behavior remains in the shared Rust core.

| Pinned OBrush browser observation | C159 verified contact | C160 prepared seal |
| --- | ---: | ---: |
| Browser WASM SHA-256 | `e0c5aac5d8ca8e3a248dc000393f4b8b06c4cbb0b25f595d14c6df25d73ba3cb` | `12fe2bdf73bf42309821220c237e9db582eece1903beb1b3a660effbcda6f220` |
| Delivered moves / applied dabs | 827 / 748 | 851 / 778 |
| Mean preparation / deformation | 7.559 / 19.959 ms | 7.461 / 17.577 ms |
| Mean queued Source dab | 27.523 ms | 25.043 ms |
| RAF P95 | 86.7 ms | 79.5 ms |
| Release acknowledgement | 273.6 ms | 265.4 ms |
| Independent whole-project geometry/history audit | pass | pass |

The runs received different delivered pointer events and changed different vertex sets. These numbers are observations, **not** a controlled causal speed ratio or matched brush-response error. The final-build C160 result is `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-e5dede09-9cd8-48de-9f73-aa525c24edb3/result.json`; the independent audit has zero failed data checks and 40,795 changed vertices. An earlier accepted pre-lint-artifact run remains at `.../SP-LONG00-1m-60s-root-7c732acc-d0a4-4f3a-afac-65b77fc9602e/`, with 25.111 ms mean dab and 79.4 ms RAF P95. All four pinned responsiveness gates still fail: RAF P95 ≤33.34 ms, RAF maximum ≤250 ms, app CPU P95 ≤16.667 ms and release idle by 100 ms. Normal browser strokes remain CPU/WASM geometry with WebGL2 drawing. This does not establish SculptGL or ZBrush parity.

Validation: `cargo fmt --all -- --check`, `cargo test --workspace --all-targets` (2,479 passed, zero failed, one ignored across 101 groups), strict native and WASM-target Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, `git diff --check`, and the final-build browser/output/history audit pass. The first native strict-Clippy run failed because the private helper gained an eighth explicit admission argument; a scoped lint annotation documents that captured-input signature, and the repeated strict run passes. The post-annotation browser rebuild changed the WASM hash, so the complete trace and audit were repeated on the final artifact; all records remain in ignored `.artifacts`. The core's remaining measured work includes about 7.3 ms retained match and 7.1 ms state advancement per dab, followed by viewport/release cost. The full pinned inventory, native/browser reference comparisons and million-vertex responsiveness remain open.
