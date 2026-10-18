# Static-buffer reuse trial (C72, rejected)

The delivered browser renderer repacks and uploads the complete vertex, material and triangle streams when a mesh revision changes. A retained C69 million-vertex/60-second trace showed 612 repeated 23,976,000-byte triangle-index uploads and 612 repeated 7,992,016-byte material uploads during a fixed-topology stroke. Those calls submitted about 14.65 GB of index and 4.88 GB of material data. They suggested a narrow WebGL2 optimization before attempting a GPU-resident WebGPU path.

Two local candidates retained exact CPU copies of triangle/material streams and skipped their WebGL uploads when the bytes matched. The second candidate compared triangle bytes directly. A pure test rejected same-count topology and one-bit material changes. In the ordinary browser route, two Mask strokes changed the vertex buffer while the active index-buffer upload count stayed fixed; the full ten-case mask/extraction workflow suite passed with saved-project/GPU and Undo/Redo checks. These correctness results do not establish a speed improvement.

The unchanged 999,002-vertex / 1,998,000-triangle, 60-second, 400-request/s cohort was run on Chrome 155.0.8059.80. The table reports the active application-frame CPU P95 derived from each retained scalar recording. The delivered motion count differs across runs, so the timings cannot be treated as a controlled causal speed ratio.

| Browser build | Delivered moves | Active CPU-frame P95 | Full index uploads | Full material uploads |
| --- | ---: | ---: | ---: | ---: |
| C72 exact witness | 631 | 108.0 ms | 0 | 0 |
| C72 byte comparison | 636 | 111.3 ms | 0 | 0 |
| Restored C71 build | 684 | 97.3 ms | 631 | 631 |

All three timed cohorts completed the normal public Save, stable-after-release Save and Undo/Redo file route, with no browser exceptions and clean owned process/server closure. Each runtime record explicitly defers the independent complete mesh/shape audit; none qualifies the requested million-vertex responsiveness or SculptGL parity gate. The two C72 candidates removed the transfers but remained far above the 16.667 ms CPU-frame gate and were slower in these runs. Their complete recordings and failed setup attempts are retained in the local artifact directories indexed by [the C72 evidence](evidence/sculptgl-static-upload-trial-c72-0.1.20.json).

The first two attempts to replay the restored baseline failed **before timing** because the old driver treated a visible camera button ending at screen y=822 as offscreen under its hard-coded y<=816 rule. The failed screenshots and errors remain. The helper was changed to use the observed panel bottom (y<=823) only for this pre-run hit test; fixture, settings, 60-second motion schedule and gates were unchanged. This produced the completed restored-baseline run. The final source and served WASM returned byte-for-byte to the validated C71 build (`d1dfe41a3c2582616bc8181e3fcae6747bdf97628ca78f9b6ce6a940df53dbf1`).

**Decision:** the exact-copy transfer-skip path is removed from the product. Whole-stream comparison adds CPU work on every revision and does not address the full vertex packing, Source validation/normal updates, document publication or input backlog. Further performance work should establish trusted edit-region/stream ownership and an integrated GPU-resident compute/draw pipeline, while keeping the shared GPU-independent Rust core for validation, history, interchange and fallback. No WebGPU product speedup is claimed from the separate prototype.
