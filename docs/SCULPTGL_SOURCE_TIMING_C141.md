# C141: full-size Source brush timing matrix

The C140 queue timer was run through the frozen 10K, 100K and 999,002-vertex Source Brush protocol for both 60 and 120 seconds. Each owned Chrome run scheduled 400 motion requests per second with a 64 CSS-pixel radius, 0.25 intensity, fixed topology, and no alpha, mirror, clay, accumulation or fold guard. The official WebGL2/WASM build hash was `54c6db05210d9f268c0071230c649a7d762b902d39500af02bcb8046c65c3cd3`. The original fixture and protocol hashes, complete runtime files, independent whole-model audits, and all failed gates are in [the C141 evidence](evidence/sculptgl-source-timing-c141.json).

| Vertices | Trace | Delivered pointer moves | Applied Source dabs | Mean queued Source dab | Frame CPU P95 | Independent RAF P95 / max | Local result |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| ~10K | 60 s | 20,199 | 1,142 | 0.91 ms | 5.1 ms | 18.5 / 24.0 ms | All named gates pass |
| ~10K | 120 s | 40,175 | 2,292 | 0.92 ms | 5.3 ms | 18.7 / 25.8 ms | All named gates pass |
| ~100K | 60 s | 16,049 | 1,147 | 4.84 ms | 12.1 ms | 24.6 / 52.4 ms | All named gates pass |
| ~100K | 120 s | 31,013 | 2,288 | 4.84 ms | 12.1 ms | 24.6 / 55.3 ms | All named gates pass |
| 999,002 | 60 s | 718 | 658 | 40.81 ms | 92.5 ms | 93.0 / 412.9 ms | Four responsiveness gates fail |
| 999,002 | 120 s | 1,455 | 1,327 | 40.05 ms | 92.1 ms | 92.6 / 422.1 ms | Same four gates fail |

For both million-vertex runs, the failed gates are independent RAF P95 ≤33.34 ms, independent RAF maximum ≤250 ms, summed app-frame CPU P95 ≤16.667 ms, and observed release queue empty within 100 ms. The first recorded empty queue occurs later; the test does not waive this because it eventually empties. All six runs completed actual public Save, stable-after-release, exact whole-project Undo/Redo, memory sampling and independent whole-model checks. They had useful finite deformation, no new degenerate or reversed original faces, and bounded edge stretch. The 10K and 100K results are **local workload passes**, not SculptGL feature or performance parity.

The million-vertex queued Source application totals were 26.85 seconds over the 60-second trace and 53.15 seconds over the 120-second trace. This measures real product work on the WASM main thread. It excludes input delivery, viewport packing/upload/draw, GPU completion and input-to-photon latency, so it is not a decomposition of whole frame time. Different sizes delivered different coalesced contacts and dab counts; these figures identify a strong size-dependent bottleneck but do not establish a controlled cross-size speed ratio. The existing WebGPU point program uses different brush mathematics and remains separate. A production GPU path must move the validated pick/deform/normal/render state together while preserving the shared-core history and fallback contract, then rerun this matrix and a pinned SculptGL comparison. Physical pen, native GUI, matched source brush response, arbitrary meshes and ZBrush 2026.2.1 remain open.
