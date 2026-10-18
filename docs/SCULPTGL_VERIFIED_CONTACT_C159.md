# C159: borrow-scoped exact-domain contact preparation

This follow-up uses the C156–C158 independently authored 999,002-vertex, 1,998,000-triangle SGL fixture (SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`) and frozen 1M/60-second browser protocol (SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`). It requests 400 mouse moves per second with Source Ordinary Brush, 64-pixel radius and 0.25 intensity, and no clay, accumulate, alpha, symmetry or fold prevention. The pinned SculptGL bundle remains `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`; its separate C156 run recorded 21.5 ms RAF P95 and 2.11 ms mean dab.

Normal-browser contact preparation previously verified retained positions/topology, then scanned the entire normal, mask, color, material and UV domain a second time inside picking. A warm Source stroke now pairs the exact retained-geometry match with its already captured, exact nongeometry witness during the **same exclusive Document borrow**. Only that combination permits a bounded length/storage check at picking instead of repeated per-row attribute validation. A cold or mismatched witness retains the existing full validation path; the later controlled-dab entry retains its independent stroke/domain checks. The caller cannot construct this fast-path proof from an arbitrary mesh or carry it across a mutable borrow.

| Pinned OBrush browser observation | C158 shared witness | C159 verified contact |
| --- | ---: | ---: |
| Browser WASM SHA-256 | `2af30e489017f2706ef1893b89be18bd53ccdde2a0bc3f6e0bbf0d8c22ee40db` | `e0c5aac5d8ca8e3a248dc000393f4b8b06c4cbb0b25f595d14c6df25d73ba3cb` |
| Delivered moves / applied dabs | 746 / 685 | 827 / 748 |
| Mean retained match / contact preparation | 5.367 / 10.347 ms | 7.438 / 0.119 ms |
| Mean total preparation / deformation | 15.716 / 19.593 ms | 7.559 / 19.959 ms |
| Mean queued Source dab | 35.316 ms | 27.523 ms |
| RAF P95 | 89.7 ms | 86.7 ms |
| Release acknowledgement | 269.5 ms | 273.6 ms |
| Independent whole-project geometry/history audit | pass | pass |

The two OBrush runs received different event sequences and applied different dab counts; their timing differences are observations, **not** a controlled causal speed ratio or matched brush-response error. The C159 browser result is `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-141f6735-9772-4513-a508-70b5b36ad7ea/result.json`. Its independent audit has zero failed data checks and 40,635 changed vertices, but all four pinned responsiveness gates remain false: RAF P95 ≤33.34 ms, RAF maximum ≤250 ms, app CPU P95 ≤16.667 ms, and release idle by 100 ms. The ordinary brush still uses WASM CPU geometry and WebGL2 drawing. Neither SculptGL nor ZBrush parity is established.

Validation: `cargo fmt --all -- --check`, `cargo test --workspace --all-targets` (2,479 passed, zero failed, one ignored across 101 groups), strict native and WASM-target Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, `git diff --check`, and the full browser/output/history audit pass. Prepared-dab tests still reject invalid or externally changed normals, masks, colors, UVs, geometry and mask convention with their expected errors. Remaining measured costs include about 7.4 ms retained geometry/domain matching, 7.2 ms state advancement, the rest of deformation, rendering and release. This local admission improvement does not replace the larger resident sculpting/picking/viewport work needed for responsive million-vertex strokes.
