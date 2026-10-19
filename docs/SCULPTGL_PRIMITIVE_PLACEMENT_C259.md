# C259: default Source primitive scene placement

Pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
wrapper SHA-256 `a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`.
The independently controlled [normal-browser pair](../scripts/check-sculptgl-primitive-placement.mjs)
clears each scene, invokes the visible OBrush default-detail SculptGL-profile
Add controls, invokes pinned SculptGL's `addSphere/addCube/addCylinder/addTorus`,
and reads OBrush's ordinary downloaded `.obrush` project. The harness SHA-256
is `2164881d31209de92c1628778d46cb0ed7f0fc46467ad74fd896952c7a02e73d`.
The tested OBrush WASM SHA-256 is
`db371cbe632b02d411bd819874f46a2a9386242d8fdaff7dd58e6c556e0d5797`.

SculptGL's default Add normalizes each seed's local-bound diagonal to 100
world units before subdivision. Cube and Cylinder then multiply the
float32 matrix by 0.7. It does not move a second Add along X. OBrush now
calculates this pose in shared Rust, leaves every retained subdivision level
in local coordinates, and stores one validated editable affine for the new
part. Generic-profile and manually selected detail additions keep their
prior spacing. One scene/history transaction owns the new mesh, pose and
retained levels.

The passing [receipt](../.artifacts/sculptgl-primitive-placement/e424b40e-42c0-4932-a8f8-41cee0c7c9f6/result.json)
(SHA-256 `42bae395f235af84e4849e2ea8f310a916a6bad93b9b99ec2442a31d419f1cb0`)
records the complete matrix and six world bounds for each fresh first Add:

| Default Add | Reference matrix scale | OBrush scale | Maximum world-bound difference |
| --- | ---: | ---: | ---: |
| Sphere | 57.7350273 | 28.867514 | 8.94e-7 |
| Cube | 40.4145203 | 20.207260 | 1.32e-7 |
| Cylinder | 28.5773811 | 28.577381 | 3.69e-7 |
| Torus | 58.5205727 | 58.520573 | 2.18e-7 |

Sphere/Cube use an independently authored cube seed with twice the
reference local extent; their matrices are correspondingly half size.
The equivalent matrix-scale difference is below `7e-7` in all four cases.
The browser then adds all four parts without clearing: their saved affine
translation stays exactly zero, and the four equivalent scales differ from
the reference by at most `3.44e-7`. Public Undo reduces four parts to three.
After Clear and public project Open, all four parts return and a second Save
has the exact same SHA-256
`298bbc4033cc6fe5f10d6e5c8cf9c5caee6db8d9c08623959c2bdf82c64e57bd`.

`cargo fmt --all`, the three focused shared-core tests and
`bash scripts/build-web.sh` passed. The first browser attempt requested a
large project through CDP `Runtime.evaluate` and timed out; its unchanged
[failed receipt](../.artifacts/sculptgl-primitive-placement/353d560f-8a1b-473f-afda-0e081b851eb2/result.json)
has SHA-256 `564a8346e2e9d714927a6435e35dd295ba5ec7aa36f390885d1a4bdf1aa0aea4`.
The succeeding harness uses the ordinary browser download. Full workspace
tests and strict native/WASM Clippy are integration checks for the combined
branch.

This is a bounded default-placement result. It does not yet compare camera
pixels, picked brush response, source UV/material state, exact vertex order,
normal appearance, native GUI, or sustained high-density responsiveness.
SculptGL's active scene gates and ZBrush parity remain open.

## Combined shape-harness correction

After integration, the older cylinder/torus shape harness still compared
SculptGL **world** positions to OBrush **local** positions. Its first combined
receipt at
`.artifacts/sculptgl-cylinder-torus-shape/20b37960-9496-439d-85d1-3ed52f543184/result.json`
(SHA-256 `f0893589fce0cd10edc1ceb21718b25262f8705cd9e7d575262491066b12e373`)
therefore reported large, invalid world differences despite passing its local
geometry gates. The corrected harness reads the active affine from the public
`scene_v7` project record, validates its 16 finite values, and compares both
world surfaces with their saved matrices. Script SHA-256:
`8af38264e76260c12195012225f81bcda2faa8c528c06107e9b5689d39103167`.
The repeated normal-browser receipt at
`.artifacts/sculptgl-cylinder-torus-shape/2558e997-a345-408f-8013-29574b649168/result.json`
(SHA-256 `196f2339c8b1f8604b7c0dcb562e8b1dd15c449cb9a7b15ce1abefe032a83751`)
used the combined browser WASM SHA-256
`66f80111dd6c871f557050037cdbba9068828220f5da430c4c501d323d15fb90` and
passes the same local topology/position gates and now checks world bounds:
maximum absolute differences `3.690e-7` for Cylinder and `2.180e-7` for
Torus. Normal/pixel appearance and edited-level response remain open.
