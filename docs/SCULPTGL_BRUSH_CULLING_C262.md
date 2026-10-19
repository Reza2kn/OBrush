# Source Brush culling scope check (C262)

The frozen `SGL-BRUSH-BRUSH-CULLING` row describes OBrush 0.1.14. Current
Source ordinary Brush/Clay already exposes a per-tool Culling checkbox,
defaults it to off, and writes only vertices whose retained normal satisfies
`normal · eye <= 0` when on. Legacy OBrush uses its separate brush policy.
This change adds an independent opposed-sheet regression fixture and a
reproducible normal-browser control/predicate check; it does not change brush
math.

Pinned reference: SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The browser run called its actual `getFrontVertices` method on five normals:
`[0,0,0]`, twice `[0,0,1]`, and twice `[0,0,-1]`, with eye `[0,0,-1]`.
It returned IDs `[0,1,2]`. The visible reference Thin surface checkbox
changed the write subset from `[0,1,2,3,4]` to `[0,1,2]`. The normal OBrush
Source culling checkbox changed its retained per-tool option from false to
true on an imported 10,114-vertex independent SGL scene. Both controls were
initially false. Browser outputs:

- Reference: `.artifacts/sculptgl-brush-culling-c262/source-d290d514-bebf-4d66-b0ae-e4b9c5536f01/result.json`, SHA-256 `04ffcd397e3b0a1c5b1496313a4fad9a1e0d714e563da068145a624503c4932d`.
- OBrush: `.artifacts/sculptgl-brush-culling-c262/root-f5311207-a418-41ad-a0a7-b89f806bccd9/result.json`, SHA-256 `3151f1fe727cddb3389beed4e8550341ae0847a6afbdacba255853809da8febe`.
- OBrush portable WASM SHA-256 `76b5a1d3b9ce20267f60a76e146b44ff34340a0f66e1159190951d9f669aa7ac`.

The shared-core regression authors two overlapping opposite-wound triangles
with five distinct IDs. The common vertex's retained normal is exactly zero.
At center `[0.2,0.2,0]`, radius² `4`, eye `[0,0,-1]`, intensity `0.5`,
ordered static selection, no fold prevention, no symmetry and full masks,
both ordinary Brush and Clay select five, classify three front, and move all
five with culling off versus IDs `[0,1,2]` with culling on. IDs `[3,4]` stay
bit-exact when excluded. The pre-culling selection remains in one history
event; public `.obrush` JSON is exact through Undo and Redo. The focused core
test passes. The browser build used the lockfile-matched wasm-bindgen CLI
0.2.129 and passed.

Failed browser runs were retained in the same `.artifacts` cohort. The first
looked for the wrong English reference label; a later pass clicked an OBrush
off-screen logical rectangle; a two-page driver passed functional assertions
but failed its single-server closure contract. The final independent
single-side browser runs pass without relaxing culling assertions.

This is a scoped control and front-set result. The reference browser's
five-normal call is a direct method probe, while the OBrush geometry result
is from shared core. A paired browser stroke on the same two-sheet mesh,
camera pick/radius behavior, masks other than one, loaded normals, repeated
strokes, native GUI and visual appearance remain unverified. The full active
inventory gate stays open.
