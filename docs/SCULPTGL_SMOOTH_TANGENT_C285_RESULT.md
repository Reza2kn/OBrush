# C285 Smooth Tangent normal-UI paired result

C285 corrected only the visible Source-mode setup order after the retained
C283 setup failure. No OBrush product code changed. The independent fixture
SHA-256 was `5cc5947599f58cbfc738035d64504e7763368631f70a93cbfe55c6263da5c777`;
the pinned SculptGL reference commit was
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and bundle SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The frozen C285 harness SHA-256 was
`d9bf13e086ff6c415f89321e43db6bd14165d0201c2d993cbb560aae4e02e89b`;
the frozen audit SHA-256 was
`57c6c001949903cc9e641cbc16905af422bebdafd3434931d546faf1ab585d69`.
The served OBrush source commit was `9524b5a8c903be40fdfd356aa61a43e8401f20c3`,
WASM SHA-256 `913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`,
and JS SHA-256 `03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.

Raw normal-UI runs:

- Source tangent on: `.artifacts/sculptgl-smooth-tangent-c285-ui/source-on-2eb01d65-2c0b-4c19-b5de-a7f24fa400a8/result.json`
- OBrush tangent on: `.artifacts/sculptgl-smooth-tangent-c285-ui/root-on-6fd5672e-818d-405f-8497-6a09288f3c02/result.json`
- Source tangent off: `.artifacts/sculptgl-smooth-tangent-c285-ui/source-off-15cfbfb8-f732-4b3b-93cb-78317a691a6c/result.json`
- OBrush tangent off: `.artifacts/sculptgl-smooth-tangent-c285-ui/root-off-f4fa4fab-90ac-48f5-b4fe-c7ad05250a7c/result.json`

The paired audit is
`.artifacts/sculptgl-smooth-tangent-c285-ui/source-on-2eb01d65-2c0b-4c19-b5de-a7f24fa400a8/paired-audit-root-on-6fd5672e-818d-405f-8497-6a09288f3c02-with-off.json`.
It passed every frozen scoped gate. Tangent-on and tangent-off mapped XYZ
were bitwise equal between Source and OBrush at Before, held preview,
After, Undo, and Redo; active GPU display-normal words also matched exactly
at those stages. The active GPU position words matched each public Save,
with one active object/VBO and unchanged quiet post-release words. Both
sites made exactly one Undo step and restored exact public Before/After
bytes; the fully protected ID219 stayed fixed. The center spike ID220
changed in both modes: tangent on `[+0.000228156,+0.000053079,-0.000013677]`,
tangent off `[0,0,-0.004025768]`, with an on/off maximum difference of
`0.004012091`. Changed RGB-ID memberships matched.

The published projection ratio differed by at most `4.36335e-6` under the
frozen `5e-6` setup bound; world brush-radius difference was `2.06808e-7`
and maximum requested-world-ray difference was `6.48063e-8`, both under
the frozen `1e-6` bounds. These small setup differences did not affect the
exact paired outputs observed on this fixture.

This is one 441-vertex curved grid, one normal-UI held move, and one
on/off comparison. It supports a scoped Smooth Tangent parity finding;
the full `SGL-BRUSH-SMOOTH-TANGENT` inventory row stays open for broader
coverage. C283's setup failure remains a separate retained receipt.
