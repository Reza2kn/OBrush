# C241 selected-mesh face counters

The pinned SculptGL top bar reads `getMesh()` and displays that selected
mesh's `getNbVertices()` and `getNbFaces()`; with no active mesh it displays
zero. OBrush previously displayed vertices and render triangles only in the
inspector. This slice adds a visible top-bar vertex/face counter and an
inspector face row. Both use shared Rust `Mesh::face_count()`: retained original
triangle/quad polygons count once; a mesh without retained source faces counts
its editable triangles. The existing triangle row remains distinct.

Reference: SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, pinned bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
mirror wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
The referenced `src/gui/GuiMesh.js` source is identified by frozen inventory
SHA-256 `d88e452e148b965071c7c482d421aed4dcaf6a63227095eacfcd44c0803d1869`.
OBRush browser WASM SHA-256
`c7a391373ebc1893c0e112bdaad2945c5263e7959ffdaa4dcbb0094dd87f01fe`;
the candidate began from `e20eb401aff998591d421bf65b2f3c842bd45edc`.

The independently authored 86-byte mixed OBJ fixture has SHA-256
`8541545630fbd560bbef5ee1a1b428ff47963c7704ded2ed9073598806332aa1`:
six vertices, one quad and one triangle, represented by three render
triangles. A dedicated owned Chrome run used the normal file inputs and
screenshotted both visible top bars. No application diagnostic assigned
geometry or UI state. The measured integer counts were:

| State | Pinned SculptGL vertices/faces/triangles | OBrush vertices/faces/triangles |
| --- | --- | --- |
| Mixed OBJ active after import | 6 / 2 / 3 | 6 / 2 / 3 |
| Same OBrush part, static smooth level 1 | — | 14 / 8 / 12 |
| Select lower level 0 | — | 6 / 2 / 3 |
| Return to level 1 | — | 14 / 8 / 12 |
| Clear scene | source code zero branch; browser not repeated | 0 / 0 / 0 |

The final browser receipt is
`.artifacts/sculptgl-mesh-info-browser/761077bf-e291-4ad1-bab0-08bc13c34c34/result.json`
(SHA-256 `2ceb87ff98405ff1ee19dfb55ed93e0010226ef2e99f663a56b08483f1933f23`).
Its screenshots include the pinned reference top bar (`Vertex : 6`, `Faces : 2`),
OBRush imported top bar (`Vertices: 6`, `Faces: 2`), fine level, and empty
scene. The local full result/screenshot directory is intentionally ignored.

The focused shared-core test uses the same independent mixed face structure
and checks active selection, static lower/higher, plain dynamic Loop
subdivision on a no-surface mesh, merge, versioned save/reload and Undo/Redo.
The face count is derived from current mesh arrays, so edits that retain
topology cannot leave a separate cached count stale. The scene save/reload
test checks all three counts, including a 16-vertex, 21-face, 22-triangle
mixed merge. A separate no-surface scene is decoded as both its natural
version 5 and an accepted version 7 encoding; both count triangles as faces.

Failed checks remain recorded. The first core test tried legacy triangle
subdivision directly on retained quads and was correctly rejected with
`Triangle subdivision cannot preserve original quads`; it was changed to the
ordinary source-static quad path. The first browser harness run misread egui's
min/max control rectangle as width/height and timed out clicking Clear scene;
its unmodified receipt is
`.artifacts/sculptgl-mesh-info-browser/590e32f0-57ce-40b5-9d3e-34796056209c/result.json`
(SHA-256 `57cad857929801af24ddf66eb29d7c5630f744a1afa047965b8ea39e851fc366`).
Correcting only the click coordinate made the imported/empty case pass, then
the static-level case passed in a separate run. An initial browser build also
hit the pre-existing `serde_json::json!` macro recursion bound when the new
diagnostic field was inserted into its large object; the field was moved into
the existing post-build insertion block. No compiler recursion setting changed.
The first added no-surface version assertion expected automatic version 7,
but the serializer correctly chose version 5 for a plain scene; the test now
checks version 5 and independently admits the same plain scene as version 7.

This does not close `SGL-APP-MESH-INFO`: reference static/dynamic/multires
face-count sequences, paired merge and selected-object browser routes, native
GUI and performance still need matched checks. Counts are exact in the scoped
fixture; surface shape and brush response are outside this case.
