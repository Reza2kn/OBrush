# C157: million-vertex Source Brush phase timing

This diagnostic keeps the pinned C156 1M/60-second Source Ordinary Brush protocol. Its independently authored SGL has 999,002 vertices and 1,998,000 triangles (SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`). The frozen protocol SHA-256 is `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`: 400 requested mouse moves per second for 60 seconds, 64-pixel radius, 0.25 intensity, no clay, accumulate, alpha, symmetry or fold prevention. The pinned SculptGL bundle is `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`; its C156 run delivered 19,573 moves and 1,139 dabs, with 21.5 ms RAF P95 and 2.11 ms mean dab. These are local-browser observations, not an exhaustive performance comparison.

The core now records six read-only phases inside a successful Source geometry pass, using `web_time::Instant` so the clock works in native and WASM builds. The normal controlled stroke aggregates those phases across its active passes. The `SourceGeometryReport` timing field is omitted from serialized reports; geometry, project and history formats are unchanged. For a **single** active pass, the controlled report reuses that pass's exact changed-row count instead of scanning every mesh position again. Two-pass/mirrored operations still compare the complete before/after mesh, because overlap or cancellation can change their union count.

| 1M/60s OBrush browser observation | Instrumented before count shortcut | After count shortcut |
| --- | ---: | ---: |
| Browser WASM SHA-256 | `57f49ec8f5dd4984b28fd91417760644788ed2f5b4f8c47629aa9ab959297c08` | `de056f1e40def579ffb76f734d9c4b9d2286bef5d38eafd7535b9694b4daf9df` |
| Delivered moves / applied Source dabs | 727 / 655 | 732 / 675 |
| RAF P95 | 94.0 ms | 91.0 ms |
| Mean queued Source dab | 38.369 ms | 36.710 ms |
| Mean preparation / deformation | 15.921 / 22.443 ms | 15.741 / 20.963 ms |
| Mean contact preparation / retained match | 10.446 / 5.473 ms | 10.339 / 5.399 ms |
| Mean state advance / normal refresh | 7.355 / 2.185 ms | 7.114 / 2.106 ms |
| Mean old-position copy / candidate and guard / publish | 0.345 / 0.168 / 0.615 ms | 0.335 / 0.162 / 0.597 ms |
| Release acknowledgement | 281.8 ms | 300.1 ms |
| Independent full output/Undo/Redo audit | pass | pass |

The phase rows are **subsets** of the deformation and preparation timers, not a complete partition. Other controlled-path work remains. The two runs received different mouse moves and produced different valid changed-vertex counts, so their timing differences do not prove a causal end-to-end speed ratio or matched brush response. Both still fail the pinned four responsiveness gates by a large margin; the ordinary browser brush remains CPU/WASM geometry with WebGL2 drawing. The source reference and OBrush also received different delivered event sequences, so this does not establish SculptGL parity.

The first instrumentation build failed its browser run: `std::time::Instant` panicked with “time not implemented on this platform.” Its failed result is preserved at `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-17fc0357-9158-4721-b250-f6f31d8859cd/result.json`. The second build completed with zero phase samples because the controlled report had not forwarded the per-pass timers; its valid data/history and independent audit remain at `.../SP-LONG00-1m-60s-root-dd1498ee-3dc8-42e4-b844-7fd0e232f6a6/`. The final before/after runs and audits are under `.../SP-LONG00-1m-60s-root-46e676c2-5e0b-42ab-a723-65e1f24838d0/` and `.../SP-LONG00-1m-60s-root-911bfc47-51ab-4ce2-b333-891650513c68/`; the three-dot prefix here is `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual`. All are ignored local evidence, not public fixtures.

Validation after the code change: `cargo fmt --all -- --check`, workspace tests (2,478 passed, zero failed, one ignored across 101 groups), strict native and WASM-target Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, and the final frozen browser plus independent audit pass. The remaining work includes retained-state and contact preparation, the rest of controlled deformation, resident drawing and document release. WebGPU may accelerate appropriate resident compute, but this evidence does not support a renderer-only speed claim.
