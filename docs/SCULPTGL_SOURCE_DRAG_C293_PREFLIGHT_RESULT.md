# C293 settled-camera Source Drag preflight result

**Outcome: the frozen read-only camera and pick preflight passed. No Drag stroke
was made, no product code was changed, and `SGL-BRUSH-DRAG` remains open.** This
result admits a separately built candidate under the prospective C293 stroke
gate; it does not reclassify C288's failed XYZ/Redo-normal pair or C290's
unstable-camera failure.

The Source and OBrush captures used separate owned browser/server processes and
closed cleanly. Root port 8792 was untouched. The capture harness SHA-256 was
`e23767b67d8f8b73ae7559ada388d44267ec668658a7ced81a02302fca17e5c3`;
the independent paired scorer was
`5feb9b56b1f8c5b04af03ade73aa3749bb0852db9bce9746e5670b5f3a60b51c`.
Capture worktree HEAD was `04880579bb2cbab40761c704d9badca6e0e0ae16`;
the served OBrush repository HEAD was
`4fb363c5ef9903f271c31beeb582acd0a96e8b79`, with WASM SHA-256
`1380a9c6bbfaee43d637846803bdaddf941dba8f43893af747ccf3323778737d`
and JS SHA-256
`35d354ed50431b29537bd2591b737aeaccecbf3b92ef37265316c205017b710b`.
The pinned SculptGL commit was `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
the independent 1,089-ID fixture SHA-256 was
`fec6d8a1598f719e71f0af821a2462f7f70f4ffdb1740ca0bcdca768658f8a6e`.

Both normal UIs reported terminal, identical camera states over at least 12
animation frames and 250 ms after Reset and after fit/Center. Source elapsed
397/400 ms and OBrush 564/584 ms. At the settled fit, X/Y projection ratios
were exactly 1; all four requested world rays had zero paired difference and
both 96 px radii were `0.0855945285292783`. The two actual Source hover picks
agreed with independent paired ray/triangle predictions within
`1.942554050948786e-8`, below the frozen `1e-7` bound. Source's observed
reset/fit branch used pre-fit near `875.4363385484222`, returned the exact
reference formula result `2.3865350093528384`, and focused on Z
`0.02551280055195093`. All 1,089 public Before RGB-ID positions matched
exactly, as did attributes and topology. Public Before and after-hover Saves,
history depths, and active GPU positions/normals proved the hover sequence
read-only; each active GPU position buffer equaled its own public Save.

Raw receipts are preserved locally at:

- `.artifacts/sculptgl-source-drag-c293-settled/source-5f8c1b55-f431-4c07-b232-8d8ee0f4b627/result.json` (SHA-256 `4f8832f0302ec7c1ecbe27093c0b62069cda8a5fd251db20a3f76e629b2203c7`)
- `.artifacts/sculptgl-source-drag-c293-settled/root-be5f6767-078f-4e52-9c42-ad4742821f65/result.json` (SHA-256 `7ccb116e110d4f1217e22affb91240a9efd8c9b85035b8ce33645255f16c04ed`)
- `.artifacts/sculptgl-source-drag-c293-settled/source-5f8c1b55-f431-4c07-b232-8d8ee0f4b627/paired-audit-root-be5f6767-078f-4e52-9c42-ad4742821f65.json` (SHA-256 `da28020451312241c1c4859e0050312f16b3683ff94c9a0b79eaf74157d6a57b`)

The scorer reported `pass: true`, `failed: []` across every frozen preflight
check. A candidate must still pass the separate held-stroke/public
Save/Undo/Redo/GPU gate at `1e-6` XYZ and display-normal tolerance. Dynamic
topology, alpha, symmetry, alternate cameras/settings, and long-session
response remain untested.
