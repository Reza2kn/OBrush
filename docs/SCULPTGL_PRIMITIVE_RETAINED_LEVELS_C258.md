# C258: retained levels for Source default primitives

The pinned SculptGL reference is commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; its hosted browser bundle
SHA-256 is `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
`Scene.subdivideClamp` (source SHA-256
`092d17a08278700d6d85d3f8b9880e9152ce8c71be607fbe1584d0c5b5602f52`)
subdivides default primitives until at least 50,000 original faces, then keeps
at most the last four multiresolution levels. The resulting retained counts are
four for Sphere and Cube (the last four of eight generated levels), and three
for Cylinder and Torus (base plus two subdivisions).

OBRush's default-detail SculptGL-profile Add controls now construct those
Source-static hierarchies inside `obrush-core` and add the selected part as one
validated scene/history transaction. Only the default Source-profile path
changes; ordinary mode and manually selected nondefault detail retain their
previous constructors. The same core owns native/WASM geometry, project
serialization and Undo/Redo. The finished mesh retains original quads (and
the cylinder's cap triangles), while lower levels remain editable through
OBRush's existing level controls.

| Default part | Pinned retained levels | OBrush retained levels | Final vertices / original faces / triangles |
| --- | ---: | ---: | ---: |
| Sphere | 4 | 4 | 98,306 / 98,304 / 196,608 |
| Cube | 4 | 4 | 98,306 / 98,304 / 196,608 |
| Cylinder | 3 | 3 | 66,562 / 67,584 / 133,120 |
| Torus | 3 | 3 | 65,536 / 65,536 / 131,072 |

The independently authored shared-core fixture builds each base, adds the
required Source-static levels, checks the four final counts against the prior
constructors, compares every final position and original face by coordinates,
navigates lowest→highest, loads versioned `.obrush` JSON, and checks Undo/Redo.
An out-of-range level request preserves the original document and redo branch.
Fixture source SHA-256:
`926bdc27b9c6da2b2a2eb47c8d8caab081261ff9cf519fb6c8d3112b767f88a9`.
The sphere/cylinder/torus constructors use the same refinement kernel as their
prior static final surfaces. The cube's generated vertex ordering can differ
from its prior direct grid; the coordinate and face-set assertions test the
visible geometry without claiming identical vertex IDs.

The paired normal-browser primitive harness (script SHA-256
`76dadfc90addf62d89877d2822917196d49fa775da4be15a5da84b9bdc10256d`)
reports selected retained levels `4/4/3/3` in both pinned SculptGL and OBrush,
using normal Add controls in OBrush and reference app add actions. It also
checks retained imported geometry, Torus Undo/Redo, Clear, and a direct dense
Cube Add→Undo→Redo→Save→Clear→Open→Lower→Higher→Save workflow. The public
Cube project is 15,981,748 bytes (SHA-256
`10484a1e583d5cee0791beacc270581dfa43db62af3907a5f96690630c531d34`),
with four levels and active level three. Lower gives 24,578 vertices/24,576
original quads; Higher returns 98,306/98,304. The two public saves match
byte-for-byte. The passing browser receipt is
`.artifacts/sculptgl-primitives-browser/9bce2562-a733-4c59-9bd4-7e3c779aed4f/result.json`
(SHA-256 `a6a6dd0e8e0eb1c69a58cdc48339f55dae9743be7bc7e3a14378b7c0bf0f56d1`).

The existing C256 full-sphere shape harness (script SHA-256
`ec10ea430baebdd3370923a2bf6d7ce1b7fa8778827f998a59ceb44b68fe743a`)
still matches all 98,306 normalized positions to distinct reference positions:
maximum normalized position error `7.057829523173553e-7`, maximum paired
normal angle `0.027618218844114053°`. Its passing receipt is
`.artifacts/sculptgl-primitives-browser/c76480f8-c84d-4dab-82ed-0b711f3c1374/result.json`
(SHA-256 `7d77623c50de232e6030524f1e17cd681d53d9cf6c22a82e8ec76e1a46a1d681`).
The tested browser WASM SHA-256 is
`4cbddfcbe6b4a6c8c2d6e905ee6dde3f20f28cb68cb6e20cf9fdc77df5a23239`.

Validation on the isolated branch: `cargo fmt --all`, the focused core and app
tests, strict native workspace Clippy, strict WASM core Clippy, and
`scripts/build-web.sh` pass. The first isolated web build failed immediately
because `.tools/wasm-bindgen` was absent in this worktree; using the installed
lockfile-matched CLI and the shared target passed. The first redundant full
workspace test run was intentionally interrupted after no failure while the
existing slow `crease_detail` group ran, to release the shared Cargo lane for
other parallel work; its incomplete log is retained at
`/tmp/obrush-primitive-levels-workspace.log`. A complete integrated workspace
run remains required after merging.

This closes only the missing retained-level *presence and count* for default
Source primitives, plus the scoped cube browser navigation/roundtrip. Edited
coarse-to-fine detail transfer against SculptGL, all-part browser navigation,
exact cube vertex order, cylinder/torus geometric pairing, Source object
placement/UV/material fields, native GUI and sustained performance remain
open. All SculptGL and ZBrush acceptance gates remain open.
