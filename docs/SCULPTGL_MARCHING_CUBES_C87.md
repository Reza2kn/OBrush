# SculptGL Marching Cubes comparison — C87

This is a paired, narrow comparison of the **separate** triangular voxel-remesh action in the pinned SculptGL browser build and OBrush's shared Rust core/browser. It is not a SculptGL or ZBrush parity claim.

SculptGL was run from the pinned local mirror (`sculptgl.js` SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`) through its visible second **Remesh** button under **Voxel remeshing (manifold tris)**. Both runs used resolution **16** and separately set **Relax topology off/on**. The source geometry is an independently authored closed sphere (266 vertices, 528 triangles). The rich OBJ SHA-256 is `4bfebeb8ac713741ee5b70dedcefd8d746e21f610ebc788783f1008cc9d375ed`; its plain counterpart removes `#MRGB`/`#MAT` rows and has SHA-256 `e953052a8dc4db15026d9ca684c69a36e6ef31b78dd383fb153d4efffba9b35a`. Geometry and face IDs are the same in both fixtures. Reference captures and exact outputs are listed in [the evidence manifest](evidence/sculptgl-marching-cubes-c87-0.1.20.json).

OBrush now has a dedicated **Remesh triangles** action and **Relax topology** checkbox. The geometry runs in `obrush-core`, with one grid-edge vertex shared across cells and bounded extraction. The standard 256-case triangulation lookup data comes from MIT-licensed SculptGL; its license is retained in `crates/obrush-core/SCULPTGL_MIT_NOTICE.txt` and copied into local web/macOS artifacts. OBrush's field sampling, extraction, work limits, attribute transfer, topology validation and history are Rust implementations. Relaxation uses a one-pass tangent Laplacian with area-weighted normals. The old tetrahedral remesher remains a separate path.

| Resolution-16 sphere | SculptGL | OBrush | Paired difference |
| --- | ---: | ---: | ---: |
| Relax off, vertices / triangles | 1,172 / 2,340 | 1,172 / 2,340 | All 2,340 oriented triangle sets match after a unique vertex map. |
| Relax off, signed world volume | 4.207933080 | 4.207933857 | +0.000000777 |
| Relax off, maximum / RMS nearest world position | — | — | `2.3144e-7` / `1.0251e-7` |
| Relax on, plain sphere signed world volume | 4.205061592 | 4.205062394 | +0.000000803 |
| Relax on, plain sphere maximum / RMS mapped world position | — | — | `2.8923e-7` / `1.2572e-7` |
| Relax on, rich sphere maximum / RMS mapped world position | — | — | **`0.029677` / `0.004007`**, failed matching response |

The rich-fixture failure is material: SculptGL reads `#MAT`'s third component as a per-vertex unmasked weight, transports it through voxelization and applies it to relaxation. OBrush's OBJ importer currently ignores `#MAT`, so its relaxation uses full weight. The rich reference has 7 fully protected source vertices and interpolated output weights below 1. Matching topology and plain-sphere geometry does not close this attribute/interchange gate.

The isolated OBrush browser exercised both visible settings on the rich OBJ with the same resolution. Each produced 1,172 vertices / 2,340 triangles, a closed single solid, exact undo/redo, a saved `.obrush` snapshot matching GPU buffers, and triangle OBJ export/import retaining all face IDs. Native-core and browser saved positions differed by at most `4.25e-8` on these two runs; face IDs were exact. The browser run used synthetic input in isolated Chrome, not a physical pen or native GUI. A separate independently authored torus retained a closed, single-component hole in a native core test; no SculptGL torus pairing has been run.

Directly comparing the browser's `.obrush` saves with the reference gives the same 2,340 oriented face matches. The plain-sphere relaxed maximum/RMS position differences are `2.8481e-7` / `1.2715e-7` world units; the rich-sphere differences are `0.029677` / `0.004007`. This bypasses the native-core result when checking the actual shipped WASM behavior.

The first cube-edge prototype used the older tetrahedral grid and produced 1,166 vertices / 2,328 triangles on this sphere. Sharing the Surface Nets voxel grid fixed the count. A face-loop fan prototype matched only 1,184 of 2,340 oriented reference triangles; the standard cube-case table fixed all of them. The original paired rich-fixture relaxation check remains failed above. Reference behavior for selected multi-mesh fusion, nontrivial holes/cavities, voxel attributes, UV policy, resolution beyond 16, arbitrary topology and latency remains unverified. The browser smoke test and native unit tests are not sustained performance benchmarks.
