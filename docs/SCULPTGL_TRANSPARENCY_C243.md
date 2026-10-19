# C243: selected-part Source transparency

Pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
maps its Rendering > Transparency percentage to `opacity = 1 - value / 100`
on every selected mesh (`GuiRendering.js`, `onTransparency`). Its transparent
classification begins below opacity `0.99` (`Mesh.js`); `Scene.js` draws opaque
meshes first and depth-sorted transparent meshes afterward. Each transparent
mesh draws back and front faces with blending on and depth writes off. Its
wireframe overlay draws before transparent shells, and picking does not exclude
zero-opacity meshes. The controller does not add a state-history entry.

OBRush already had per-part opacity and that GL rendering path, but its Source
Appearance control exposed opacity directly and edited only the active Source
part. This change exposes the reference's integer transparency percentage and
applies it atomically to all selected Source parts through `obrush-core`.
Ordinary non-Source opacity remains separate. The selected edit validates the
complete scene, rejects an out-of-range percentage or mixed Source/plain
selection without publication, and creates one undoable document edit under
OBRush's established Source-appearance policy. That history behavior differs
from pinned SculptGL's history-neutral Rendering controller.

The independently authored overlapping two-mesh SGL fixture is
`crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl` (540 bytes,
SHA-256 `99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`).
The pinned hosted reference bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
its wrapper is
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
The browser script `scripts/check-sculptgl-transparency.mjs` uses that fixture
in both browsers at 1600×1000. The default sphere is hidden so the two imported
parts are visible, and both parts are selected. It instruments actual WebGL
draw calls, takes screenshots, and uses OBrush's visible Source Add, scene
selection, slider, Save, Undo/Redo, and W controls. The pinned reference's
Rendering controller is called through its hosted application instance.

The final paired receipt is the ignored local artifact
`.artifacts/sculptgl-transparency-browser/445490d8-d3ff-453b-ac4f-135aea1cb43e/result.json`.
At Transparency 0%, 50%, and 100%, the two selected reference opacities are
respectively `[1,1]`, `[0.5,0.5]`, and `[0,0]`; OBrush's public `.obrush`
Save files contain those **exact same pairs** for source IDs 2 and 3. The
reference's unselected sphere retains opacity 1. At 0%, both applications
issue opaque part-triangle draws with blending off and depth writes on. At
50% and 100%, both issue transparent 3- and 6-index part draws with blending
on, depth writes off, and front/back culling values `1028`/`1029`. At 50%
with wireframe enabled, both issue indexed line draws of 6 and 8 before the
transparent shells. `gl.getError()` is zero in every sampled case. These
matching GL states and screenshot hashes establish a bounded path check;
pixel equality was **not** measured.

The reference history depth remains 2 through those Rendering changes. OBrush
undo depth rises by one for each percentage change (3, 4, 5 in the final
receipt, including the prior visible sphere-hide edit). Public Save SHA-256
at 50% is `46c0d68d486b3b3deacfdbccb9762fddf4f30d3f4c729583451d4a868cacf461`;
at 100% it is `02675e7d242863be66dccecfe32cab7a9e4ad8ce7496b3b2d43e044afd6c8f12`.
One Undo recovers the exact 50% bytes and Redo recovers the exact 100% bytes.
Focused shared-core tests also cover geometry identity, SGL export, a legacy
Source record without render metadata, invalid/mixed-selection atomicity, and
opacity-independent ray picking.

The first focused core test failed because its fixture assertion assumed the
second part started opaque; its authored value is 0.65. The assertion was
corrected to capture the original value. Earlier browser attempts failed on
scene-control timing, a control inventory mismatch, an ineffective synthetic
Save shortcut, and a harness variable; those failures were retained under
ignored `.artifacts/sculptgl-transparency-browser/` and not counted as
product passes. A later paired attempt at
`5c4acaff-407c-4447-9283-04bbbc79f73f/result.json` was incorrectly labeled
passing by the harness's ±0.011 tolerance even though its nominal 50% slider
hit saved opacity 0.49. The final script requires **exact** saved 0.5 and the
final paired receipt above passes it. C167's transparent *PNG brush alpha*
report concerns a different feature and is not viewport-transparency evidence.

`cargo fmt --all -- --check`, the locked workspace/all-targets test suite
(2,540 passed, one ignored), strict native and WASM Clippy with `-D warnings`,
`scripts/build-web.sh`, and `scripts/package-macos.sh` passed. Logs are retained
under ignored `.artifacts/transparency-c243/`. The browser-tested WASM SHA-256
is `aa4aff8a64c2ee36f88dd83f856657fc96efb82865f528a2123c5eaedac95d36`;
the signed macOS package executable SHA-256 is
`b2db991a37148ca3831832d33e51e52831457dcaaf2653e2bfbe161b451ed23d`.

## Remaining gates

This is scoped evidence for `SGL-RENDER-TRANSPARENCY`, not a completed
acceptance gate. Matched viewport pixels, depth ordering across more than two
overlapping parts, zero-opacity selection by physical pointer in both browsers,
transparent static/dynamic/multires surfaces, native GUI appearance, and
large-scene responsiveness remain unverified. OBrush's undoable Source edits
remain an intentional history-policy difference. The frozen inventory row
stays unchanged and its active acceptance gate stays open.
