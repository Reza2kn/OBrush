# C299: remove unintended Source UV clay tint

The [frozen C299 check](SCULPTGL_UV_TINT_C299_PREREG.md) identified a Source renderer inconsistency: with vertex color disabled, the UV fragment multiplied imported JPEG texels by the default object color. The correction samples UV image color directly and applies vertex-color tint only when **Show vertex paint** is enabled. Core geometry and file-format code were not changed by this correction.

Using the ordinary OBrush browser at 1280×720, the same independently authored C294 OBJ and JPEG loaded through visible **Open**, **UV texture**, and **Import UV image** controls before and after the correction. The baseline screenshot `.artifacts/c299/obrush-before.jpg` (SHA-256 `aef23a8c881b4928c2099e0b5550632b5f0e761b75851695775571c820572897`) and corrected screenshot `.artifacts/c299/obrush-after.jpg` (SHA-256 `29ebcda9b36eacff4943b10a3abea0ef23cf2393b1bd2572fceb278a57ecbd31`) use the same viewport and interior sample coordinates `(550,270)`, `(735,270)`, `(550,460)`, `(735,460)`:

| Quadrant | Before RGB | Corrected RGB | Pinned SculptGL C297 RGB |
| --- | --- | --- | --- |
| Red | 132,28,17 | 216,63,47 | 223,67,52 |
| Green | 34,77,23 | 100,209,90 | 104,213,94 |
| Blue | 21,18,61 | 47,66,222 | 47,68,223 |
| Yellow | 139,84,28 | 238,218,83 | 245,223,86 |

The largest difference among these representative sample channels fell from 162 to 7. The pinned reference used a different viewport and camera, so this is a color-response diagnostic rather than calibrated matched-pixel parity. Quadrant orientation remained correct; the imported mesh still showed four vertices and two faces. No browser error was visible.

The independent half-red vertex-color OBJ `.artifacts/c299/c299_uv_halfred.obj` (SHA-256 `aa07a8eb2039478d602c69f6fd851e2def3290b2dfa8f970a6231e9ca3c1ca9a`) tested the toggle on the same JPEG. With **Show vertex paint** on, red and yellow centers were RGB `(107,47,36)` and `(144,215,77)`; with it off, `(216,63,47)` and `(238,218,83)`. Green and blue changed at most 3 and 16 channels respectively. On/off screenshots have SHA-256 `aecb0dc4b09c1699f6a41ac08a56b22255ebbd29f997d85d2589bd53489dfb36` and `2583a6d68901a3ef9db67983f735a0b57d850458488264dd8aa72f1315119a51`.

Integrated verification after C298 passed `cargo fmt --all -- --check`, complete `cargo test --workspace --locked --quiet`, strict native and WASM Clippy, `bash scripts/build-web.sh`, and `bash scripts/package-macos.sh` with valid local code signature. The portable WASM SHA-256 was `d6b7939cc5dd0e0180fd364ba2a2e5f4f738e6f5d1a21d04d4954dade447a9e9`, SIMD WASM `ef43e0fce92e674d13daf4f174b938d3874ca837afd08b6f7058daab335631af`, and macOS executable `5d9a6264e19eb8f2c5658ef649395521bff5a5eddba90bf4e9ac8d477995a6d3`. Directly invoking `scripts/build-web.sh` first failed with permission denied because the script is not executable; invoking it through `bash` passed. The remaining full UV import, native GUI, calibrated reference-pixel and SculptGL/ZBrush parity gates remain open.
