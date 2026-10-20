# C317 Twist selector correction: frozen before rerun

C306 produced complete browser receipts, but its source normal-UI selector used
index 3. The pinned `Enums.Tools` maps `TWIST` to **2** and `SMOOTH` to **3**.
C306 therefore compared SculptGL Smooth to OBrush Twist. Its failed geometry
score is a protocol failure and cannot support a Twist product correction or
parity claim. Preserve the raw Source result SHA-256
`6032517637f7ce26859542b735edc219df2a4b4cd993f719a513a33a3e478e5e`,
OBRush result SHA-256
`a0d3e67997880f538d7bcb09e948f5b19d6c755346e4f8930c5c6dec00c66b0a`,
and failed paired audit SHA-256
`b82c07dc9b32ced4e91bc0de90ca53cd7576d5fc1f01046fc4754d89589bcbcc`
in `.artifacts/sculptgl-source-twist-c306-ui/`. The C306 audit reported
eight failures, including max paired XYZ `0.009145651`; these numbers describe
the wrong source tool and are not a Twist baseline.

C317 changes only the source tool selector and source-index assertions from
3 to 2, plus protocol/output identifiers so the failed C306 evidence is not
overwritten. It retains the C306 independent fixture SHA-256
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`,
the exact four integer pointer points, 75-pixel radius, camera settlement,
normal-UI Save/Undo/Redo, RGB mapping, mask/history/active-GPU checks and
`1e-6` paired XYZ/display-normal limits. The pinned SculptGL bundle remains
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`;
the unchanged aligned OBrush baseline WASM is
`b530a5a00237706fc39a64769413aa54d38f79ef1347455df6e731503dba092c`.
Record source/root build and harness hashes again in the new raw receipts.
No Twist product code changes before this corrected baseline and audit.

The C317 normal-UI result remains one scoped gesture. Other Twist settings,
dynamic topology, responsiveness and full SculptGL/ZBrush parity remain open.
