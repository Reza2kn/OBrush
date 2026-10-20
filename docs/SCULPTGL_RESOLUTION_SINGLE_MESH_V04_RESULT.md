# V04 clean single-mesh resolution image gate

**The preregistered V04 scoped gate passed at DPR 1 and 2;
`SGL-RENDER-RESOLUTION` remains Open.** A fresh page in each app and ratio
held exactly one imported, visible 18,242-vertex sphere. Normal visible
controls turned the floor grid off in both apps; Source's topbar dropdowns
were verified hidden twice, 350 ms apart, before capture. Source Symmetry
and render ratio still used recorded non-visible GUI controller setup, so
this is not full ordinary-UI parity. The original [V01 image bounds](SCULPTGL_RESOLUTION_SINGLE_MESH_V01_PREREG.md)
were applied to each entire, unmasked CSS scene crop. The earlier
[V01](SCULPTGL_RESOLUTION_SINGLE_MESH_V01_RESULT.md),
[V02](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_RESULT.md) and
[V03](SCULPTGL_RESOLUTION_SINGLE_MESH_V03_RESULT.md) failures remain
independent retained results.

Both 1600×1000-CSS-pixel Chrome processes captured 24 normal-browser PNGs
each: before, 350-ms post-release and 500-ms-later stable at 0.5×, 1× and
2× render resolution in each app. The source scene crop was 1290×960 CSS
pixels and OBrush's was 1140×912. Backing dimensions and actual OBrush GL
scene texture allocations matched the recorded ratios within ±1 pixel,
with no GL or scene error. The trusted held +12 CSS pixel move changed
37 original Source vertices and 104 original OBrush vertices per ratio;
OBRush added 788 adaptive vertices. History gained exactly one step per
stroke. Within each app and DPR, all three ratios had identical changed
IDs, pre-contact project/geometry and complete post-stroke coordinates
(maximum model delta `0`). The selected mesh and one-object scene persisted.

The table reports the **worst of all before/after 0.5×/2× comparisons**
within each app and DPR. Bounds were mean ≤16/p99 ≤64/fraction >64 ≤2%
for 0.5× and mean ≤8/p99 ≤40/fraction >64 ≤0.5% for 2×. The silhouette
bounds were p95/max ≤4/10 CSS px at 0.5× and ≤2/6 at 1×, both versus
2×. Each 500-ms static pair had mean `0`, p99 `0` and fraction >32 `0`,
below its ≤2/8/0.1% bounds.

| DPR | App | Worst interior RGB mean | Worst RGB p99 | Worst fraction >64 | Worst silhouette p95 / max CSS px |
| ---: | --- | ---: | ---: | ---: | ---: |
| 2 | Source | `0.5753` | `1` | `0.1133%` | `1.414 / 2.236` |
| 2 | OBrush | `0.1541` | `1` | `0%` | `1 / 2` |
| 1 | Source | `0.6795` | `2` | `0.1571%` | `1.414 / 2.236` |
| 1 | OBrush | `0.1784` | `1` | `0%` | `1 / 2` |

The Source and OBrush shaders, brush units and topology differ, so these
within-app bounds are **not a cross-app image similarity score**. They
support only one guide-free dense sphere, one short mouse stroke, this
browser/GPU and these two DPR values. Fine jagged edges, overlapping or
long strokes, other meshes, pressure, native rendering, browser zoom and
the complete resolution UI family still need independent gates.

Pinned Source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
original bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
mirror wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
Official OBrush product revision
`ff81bbf7e837ebf5c50c7462cf7b7e421b732450`, portable WASM SHA-256
`d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`.
Independent OBJ SHA-256
`492dd0cfa56544bab0e816efa68a4b48b522ddaf5c70cbb2f95b6c92d167ec2e`.
Final V04 runner SHA-256
`b94f73fa2870ad550d45e39b4881abdaee33c4c72cfba6de22a9f1db953fadb8`;
pixel analyzer SHA-256
`3f24216c324e09ab79e7ddf6272605c13a9c6786ee0285d49565614d4f1d41e9`.
Static JS/Python self-checks and `node --check` passed. No product code,
Cargo build or macOS package changed in this documentation/harness lane.

| DPR | Local retained receipt | `result.json` SHA-256 | `pixel-analysis.json` SHA-256 |
| ---: | --- | --- | --- |
| 2 | `.artifacts/sculptgl-resolution-single-mesh-v04/25dc8294-60fb-4cd1-b159-60550174f18d/` | `7616a40d4e58b8d85c217b77177d14991d00794e5b362822717b903d88c94286` | `ac483a2eb74d4f189363d8179ef4334a228546fb922dd7431491eb45c644ed2c` |
| 1 | `.artifacts/sculptgl-resolution-single-mesh-v04/ee9ad006-00ef-4945-a866-1270a3e47b3a/` | `1aaa09482ebfd23f0000769ed5a0681f3bc45d6881463aa44026b5c3b48e38a1` | `b1d3a5a49e5852e152869197abf9130fb77a031a256e991734697b1a90a604af` |

Two earlier DPR-2 V04 runs stopped before screenshots on the frozen
camera-unchanged setup gate. Immediately after OBJ import, Source's camera
was still animating (`z=27.787692906449763` before the visible grid click,
`z=3.6208536231493653` afterward). The harness now waits for two camera
samples 350 ms apart with maximum delta ≤`1e-10` *before* the grid click;
this enforces the original camera-invariance requirement and does not alter
image thresholds. Preserve the failed receipts
`.artifacts/sculptgl-resolution-single-mesh-v04/9c6e8933-eb8b-4b84-9029-c614d74b6cb6/result.json`
(SHA-256 `b190eeb7e1e8c9225918a6a901d8340a52b55fc6c26b2e90b015f6947d2843d5`)
and
`.artifacts/sculptgl-resolution-single-mesh-v04/906ce73a-82c3-4f0d-a1b9-100e87c01ec5/result.json`
(SHA-256 `a55caf8633c49a174f95d638f01f74fb81774628182aad83f7608d278643bc30`).
