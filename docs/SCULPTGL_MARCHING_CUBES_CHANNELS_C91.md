# SculptGL Marching Cubes voxel channels — C91

The pinned SculptGL browser build and OBrush now consume the same bytes of the independently authored UV-bearing sphere OBJ from C90 (SHA-256 `4bfebeb8ac713741ee5b70dedcefd8d746e21f610ebc788783f1008cc9d375ed`). The source has 266 vertices, 528 triangles, 1,584 UV corners, varying RGB and `#MAT` values, and seven fully protected vertices. Both runs use Marching Cubes resolution 16, with Relax topology separately off and on. OBrush explicitly discards UVs for this reconstruction; undo restores them.

SculptGL's voxelizer assigns each closest voxel sample a source triangle's mean RGB, roughness, metallic and unmasked weight. Its Marching Cubes extractor averages the eight surrounding voxel values using inverse-distance weights. OBrush now carries all these channels through the same shared Rust voxel sampler; its previous nearest-surface barycentric transfer is retained for other remesh modes. The independent comparison pairs output positions within `1e-6` world units before comparing every mapped channel value.

| Same-byte source, resolution 16 | C90 browser save | C91 native core |
| --- | ---: | ---: |
| RGB values (3,516), maximum / RMS absolute difference | `0.188202` / `0.019707` (Relax off) | **0 / 0**, off and on |
| Roughness and metallic values (2,344), maximum / RMS | `0.320344` / `0.032548` (Relax off) | **0 / 0**, off and on |
| Unmasked weights (1,172), maximum / RMS | `1.35e-8` / `2.17e-9` (save rounding) | **0 / 0**, off and on |
| Mapped oriented triangles | 2,340 / 2,340 | 2,340 / 2,340 |
| Relax-on maximum / RMS world-position difference | `3.2758e-7` / `1.2668e-7` | `3.3404e-7` / `1.2535e-7` |

In the isolated Chrome run, browser-save maximum differences are `2.98e-8` for RGB, `2.98e-8` for roughness/metallic, and `1.35e-8` for unmasked weight, for both Relax settings. The browser harness also checked default UV-loss rejection, exact undo/redo, OBJ export/import, closed topology and actual visible WebGL2 buffers against each save. The served WASM SHA-256 is `885e725a53b7f0739f471c92d2aa465c0cb3e3f92a29c8a4c0c3d0e35d7bec64`; the active local server on port 8792 served those same bytes.

The exact native attribute values and nearly equal positions are a **one-fixture comparison**, not a general remeshing result. Full build and test receipts, reference snapshots and both numeric comparisons are recorded in [the C91 evidence record](evidence/sculptgl-marching-cubes-channels-c91-0.1.20.json). Other shapes/resolutions, quad UV import order, selected-mesh fusion, UV-preserving remesh, sustained responsiveness and full SculptGL/ZBrush parity remain open.
