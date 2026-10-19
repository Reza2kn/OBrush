# C249: bounded Source-profile cube grid

The pinned SculptGL reference is commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`. Its `Scene.addCube`
starts with the eight-vertex/six-quad cube in `src/drawables/Primitives.js`,
sets linear subdivision and calls `subdivideClamp` until at least 50,000
original faces. Seven four-way levels produce 98,304 quads, 98,306 welded
vertices and 196,608 render triangles. The independently collected normal
browser baseline is [C248](SCULPTGL_PRIMITIVES_C248.md), whose pinned bundle
SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.

The implementation commit is `4f2d58fc45293e13860948e16f42705aba7f5a48`.
OBRush's default-detail SculptGL-profile Add Cube now constructs a shared-core 128×128
integer lattice on each cube side. Shared boundary coordinates weld the
six sides, and original outward quads remain in `SurfaceTopology`. Mesh
positions use OBrush's existing ±1 local extent; Source uses a ±0.5 base
cube transformed by `normalizeSize()` and an additional 0.7 world scale.
An independent analytical fixture checks all 98,306 unique local lattice
points, the original-face and render-triangle counts, every undirected edge
used exactly twice, Euler characteristic 2, outward winding, finite normals,
bounded level rejection and exact document Undo/Redo. An app
unit contract checks that default Source Add Cube selects this grid while
ordinary and manually selected detail levels preserve the prior path.
The fixture test source SHA-256 is
`ec16a16d42eecc00d68634a3af1b94d7bedaaf0482171934bd2728240ae0a076`.
OBRush's ordinary cube tool and manually selected nondefault detail levels
retain their earlier behavior.

| Quantity | Pinned Source | C248 OBrush cube | C249 generated cube |
| --- | ---: | ---: | ---: |
| Welded vertices | 98,306 | 24,578 (−73,728) | 98,306 (difference 0) |
| Original quad faces | 98,304 | 0 | 98,304 (difference 0) |
| Render triangles | 196,608 | 49,152 (−147,456) | 196,608 (difference 0) |

The C249 counts pass in both the shared-core fixture and the normal browser.
The browser ran the existing C248 paired harness without edits (script
SHA-256 `200d3fabb7f70122aa0c8c18b2f4e98f84b5c78c58634bc862829a5387d8708a`)
against the C249 browser WASM SHA-256
`e44fa1bf8681d9432f0c49b0516d44c5368898c9ec673f70e8a59febffd5cc86`.
Its passing receipt is
`.artifacts/sculptgl-primitives-browser/ab55c712-602c-4a49-ba91-cc7687697211/result.json`
(SHA-256 `07f861de34ba34a782cf519c41951919bf417e881112a12fa2d2c9b8d968f0e0`).
The actual Add Cube action selected the newly added part in both browsers;
their top-bar mesh totals each read 98,306 vertices, 98,304 original faces
and 196,608 triangles. Prior imported and added parts stayed identical,
and the check reported zero runtime errors. Browser Undo/Redo exercised the
last Torus action, while the shared-core test exercised exact cube-scene
Undo/Redo at a smaller grid level. Direct browser Undo/Redo of the new dense
cube remains open.
There were no failed C249 browser attempts; the earlier C248 failed harness
receipts remain preserved in the C248 report.

The independent lattice fixture checks OBrush positions after its own ±1
convention; a pairwise reference coordinate or normal-distance check remains
open. Verification on this branch: `cargo fmt --all`, workspace tests,
strict native and WASM Clippy, `bash scripts/build-web.sh`, and the paired
browser harness all passed. The generated GPU and SIMD WASM SHA-256 values
were `e6aa5aa5732e252c4a32e071d6a0e37d392fc6ed311521049ebc50c5f907e841`
and `119a57bbf93031fdf5ba03e857aeb47aaaa75beb1a1f2c7e813937b758ca12ee`.

This closes only the C248 cube's raw count and quad-topology discrepancy.
Vertex ordering, world placement, corner UVs, color/material words, normal
words, four retained Source subdivision levels, appearance, save/reload,
repeated additions, native
GUI and browser responsiveness remain unverified. The active inventory
acceptance gate stays open.
