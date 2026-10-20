# C294 UV image import: frozen JPEG extension contract

This contract precedes C294 product edits. The active inventory row is
`SGL-RENDER-IMPORT-UV`. OBrush already imports bounded PNG images for its UV
viewport; the current visible chooser, native picker, and request completion
reject JPEG. The pinned SculptGL chooser accepts `image.*`. This scoped JPEG
extension does not close the full row or establish matched color rendering.

## Identity and independent fixture

- Source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
  `src/gui/GuiRendering.js` SHA-256
  `af2ac44cff1ace87eeeea8a352b5b5dd197feaa36684a6a585526b719d9bbc95`;
  local hosted `sculptgl.js` SHA-256
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
- OBrush pre-product HEAD `0e0c630e128ab99182c81813ce9be6fb3554ed83`.
- Authored two-triangle UV plane `c294_uv_quad.obj`, SHA-256
  `10ff58d736037223672442a8e4bbeb5697224b9c65cb09235472beddb8845819`.
  Counterclockwise +Z faces map corners `(0,0),(1,0),(1,1),(0,1)`.
- Authored 64×64 RGB image: top-left red `(240,45,35)`, top-right green
  `(35,215,70)`, bottom-left blue `(40,65,230)`, bottom-right yellow
  `(245,220,45)`, each channel offset by `(x % 8)-4`, then clamped. PNG twin
  `c294_uv_quadrants.png` SHA-256
  `384666bb9aa66c8150ca5c11a72fb860eb95e171d042c9ad490bd6194e21e441`;
  JPEG 4:4:4 quality 95 twin `c294_uv_quadrants.jpg` SHA-256
  `98956aa6d0717701f23d34c3414c371bafe1e6ed9c45914aec5217f454926b84`.
  Both were generated locally with Pillow 12.3.0; no reference asset is copied.

## Frozen acceptance gates

1. Use visible File Add/Import to load the OBJ and visible UV texture import
   action to load each image in pinned SculptGL and a fresh C294 OBrush normal
   browser session. Select UV rendering. Record URLs, product hashes, DPR,
   canvas dimensions, controls, and screenshots. Both JPEG and PNG must be
   accepted. On-mesh fixed corner samples must preserve the red/green/blue/
   yellow quadrant ordering. Record exact sample coordinates and RGBA words;
   cross-app numerical equality is not presumed without calibrated shading.
2. OBrush GPU metrics must report the active 64×64 texture. If ordinary WebGL
   upload interception is feasible, record exact decoded RGBA sample texels,
   bottom-up row policy, resource size and GL errors. A source image upload
   and OBrush Rust byte upload must first be normalized for unpack flip.
3. In OBrush, a malformed JPEG, JPEG with dimensions 2049×1, and input over
   16 MiB must fail before replacing the previous texture. A stale/cancelled
   request also must not replace it. The same Rust decode/ownership path must
   serve native and browser hosts. Valid PNG behavior must remain intact.
4. Loading, replacing, failing, and resetting a UV viewport image must leave
   public OBJ/project geometry, selected object, revision and Undo/Redo stack
   unchanged. Save before/after when possible. A texture is a viewport asset,
   not embedded into `.obrush`; this session-only limitation remains explicit.
5. Run `cargo fmt --all`, complete workspace tests, strict native and WASM
   Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, and the normal
   browser pair. Preserve failed or unmeasured gates. Do not promote full UV
   import parity from this one-format extension.

The 16 MiB compressed-input, 2048-pixel-side, 16 MiB decoded RGBA, and
22 MiB mipmapped GPU preflight limits remain in force. File format detection
uses magic bytes, so a JPEG renamed `.png` does not bypass decode policy.
