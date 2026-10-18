# C145: retained-geometry contact validation

The ordinary Source contact path previously called a full `Mesh::validate()` even after its retained normal/pick state had matched every position and canonical triangle word against a state seeded from a fully validated mesh. That repeated the largest geometry scans on every dab. The shared Rust core now uses the retained match as an exclusive, ephemeral geometry proof during contact preparation. It still revalidates normals, masks, colors, material/UV channels, surface metadata, row counts and allocation limits; a stale or missing retained state falls back to complete mesh validation. No external caller can construct the proof. The native and WASM paths share this code.

A focused regression compares the public full-validation pick and retained pick on valid data, malformed normals, out-of-range masks/colors, malformed UVs, stale positions and invalid triangles. Each produces the same hit or rejection. An initial focused command mistakenly used `--exact` with an incomplete test path and selected zero tests; it was interrupted and rerun correctly with the named regression passing. The complete workspace suite, strict native/WASM Clippy, official web build, and signed macOS package also pass.

The frozen 999,002-vertex, 60-second Source browser cohort was rerun against the official app WASM SHA-256 `35689ce96cb53dd594a82bc4ac9e03f583c3f4e940f4eb2037ed0d30e0e9f248`. Its parameters and protocol match [C144](SCULPTGL_SOURCE_PREPARATION_C144.md). The browser delivered 747 pointer moves and applied 684 dabs, versus 708 and 658 in C144. This difference prevents a controlled speedup claim. The complete browser outputs, independent whole-model audits, code/build hashes and failed gates are in [the C145 evidence](evidence/sculptgl-retained-pick-c145.json).

| Synchronous queued Source work | C144 mean | C145 mean |
| --- | ---: | ---: |
| Retained-state match | 5.316 ms | 5.326 ms |
| Contact preparation including validation/pick | 12.258 ms | 8.803 ms |
| Whole preparation | 17.576 ms | 14.131 ms |
| Deformation | 23.278 ms | 23.215 ms |
| Whole queued geometry | 40.860 ms | 37.350 ms |

The C145 independent saved-project/history/shape audit passed with 40,630 changed original vertices, useful finite deformation, no new degenerate or reversed original faces, and bounded edge stretch. Frame CPU P95 was 89.2 ms and independent RAF P95 was 89.7 ms. **The same four responsiveness gates still failed:** independent RAF P95 ≤33.34 ms, RAF maximum ≤250 ms, summed app-frame CPU P95 ≤16.667 ms, and release queue empty by 100 ms. This is a local improvement indication under differing delivered contacts, not SculptGL performance parity.

The remaining measured cost is large enough that the production WebGPU path must own actual picking, deformation, normal refresh and visible drawing across a stroke, then reconcile a bounded result with the core document and Undo/Redo. The separate resident point probe and one-use studio handoff are still diagnostic; the normal browser remains WASM CPU geometry plus WebGL2. Rerun the full frozen matrix and pinned SculptGL comparison after GPU integration.
