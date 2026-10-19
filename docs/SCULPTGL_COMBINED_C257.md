# C257: combined primitive and mirror-guide browser admission

OBRush commit `584344c2340b96e5adba871074202b696fdc3fa7` integrates the Source-profile primitive topology
changes and imported-SGL mirror-guide fixes on top of C252. The pinned
SculptGL comparison remains commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
This is a bounded integration check, not a SculptGL or ZBrush parity claim.

`cargo fmt --all`, `cargo test --workspace --locked --quiet`, strict native
workspace/all-targets Clippy, strict WASM workspace/lib Clippy,
`bash scripts/build-web.sh` and `bash scripts/package-macos.sh` all passed
on the combined checkout. The browser app WASM SHA-256 is
`1011b3424506ed58d39310c0c09e2692d78d404dec46760854888b87df868d79`;
the signed macOS binary SHA-256 is
`e4b87d29da433156f8850e45e0b3fe1c861b2dcdfdd2098c6510319f12a299fc`.
`codesign --verify --deep --strict` passed, and the existing local browser
server at `http://127.0.0.1:8792/` returned HTTP 200 after the build.

The normal-browser additive-primitive harness imported the independently
authored mixed-face OBJ, added all four primitives via visible controls,
undid/redid Torus, cleared, and directly undid/redid/saved/reopened a new
dense Cube. Its passing receipt is
`.artifacts/sculptgl-primitives-browser/74e58627-e623-41f7-8d87-7f3d76511b05/result.json`
(SHA-256 `cfee61e0f0e8f5737f5df5e34a7ce01b73c2f8a79e7dd706e96fa9553cfdcbf3`).
The committed harness SHA-256 is
`3db3b8525bdc47df76872bfcadd50dc76933baa9c2d7b44773a36a2691a8314f`.
All four new OBrush Source defaults have the pinned counts:

| Part | Welded vertices | Original faces | Render triangles |
| --- | ---: | ---: | ---: |
| Sphere | 98,306 | 98,304 quads | 196,608 |
| Cube | 98,306 | 98,304 quads | 196,608 |
| Cylinder | 66,562 | 67,584 mixed faces | 133,120 |
| Torus | 65,536 | 65,536 quads | 131,072 |

The separate full-sphere browser shape check passed on the same WASM with
all 98,306 vertices paired after each mesh's bounding-box normalization:
maximum position difference `7.058e-7`, maximum paired normal angle
`0.027955°`. Receipt:
`.artifacts/sculptgl-primitives-browser/3d752b3f-bd21-4045-884e-c80ef5b3655a/result.json`
(SHA-256 `e1b20c72f9138494899531b6962e3a2358d43cd35ed7f33997989a066afff298`).
The imported affine-SGL Source mirror comparison also passed with no page
errors; its paired reference guide/pick scope and OBrush guide update are
recorded in
`.artifacts/sculptgl-symmetry-offset-c255/647f0a6f-5a28-4301-bb1e-6c45c84f1d9a/result.json`
(SHA-256 `12fb9729cdab381bfe628984c1a1b0c4ee544741f0bf5d6c9377938021eddaad`).
That receipt does not test an OBrush mirrored sculpt stroke.

The earlier failed isolated symmetry and sphere checks remain in their
respective C255/C256 reports. The cylinder/torus isolated web build's stale
shared-target failure and successful targeted rebuild are preserved in
`.artifacts/cylinder-torus-build/` and described in
[the topology report](SCULPTGL_CYLINDER_TORUS_SOURCE_TOPOLOGY.md).
Reference cylinder/torus coordinates and normal fields, exact primitive
world placement, retained four-level Source hierarchies, native GUI details,
and ordinary held-pointer responsiveness at 100k/1m vertices remain open.
Every active inventory acceptance gate remains open.
