# C272: Brush Negative normal-UI comparison

This is a scoped comparison for the pinned SculptGL inventory row
`SGL-BRUSH-BRUSH-NEGATIVE`, following the committed
[prospective protocol](SCULPTGL_BRUSH_NEGATIVE_UI_PREREG.md). Four fresh owned
Chrome sessions used the ordinary import, visible Brush/Negative controls,
one ordinary mouse gesture, public Save/Undo/Redo and active GPU observations.
The final four runs share harness SHA-256
`4d253189ba4a519c1f49ff6e3e9079e90dc5661f650583982800dafa0f19e142`.
The reference is pinned SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The independently authored RGB-ID sphere has 10,114 vertices, 20,224 faces,
and SHA-256 `39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.

The paired audit passed the preregistered `1e-7` maximum component bound in
both Negative states. The complete initial position fields matched exactly.
After the stroke, both sides changed the same 87 vertex IDs; 10 IDs/11
components differed between apps, with maximum absolute difference
`2.9802322387695312e-8` in each polarity. Source and OBrush each restored
the exact Before file on Undo and the exact After file on Redo, with unchanged
face and attribute fields. Within either app, Negative on reversed the
Negative-off displacement at all 10,114 IDs exactly as Float32 saved words:
the four-way opposite-sign audit found zero differing vertices/components.
The signed radial displacement sum was `+0.20504501756698093` Source and
`+0.20504499751773073` OBrush for Negative off, and the same magnitudes with
opposite signs for Negative on. Source and OBrush requested world-space radii
`0.232804338527813` and `0.23280433852781302`; the largest paired pick-ray
X difference was `1.1102230246251565e-16`.

Active GPU positions exactly matched the corresponding public Save positions
at Before, After, Undo and Redo on both sides. Source WebGL1 evidence is the
actual active-draw forwarded position/normal uploads, not a saved-normal
substitution. OBrush WebGL2 evidence is a readback of the one active object's
404,560-byte VBO, bound to visible selected ID 1 with 10,114 vertices. The
first stage captured a fresh draw; the quiet After/Undo/Redo stages retained
the identity of the last drawn VBO and reread its current bytes. Each stage
had one candidate, a stable buffer token, a last-draw sequence, exact saved
position words and GL error zero. Across the two active GPU display-normal
streams, Before and Undo were exact; After and Redo differed at 13 vertex
IDs/23 components, maximum `4.656612873077393e-10` in either polarity.
Public Save normals are **not** the GPU display-normal oracle.

The OBrush run served the existing root `web/dist` bytes through an owned
local HTTP server; it did not rebuild. The bundle observed at execution was
root repository HEAD `bac2a59c0a7a5ed95039e0c1d20e2dbe8321b0f8`, but
the build provenance is bound by served bytes rather than inferred from that
HEAD. Chrome actually requested SIMD JS SHA-256
`29c3771e7aa3e5a105fef4cc95e22b8e8d9df7f8ecd9abed98122c82825522a9`
and SIMD WASM SHA-256
`e54c3d2edbe6f0e31e823940fb10c896313d117b49905963a3ca78e35bcb9494`.
The same served tree also contained portable JS SHA-256
`6083bbc43b3a69202ca3cdb31a7fb89ae369a26b7f135a737acb433129220136`
and portable WASM SHA-256
`e5439ad67d87583446961d46d97e2e7427e766475a331a3dfe800181c4a40351`.
The final tests used Chrome 155, DPR 1, orthographic front view, radius 64
screen points, intensity 25%, neutral force, Clay/Accumulate/Culling/X
symmetry/Continuous/Prevent folds/pressure factors off and Alpha None.

The raw receipt directories under ignored
`.artifacts/sculptgl-brush-negative-ui/` are `source-off-f9c96cde-caac-4cab-98fa-e645d1da4001`,
`source-on-fb42c474-e9cd-4ded-b62c-ae804c5e28b2`,
`root-off-f6975439-9359-4eeb-9543-86e19719e20c` and
`root-on-9c9e408b-117d-4d41-af14-1999a0f34465`. The paired audit JSON is
in each Source directory and the four-way audit is
`source-on-fb42c474-e9cd-4ded-b62c-ae804c5e28b2/negative-toggle-effect-audit.json`.
Earlier failed attempts remain in the same receipt tree: Source parent-row
Negative click did not publish (`source-on-a75dfb9e-*`); clicking the hidden
checkbox input was not an actual hit (`source-on-01266ace-*`); direct use of
root port 8792 failed the exactly-one-owned-server closure gate
(`root-off-cf643aeb-*`); the first OBrush GPU observer returned no candidate
at quiet After/Undo/Redo (`root-off-ecff9534-*`, `root-on-0ac06a11-*`);
and an unrelated Source Alpha Square-to-None UI setup hit failed twice before
the stroke (`source-off-a242c405-*`, `source-off-a378aaa6-*`). The final
Source setup asserts the already visible Alpha None state instead of making
that unrelated UI excursion; the signed stroke settings and acceptance bound
were unchanged. These failures are retained, not counted as passing runs.

This one-move unmasked Brush checkbox witness does **not** cover held Alt,
the N shortcut, mid-stroke toggle, Clay, other brush tools, sustained response,
physical pen, camera variation or visual appearance. Requested pixel offsets
and matching ray X values do not alone prove all delivered contact/dab paths.
Source WebGL1 upload capture and OBrush WebGL2 readback have different
provenance. The complete inventory row and SculptGL/ZBrush parity remain open.
