# SculptGL Marching Cubes UV-source comparison — C90

This pairs the **same bytes** of an independently authored OBJ with per-corner UVs in OBrush and the pinned SculptGL browser build (bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`). The source has 266 vertices, 528 triangles, 1,584 UV corners, nonuniform RGB and `#MAT` values, and seven fully protected vertices. Resolution is 16 with Relax topology separately off and on.

OBRush now permits a UV-bearing Marching Cubes remesh only after **Discard UVs during remesh (Undo restores)** is enabled. The default attempt rejects without changing the document; an accepted remesh reports discarded UV coordinates/faces and removes the source UV channel. Undo restores the UV-bearing source exactly. The shared Rust core computes SculptGL's imported UV-seam face order for this triangular fixture without reordering the document's authored faces.

| Same-byte UV-bearing OBJ | OBrush versus pinned SculptGL |
| --- | ---: |
| Relax off/on vertices and mapped oriented triangles | 1,172 vertices; all 2,340 triangles match |
| Native-core per-vertex unmasked weights, off/on | All 1,172 match exactly after position mapping |
| Browser-save unmasked weights, off/on | Maximum absolute difference `1.35e-8` after save serialization |
| Native Relax-off maximum / RMS world-position difference | `2.3144e-7` / `1.0251e-7` |
| Native Relax-on maximum / RMS mapped world-position difference | `3.3404e-7` / `1.2535e-7` |
| Browser Relax-on maximum / RMS mapped world-position difference | `3.2758e-7` / `1.2669e-7` |

The isolated browser check exercised the visible explicit-loss control, rejection, both remesh settings, closed output, exact undo/redo, OBJ export/import and agreement between saved mesh and actual WebGL2 buffers. The rejected attempt and subsequent undo restore the same saved-project SHA-256 as before (`f1ba0d9dc15fecc060cae73ad0e2d71458694135b853028f4d7814cb356a2aae`). The browser run had no recorded console/runtime exceptions. [The C90 evidence record](evidence/sculptgl-marching-cubes-uv-c90-0.1.20.json) binds the authored fixture, reference and local run hashes, parameters, checks, numerical comparison and retained failed first attempts.

This is a narrow match for one closed triangular sphere at one resolution. UV preservation through remeshing remains unsupported by design; accepting the operation discards UVs. Quad/unmatched imported topology, voxel RGB/roughness/metallic transfer, selected-mesh fusion, other shapes/resolutions and sustained performance remain open. The browser still runs shared Rust geometry in WASM and draws with WebGL2. This does not establish full SculptGL or ZBrush parity.
