# V03 single-mesh image gate: UI and grid still confounded pixels

**V03 failed its frozen pixel bounds; `SGL-RENDER-RESOLUTION` remains Open.**
Both owned Chrome runs completed all 24 normal-browser screenshots and
geometry/history checks, then the [V03 analyzer](SCULPTGL_RESOLUTION_SINGLE_MESH_V03_PREREG.md)
exited 1. This does not change [V01](SCULPTGL_RESOLUTION_SINGLE_MESH_V01_RESULT.md)
or [V02](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_RESULT.md) verdicts.

| DPR | Local receipt | `result.json` SHA-256 | Frozen first pixel failure |
| ---: | --- | --- | --- |
| 2 | `.artifacts/sculptgl-resolution-single-mesh-v03/695f9496-46d5-47ad-afa3-3ffe1fe4a07c/` | `273ef3d045fd9a2941875837626387a4fa2af181beef17e04dc05b9b86f24907` | Source 0.5× before versus 1×: mean `6.290`, p99 `79`, fraction >64 `6.355%`; preregistered maxima `16`, `64`, `2%` |
| 1 | `.artifacts/sculptgl-resolution-single-mesh-v03/a9a9fee8-c915-4cbc-9519-f3add38fc03b/` | `b959797de82068a34a5cc5329ba2a0087d218cadae4cc9f66347b3eaeb233697` | OBrush 1× before versus 2× silhouette: p95 `118.330` CSS px, max `219.572` CSS px; maxima `2`, `6` |

The corresponding `pixel-analysis.json` SHA-256 values are
`f41345d0b7338dedda80afc1803bcab37273aa46baaccc14e4f87ac8941e62ea`
and `fc87b086913dffff415d8801a1a9c7bd8d7c2b7b3792ba10226de46f59578038`.
All original PNGs, normalized crops and masks remain in those ignored local
receipt directories. Both command exits were 1; owned Chrome closed and the
browser lane was released.

Offline inspection of the untouched PNGs explains why these failures do
not establish a mesh-render defect. At DPR 2 the visible Source Scene
dropdown had closed, but a different **Extra UI** topbar dropdown was still
fading across the sphere in all three *before* images. It was absent in
the *after* images. In the same DPR-2 Source 0.5× pair, the after-image
mean was only `0.567`, p99 `1`, fraction >64 `0.114%`; Source static images
were byte-identical in the measured interior. At DPR 1, the Source 0.5×
before pair had mean `0.389`, p99 `1`, fraction >64 `0`, and the Source
silhouette 1× versus 2× p95/max was `1/1` CSS px. The DPR-2 failure is
therefore attributable to unclosed transitional UI in the browser image,
not to a demonstrated resolution-dependent model or shader change.

OBRush's unmodified visible floor grid crosses the lower sphere in every
V03 screenshot. The frozen foreground threshold includes dark grid lines
as internal holes, producing widely separated mask boundaries even though
the true outer sphere silhouette is nearly identical by visual inspection.
For DPR 1, OBrush 0.5× versus 1× *before* color mean was `0.338`, p99 `1`,
fraction >64 `0.031%`; 2× versus 1× mean was `0.325`, p99 `1`, fraction
>64 `0.0011%`. DPR-2 OBrush values were similarly below the color bounds
(0.5× mean `0.262`, p99 `1`, fraction >64 `0`; 2× mean `0.279`, p99 `6`,
fraction >64 `0`). Static color comparisons were exactly zero in both
apps and DPR values. The grid-contaminated mask cannot support a shape
fidelity conclusion.

All six Source contacts changed 37 original vertices, retained 18,242
vertices, and gained one history step. All six OBrush contacts changed
104 original vertices, added 788 adaptive vertices and gained one Undo
step. Within each app at each DPR, 0.5×/2× matched 1× on changed IDs and
complete post-stroke coordinates exactly. The original [R03/V02](SCULPTGL_RESOLUTION_SINGLE_MESH_V02_RESULT.md)
limits still apply: one sphere, one short held move, non-visible Source
Symmetry and resolution controller setup, no cross-app visual comparison.

Pinned Source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
mirror wrapper SHA-256 `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`;
official OBrush product revision `ff81bbf7e837ebf5c50c7462cf7b7e421b732450`,
portable WASM SHA-256 `d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`.
Independent OBJ SHA-256
`492dd0cfa56544bab0e816efa68a4b48b522ddaf5c70cbb2f95b6c92d167ec2e`.
V03 runner SHA-256 `9bb1f3f9c6272355d2d6dee4419b6563bdf3556cf975a8d0935f79c17d2e9a0d`;
V03 analyzer SHA-256 `56ad9971dc00426387574d8868e85100651e30128a6894abd959deebbc2e102b`.
Static JS/Python self-checks and `node --check` passed before browser use.
