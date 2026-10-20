# C291 custom MatCap import: frozen test contract

This contract precedes any C291 product edit. The inventory row is
`SGL-RENDER-IMPORT-MATCAP` (active, open). OBrush already has nine built-in
MatCaps and a MatCap shader; the missing action is importing and selecting a
user image. This slice does not close the row or claim full SculptGL parity.

## Frozen reference and independent fixture

- SculptGL source commit: `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`.
  Unchanged hosted `sculptgl.js` SHA-256:
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
  `src/gui/GuiRendering.js` SHA-256:
  `af2ac44cff1ace87eeeea8a352b5b5dd197feaa36684a6a585526b719d9bbc95`.
  `src/render/shaders/ShaderMatcap.js` SHA-256:
  `43ef3adeee7a0377348a47a2d3a135b56ac0affde6d67d86f5ecc9d83d85a739`.
- OBrush before product edits: `6baac8c1a36dac03c111ff008536d5e62081e482`.
- Independent image: `crates/obrush-app/tests/fixtures/c291_asymmetric_matcap.png`,
  SHA-256 `33cd9048b3665b9f24ba0739b9e2a2752400b3249a05cd4a31e3b7be3487ad07`.
  It is 64×64 RGBA, opaque. Top-down pixel `(x,y)` has
  `R=round(255*x/63)`, `G=round(255*y/63)`, and `B=40` when `(x<32)==(y<32)`
  else `216`. It was generated locally from that formula; no reference image
  or commercial asset is copied.

## Ordinary UI pair and pass gates

1. Start fresh normal browser UIs at pinned SculptGL and the C291 OBrush
   build, each on a fresh sphere, MatCap shader, the same 64×64 viewport CSS
   region for sampled image points, and no sculpt stroke. Record both URLs,
   bundle/product hashes, device pixel ratio, canvas size and controls used.
   Import the fixture through each visible **Import MatCap** action and native
   file chooser (test automation may fill the chooser input after the action).
   The new option must be named `c291_asymmetric_matcap.png`, selected, and
   visible on the mesh without another action.
2. Record the visible reference result before adapting OBrush. The uploaded
   image must orient left/right and top/bottom as the fixture defines. Capture
   a whole-canvas PNG and four fixed on-mesh samples on each side, with exact
   CSS coordinates and RGBA values; compare side/channel ordering. A visual
   comparison is qualitative unless camera, color management and pixel
   coordinates match; no post-hoc numerical tolerance is invented. Where
   feasible, inspect the ordinary GPU texture upload: width/height 64×64,
   `UNSIGNED_BYTE RGBA`, and exact normalized texel words at corners and
   center. A source image-upload call and a Rust byte-upload call are
   normalized for the WebGL unpack-flip state before comparison.
3. Select a built-in preset and reselect the imported image in both UIs.
   The same named option and rendered orientation must return. Resize each
   canvas once and repeat the four samples. Multiple valid imports must
   append distinct options. OBrush's documented safety cap may reject later
   imports; such a rejection is an explicit scope difference, not a pass for
   unrestricted reference behavior.
4. On OBrush, malformed PNG/JPEG, a PNG header declaring 2049×1 pixels,
   input over 16 MiB, stale/cancelled chooser responses, and catalog-cap
   exhaustion must retain the prior selection, texture and document. The
   error must be visible. Decode and GPU allocation limits must be checked
   before selection changes. A valid JPEG import is checked separately on
   both browser and native paths.
5. Save the public project before and after import and require exact geometry,
   material and history state. Undo/Redo must retain the edit stack and must
   never silently select a different built-in MatCap. Reloading a project may
   drop the custom MatCap because current `.obrush` has no embedded image
   contract, but the UI must say it is session-only. Source per-object
   MatCaps remain preset-only for this slice, so no custom numeric index may
   leak into saved Source metadata. That persistence gap stays open.

Run `cargo fmt`, all workspace tests, strict native and WASM Clippy,
`scripts/build-web.sh`, `scripts/package-macos.sh`, normal OBrush browser and
pinned SculptGL browser checks after implementation. Preserve every failed
gate and any case that could not be measured; a build or isolated unit test
does not qualify the row.
