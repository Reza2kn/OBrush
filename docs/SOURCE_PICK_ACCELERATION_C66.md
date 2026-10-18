# Retained Source pick candidates (C66, OBrush 0.1.20)

The ordinary Source geometry brush previously performed a precise ray test against every triangle at each primary contact. C66 reuses the Source normal state's ordered tree, whose loose cells and original-face boxes are refreshed after accepted geometry edits, to collect a conservative set of current-face candidates. It sorts those candidates by original face ID before the existing f64 triangle intersection, preserving the original face/triangle ordering and equal-distance tie behavior. The ordinary document entry compares the retained state with the current mesh once and binds the result to that borrowed mesh; a missing or stale state uses the original linear picker. Whole-mesh validation, the public pick API, atomic stroke application and history checks remain in place. The mirrored second contact still uses the original linear scan.

This change is a CPU-side step toward the pinned [SculptGL inventory](SCULPTGL_PARITY.md). It is not a WebGPU backend, a complete brush parity result, or a real-time million-vertex fix. [The C66 evidence index](evidence/sculptgl-source-retained-pick-c66-0.1.20.json) binds the staged source, builds, tests and browser audits; failed runs remain in the local ignored artifact stage.

The staged core ran 2,349 workspace tests with no failures and one ignored test, strict native/WASM Clippy, the official browser build and an ad-hoc-signed native package. A 289-ray dense sphere comparison gave exact selected face, triangle and position results between indexed and full-scan preparation; separate tree tests cover a miss and a deformed face. The prepared-document tests cover stale channels, alpha, mirror, all Source geometry tools and whole-stroke undo/redo. These are independent OBrush fixtures, not matched SculptGL brush-response measurements.

The original 400 Hz sustained input protocol was replayed through normal owned Chrome instances. Each run retained complete public before/after/stable/undo/redo files, scalar observations and independent whole-model shape checks. The table reports one current trial per duration, with the 100K/60-second repeat shown separately; all failed checks are retained.

| Model and duration | Frame CPU P95 | Named local result | Failed checks |
| --- | ---: | --- | --- |
| 10K, 60 s | 5.1 ms | Pass | None |
| 10K, 120 s | 4.8 ms | Pass | None |
| 100K, 60 s | 12.2 ms | Fail | One input send was 106.9 ms late against the 100 ms maximum |
| 100K, 60 s repeat | 12.5 ms | Pass | None |
| 100K, 120 s | 12.4 ms | Pass | None |
| 999,002 vertices, 60 s | 105.9 ms | Fail | Frame CPU P95, independent RAF P95/maximum, release queue by 100 ms |
| 999,002 vertices, 120 s | 108.0 ms | Fail | The same four responsiveness gates |

The earlier delivered C58 million-vertex 60-second trial measured 114.2 ms frame CPU P95. C66's 105.9 ms is an observed difference between separate runs with different delivered/coalesced contact counts, not a controlled speedup ratio. Both remain well above the 16.667 ms frame CPU target. The 60/120-second million-vertex runs preserved finite geometry, original directed faces and attributes, saved bytes after release, and exact whole-project undo/redo. They do not establish global self-intersection prevention or broader mesh quality.

Two earlier C66 million-vertex attempts remain failed: an initial double state comparison measured 194.5 ms frame CPU P95 and lost a 1 Hz memory observation; a later run missed the normal Frame-all action, applied no dabs and correctly Undid the import. A further setup attempt timed out during CDP observation before the timed stroke. The final driver admits the expected 1M camera frame before recording and allows a longer CDP response wait without changing the 60/120-second input stream or acceptance thresholds. All outcomes and the one 100K pacing failure are preserved in the evidence index.

The remaining dominant work is broader than ray candidate pruning: whole-mesh validation/state comparison and original-face traversal still occur per contact; the mirrored pick remains linear; CPU geometry and WebGL2 rendering do not share GPU-resident positions. The [WebGPU investigation](WEBGPU_COMPUTE.md) is isolated from the shipped stroke/document/undo path. The full 317-ID SculptGL inventory plus five hosted controls, matched native/browser workflows and performance, and ZBrush 2026.2.1 comparison remain open.
