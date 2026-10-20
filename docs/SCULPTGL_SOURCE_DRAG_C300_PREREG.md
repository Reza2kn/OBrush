# C300 Source Drag symmetry and mask matrix preregistration

Frozen before C300 browser capture. The earlier C298 Drag draft at isolated
commit `be20d73945cb136e96a0901e733bffb1649bd0a7` was superseded by this
ID-only rename before any browser execution; C298 belongs to separate UV/MatCap
persistence work. The Drag fixture, path, numerical gates and trial product code
are unchanged. Candidate code was authored after the earlier prereg and remains
unbuilt. This is an adjacent, scoped normal-UI SculptGL comparison, not a claim
of full Drag, SculptGL, or ZBrush parity. C295's passed single-gesture receipt,
the C288 failure, and every other failed check remain unchanged. The root
checkout and its normal server are not used for experimental product edits.

## Source and independent fixture

- Pinned SculptGL commit: `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
  locally mirrored `sculptgl.js` SHA-256:
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
- Independently Python-authored CC0 SGL v3 curved grid:
  `tests/fixtures/independent-source-drag-c286.sgl`, SHA-256
  `fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`,
  1,089 RGB-ID vertices, 2,048 triangles. `scripts/fixtures/author-source-drag-c286.py`
  supplies the formula and byte identity. ID544 is fully protected, ID545 is
  half unprotected; all others are fully unprotected. No OBrush or SculptGL
  encoder authored this mesh.
- Product baseline is integrated root HEAD
  `f053e5be449a5d11866bee5e18a169f41e077dd3`. Its current ordinary
  `web/dist` portable WASM/JS hashes are
  `944cd8fec4350d0fd95829e47846c63d613d6ab1a5e905055c850bc01fbbf096` /
  `80fca3b2e9ad20bc3cb1fcefb2780f057108fb6a5c07738d39ebfe17a40d3754`;
  actual served hashes and candidate hashes must be recorded anew at capture.

## Frozen pair and gates

Two cases differ only in the normal-UI global Symmetry checkbox: `off` and
`on`. Both use SculptGL tool index 7 / OBrush opt-in Source Drag, orthographic
projection, 1024 × 808 logical viewport, public import, Reset and fit/center,
96 px radius, Alpha None, no Continuous, no Lock, no culling or negative,
pressure factors zero, ordinary mouse pressure 1, and a trusted integer
pointer path at viewport-center offsets
`(-100,-20) → (-64,-20) → (-28,-20) → (8,-20)`. Both use the same
12-terminal-frame/250 ms settled-camera barrier and the same public normal UI
Save/Undo/Redo route as C295. There are three held GPU captures, and no UI
Save or click during a held stroke. All browser runs use owned ports and
processes, never root port 8792.

Before scoring geometry, each case must match X/Y projection ratios, the four
requested world rays and world radius within `1e-8`; verify settings,
trusted pointer delivery, one history step and unchanged mesh/attributes.
For each public Before/After/Undo/Redo and three held stages, maximum mapped
RGB-ID XYZ and **active GPU display-normal** component difference must be
`<=1e-6`; saved normals are not the display oracle. The exact changed-ID
membership must match the paired reference. Public Undo must restore the exact
own Before bytes and Redo the exact own After bytes; active GPU position words
must match each own public Save, with one active VBO, matching object identity,
quiet After GPU words and unchanged topology. After→Redo active-normal changed
RGB-ID membership must match. Within each host, ID541 and half-masked ID545
must move, protected ID544 must remain byte-exact; with Symmetry on, ID548
must move. The matrix gate requires Symmetry on to increase ID548's After
world-space displacement by more than `1e-6` versus Symmetry off for each
host. This establishes a measurable mirrored response while retaining mask
witnesses. C295's exact prior gate also remains a required regression check
for any product candidate.

Frozen scripts and SHA-256:

| Script | SHA-256 |
| --- | --- |
| `scripts/check-sculptgl-source-drag-c300-ui.mjs` | `68d08a3bfcf487881e70e3039e04b9cd2f9dc89a375bfa58cc6afd867f79b354` |
| `scripts/audit-sculptgl-source-drag-c300-ui.mjs` | `c023be7a3a7083629a15c66c327cfa16adbdb70a8aa455877c9cb6dcb8649094` |
| `scripts/audit-sculptgl-source-drag-c300-matrix.mjs` | `aa8287a8ccb92ee538400b8bdf69883475f6b1f1cd1c59fd73d73af002983fff` |

`node --check` passed for all three. Capture each side/case once on the
unchanged product first; preserve any setup or numerical failure. The concrete
known baseline gap is OBrush's explicit Source Drag symmetry refusal, but a
visible checkbox alone cannot pass. Only a shared-core/host implementation
that passes this fixed matrix, C295 regression, rejection/undo tests, fmt,
workspace tests, strict native/WASM Clippy, WASM check, web build and browser
check may be proposed for integration. Do not relax these gates or repurpose a
failed C300 result.
