# C244: selected Source flat shading

The pinned SculptGL source is commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`. Its
`GuiRendering.onFlatShading` applies one checkbox value to every mesh returned
by `getSelectedMeshes()`. `ShaderBase.getNormal()` selects interpolated vertex
normals when `uFlat` is false and a screen-derivative cross product when true.
`ExportSGL` writes the flag per mesh. This controller does not push a geometry
history state. The pinned hosted bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
its wrapper SHA-256 is
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.

OBRush already resolved a per-part Source render flag into the renderer and
used fragment derivatives for flat normals, but its Source checkbox edited
only the active part. The C244 checkbox now calls a shared-core selected-part
transaction. It sets one value across the selection, validates the complete
candidate before publication, rejects a mixed Source/plain selection without
a partial edit, and preserves OBrush's undoable Source appearance contract.
The ordinary non-Source viewport flat-shading control is unchanged.

The independently authored two-part SGL fixture is
`crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl` (540 bytes,
SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`).
Four shared-core tests cover selected pair and one-part scope, uniform setting
from mixed flags, unchanged positions, one-step Undo/Redo, `.obrush` JSON and
SGL export flags, mixed Source/plain atomic rejection, and a legacy source
record with no render field. The isolated full workspace test run passed 2,542
tests across 85 groups with zero failures. `cargo fmt`, strict native and
`wasm32-unknown-unknown` Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh` passed. The package passed ad-hoc `codesign --verify`.
Logs are retained under ignored `.artifacts/flat-c244/`. The built WASM
SHA-256 is
`9686f5c1ea013e8fec679c49bd02f19af389d07f1003e5b98cd03651ed3dd6b5`;
the native executable SHA-256 is
`9bb06fe11959c3786dfb420f399df892d145b1e18997a6f450b48f0de2ebcef5`.
The isolated generated target occupied 2.8 GiB; after packaging, the Data
volume had about 91 GiB available.

The final owned-browser result is retained at
`.artifacts/sculptgl-flat-browser/8650efae-1b12-44e4-8ccf-be1d8f6276e5/result.json`.
Its harness is `scripts/check-sculptgl-flat-shading.mjs` (SHA-256
`38f4ee886aa453628ed5c13d66b840802662620847ab2bb12e5a8956954b8519`),
and the result binds to implementation commit
`215ff4ef67005336ac86043b193a06028c86989e` and the WASM hash above.
The same 540-byte fixture was imported through both normal browser file inputs.
In pinned SculptGL, selected mesh index 2 alone changed flat flags
`[false,false,false] → [false,false,true] → [false,false,false]` through the
visible Rendering checkbox; history depth remained 2. Its triangle draw
observed `uFlat` false/true/false. In OBrush, selected part ID 3 stayed
selected while the Source flat checkbox changed its triangle-draw
`u_flat_shading` 0/1/0 and history depth 1/2/3. `gl.getError()` was zero in all
six sampled states. All six 1600×1000 screenshots and their SHA-256 values are
retained in the result. The before/on/off viewport images are evidence of the
normal UI path, not an exact visual match: this fixture contains small,
single-face parts and the side panels move between frames.

Early browser runs failed due to an initially closed SculptGL Rendering menu,
an incorrect assumed selected mesh index, a boolean-versus-integer GPU oracle,
reading OBrush scene controls instead of layer controls, and wheel input
without hovering the inspector first. Those failed result JSONs remain in the
same ignored artifact directory. A first strict Clippy run also failed because
a shared Cargo target picked up another worktree's `obrush-core` build; its
log is retained as `clippy-native.log`. The complete suite and strict Clippy
were rerun successfully with this worktree's independent target, followed by
the web build and final browser run.

The full `SGL-RENDER-FLAT` acceptance gate remains open. This slice does not
measure analytic face-normal versus interpolated-normal pixels on a large
nonplanar surface, multi-selection through the pinned browser UI, alternate
shader and transparency combinations, complete save/reload through both UI
file paths, native GUI, or sustained responsiveness. SculptGL's checkbox is
history-neutral while OBrush's Source appearance edit is undoable. These
differences are not hidden by the scoped selected-part result.
