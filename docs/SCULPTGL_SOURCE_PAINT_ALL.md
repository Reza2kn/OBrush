# Scoped Source Paint All comparison

The normal browser now exposes `Paint All` while Source Paint mode is selected.
That action uses the Source RGB, roughness, metallic and write-channel settings
in the shared Rust core. It blends the active part's unmasked vertices once by
their mask weights. It does not use brush radius, intensity, alpha, hardness,
culling, pressure or symmetry. The pre-existing Legacy Fill remains separate.

## Reference and fixture

- Pinned local SculptGL source commit:
  `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`.
- `src/editing/tools/Paint.js` SHA-256:
  `18297f0ebbfc2b49865cf693f96d9be16e4b53cf447860a10010220cf7b7d09c`.
- [`check-sculptgl-paint-all-reference.mjs`](../scripts/check-sculptgl-paint-all-reference.mjs)
  SHA-256: `d3e5bc29340361de61c4485be04949d12ce48f878c181a9ad158c09842250788`.
  It executes the pinned `Paint.paintAll` method on an independent four-point
  mock mesh. Masks are `[1, .5, .25, 0]`; target RGB is `[1, .5, .25]`,
  roughness `.75`, and metallic `1`. RGB and roughness writes are on; metallic
  is off. Every resulting channel equals the stated analytic f32 values,
  geometry and the other part are unchanged, and the source method invokes
  its state/vertex, duplicate-channel, draw-array and render hooks once.
  This method check does not prove a reference browser GUI or undo response.
- The Rust host fixture uses the same four channel/mask rows on one selected
  part and an independent second part. It verifies exact expected RGB and
  roughness arrays, bit-identical disabled metallic/masks/geometry, unchanged
  other-part channels, public JSON Save→Open identity and exact Undo/Redo.
  Its all-off and invalid-target cases retain the prior redo.

## Normal browser

[`check-sculptgl-paint-all-browser.mjs`](../scripts/check-sculptgl-paint-all-browser.mjs)
SHA-256: `fe0b0e21cfdf2e747cb4f9e0aa8cef1473edcb28403242ff2b7ff98a5a89452e`.
It imported an independently authored four-vertex, four-triangle `.obrush`
tetra through the normal file input, chose Paint and Source mode through actual
controls, clicked the new Source `Paint All` button and downloaded the public
project. Masks were unprotected `[1, .5, .25, 0]`; RGB and both material writes
were enabled with Source defaults. Three vertices changed in both RGB and
material; geometry, triangles and masks were exact. Maximum absolute
differences from independent f32 target/blend expectations in the saved JSON
were `2.718e-8` (RGB) and `2.384e-8` (material).

The isolated browser WASM SHA-256 was
`0e9160cefcb25b8e451cb6d523e32bbc9cc472a342f4864a87b7622d0f8a0fd3`.
Two complete browser runs produced byte-identical result receipts at
`.artifacts/sculptgl-paint-all-browser/f0070650-fc49-4378-8bb7-988c97ded609/result.json`
and
`.artifacts/sculptgl-paint-all-browser/b2842315-5516-4da8-b978-13a9995d6473/result.json`;
each receipt SHA-256 is
`c970678e20c55383516175adae3eb47f59ed3d79c0e2f458a7ebcca9b83fb654`.
The fixture project SHA-256 was
`11022d15d88f48e9ab3f75e610514a1769c64de4eb46e83ec64c626020a37b1e`;
the downloaded project's SHA-256 was
`573f4125aed182495c40722d41230785c3330dc034faa228bb60cec6e048203f`.

## Validation and limits

`cargo fmt --all`, the full locked workspace suite, strict native and WASM
workspace Clippy, and `scripts/build-web.sh` passed for the shared-core route.
After adding the reachable Source control, `cargo test -p obrush-app --lib
--locked --quiet`, both strict Clippy passes and the fresh browser build passed.
The first isolated browser build attempt stopped before compiling because the
ignored worktree `.tools` directory lacked the lockfile-matched wasm-bindgen
CLI; rerunning with the main checkout's verified `0.2.129` CLI passed.

Failed browser receipts remain in `.artifacts/sculptgl-paint-all-browser/`:
`5fbb79fa`, `22f9cf50` (the first fixture used the protected mask convention,
so Source mode was disabled), `3513daff`, `6db5615f` (revealed that Source
mode hid the old Legacy Fill control), `1963773b` (the checker initially
expected no metadata fields in its fill report), and `a34a02e3` (a synthetic
click released before a browser frame). The product control and checker were
corrected; none of these failures is relabeled as a pass.

This is one explicit-material, soft-mask fixture and a pinned method-level
reference response. A paired SculptGL browser UI run, implicit material
defaults, multiresolution/locked/dynamic meshes, duplicate vertices, physical
input, native GUI and broad project/pixel comparisons remain open. The active
SculptGL Paint All inventory gate is still open.
