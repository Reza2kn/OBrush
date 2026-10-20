# C278 Source Brush Clay: scoped normal-UI pair

The unchanged shared-core Source Brush/Clay implementation passed one
preregistered, independently authored normal-browser four-run comparison with
pinned SculptGL. No product code was changed. This is a scoped witness for
`SGL-BRUSH-BRUSH-CLAY`, not a full-row or ZBrush parity claim.

The reference was SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted JavaScript SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The authored 10,114-vertex / 20,224-triangle RGB-ID sphere was SHA-256
`39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`.
The OBrush product WASM SHA-256 was
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`;
its JavaScript SHA-256 was
`03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.
The noninterrupting capture and audit harness SHA-256 values were
`aa00cd9407648b42b3882ac712fe647c31a8827b55eb4e73c197e626c1037217`
and `299b82cc16dba580d9a1f3bfc7219a89923a5bd0bf8da42a83247ff1c8d7cf71`.

Each application imported the same sphere through its normal UI into a fresh
document for Clay on and Clay off. Brush/Draw used Accumulate on, radius 64
screen points, intensity 25%, Alpha None, zero pressure factors, and no
Negative, Culling, X symmetry, Continuous, Prevent folds or Lock. Each scene
used an orthographic front view at a 1024×808 logical sculpt viewport, DPR 1
and UI zoom 1. The trusted integer gesture pressed at viewport center+80 X,
moved six times to X offsets `92,80,92,80,92,80`, then released at 80.
Published projection scales were exactly 1 and paired world brush radii were
identical. The DOM trace contained only the expected sculpt pointer down,
held moves and up. Active GPU positions and display normals were captured
at all six held states; public Save, Undo and Redo occurred after release.

Both apps changed the same **98 original RGB IDs** in each Clay state. Maximum
paired final XYZ component difference was `5.960464477539063e-8` for Clay on
and off. The Clay-on/off effect changed the same 98 IDs within each app;
maximum paired effect-component difference was `5.960464477539063e-8`.
Across the six held states, the maximum paired active GPU position difference
was `5.960464477539063e-8` and the maximum paired GPU display-normal
difference was `9.89530235528946e-10`, below the frozen `1e-6` limits.
The first and sixth held GPU positions differed in all four runs, proving the
repeated path continued to sculpt. Public Before=Undo and After=Redo were exact
in each app, and each stroke added one history step. Before/After/Undo/Redo
active GPU position words matched the corresponding public Save words exactly;
each active object identity and GPU stream was unique. Attributes, faces and
untouched vertices stayed exact. Quiet frames after release did not change.
The GPU display-normal oracle was each application's active renderer stream,
never saved mesh normals.

Raw C278 results are retained locally under ignored `.artifacts`:

| Side / Clay | Result | SHA-256 |
|---|---|---|
| SculptGL / on | `sculptgl-brush-clay-c278-ui/source-on-d3ba8a92-8bb6-4232-9dce-448c1887169c/result.json` | `a11633ce506d67cb9cb563a2442081ef83884259ea9e81a40c901704ea82c365` |
| OBrush / on | `sculptgl-brush-clay-c278-ui/root-on-d8f8bbbf-b456-44b4-880c-bb9145e6c24e/result.json` | `9eb821bf938ef0cc37590f637cdda237eb6cf1e97de9ec17e0941f58fcaa19c7` |
| SculptGL / off | `sculptgl-brush-clay-c278-ui/source-off-a154f6d9-bf80-424c-89f0-5f4a3893fa94/result.json` | `5d1138309617c051067106f5094cdadda0a1dd63337c60a9f90d811e54b152c9` |
| OBrush / off | `sculptgl-brush-clay-c278-ui/root-off-7e5955af-8ced-4f38-9806-a78047e039fe/result.json` | `c667282ace8bee4a7dacf7e402ef6a1bc80d139880564a3a82b92a108f9db4f4` |

The complete paired audit is
`.artifacts/sculptgl-brush-clay-c278-ui/source-on-d3ba8a92-8bb6-4232-9dce-448c1887169c/paired-audit-root-on-d8f8bbbf-b456-44b4-880c-bb9145e6c24e-with-off.json`,
SHA-256 `776b38bf8978f16844afe4d90880f5e5d1f2a8927ce9d6bbcbff55c07e4feb25`.
The first audit invocation accidentally named a nonexistent OBrush off result;
its `ENOENT` log remains in `c274-checks/c278-audit.log`. The corrected
invocation used the exact produced result and the same audit script hash.

**Preserved failed C277 check:** The earlier preregistered C277 protocol
opened public Save during the held pointer. SculptGL's own DOM trace records
the extra file-menu movement and click; OBrush likewise stopped after one
applied dab. All four first/last meshes were identical, so C277 failed its
repeated-response gate. The failed audit is
`.artifacts/sculptgl-brush-clay-ui/source-on-9ce0a127-ea7e-4528-ae72-f6a2a17619f6/paired-audit-root-on-520f5a4d-b2f3-4e14-9603-3d3c64520b7d-with-off.json`,
SHA-256 `fdcbbf5925a4b2b325d78ce1ebfa4467a32cc8866e8a7cbb4a3949f57dc60a4c`.
C278 was separately preregistered to remove that measurement interference;
the C277 failure is neither overwritten nor counted as a pass.

The passed fixture covers one static sphere, one camera and viewport, one
integer gesture and zero pressure factors. Other shapes, masks, alphas,
negative/mirror/culling combinations, fractional input, alternate viewports,
dynamic topology, cancellation, dense meshes and native GUI behavior remain
unpaired. The full Clay inventory row stays open.
