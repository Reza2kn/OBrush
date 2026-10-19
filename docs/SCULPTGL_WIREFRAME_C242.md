# C242: selected-part wireframe and W

The pinned SculptGL source at commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` applies the Rendering
checkbox to every selected mesh (`GuiRendering.onShowWireframe`, lines 162–167).
Its W handler changes that same checkbox on key release (line 300). The
`ONLY_DRAW_ARRAYS` fallback hides the checkbox, and `Mesh.setShowWireframe`
forces false in that mode. The pinned build's `RenderData.ONLY_DRAW_ARRAYS` is
false. `ExportSGL` writes each mesh's flag, while this controller does not push
a state-history entry.

OBRush previously exposed a global wireframe flag to W and changed only the
active Source part from the Source Appearance checkbox. C242 routes W and that
checkbox through one selected-part shared-core edit when a Source part is
active. Ordinary non-Source W continues to use the global overlay. The edit
validates the complete candidate scene before publication, retains the
project's established undoable Source appearance contract, and rejects a
mixed Source/plain selection atomically because plain parts have no per-part
Source render record.

The independent source fixture is
`crates/obrush-core/tests/fixtures/independent-affine-two-mesh.sgl`
(`SHA-256 99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`).
Focused shared-core checks cover selected pair, one-part scope, uniform setting
from mixed values, `.obrush` return, SGL render flag export, geometry identity,
one-step Undo/Redo, source-plus-plain atomic rejection, and a legacy source
record with no render field. The browser fixture is a separate independently
written 7-vertex OBJ with one quad and one triangle; the owned browser script
checks actual W delivery, draw-elements line counts, GL errors, screenshots,
selected scope, and pinned reference history depth.
Its 109 bytes hash to
`9cf9aea8a7a99c3890fc2c00353fe25cba035d869b64252b304319ad725e75ad`.
The hosted reference bundle and wrapper SHA-256 values are respectively
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
and `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.

The first pinned reference browser run incorrectly assumed OBJ import selected
both new objects. It failed and remains in ignored local artifact
`cfcb9756-e8b3-47e3-a936-05aa0e437156/result.json`. The corrected
reference-only run is
`72376680-5870-42d4-b285-b05716ce3882/result.json`: Import added the
quad and triangle to the default sphere (3 scene meshes) and selected only the
triangle. W changed mesh wire flags from `[false,false,false]` to
`[false,false,true]` and back; indexed GL line draw counts were `[]`, `[6]`,
`[]`; `gl.getError()` was zero in each state. The 1600×1000 screenshots have
SHA-256 `b522423c26cb47f23ae39ed84fe2d71b11eb9cdc2ff5e52e61ac97cf96d277ff`
before/after off and
`2e806173a83af0780a0791cc76db7dbf31c15f4a799ad341ce8eddbc083bf7ef`
on. The reference history array length stayed 2 through W on/off.

The first isolated app-build attempt and a focused core-test linker retry
failed with `No space left on device` before tests ran. The Data volume had
149,224 KiB available and this branch's generated `target` occupied
1,136,744 KiB. After confirming no open file used that target, only this
branch's ignored target directory was removed; measured free space rose to
1,276,044 KiB. These infrastructure failures do not count as passing source
checks and are retained in the session output.

The rerun used the existing root build cache through a verified target symlink,
with `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0`. All four focused
wireframe tests and `cargo test --workspace --locked` passed. The full suite's
log is retained under ignored `.artifacts/wireframe-c242/workspace-tests.log`.
Strict native Clippy initially failed on `clippy::unnecessary_map_or`; the
`is_some_and` correction passed the rerun. Strict native and `wasm32-unknown-unknown`
Clippy with `-D warnings`, `scripts/build-web.sh` using wasm-bindgen 0.2.129,
and `scripts/package-macos.sh` then passed. The package passed ad-hoc
`codesign --verify`. The built WASM SHA-256 is
`1e2a8a3a58a1e072e7e5478debe700c99d0daf2bace108bbc4f46583e7895c26`;
the macOS app executable SHA-256 is
`20dab3700fd714bac5352263777a57ff1b90de815fcae3f1398801428249e46c`.
The native and WASM Clippy, web build, package, and workspace test logs remain
under ignored `.artifacts/wireframe-c242/`.

The bounded paired browser run is retained as
`53b90434-a907-4063-9abc-70022eaaa026/result.json` with **failed** status.
It used the 1600×1000 owned Chrome route and observed pinned SculptGL's
selected imported triangle with six wire indices. OBrush Source Add created
the two imported parts beside its sphere; W on the triangle produced six wire
indices, W on the selected quad/triangle pair produced eight and six indices,
and post-transition pointer repaint samples after W off observed no line draws
in both cases. `gl.getError()` was zero in every sampled case. OBrush's history
depth changed 1→2→3→4→5 for W on/off/on/off because its Source appearance
is an undoable document edit; the reference history depth stayed 2. The
OBRush on/off screenshot SHA-256 pairs were
`c4b95877a7a68f9504dedf5c6ce6603fcea5b92ffbdfa7fe01f80d9f2ed3c13e` /
`508af5f482ecfe9abed7b3dd719f5cd9a6a41ff9761a0cab414a3a48618bf333`
for the triangle and
`7a60e2ea012a1aaead0db6b9faae9b71ee7585b4bb0197209898f4f0db679c42` /
`1daf35b413ad4d5356423bc4e027f80af1cbe1cb3969e25ab3f56859d705e220`
for the pair. The final quad-only click in the browser harness was not
delivered: the tracked control had scrolled to y=-267..-241, outside the
viewport. That last case is unverified, and the script deliberately reports
failure rather than inferring its result. Earlier failed browser attempts
remain in the same ignored artifact tree: one omitted the metrics field, one
did not intercept the file chooser, one sampled a transitional off-frame line
draw, and one hit a navigation-context race. These were harness faults and
were retained rather than treated as reference passes.

## Remaining gates

This slice does not establish full `SGL-RENDER-WIREFRAME` acceptance. Exact
pixels across static/dynamic/multires levels, transparent-part ordering,
selection changes while mixed wireframe values are visible, UV seam handling
after topology edits, a forced `ONLY_DRAW_ARRAYS` fallback, native GUI, and
large-scene responsiveness remain to be compared. The quad-only click also
needs a visible UI control and rerun. Pinned SculptGL's
wireframe toggle is history-neutral; OBrush's Source render settings use an
undoable document transaction. `SGL-URL-WIREFRAME` remains a separate gate.
