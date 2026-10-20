# C309 default Smooth: prospective normal-UI pair

This is a prospective comparison, not a parity result. C285 covered 64 px,
50% Smooth Relax-only on/off on one 441-vertex curved grid. C309 uses an
independently authored asymmetric 625-vertex curved grid and SculptGL's
default Smooth radius 50 px and intensity 75%. It tests the ordinary default
Smooth path (Relax only off) plus a Relax-only on control, so a difference
between those algorithms must be visible. No product change or reference run
preceded this preregistration. The existing C283 failed setup and C285 pass
remain separate, unchanged evidence.

Reference identity: SculptGL source commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, locally pinned bundle
SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
Independent fixture:
`tests/fixtures/independent-default-smooth-c309.sgl`, 41,092 bytes,
SHA-256 `bcd0a2f0e5a6787354a8bbc81d3b1ce03006a3df540b78f073cf80c36c6b3825`.
The Python author is `scripts/fixtures/author-default-smooth-c309.py`;
its RGB ID encodes each vertex ID, with fully protected ID 286,
half-unprotected ID 288, and raised ID 287. The 25×25 triangle grid has a
phase-shifted ridge and off-center Gaussian mound rather than C285's
sinusoidal spike field. The authored SGL is not an export from either app.

Frozen normal-UI harness SHA-256:
`b581affddb863034b71cef7b3dd54d227afbcc9d0d583bd80f1bda91b0cff08a`.
Frozen audit SHA-256:
`4bbf11245783d5c09e7c6c191d0c1365bf9ea48366f3a64f2880b63bd76aaec0`.
Before execution, verify those hashes, source commit, fixture and served
OBRush JS/WASM hashes. Run four fresh documents in order: Source off,
OBRush off bound to the Source published camera/radius, Source on, OBrush on
bound likewise. Use a 1024×808 logical viewport, front orthographic reset
and settled fit, one trusted integer mouse gesture from viewport center
`(+3,-20)` to `(+16,-20)`, then release. Source Smooth must publish 50 px,
75%, no pressure, Alpha None, Culling/Thin surface/Symmetry/Continuous/Lock
off. OBrush must visibly select Smooth and `SourceOrdinary`, set pressure
factors zero, Alpha None, Culling/Fold prevention/Symmetry/Continuous/Lock
off; Smooth tangent follows the off/on case. No invisible direct sculpt or
mesh mutation is allowed. Record the normal UI and GPU observations, public
SGL Save before/after/Undo/Redo, exact build hashes, and all failures.

The frozen paired audit gates both on/off cases:

- Published X/Y projection ratios within `5e-6` of unity; world-radius and
  each requested world-ray difference at most `1e-6`.
- Exactly one trusted pointer down, one held move and one pointer up at the
  requested integer positions; one held GPU snapshot and one Undo step.
- After and Redo positions equal within `1e-6` per mapped XYZ component;
  Before mapped XYZ exactly equal; held active GPU positions within `1e-6`.
  Changed RGB-ID membership must match, with ID 287 changed and protected
  ID 286 fixed.
- Active GPU display normals match within `1e-6` per component at Before,
  held, After, Undo and Redo. GPU positions must equal each own public Save
  at Before, After, Undo and Redo, with one active object/VBO and no GL error.
- Per-side Undo Save bytes equal Before; Redo bytes equal After; original
  topology and non-position attributes remain unchanged; post-release GPU
  words remain quiet.
- The off/on center-spike effect differs by at least `1e-4` in each app, so
  the control is capable of discriminating tangent smoothing.

Only after the unchanged baseline is captured may a specific divergence be
diagnosed and a product edit attempted. Any setup failure, numerical failure,
or later changed protocol gets its own retained receipt and new preregistered
run ID. One grid, one radius/intensity pair and one gesture cannot close the
full `SGL-BRUSH-SMOOTH` inventory row or ZBrush parity. Source WebGL1 upload
shadow versus OBrush WebGL2 readback is a display-observation limitation;
saved normals are not accepted as a display-normal oracle.

Static source inspection before execution: the pinned `Smooth.js` defaults
to radius 50, intensity 0.75 and `_tangent=false`, computes Laplacian means
before per-vertex blending, multiplies by material protection and sampled
alpha, and updates geometry after the stroke. OBrush's shared-core
`source_smooth.rs` likewise precomputes means before candidates and uses
the published Smooth options. This inspection predicts a plausible match,
but does not establish one or excuse any frozen gate.
