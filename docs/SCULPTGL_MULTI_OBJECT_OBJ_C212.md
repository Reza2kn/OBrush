# C212: Multi-object OBJ Source Add

The pinned SculptGL OBJ importer begins a new mesh at each `o ` record and
normalizes the resulting meshes together. C211's OBrush Source File Add
collapsed the same input into one mesh. OBrush now uses a bounded shared-core
OBJ split path: it rebases global positive face/UV/normal indices into each
part, retains local negative indices, validates every part, and rejects
cross-object references. The host prepares one union-bound normalization
matrix for all parts and appends them in one checked document transaction.
The existing single-object OBJ/PLY/STL/SGL path is unchanged.

The independently authored 115-byte two-object fixture (SHA-256
`ad45e6931301bf1030c886a69ad9aec3c4aacf9c2655cb73c5fb6cfe300d2289`) has one red triangle
at X `[0,1]` and one blue triangle at X `[4,5]`, with global positive face
indices `1 2 3` and `4 5 6`. Source options were `scalecenter=1` and
`vertex_srgb=1`. The two source triangles each receive the established flat
depth bound of `±0.28284271247461906`. Their union is
`[0,0,-0.28284271247461906,5,1,0.28284271247461906]`; the shared scale is
`100 / 5.130302135352264 = 19.49202938963626` before `f32` storage.

The normal browser used Source-profile Ctrl+O, its real file chooser, the
visible Save control, keyboard Undo/Redo, and the pinned hosted SculptGL
importer. OBrush's saved scene grew from four to six parts. One Undo returned
the exact prior four-part project SHA-256
`515d251c1b60d2b54551bc5c386d2ab8e6246eeed9fdddbe9f803b94f3b78ac2`;
Redo exactly reproduced the six-part SHA-256
`73da411033eb00ebd2d78ff0fc378c88218b0d2c32f2e4c7b55f2b24271be63c`.
A later OBJ with a valid first object and degenerate second object left the
six-part project unchanged. For both valid imported objects,
all 16 affine `f32` words and all RGB `f32` words match the pinned SculptGL
browser. Maximum nearest world-position component differences after OBrush
JSON decimal parsing were `8.315e-7` and `4.072e-6`, respectively. No
browser runtime exception was recorded.

The paired fixture and browser result are produced by
`scripts/check-sculptgl-source-file-add.mjs`. The pinned hosted reference
kernel SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The result is
`.artifacts/source-file-add-browser/f0b30032-c421-4101-8009-463480d39787/result.json`
(SHA-256 `d2f1ed2758036e6504dd513b5b8a8fbff7ddb4d95be7452582169bea90fdbdb8`).
The served portable WASM SHA-256 is
`44ae2f6496c762cef1b151456a61aa0e9309f8a4414240987a32efef279f0952`.
The local macOS package passed `codesign --verify`; its executable SHA-256 is
`0e78f3683ac179472766dfa14f7f2e279d2cd325031280f0a6b6d812a5e94500`.
`cargo fmt --all -- --check`, strict native and WASM Clippy, the browser
build and the paired browser check pass. The final workspace suite passed
2,515 tests with zero failures and one intentionally ignored test across 103
groups; the log SHA-256 is
`3aceeb08e3c8fff153ea90daaf4e82d1e6f09901bc7ed498398aa605782f0f4d`.
An earlier partial suite was interrupted after a subsequent host edit; it
was replaced by this complete run rather than counted as final evidence.

An initial independent test expected `100/sqrt(26)` from the raw triangle
coordinates and failed because it omitted the source-style flat-depth padding.
The test now derives the padded union diagonal, then checks the shared affine,
two appended parts and one history entry. This failure is retained in the
turn log; it is not counted as parity evidence. The group importer still does
not preserve object labels from `o` records as editable SubTool names, and
multi-object PLY/STL, material exchange, wider OBJ syntax, surface rendering
and sustained large-mesh response remain unchecked. This scoped File Add
improvement does not establish full SculptGL or ZBrush parity.
