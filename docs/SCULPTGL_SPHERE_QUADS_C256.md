# C256: smooth-quad Source Add Sphere candidate

The pinned reference is SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and the hosted bundle
SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
`Scene.addSphere` starts with the six-quad cube, normalizes the base object,
then applies non-linear subdivision until it has at least 50,000 original
faces. Seven four-way levels make 98,306 vertices, 98,304 original quads and
196,608 render triangles. Source `Scene.js` SHA-256 is
`092d17a08278700d6d85d3f8b9880e9152ce8c71be607fbe1584d0c5b5602f52`;
its `Subdivision.js` SHA-256 is
`2ba2b8a13e13acfda33cdc46ec04f9604ee2e99538526fbc9cb25f00fae4532c`.

The shared Rust core now builds a bounded smooth-quad sphere from an
independently authored closed cube grid and seven static smooth subdivisions.
Only default-detail Add Sphere in the SculptGL profile takes this route.
Ordinary Add Sphere and manually selected nondefault detail levels retain the
existing icosphere. The new mesh keeps original quad faces in
`SurfaceTopology` so browser and native use the same public geometry.

An independent analytic core fixture checks levels 0, 1, 3 and 7 for exact
counts, two uses per undirected edge, Euler characteristic 2, outward finite
normals, inversion symmetry and bounded-level rejection. Its first smooth
corner is `(5/9, 5/9, 5/9)` from the closed-cube valence-three rule. Fixture
source SHA-256: `ccf5767b80d6873c48e0dbe44b15ac20d29a8e9b567971b8ad473172cd90be1a`.
The app unit check verifies routing and the ordinary/manual exclusions.

The paired browser harness was originally run at SHA-256
`3760dfae2e6f89cc0bafd28d73a26787cc410194ba79fdb4a82def4e1b77d195`;
the renamed `scripts/check-sculptgl-sphere-shape-c256.mjs` has SHA-256
`ec10ea430baebdd3370923a2bf6d7ce1b7fa8778827f998a59ceb44b68fe743a`.
It uses actual Add Sphere controls in the pinned reference and OBrush, intercepts
only OBrush's ordinary public project Save, and matches normalized local
positions by nearest neighborhood. Its shape report compares normal angles
only after matching positions. Geometry counts alone do not establish a shape
or normal match. The final normal-browser receipt is
`.artifacts/sculptgl-primitives-browser/948c5a0d-2ff4-4481-8b49-c5580d4004d2/result.json`
(SHA-256 `a3c809ddc31508bb704eed70ad4779c6f6e0d60b0612c5eb67642b8f7b9730b8`).
It ran on the isolated worktree from base `a4b64005f8d24eaa9fe08dbe8558dfbcfb458383`
with generated baseline WASM SHA-256
`08d0189452ca84c6abfdcb8e362af818946d9ce9a291b2ebc89d1a67522deb97`.

Both browser additions selected 98,306-vertex/98,304-original-quad/
196,608-render-triangle spheres. The same harness also passed prior-part
retention, Torus Undo/Redo and Clear. For the sphere, it compared every one of
98,306 saved OBrush vertices to a distinct pinned-reference vertex after
centering each local mesh and dividing by its own X half-extent. There were
zero missing or unused vertices. The normalized position error had mean
`1.435e-7`, P95 `2.973e-7` and maximum `7.058e-7`. Corresponding normal
angles had mean `0.000958°`, P95 `0.002342°` and maximum `0.027955°`.
Reference local bounds were ±`0.41977194` on each axis; OBrush's translated
saved part had Y/Z bounds ±`0.8395439`, showing the expected factor-two cube
seed convention. The harness deliberately compares scale-independent local
shape; it does not imply the two applications place or size new parts alike.

The first browser attempt failed when one Chrome DevTools response tried to
return the full 98k-vertex mesh and timed out. Its retained receipt is
`.artifacts/sculptgl-primitives-browser/898b6d34-c4b1-4d94-89cb-88c0883fd052/result.json`
(SHA-256 `b4dec78a5d9eadb7cf48a4a7ee9eaa2ed8bd29e6126b3b5da2e96507b690d05f`).
The checker now reads bounded 4,096-vertex chunks. Its first passing shape
receipt is `.artifacts/sculptgl-primitives-browser/792aac1a-8471-40e3-badf-2a6dcec95235/result.json`
(SHA-256 `5da5123450d73b44791994c348f5a837d6b97c727799805170d089d7b73fbc9c`);
the final rerun updated the status wording to state the measured shape result.

`cargo fmt --all`, workspace tests, strict native/WASM Clippy, and
`bash scripts/build-web.sh` passed. The shared Cargo target first supplied a
stale `obrush-core` artifact from another isolated worktree, causing the
initial strict Clippy and web build to fail with `sphere_quad_smooth` missing.
Clearing only the affected shared-target package/profile and rebuilding made
both checks pass. The generated SIMD/GPU WASM SHA-256 values were
`aeadfdfae3626ddfc966b850d41b9c70b963a584ab4617d30c2b53fce4c78c32`
and `b7bc311cb292beafd356bbe3cdbc95b2e05a4edf3e2effef6d649a6f4bc9f454`.

The reference keeps up to four primitive multiresolution levels; this
candidate publishes only the final editable surface. Source object matrix,
part placement, UVs, colors/materials, exact vertex order, pixel appearance,
native GUI and million-vertex responsiveness remain separate gates. All
active SculptGL and ZBrush acceptance gates remain open.
