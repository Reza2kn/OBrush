# C248: additive primitive and empty-scene browser pair

The pinned reference is SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, browser bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
and hosted wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
Its Scene controller calls `addSphere`, `addCube`, `addCylinder`, `addTorus`
and confirmed `clearScene`. `subdivideClamp` retains up to four Source
levels while making at least 50,000 original faces. The independent 86-byte
mixed OBJ fixture has SHA-256
`8541545630fbd560bbef5ee1a1b428ff47963c7704ded2ed9073598806332aa1`.
The browser harness is `scripts/check-sculptgl-primitives-c248.mjs` (SHA-256
`200d3fabb7f70122aa0c8c18b2f4e98f84b5c78c58634bc862829a5387d8708a`).
The passing owned-Chrome receipt is
`.artifacts/sculptgl-primitives-browser/43191adf-b2d2-43bd-8d78-7d1b35d894f6/result.json`
(SHA-256 `ffbbe15e1bb8bfe065efe4c6e53cd54ee94396e8c83b16c2b2e46288740cb165`).
The served OBrush WASM SHA-256 was
`f9d50dc61472d31bd1a4834cbc944248c47b9ce5835b7ab2246318e80626dfa7`.
The harness recorded Git HEAD `7ca3b47de07e319c987c31a915d5d6757ab034d2`,
but the served WASM predates that HEAD's unrelated C247 render-resolution
change; this C248 check exercises the existing primitive UI and geometry.

The reference's startup sphere is cleared before normal File Import so both
applications start the pair with only the same six-vertex, two-face OBJ.
Reference primitive actions call the real Scene menu callbacks; OBrush
primitive actions click its visible add buttons. Each addition selects its
new part and retains the imported object and every prior part. The two apps
use distinct placement: SculptGL adds shapes around its origin, while OBrush
places each new part to the right of the existing scene. The screenshots
therefore are UI-path evidence, not pixel-matched shape comparisons.

| Added part | SculptGL vertices / original faces / triangles | OBrush vertices / faces / triangles |
| --- | ---: | ---: |
| Sphere | 98,306 / 98,304 / 196,608 | 40,962 / 81,920 / 81,920 |
| Cube | 98,306 / 98,304 / 196,608 | 24,578 / 49,152 / 49,152 |
| Cylinder | 66,562 / 67,584 / 133,120 | 66,562 / 133,120 / 133,120 |
| Torus | 65,536 / 65,536 / 131,072 | 65,536 / 131,072 / 131,072 |

The cylinder and torus match reference vertex/render-triangle counts, but
OBRush's generated objects keep triangle faces rather than Source original
quads and have no retained primitive subdivision levels. Matching counts
alone do not establish matching coordinates, normals, caps, manifoldness or
appearance. OBrush's lower-density sphere and cube are material differences.
The last Torus addition is reversible in both apps. The pinned reference's
History menu Undo/Redo moves the count 5 → 4 → 5; OBrush's selected SculptGL
profile Ctrl+Z/Ctrl+Y moves its count 5 → 4 → 5 with Redo depth 1 → 0.
Both applications reach zero meshes through their clear controls; SculptGL
resets history to Undo=0/Redo=0, while OBrush retains Undo=6/Redo=0 for the
complete prior scene. OBrush deliberately preserves that undoable clear
policy. The passing run also confirms each added part was active and prior
part summaries stayed identical.

Early failed runs remain in `.artifacts/sculptgl-primitives-browser/`:
`a14ba7e4-7ca8-4d10-8a0b-717cdadfe439` tried to serialize a cyclic
SculptGL mesh return value through CDP; `916d2644-2d97-45af-beb2-eaf2c3f6ca1b`
and `e529c142-852e-49c6-a872-8ad76cf9b0a7` used Meta+Z for OBrush
while its selected SculptGL shortcut profile binds Ctrl+Z/Ctrl+Y. These are
harness failures, not evidence that OBrush Undo failed. The final rerun uses
the actual profile shortcut after returning focus to empty viewport space.
No failed receipt was replaced or deleted.

All five inventory rows remain open. The broader gates still need matched
shape/normals, retained subdivision behavior, save/reload, repeated additions,
clear confirmation/cancel, camera/action state and native GUI checks.

The same harness passed again after the merged C247/C248/symmetry-offset
browser build. Its second receipt is
`.artifacts/sculptgl-primitives-browser/7bf81978-c9af-422e-bac4-ad8b8e253ae9/result.json`
(SHA-256 `66411753dffbd98ae67d704f5c9751132882ca88e59d3d20f4b2943f3c4044b5`),
with served baseline WASM SHA-256
`88c1be7ca7d4d6a9dc45b59025bedcdc41b174c4e16d212cb9e9f87b439df13f`.
The same counts and undo/redo/clear gates passed; this rerun does not close
the shape or retained-level gaps.
