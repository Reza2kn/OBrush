# Source default cylinder/torus normal diagnostic

The prior [default shape pair](SCULPTGL_CYLINDER_TORUS_SHAPE_PAIR.md) measured a
`6.042878°` maximum cylinder normal-direction difference between pinned
SculptGL and OBrush's downloaded `.obrush` mesh. This follow-up distinguishes
the saved mesh-normal cache from the **normal buffer used by the actual OBrush
viewport**. It does not change product code or close an appearance-parity gate.

The pinned SculptGL commit is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
OBRush was built from `49a736ebc51f387fb5f7f2e087e9b2ae6d119188`, browser
WASM SHA-256
`1011b3424506ed58d39310c0c09e2692d78d404dec46760854888b87df868d79`.
The [extended normal-browser harness](../scripts/check-sculptgl-cylinder-torus-shape.mjs)
used the same fresh-scene default Add parameters and downloaded projects as
the original pair. It independently accumulated original-face area vectors
from the saved positions and 3/4-corner faces. Before OBrush navigation it
installed a read-only WebGL2 observer; after each real Add/Save it used
`getBufferSubData` on the active triangle draw's 40-byte vertex stream to
read the normal at the old worst saved-cache vertex. The GL buffer binding was
restored. No mesh, camera, product GL call or project data was assigned.

The complete receipt is
`.artifacts/sculptgl-cylinder-normal-gpu/8dc183c8-6edd-45a8-a430-766ee4def2e2/result.json`
(SHA-256 `bff2008f0c72d310f295be374267add086e4ddb8cf124d52640520b7700a0538`);
the two actual `.obrush` downloads are retained beside it. All 66,562 cylinder
and 65,536 torus local vertices still pair within `1e-6` units. Angles below
normalize the reference's area-weighted raw normal and OBrush's compared
vector before taking their dot product:

| Default solid | Saved `.obrush` mean / max | Independent original-face area-sum mean / max | Active VBO versus reference at old worst saved vertex |
| --- | ---: | ---: | ---: |
| Cylinder | `0.100768° / 6.042878°` | `0.0000201° / 0.0001001°` | `0.0000153°` |
| Torus | `0.000941° / 0.001994°` | `0.0000584° / 0.000510°` | `0.0001248°` |

At the cylinder's old worst point, local position
`(0.3639198, 0.9837298, -0.32983768)`, the pinned raw normal is
`(0.00011258398445, 0.00038968684385, -0.00010204011051)`. OBrush's
saved unit normal is `(0.34525725, 0.8883604, -0.30267677)`, while the
actual 2,662,480-byte active VBO contains
`(0.00011258394079, 0.00038968629087, -0.00010203997226)` at that vertex.
The latter is `6.042866°` away from the saved normal direction. Thus the
original 6° statistic identifies a **serialized cache-policy gap**, not a
6° difference in the observed initial viewport normal at that point. The
torus active VBO is 2,621,440 bytes; its sampled active-draw normal likewise
follows the raw original-face direction.

The cause is visible in the independent Rust paths: `Mesh::recompute_normals`
uses corner-angle-weighted unit render triangles; the Source normal field
accumulates area vectors from retained original triangle/quad faces, and the
Source viewport uploads that field instead of `mesh.normals`. I briefly
tested assigning the Source field's normalized direction to the new default
primitive cache. The pinned cylinder shoulder regression passed in memory,
but a required Save→Open test failed: `Document::from_json` recomputes loaded
mesh normals under the generic policy. Its first serialized difference was at
byte `2,274,078` (`10,077,367` original versus `10,412,907` reserialized
bytes). This attempted product change was **reverted**; a passing initial
cache comparison that silently changes on reload would be misleading. The
failed focused test and first-difference metadata are retained beside the
browser receipt as `failed-core-reload.log` (SHA-256
`2d5af2fcb0e32f5bd093987aa7e00db9f9430b7a02d46fe85a33b34ddb3c2dba`)
and `failed-candidate.json`.

The VBO observation samples one known high-disagreement vertex per solid,
not every displayed normal or final pixels. It does not test brush edits,
Undo/Redo, file reopen in the browser, transformed lighting, or the reference
object's different scene scale. A future fix needs an explicit, persisted
normal policy that covers Source edits and reload without changing generic or
imported mesh semantics, followed by full VBO and saved-file comparisons.
