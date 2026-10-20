# C327 preregistration: one Draw contact after changing Source resolution

**Status: frozen before browser execution. `SGL-RENDER-RESOLUTION` remains Open.**
C326 proved one Draw contact at the default 1× resolution at DPR 1 and 2,
but did not test contact after a 0.5× or 2× change. Pinned SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` multiplies its canvas,
camera viewport, mouse coordinates and brush screen radius by `_pixelRatio`.
OBRush uses a scaled scene target with host-sized camera and picking. A
resolution-only change should therefore preserve the model-space result of
the same CSS-point contact within each app. This is the hypothesis, not an
accepted result.

## Frozen inputs and normal-browser route

- Pinned SculptGL bundle SHA-256
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
  mirror wrapper SHA-256
  `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
  The independent sphere OBJ is
  `crates/obrush-core/tests/fixtures/independent-reference-sphere.obj`,
  24,272 bytes, SHA-256
  `165e2cfacb2f0d99312fd3f4f3f79c2696be3eee9b05047ac7f57c891783ba9b`.
  The script records the exact OBrush source commit and official portable
  WASM hash before interpreting data.
- Owned Chrome at 1600×1000 CSS, 100% zoom, DPR 1 and 2 as separate runs.
  Each app/ratio uses a fresh page and imports the same OBJ through its normal
  file input. There is no replay/Undo reset between ratios. The imported
  sphere must be selected and its center visible. In each app, choose the
  center of that app's measured canvas/viewport in CSS pixels; that fixed
  coordinate is reused across its three ratio runs.
- Ratios are exactly 0.5, 1 and 2. The pinned reference may use its public
  `onPixelRatio` GUI controller because the mirror topbar lacks a reliable hit
  target. OBrush must use the visible `viewport.resolution` slider. Neither
  application receives a direct geometry or input API call. CDP delivers a
  trusted mouse move, left press and release at the frozen CSS coordinate.

## Acceptance and stop rules

1. Before contact, log DPR, CSS/backing/scene pixel dimensions, selected
   object, brush radius/strength and history. Require each app's three ratios
   to preserve its own selection and history. Reference backing pixels must
   equal its CSS viewport multiplied by ratio (±1); OBrush scene target must
   equal its CSS scene size multiplied by ratio (±1) and stay within its
   renderer's allocation budget. Browser/GL/scene errors fail the run.
2. The cursor/ring center must lie within 2 CSS pixels of the planned contact
   in OBrush; capture screenshots before and after. Require an actual picked
   sphere and a changed vertex set after exactly one released stroke. In
   OBrush, one processed/applied dab and Undo +1 are required. Other objects
   must remain unchanged; a no-op or camera action fails.
3. Compare each application's own 0.5× and 2× outputs to its 1× output.
   Require identical vertex count and changed-vertex IDs, and maximum
   coordinate difference ≤1e-5 model units after one contact. OBrush public
   project before contact must be byte-identical at all ratios; after-contact
   project bytes may differ in metadata, so compare numeric geometry and
   history. Source positions are read from its active mesh after normal UI
   input. Compare the projected displacement centroid to the contact within
   2 CSS pixels in OBrush.
4. Preserve every failed receipt. If reference itself varies by ratio, report
   that separately; do not declare OBrush parity merely because both vary.
   If OBrush alone fails, diagnose the recorded deltas before proposing a
   bounded product change. Do not promote C327 on a harness failure.

This gate does not cover long strokes, pressure, symmetry, other tools,
arbitrary meshes, native GUI, browser zoom other than 100%, exact cross-app
brush shapes or full SculptGL/ZBrush parity.
