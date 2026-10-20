# C280 Brush Culling opposed-sheet result: failed second-held geometry gate

The unchanged-product, four-document normal-UI comparison completed against
the pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`
and the independently authored 882-vertex opposed-sheet SGL fixture SHA-256
`80b6430b152aad76fface01e9bf4890fdd525b2d9f11c60cd74e6c1b021d876b`.
The frozen C280 harness SHA-256 was
`f044286427f0e660608f8ec11173ae4f11db0e3204afc6d468445274ef4f74ba`;
the audit SHA-256 was
`6b0037eb05951cfab9686a1df4a02b204e6785a8286dd871c3310e93aed62d77`.
The Source JS bundle SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The OBrush WASM SHA-256 was
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`
and browser JS SHA-256 was
`03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`;
the served web repository HEAD was `1cdcdd37d76f4999ede37434f1c5663911d0dc7f`.
C279's earlier failed setup remains separate and unchanged.

Raw runs, including public saves and active GPU words:

- Source on: `.artifacts/sculptgl-brush-culling-c280-ui/source-on-c3daa10c-dd1f-4fbc-aeea-32444c5005e0/result.json`
- OBrush on: `.artifacts/sculptgl-brush-culling-c280-ui/root-on-318303f5-0440-4919-ad4c-c2063dbba3a5/result.json`
- Source off: `.artifacts/sculptgl-brush-culling-c280-ui/source-off-bc492c29-20ab-4664-9dd3-ee29ce647b1b/result.json`
- OBrush off: `.artifacts/sculptgl-brush-culling-c280-ui/root-off-c12d5c92-8e6d-425f-9732-e73c9bfc42a5/result.json`
- Frozen audit: `.artifacts/sculptgl-brush-culling-c280-ui/source-on-c3daa10c-dd1f-4fbc-aeea-32444c5005e0/paired-audit-root-on-318303f5-0440-4919-ad4c-c2063dbba3a5-with-off.json`

The culling-off pair passed the scoped numerical comparison: maximum mapped
public XYZ difference `2.421438694000244e-8`. Culling-on first held GPU XYZ
matched within `1.1175870895385742e-8`, but second held and public
After/Redo reached `3.120319831850793e-6` across front RGB IDs
`[200,201,202,221,222,223,242,243,244]`, exceeding the frozen `1e-6` gate.
The paired active GPU normals stayed within `3.370223566889763e-8`.
Culling-off moved front and back; culling-on moved front and left all back
vertices bit-exact. Per-state changed-ID membership, public Before/Undo and
After/Redo identity, one-step history, topology/attributes, and active GPU
position-versus-public-Save gates passed. Overall C280 **failed**. The
full Brush Culling parity row remains open.

The C280 setup accepted published X/Y projection ratios within `5e-6` only
after actual rays and radius were bounded: the maximum on-state world X ray
or radius difference was `1.4656441573585033e-7` (Y ray difference zero).
At world radius about `0.04754` and intensity `0.25`, the brush's
`0.1 × intensity × radius` displacement amplitude is about `0.0011885`.
Its cubic falloff has maximum normalized-distance slope `16/9`; a
`1.47e-7` smooth center shift changes the amplitude by at most about
`6.6e-9` before radius sensitivity, far below the measured `3.12e-6`.
This bound does **not** apply at a discontinuous triangle-pick tie.

Static diagnosis of the held GPU words and pinned `Picking.polyLerp` explains
the discontinuity. The second ray reaches approximately
`x=0.04754, y=0`, on the shared edge `[221,222]` after the first dab.
Source culling-off's second-dab displacement direction is approximately
`[0.010459,-0.002208,0.999943]`, matching interpolation of the first-held
GPU normal words on face `[200,222,221]`:
`[0.01045805,-0.00220786,0.99994288]`. OBrush on and off both report the
corresponding picked triangle and normal
`[0.01045815,-0.00220786,0.99994287]`. Source culling-on's second-dab
direction is approximately `[0.011412,0.000419,0.999935]`, matching the
adjacent face `[221,222,243]` interpolation
`[0.01141342,0.00041906,0.99993478]`. This evidence strongly implicates
face selection at an exact edge; the Source picked-face/normal fields were
not captured in C280 and therefore remain to be directly verified.
No product code changed on this result.
