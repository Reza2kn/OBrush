# Surface Nets quad persistence (C85)

OBRush's Smooth Surface Nets and Block voxel remesh now retain each generated
four-corner face in the shared core's `SurfaceTopology` alongside its canonical
two render triangles. The original face order survives versioned `.obrush`
save/load and OBJ export/import. A UV-free remesh result can be remeshed again;
a source with corner UV metadata still requires explicit detachment before a
topology-changing remesh. The existing triangle voxel remesher remains a
separate choice.

The pinned hosted SculptGL bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The paired inputs are independently authored triangulated cube and sphere
fixtures. Face IDs were mapped by their unique Surface Nets cell positions;
quad winding was compared under cyclic rotation, never reversal. The complete
oriented quad sets match:

| Fixture | Resolution | Reference / OBrush quads | Unmatched oriented quads | Max / RMS world-position error |
| --- | ---: | ---: | ---: | ---: |
| Cube | 64 | 24,576 / 24,576 | 0 / 0 | `1.446e-7` / `4.927e-8` |
| Sphere | 32 | 4,744 / 4,744 | 0 / 0 | `1.236e-7` / `6.254e-8` |

The numerical tolerance was `2e-7` world units, retained from C84. Both
paired runs still have zero missing cell IDs and zero unmatched oriented
render triangles. The owned browser exercised the visible remesh choice,
project save, actual OBJ export, fresh OBJ import, exact history and visible
GPU buffers for both fixtures. The OBJ exporter required its visible
attribute-loss choice because these source fixtures carry material channels
that OBJ does not preserve. A Block sphere browser regression passed the same
quad, interchange and history checks. The reference comparison output and
browser results are recorded in the [C85 evidence](evidence/sculptgl-surface-nets-quads-c85-0.1.20.json).

On the final source, `cargo fmt --all -- --check`, 2,373 workspace tests,
strict native and WASM Clippy, WASM compilation, the official web build and
the signed local macOS package passed. The active local server at port 8792
served the same WASM SHA-256 as the final browser runs:
`e64dadb0f902f0bf37f1f7251e753810e6efd74e1c546f6e0844660cd3e072ed`.

The first browser export attempt failed because the test omitted that
attribute-loss choice; its failed result is retained. A broad test run also
failed one assertion for the old remesh error wording; the assertion was
updated to the explicit UV-metadata error and the suite rerun.

This covers persistent quad topology on two closed fixtures. It does not
verify UV/material transfer, arbitrary and open shapes, selected multi-mesh
fusion, Marching Cubes, sculpting fidelity after remesh, sustained browser
responsiveness or full SculptGL/ZBrush parity. Adaptive topology refinement
still requires explicit detachment of retained source face metadata.
