# C188 SculptGL-style URL display startup

The pinned SculptGL inventory includes `SGL-URL-GRID`, `SGL-URL-OUTLINE` and
`SGL-URL-OUTLINECOLOR`. OBrush's explicit `obrushprofile=sculptgl` startup
already captured those values and retained grid/outline as future SGL-import
preferences, but left the live initial viewport at its previous display
state. This change applies the parsed requests to the existing grid and
selected-contour renderer at boot. The ordinary OBrush profile remains on its
own defaults. The idle contour color UI now writes a newly converted color
only after an actual changed response; it previously changed exact query blue
`[0,0,1,1]` to `[4.76837158203125e-7,0,1,1]` without input.

The final owned browser driver
`.artifacts/source-url-display-c188/RUN.mjs` (SHA-256
`6cb0b6607d7b7c79f2aae22965670f3e60b1a6f79b0cb20f443db4b2ffdca667`)
navigated the normal built app to the default route and four explicit
SculptGL-profile routes. In a 1140×825 captured viewport, relative to
`?obrushprofile=sculptgl&grid=0&outline=0`, `grid=1&outline=0`
changed 15,900 RGB pixels. `grid=0&outline=1` with `outlinecolor=255,0,0,1`
changed 7,534 pixels; `outlinecolor=0,0,255,1` changed 7,572. The two
outline colors differ from each other at 7,543 pixels. Live flags and
contour-color words read back as requested, including exact red and blue
`[f32;4]` values. All four source-profile routes retained the same startup
document revision 1, 40,962 vertices, 81,920 triangles and clean state.
The ordinary OBrush route retained grid/contour off. A separate browser
navigation to `outlinecolor=256,0,0,1` rejected startup with the declared
bounded-color error; its one console error is expected. The valid routes had
zero browser exceptions, console errors and blocked external requests. The
complete result and images are under
`.artifacts/source-url-display-c188/browser-final/`; `result.json` SHA-256 is
`0c70de80f94a787c5133e933815cc2cfec094d7e9bfe4cf3477996e97daa8eb0`.

The final portable/SIMD app WASM hashes are respectively
`1d180059422948acb42a81f0e372768ea2c08357f725da8d92f7` and
`e5fb80446b58a4b88de404cfd30c77fec8d67d3861da1c9c7958cbd0577b355b`.
`cargo fmt --all -- --check`, `cargo test --locked --workspace`, strict
native and WASM workspace Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh` passed. The local macOS bundle was signed and
verified on disk; it was not exercised as a native GUI comparison.

This closes only OBrush's initial-view consumption of three already parsed URL
requests. It does not compare exact SculptGL pixels, grid depth/spacing,
contour edge/color response, malformed-query behavior, later imports,
native GUI appearance or long-session performance against the pinned
reference. The frozen 317-ID source inventory remains unchanged as a
historical gap snapshot, and neither SculptGL nor ZBrush parity is claimed.
