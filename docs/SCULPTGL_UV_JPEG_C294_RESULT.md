# C294: bounded JPEG UV image import

The [frozen C294 fixture and gates](SCULPTGL_UV_JPEG_C294_PREREG.md)
target one current gap in `SGL-RENDER-IMPORT-UV`. The ordinary OBrush browser
and native chooser now accept PNG/JPEG, decoded by the same bounded Rust image
path. A 16 MiB input cap, 2048-pixel side limit, decoded-storage cap and
renderer texture-size preflight occur before replacing the live viewport
asset. Malformed reads retain the previous image. The inspector explicitly
says that imported UV images are session-only. This is a format extension,
not a complete SculptGL UV import parity result.

## Identity and checks

- Product prereg commit `9b7b556`; pre-product parent
  `0e0c630e128ab99182c81813ce9be6fb3554ed83`.
  Source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and exact
  fixture/source hashes are in the prereg.
- `cargo fmt --all`, complete `cargo test --workspace --locked --quiet`,
  strict native and WASM Clippy, `scripts/build-web.sh`, and
  `scripts/package-macos.sh` passed. After changing only the UV diagnostic
  phrase from PNG-specific to image-generic, app library tests (776/776),
  strict native/WASM Clippy, web and macOS builds passed again. The package
  passed its ad-hoc code-signature check.
- Final portable WASM SHA-256
  `d7c360c8285440742b0595cd6901a8b7f07f42e73882e27d6876f7a4fa93a8d3`;
  SIMD WASM
  `081285e3fb8b20847c88f39d274f59208ea340b83fa74f94904841d60bdd5240`;
  macOS executable
  `e546f3691779657c882b4a4001ff48b734e4f60b356667688eba027a2151071c`.

The normal-browser run used own OBrush port 8796, pinned local SculptGL port
8797, Chrome at DPR 1 and 1600×1000, and the independently authored two-face
OBJ and 64×64 PNG/JPEG twins. OBrush's visible **Open** and **Import UV image…**
actions loaded the OBJ and JPEG. Its renderer reported 4 core vertices, 2
triangles, 4 UV render vertices, a 64×64 mipmapped GPU texture occupying
21,844 bytes, and no scene error. The imported image's screenshot SHA-256 is
`2123a5c8cb2824da6e551bade67d3b06d88ee51c2be944ccf1f7f178a2327222`.
On-mesh CSS-pixel samples `(650,340),(980,340),(650,680),(980,680)` were
respectively `146,11,4,255`, `18,81,14,255`, `20,18,64,255`,
`151,83,4,255`: the authored red/green/blue/yellow quadrant order is intact.
The rendering values are darker than source RGB and are not used as exact
reference color claims.

The normal chooser's corrupt four-byte JPEG displayed a rejection and kept
the JPEG name, 64×64 texture and GPU resource. A subsequent normal chooser
loaded the PNG twin. The mesh load set document revision 2 and Undo/Redo
depths 1/0; JPEG load, invalid JPEG, and PNG load all retained those values.
The host/browser route uses the same Rust decoder and request ownership.
Unit checks cover exact bottom-up PNG words, JPEG color bounded by 8 per
channel on four interior samples, malformed JPEG, header width 2049, and
input over 16 MiB. The product logic was browser-tested on portable WASM
`7fc06324c33e8086e698362da76bcb8f94cd0237fc2fdc3d8c8c7af838d2df32`;
the final rebuild differs only in the diagnostic wording and was not opened
in the browser. The exact browser state receipt is ignored
`.artifacts/c294/pair-state.json` (SHA-256
`0b6476972eae598e56fbac655a64baa966793555d9316e085801f5f4a1fc86ec`).

## Preserved failures and open gates

The first full workspace test attempt found a test-only closure lifetime
compile error; changing the fixture helper to return a four-byte value fixed
it, and the full suite subsequently passed. Early browser harness attempts
searched for an offscreen inspector control and once scrolled past it; those
failed attempts are retained under ignored `.artifacts/c294/`.

The pinned reference imported the same OBJ through visible **Add** and
exposed its 4 UV coordinates. Headless Chrome's native shader select did not
settle on UV mode: repeated trusted key sequences selected MatCap (value 5)
instead of UV (value 4). Consequently the screenshot taken after setting
the hidden UV file input is **not a valid UV comparison** and the frozen
reference quadrant/pixel gate is unqualified. Its SHA-256 is
`215310e31568f63e48b77c1de1206064eeafb2fcc1e452d0176646024768a210`.
No direct DOM state mutation was substituted for the normal UI. OBrush's
oversize-header and over-16-MiB chooser matrix, stale request in a normal
browser, exact GL upload texels, public Save/Undo/Redo comparison, native GUI
chooser, and matched reference appearance remain unmeasured. The full
`SGL-RENDER-IMPORT-UV` row stays open.
