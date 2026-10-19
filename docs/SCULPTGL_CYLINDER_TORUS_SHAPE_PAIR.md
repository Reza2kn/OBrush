# Default Cylinder and Torus shape pair

The independent [normal-browser harness](../scripts/check-sculptgl-cylinder-torus-shape.mjs)
clears the pinned SculptGL scene, invokes its default `addCylinder()` and
`addTorus()` controls, and reads their resulting mesh buffers in bounded CDP
chunks. In a separate normal OBrush browser it uses the visible `Clear` and
`Add Cylinder`/`Add Torus` controls in the SculptGL profile, then the visible
project Save control. The analysis reads the downloaded `.obrush` JSON, not a
core constructor substituted for the UI action. Each case begins from an empty
scene. It does not exercise retained-level changes, brush deformation, native
GUI, or pixel appearance.

The pinned reference commit is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
and wrapper SHA-256 is
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
The OBrush browser WASM SHA-256 is
`1011b3424506ed58d39310c0c09e2692d78d404dec46760854888b87df868d79`.
The source worktree was based at `584344c2340b96e5adba871074202b696fdc3fa7`;
the served WASM also includes later root integrations. The exact receipt is
`.artifacts/sculptgl-cylinder-torus-shape/b0d0bf62-4314-437b-a149-f4ea57ee1c11/result.json`
(SHA-256 `bcdd2249aa6e7e4252f66ca178638c19395fe0493b39acab70e60aab89f0845e`).
The independent fixture consists of the two fresh empty scenes and fixed
default Add parameters: cylinder radius top/bottom `0.5`, height `2`, radial
and height segments `64`, both caps; torus major radius `0.5`, tube radius
`0.1`, full `2π` arc, radial segments `32`, tubular segments `128`.

| Measurement on default displayed mesh | Cylinder | Torus |
| --- | ---: | ---: |
| Reference / OBrush vertices | `66,562 / 66,562` | `65,536 / 65,536` |
| Reference / OBrush original faces | `67,584 / 67,584` | `65,536 / 65,536` |
| Reference / OBrush rendered triangles | `133,120 / 133,120` | `131,072 / 131,072` |
| Reference / OBrush oriented nonmanifold edges | `0 / 0` | `0 / 0` |
| Reference / OBrush Euler characteristic | `2 / 2` | `0 / 0` |
| Unique positions paired within `1e-6` local units | `66,562 / 66,562` | `65,536 / 65,536` |
| Mean / maximum paired position distance | `7.49e-9 / 4.86e-8` | `1.30e-8 / 1.11e-7` |
| Mean / maximum paired normal direction difference | `0.101° / 6.043°` | `0.000941° / 0.001994°` |
| Maximum local bounds difference | `1.06e-8` | `3.47e-9` |

Position pairing uses 3D spatial cells of width `1e-5`, searches neighboring
cells, consumes each reference vertex at most once, and requires distance
`≤1e-6`. Both meshes have zero unmatched vertices. The cylinder's 130
near-axis cap points (local `|y|>0.99`, radius `<0.2`) have normalized Y-normal
alignment with the outward axial direction of at least `0.9999944` in the
reference and `0.9999960` in OBrush. The torus has 65,536 outward-facing
original quads under the tested centerline/radial winding check in both.
Raw reference normals are area weighted, with median length `0.0001914`
(cylinder) and `0.0000598` (torus); OBrush stores unit normals. The table
compares directions after normalization. The cylinder's maximum `6.043°`
direction difference is a real remaining normal/appearance gap near some
surface locations, despite close positions and cap-axis alignment.

The reference also attaches a uniform object matrix scale of `28.577381`
for the first-added cylinder and `58.520573` for the first-added torus after
scene clear. Its local cylinder bounds are approximately
`±0.499248 × ±1 × ±0.499248`; local torus bounds are
`±0.599174 × ±0.099400 × ±0.599174`. OBrush saves those same local bounds
without a corresponding object transform in this `.obrush` scene. Thus the
world-space bounds differ by up to `27.577381` for the cylinder Y extent and
`34.464826` for the torus X/Z extent. Camera framing can obscure this in the
viewport; this pair does not establish world-placement or export-scale parity.
SculptGL retains three resolution levels; the OBrush saves contain one
displayed mesh, so retained-level behavior remains open.

Two earlier diagnostics remain in `.artifacts/sculptgl-cylinder-torus-shape/`:
the full-array CDP extraction timed out while requesting the reference
cylinder; chunked extraction resolved that harness issue. A run against the
previous browser WASM `a69b490d...` reached OBrush Save but exposed the old
triangle-only mesh (no `surface.faces`); the final run above uses the newly
integrated build. These failures are harness/build-state evidence, not hidden
as passing parity. No browser responsiveness measurement is claimed because
other agents were running browser work concurrently.

This supports a narrow default local-shape and manifold comparison for the
two Add commands. Full SculptGL scene and ZBrush parity remain open.
