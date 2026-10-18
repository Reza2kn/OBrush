# SculptGL Marching Cubes source-order comparison — C89

This pairs **the same bytes** of an independently authored, closed OBJ in OBrush and the pinned SculptGL browser build. The 266-vertex/528-triangle source has nonuniform RGB and `#MAT` values, including seven fully protected vertices, and **no UV channel**. Resolution is 16 with Relax topology separately off and on. The pinned SculptGL bundle SHA-256 is `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.

C88 compared OBrush's no-UV OBJ to a reference OBJ with per-corner UVs. SculptGL optimized their face streams in different orders during import, so the 11 remaining unmasked-weight differences were an import-order comparison issue. A fresh no-UV reference run exposes the appropriate matched source order. OBrush now computes that triangle order locally for Marching Cubes voxel mask sampling, without reordering the document or changing its source faces. The regression check maps all 528 optimized reference face IDs back to the authored OBJ and checks their order signature.

| Paired no-UV OBJ, resolution 16 | OBrush versus pinned SculptGL |
| --- | ---: |
| Relax off/on vertices and oriented triangles | 1,172 vertices and all 2,340 mapped oriented triangles match |
| Native-core per-vertex unmasked weights, off/on | **all 1,172 match exactly** after position mapping |
| Browser-save unmasked weights, off/on | all within `1.35e-8` after save serialization |
| Relax off maximum / RMS world-position difference | `2.3144e-7` / `1.0251e-7` |
| Relax on maximum / RMS mapped world-position difference | **`3.3405e-7` / `1.2518e-7`** |
| Before local face ordering, Relax-on maximum difference against this no-UV reference | `0.00287264` |
| Browser-save Relax-on maximum / RMS mapped world-position difference | `3.2758e-7` / `1.2641e-7` |

The result is a narrow numerical match on this one fixture. The separately captured reference-before mesh and the remesh outputs, the authored OBJ, and the order diagnostic are bound by [the C89 evidence record](evidence/sculptgl-marching-cubes-order-c89-0.1.20.json). An isolated browser run passed the visible controls, closedness, exact undo/redo, explicit-loss OBJ export/import, and agreement between the saved project and actual GPU buffers. Native-core/browser positions differ by at most `4.25e-8` at matching vertex IDs; their saved masks differ by at most `1.35e-8`. Input was synthetic in isolated Chrome; physical pen and native GUI behavior were not checked.

The current voxel remesher still rejects retained UV metadata, and its voxel material-color transfer, selected multi-mesh fusion, arbitrary shapes/resolutions and sustained latency are not verified. The normal browser app continues to use WASM CPU geometry and a WebGL2 renderer. This result is not complete SculptGL or ZBrush parity.
