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
OBRush's ordinary cube tool and manually selected nondefault detail levels
retain their earlier behavior.

| Quantity | Pinned Source | C248 OBrush cube | C249 generated cube |
| --- | ---: | ---: | ---: |
| Welded vertices | 98,306 | 24,578 (−73,728) | 98,306 (difference 0) |
| Original quad faces | 98,304 | 0 | 98,304 (difference 0) |
| Render triangles | 196,608 | 49,152 (−147,456) | 196,608 (difference 0) |

The C249 numbers are shared-core fixture results; they are not yet an
observed OBrush browser run. The independent lattice fixture checks OBrush
positions after its own ±1 convention; a pairwise reference coordinate or
normal-distance check remains open.

This closes only the C248 cube's raw count and quad-topology discrepancy.
Vertex ordering, world placement, corner UVs, color/material words, normal
words, four retained Source subdivision levels, appearance, save/reload,
repeated additions, native
GUI and browser responsiveness remain unverified. The active inventory
acceptance gate stays open.
