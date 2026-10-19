# C247: viewport resolution, scoped SculptGL comparison

**Gate: Open.** OBrush now exposes a Source `Resolution` slider from 0.5× to
2×, in 0.02 steps, defaulting to 1×. It changes the scene's pixel resolution
without changing geometry, document history or saved project bytes. This is a
scoped rendering control, not a claim of viewport or full SculptGL parity.

## Reference and implementation

The pinned SculptGL source is commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundled JavaScript SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
and browser wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
Its extra topbar has `_pixelRatio=1` and a 0.5–2 slider at 0.02 steps.
`onPixelRatio` calls `onCanvasResize`: CSS dimensions stay fixed while canvas
backing pixels, camera dimensions, render targets, pointer coordinates and
screen-space brush radius use the ratio. The value is session display state,
not a scene/history edit.

OBRush uses the shared native/browser Rust renderer to draw the scene to a
scaled RGBA8/depth24 framebuffer and composite it into the unchanged egui host
canvas. This is deliberately narrower than SculptGL's whole-canvas backing
resize: OBrush's chrome and overlay text remain at host resolution. At 1× the
offscreen target is released and the direct path is used. New targets replace
old ones on size changes; allocation is rejected above 128 MiB or the GPU's
maximum renderbuffer side, and failures show an error while retaining direct
rendering. Resolution is not serialized into `.obrush`, and no core geometry,
file IO or history path changed. The renderer keeps the host frame size for
camera and picking while adjusting scene raster dimensions.

## Paired browser evidence

`scripts/check-sculptgl-resolution-c247.mjs` ran in an owned 1600×1000 Chrome
viewport against the pinned mirror and isolated OBrush build
`ee132c4211e436bed7275cf5ebca1f4717cfc5d1`. The browser's baseline WASM
SHA-256 was
`24d9c4f440ef27fab530e40d70b4f10031bd8b09402b0d568abb8e349360f140`
(SIMD WASM: `a933f3987ade5ac8579e3dd1692a5203b06facc9e814444fbbd5af704052e114`).
The macOS package executable SHA-256 was
`fd323c333a8b6615c575d16569286ff745e876c96b0dae9e52e96d5e3dd25da1`.

The independent two-mesh SGL fixture is 540 bytes, SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`.
Its passing receipt, screenshots and exact states are in ignored local
`.artifacts/sculptgl-resolution-browser/e779f6ed-b114-4b72-a2b1-5a55b0fc02b2/`.
A larger independent sphere OBJ, 24,272 bytes, SHA-256
`165e2cfacb2f0d99312fd3f4f3f79c2696be3eee9b05047ac7f57c891783ba9b`,
was run with `--sphere`; its passing receipt is
`.artifacts/sculptgl-resolution-browser/d22b512e-2652-4a45-8f02-b861586b8ad7/`.

| Ratio | SculptGL CSS / backing pixels | OBrush host / actual scene pixels |
| --- | --- | --- |
| 1× | 1290×960 / 1290×960 | 1600×1000 / 1140×912 |
| 0.5× | 1290×960 / 645×480 | 1600×1000 / 570×456 |
| 2× | 1290×960 / 2580×1920 | 1600×1000 / 2280×1824 |

The OBrush scene sizes above were confirmed by WebGL texture/depth allocation
instrumentation, not just slider labels. All six states in each run had zero
WebGL errors and no OBrush scene error. SculptGL's reference history remained
two entries; OBrush retained Undo=1, Redo=0. OBrush's downloaded project was
byte-identical at all three ratios: SHA-256
`54ad6bf55534ed793aaadb20e7818254b12ffc27901848620b3161db87962d4b`
for the two-mesh fixture, and
`a0e3553d8e1223f112f771b90e3a3ed94302b92205bd3caf84f9cc9b7c216f08`
for the sphere. The screenshots have distinct hashes for all three ratios in
each app.

For a fixed 850×830 screenshot crop over the large sphere, the 1× to 0.5×
comparison changed 139,907 pixels (19.83%) in pinned SculptGL and 126,746
(17.97%) in OBrush. The 1× to 2× comparison changed 65,807 (9.33%) and
56,544 (8.02%) pixels respectively. These are within-app pixel counts; the
different cameras, shaders and UI layouts make cross-app pixel equality an
invalid inference.

## Validation and limits

`cargo fmt --all -- --check`, all workspace tests, strict native and
`wasm32-unknown-unknown` Clippy, `scripts/build-web.sh` via `bash`, and
`scripts/package-macos.sh` passed on the isolated worktree. Logs are in
`.artifacts/resolution-c247/`. The package's ad hoc signature verified.
The first two browser attempts failed in the test harness: one reused a
stale slider rectangle after Save, and the other tested an offscreen slider
against the wrong scroll threshold. Their original receipts remain at
`28c7f428-0815-4c46-881c-825f6f91133b` and
`7088e669-0129-4e0e-aeeb-5433652d4f66`, respectively; corrected reruns
passed. The initial direct `scripts/build-web.sh` invocation also failed
with exit 126 because this checkout does not mark the script executable; the
same script passed when invoked through `bash`.

The paired script invokes the pinned SculptGL resolution controller directly
because the mirrored topbar is not reliably clickable; OBrush uses its visible
slider. Browser DPR=2, native GUI interaction, sustained performance, pen
brush size during resizes, other GPU/device limits, and exact shader/pixel
appearance remain unchecked. Full `SGL-RENDER-RESOLUTION` acceptance remains
open, as do SculptGL and ZBrush parity.
