# Default Source-profile cylinder and torus topology

The pinned reference is SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`. Its `Primitives.js`
(SHA-256 `2b9b10126d5e689a518c74a8c2b0e3024891de7a589530341965ef3f57bd0836`)
creates a cylinder with 64 radial and 64 height segments, 4,096 side quads,
and 128 triangular cap faces. The default torus starts with 32 radial by
128 tubular quads. `Scene.js` (SHA-256
`092d17a08278700d6d85d3f8b9880e9152ce8c71be607fbe1584d0c5b5602f52`)
uses smooth `subdivideClamp` until at least 50,000 original faces; both
primitives take two four-way levels. The pinned browser bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independent normal-browser count baseline is
[C248](SCULPTGL_PRIMITIVES_C248.md).

OBRush's shared-core Source-default constructors retain original mixed
cylinder faces and all-quad torus faces before applying two static smooth
subdivision steps. Its ordinary primitive constructors and manually selected
nondefault detail retain their previous triangle-only path. The default
Source-profile Add controls select the new constructors. Bounds and
allocation limits are checked before constructing each base mesh and at each
static subdivision. The generated surface remains part of the document's
normal project serialization and Undo/Redo snapshots.

| Default part | Pinned vertices / original faces / render triangles | Prior OBrush | New Source path |
| --- | ---: | ---: | ---: |
| Cylinder | 66,562 / 67,584 / 133,120 | 66,562 / 133,120 / 133,120 | 66,562 / 67,584 / 133,120 |
| Torus | 65,536 / 65,536 / 131,072 | 65,536 / 131,072 / 131,072 | 65,536 / 65,536 / 131,072 |

The independently written Rust fixture checks actual canonical quad
triangulation, the cylinder's 65,536 quads plus 2,048 cap triangles, the
torus's 65,536 quads, manifold oriented edges, Euler characteristics 2 and 0,
finite unit normals, bounded cylinder extents, unchanged ordinary constructors,
and exact torus project JSON round-trip and Undo/Redo. The app unit contract
checks selection of the Source path at default detail and preservation of the
ordinary and manual paths. Fixture source SHA-256 is
`33cda9efb916a672b254dba167d8c1b6d79e1ae3db30163a48a8095ca8ccaeea`.
The focused core fixture passed 3/3; the app contract passed 1/1; workspace
tests, `cargo fmt --all -- --check`, strict native workspace Clippy, strict
WASM core Clippy and `bash scripts/build-web.sh` passed in the isolated
worktree. The generated browser app WASM SHA-256 is
`3e07a7fdf0b175fc5a3ff7f12968e88128cf963e37d598275f8045ce633bbb17`.

The first isolated web build failed because a shared cross-worktree Cargo
artifact for `obrush-core` was stale: app compilation reported missing new
constructor methods while those methods existed in current source. Its log
is retained at `/tmp/obrush-cylinder-torus-web-build.log`. Cleaning only the
`obrush-core` `wasm32-unknown-unknown` `web-release` package removed 86.2 MiB;
the immediately repeated build passed and its log is retained at
`/tmp/obrush-cylinder-torus-web-build-retry.log`. This is build-cache evidence,
not a source regression.

**Limitations:** Matching face counts and connectivity does not establish
matched vertex coordinates, normal words, UVs, material fields, world
placement, rendered appearance, or SculptGL's retained multiresolution levels.
The added mesh stores the final static surface without lower levels. Direct
paired-browser Add, Save/Reload, Undo/Redo, native GUI and sustained-input
checks remain open until separately recorded. The inventory acceptance gates
remain open.
