# C246: Darken unselected Source meshes

Pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
defines this as a global viewport switch in `GuiScene.onDarkenUnselected`.
`ShaderBase.updateUniforms` sets `uDarken` for each mesh outside
`getSelectedMeshes()`. Its shared fragment encoder multiplies linear color by
`0.3` after curvature and before mask/symmetry/opacity encoding. The checkbox
only changes that static shader state and calls `render`; it adds no geometry
history entry. The pinned flat-color and wireframe shaders bypass the shared
color encoder, so the displayed effect must be assessed by shader role.

The current OBrush Source renderer already had the corresponding global
`darken_unselected` control, captured selected-ID list, and
`finishSourceColor` condition `u_darken_unselected && !u_selected`, followed by
the same `0.3` multiplication. The on/off state is a view preference; it is
not written to the `.obrush` document or Source render metadata. Imported
Source meshes resolve to PBR, flat, normal, wireframe, UV, or MatCap shader
roles, and do not use OBrush's separate Studio clay path. No renderer change
was warranted by this source audit.

The independently authored two-part SGLv3 fixture is
`crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl` (540 bytes;
SHA-256 `99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`).
The reference is the pinned hosted browser bundle (SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`),
with wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`
used for normal-browser instrumentation. Source-file hashes are
`c609fc4416a6f20cb0407f8ce431a4f1220512296681afe899096de520998f88`
for `GuiScene.js` and
`e14b8eaef39451591d9a3d8ae92ee456833bf94c274d39c01dcc79549bc6495d`
for `ShaderBase.js`. The paired script `scripts/check-sculptgl-darken.mjs`
records each triangle draw's
selection/darken uniforms, captures four screenshots, checks reference
opacity/shader/history and compares OBrush's public project bytes before and
after the visible checkbox action. The final harness SHA-256 is
`71e215df71be0d0e625da9f5ce98072fb4f7403c5804d36dd749978bbdea3c64`.

## Paired result

The final owned-Chrome result is the ignored local receipt
`.artifacts/sculptgl-darken-browser/2a04b836-acb4-473d-bc58-ef565be2c002/result.json`.
It used validated root commit `ff3d12d017265c91543f925a6eb46f52010c3684`
and the served WASM SHA-256
`be78245e106ac1b86d9edf3cb6ea1c8d6e1273eac7198fa37b5a61e5a900ee30`.
The reference imports the fixture with one mesh selected (index 2), three
meshes total, opacity `[1,1,0.6499999761581421]`, shader codes `[0,5,0]`
and history depth 2. With darkening on, the unselected 6-index facet and
original 589,824-index sphere receive `uDarken=1`; the selected 3-index facet
receives `uDarken=0`. Turning the reference controller off sets those draws
to zero without changing selection, opacity, shader code or history.

OBRush imports the same two-part fixture through its normal Add input,
selecting ID 3 of IDs `[1,2,3]`. With its visible checkbox on, the 6-index
unselected part has `u_darken_unselected=1, u_selected=0`; the 3-index
selected part has `1,1`. With the checkbox off, both parts receive
`u_darken_unselected=0`. All four sampled frames have GL error 0. OBrush's
initial sphere draws in MatCap mode (`u_shading_mode=4`) with the global
darken switch on and its selected flag off; the pinned original sphere is
also MatCap. Their topology and camera framing differ, so this is shader-path
evidence rather than pixel equality. OBrush's public `.obrush` Save bytes are
identical before and after the view toggle:
5,320,316 bytes, SHA-256
`54ad6bf55534ed793aaadb20e7818254b12ffc27901848620b3161db87962d4b`.
The saved Source parts retain shader 0 and opacity 1/0.65. OBrush history
remains at one Undo and zero Redo.

The reference callback was invoked through its actual hosted Scene controller
because the mirrored page's checkbox input had a zero-sized hit rectangle and
clicking its text selected a mesh beneath it. OBrush's checkbox was clicked
at its visible control rectangle after scrolling the inspector. Earlier failed
browser attempts remain under `.artifacts/sculptgl-darken-browser/`: two
looked for a nonexistent visible Scene header, one assumed triangle/quad face
order incorrectly, one clicked the reference text and changed selection, and
one hit the hidden input's zero-sized rectangle. These are harness failures,
not evidence of a product regression.

The four 1600×1000 screenshot hashes differ across on/off states, but the
applications frame this fixture differently. In OBrush the imported facets
are only a few pixels wide; the pinned reference's original sphere fills most
of its viewport. In the central `(600,100)`–`(1200,900)` crop, 282 of 480,000
OBRush pixels and all 480,000 pinned-reference pixels differ between on/off;
the crop is not a matched-scene color oracle. These captures do not establish
matched appearance or a
usable visual contrast at ordinary sculpting scale. The full
`SGL-RENDER-DARKEN` acceptance gate stays **Open**. Matched viewport pixels,
transparent/masked/curved states, multi-selection, hidden parts, alternate
shaders, native GUI and larger visible imported parts remain to be checked.
OBRush's Studio clay mode is outside the imported Source comparison. That
extra OBrush mode has a separate `0.90` active-part dimming rule rather than
SculptGL's `0.3` Source rule. No matched Studio-clay reference state was
identified, so no change was made to this extra mode.

`node --check scripts/check-sculptgl-darken.mjs`, `git diff --check` and
`cargo fmt --all -- --check` passed. This C246 branch changes no Rust, host,
renderer or shipped web files, so workspace tests, strict native/WASM Clippy
and macOS packaging were not repeated. A cold isolated `build-web.sh` attempt
was intentionally interrupted after its first `web-release` profile completed,
when the already validated, unchanged root build was confirmed; the partial
log remains in ignored `.artifacts/darken-c246/build-web.log` and is **not**
reported as a passing build. The paired browser test used the hash-checked
root web build stated above. No generated files or commercial source are
committed.
