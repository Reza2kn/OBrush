# C311 prospective default Smooth footprint-corrected pair

C309's frozen normal-UI audit failed because its authored spike/protection
controls at RGB IDs 286–288 were outside the actual brush footprint. Both
programs did change IDs 336–338 under the same 50 px/75% gesture. C311
therefore independently authors a new SGL v3 mesh with the raised control
at RGB ID 337, fully protected neighbor at 336, and half-unprotected
neighbor at 338. The earlier C309 failure and its harness/audit remain
unchanged. This document fixes the C311 protocol **before** its Source or
OBRush run; the observed C309 membership informs control placement, not
any C311 output or tolerance.

Reference source commit:
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
pinned bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independent Python author is
`scripts/fixtures/author-default-smooth-c311.py` and its new
`tests/fixtures/independent-default-smooth-c311.sgl` is 41,092 bytes,
SHA-256 `46d9e6de3587335009127dc7c15f70dd0a82db7a2c26711dbbff588514a66303`.
It is a 25×25 triangle grid with phase-shifted ridge, off-center Gaussian
mound and a raised vertex at grid `(12,13)`. RGB vertex IDs encode identity.
The SGL file is authored directly rather than exported from either app.

Frozen normal-UI harness SHA-256:
`3f484f3d9445c5a36a649d01ebcbcc488c56c8734c3392b5589dce36f217f4de`.
Frozen paired audit SHA-256:
`5ec81c7406f2ae7f7bc17bf5c54c3146d13b8947b6eb2a1f4623535932f9016a`.
Before execution verify those hashes, fixture and served OBrush JS/WASM
hashes; keep any failed setup or numerical run. Four fresh documents must
be captured: Source Smooth Relax-only off, matched OBrush off, Source on,
matched OBrush on. The OBrush root run binds the Source's published radius,
intensity and settled projection; a failed camera preflight remains a
failure receipt even if an unchanged-harness fresh retry is attempted.

Use both ordinary browser UIs in a 1024×808 logical viewport, front
orthographic reset and settled fit. Deliver one trusted integer mouse
gesture from viewport center `(+3,-20)` to `(+16,-20)` and release. Smooth
uses radius 50 px, intensity 75%, no pressure, Alpha None, no Culling,
Thin Surface/Fold prevention, Symmetry, Continuous or Lock. OBrush must
visibly select Smooth and `SourceOrdinary`. The only on/off variable is
Relax-only/tangent Smooth. Capture public Save at Before, After, Undo and
Redo, one held GPU snapshot, active object/VBO identities, settings,
camera/radius/ray inputs, and delivered trusted DOM events.

The frozen C311 audit requires:

- X/Y projection ratios within `5e-6` of unity; world-radius and requested
  world-ray differences at most `1e-6`.
- Exactly one trusted pointer down/held move/up at the requested positions,
  one held GPU snapshot, one Undo step, no GL error, and one active object/VBO.
- The same changed RGB-ID membership in Source and OBrush in both modes,
  including raised ID 337 and half-unprotected ID 338; fully protected ID
  336 must remain fixed. Initial mapped XYZ must match exactly. Paired After,
  Redo and held positions must match within `1e-6` per component.
- Paired active GPU display normals at Before, held, After, Undo and Redo
  within `1e-6` per component. Each app's active GPU positions must exactly
  match its public Save at Before, After, Undo and Redo; post-release GPU
  position/normal words must remain quiet.
- One-step public Undo restores exact Before Save bytes; Redo restores exact
  After bytes. Topology and non-position attributes remain unchanged.
- The raised ID 337's on/off final displacement differs by at least
  `1e-4` per app, showing the toggle is discriminating on the corrected
  fixture.

No product edit is allowed before this unchanged baseline. If any frozen
gate fails, retain its evidence and preregister a distinct follow-up before
changing the fixture, harness, tolerance or product. Even a pass supports
only this 625-vertex default Smooth paired gesture, not the whole Smooth
inventory row, wider mesh quality or ZBrush parity. Source WebGL1 upload
shadow and OBrush WebGL2 readback remain distinct display observables;
saved normals are not used as the display-normal oracle.
