# C144: retained-state and contact preparation timing

The normal WebGL2/WASM browser ran the frozen 999,002-vertex, 60-second Source Brush workload again with read-only substage counters in the shared Rust preparation path. The official app WASM SHA-256 was `a929f1912c3b5d9a26549d42657727205f00c2cc9f767d306acd9bfc363eff3b`. The original protocol SHA-256, fixture, complete runtime, whole-model audit, source hashes, parameters and all gate results are in [the evidence](evidence/sculptgl-source-preparation-c144.json). This run delivered 708 of 24,000 scheduled pointer moves and applied 658 Source dabs. Contact delivery differs from C142, so cross-run timing differences are not controlled speed measurements.

| Synchronous queued Source work | Mean per applied dab | Total over 658 dabs |
| --- | ---: | ---: |
| Retained mesh-state matching | 5.316 ms | 3,498.2 ms |
| Contact preparation, including mesh validation and ray work | 12.258 ms | 8,065.5 ms |
| Whole preparation envelope | 17.576 ms | 11,564.9 ms |
| Deformation envelope | 23.278 ms | 15,316.9 ms |
| Whole queued geometry envelope | 40.860 ms | 26,886.2 ms |

The whole preparation envelope exceeds its two nested stages by about 0.002 ms per dab due to surrounding work and timer granularity. Contact preparation is **not** a separate pick-only measurement; it includes validation and cannot support a claim about ray-picking cost by itself. The counters exclude browser input delivery, viewport upload/draw, GPU completion, and input-to-photon latency.

The independent saved-project/history/shape audit passed with 40,551 changed original vertices and no new degenerate or reversed original faces. The same four local responsiveness gates failed: independent RAF P95 **93.0 ms** exceeds 33.34 ms; RAF maximum exceeds 250 ms; summed app-frame CPU P95 **92.5 ms** exceeds 16.667 ms; and the observed release queue did not empty within 100 ms. These results do not establish SculptGL or ZBrush parity.

A deformation-only WebGPU kernel could remove at most the measured deformation envelope from this CPU path. The remaining preparation, document validation/state ownership, pointer-to-surface pick, normal refresh, history, and visible draw must be handled as one product integration. The existing resident WebGPU probe and one-use sparse studio handoff have not yet replaced normal pointer brushes. The unchanged full 10K/100K/1M × 60/120-second protocol and pinned SculptGL response comparison remain required after integration.
