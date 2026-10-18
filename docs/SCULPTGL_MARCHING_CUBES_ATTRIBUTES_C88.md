# SculptGL Marching Cubes material-weight follow-up — C88

This continues the paired C87 triangular voxel-remesh comparison on the independently authored 266-vertex, 528-triangle sphere. It addresses the recorded rich-OBJ failure; it does not establish the full SculptGL remeshing family or ZBrush parity. The pinned SculptGL bundle remains SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`. The same resolution **16** and **Relax topology off/on** settings were used.

OBrush now parses SculptGL `#MAT` OBJ records as two material channels and a third, unmasked-weight channel, with malformed/count/vertex-budget rejection. The shared Rust Marching Cubes path samples source-triangle unmasked weights at covered voxel nodes, interpolates them onto grid-edge vertices, and uses them for the optional tangent relaxation. The implementation retains bounded work, cancellation and the same checked document replacement/history path. The export of these material channels to plain OBJ remains an explicit attribute-loss choice; a versioned `.obrush` save preserves them.

| Rich sphere, Relax topology | Pinned SculptGL vs OBrush |
| --- | ---: |
| Off: vertex/triangle counts | 1,172 / 2,340 on both |
| Off: oriented triangle sets after unique vertex mapping | all 2,340 match |
| Off: maximum / RMS nearest world position difference | `2.3144e-7` / `1.0251e-7` |
| On: maximum / RMS mapped world position difference | **`0.0006440559` / `0.0000243014`** |
| On: mapped points within `1e-6` world units | 1,161 / 1,172 |
| Per-vertex unmasked weights outside `1e-5` | 11 / 1,172, in both off/on outputs |
| Maximum / RMS unmasked-weight difference | `0.01258856` / `0.000463387` |
| Previous C87 On maximum / RMS difference | `0.029677` / `0.004007` |

The remaining 11 point differences track voxel weight selection near competing source triangles. Their exact material weights and the resulting relaxed positions are not reference-identical; this is a **partial improvement**, not a passed rich-attribute response gate. The paired result also covers only this sphere at one resolution, with one relaxation pass. Selected-object fusion, arbitrary source topology, colors/material channels after remesh, performance and native physical input remain unverified.

The rebuilt WASM (`d650bed68c439fd6e773c084de6f1bf668247e0d240a38d1591199c3c0e8f15b`) passed an isolated browser run through both visible remesh/Relax controls. Its saved rich-OBJ outputs retain 1,172 vertices and all 2,340 mapped oriented triangles; the Relax-on browser save differs from pinned SculptGL by a maximum `0.0006440634` and RMS `0.0000243015` world units. Native-core and browser positions differ by at most `4.25e-8` at matching vertex IDs, and masks by at most `1.35e-8`. The browser checked source `#MAT` import, nonconstant output masks, closedness, project save versus actual GPU buffers, undo/redo and explicit-loss OBJ export/import. It used synthetic input in isolated Chrome, not a physical pen or native GUI. The official validation, builds and artifact hashes are indexed in [the C88 evidence manifest](evidence/sculptgl-marching-cubes-attributes-c88-0.1.20.json).

The normal app still computes geometry in WASM on the CPU and renders through WebGL2; the separate WebGPU prototypes are not integrated into this browser path.
