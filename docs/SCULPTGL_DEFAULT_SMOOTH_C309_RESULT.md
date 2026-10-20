# C309 default Smooth normal-UI baseline: frozen gate failed

The [C309 preregistered comparison](SCULPTGL_DEFAULT_SMOOTH_C309_PREREG.md)
ran the pinned SculptGL and isolated OBrush browser builds through the
ordinary UI. No OBrush product code changed. Its complete gate **failed**:
the authored raised RGB ID 287 and its nearby protected/half-protected
controls lay outside the actual brush footprint. Both programs changed the
same ten other RGB IDs (`311–314`, `336–339`, `362–363`) and left ID 287
fixed, so the frozen center-spike-change and tangent-toggle-spike gates
failed on both sides. This is a fixture/gesture coverage failure, not an
admitted default Smooth parity result or a demonstrated product defect.

Fixture SHA-256:
`bcd0a2f0e5a6787354a8bbc81d3b1ce03006a3df540b78f073cf80c36c6b3825`.
Frozen UI harness SHA-256:
`b581affddb863034b71cef7b3dd54d227afbcc9d0d583bd80f1bda91b0cff08a`.
Frozen audit SHA-256:
`4bbf11245783d5c09e7c6c191d0c1365bf9ea48366f3a64f2880b63bd76aaec0`.
Reference source commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
OBRush source was prereg commit `f193a22d305b2fc7c9af15c49cd0d579dd0ab1c2`;
served portable WASM SHA-256
`67051db667ad51adba1fc65f9b28bdb05bd23a1610058285836082c191a7ada6`,
JS SHA-256
`d32d2b1a221e1d49fa2407723dd5795ea793b17ffe01ffcd78b650f0104c1ae7`,
and SIMD WASM SHA-256
`649e50fc7f3725d45b56e7d435320382293b104ca3c49fadfaad6d37e54ba750`.
`WASM_BINDGEN_BIN=/Users/Ajab/AI/OBrush/.tools/wasm-bindgen/bin/wasm-bindgen bash scripts/build-web.sh`
completed successfully. No Rust changes required workspace tests, Clippy
or a native package build in this slice.

Retained raw runs (paths relative to this worktree):

- Source default Smooth off: `.artifacts/sculptgl-default-smooth-c309-ui/source-off-124093db-ddfe-4747-8c5d-73cab3618d56/result.json`
- OBrush default Smooth off: `.artifacts/sculptgl-default-smooth-c309-ui/root-off-10463aa9-a7c7-45ae-a834-23f4dc5f9391/result.json`
- Source Relax-only on: `.artifacts/sculptgl-default-smooth-c309-ui/source-on-3f605543-cb3f-4559-a936-ccc239fc1b69/result.json`
- First OBrush Relax-only on setup **failure**: `.artifacts/sculptgl-default-smooth-c309-ui/root-on-2d1da672-b7f8-4c55-a536-1fcf049bff8e/result.json`
- Fresh OBrush Relax-only on unchanged-harness retry: `.artifacts/sculptgl-default-smooth-c309-ui/root-on-16e18640-dcce-4866-bbdb-590a82109738/result.json`
- Frozen paired audit **failure**: `.artifacts/sculptgl-default-smooth-c309-ui/source-on-3f605543-cb3f-4559-a936-ccc239fc1b69/paired-audit-root-on-16e18640-dcce-4866-bbdb-590a82109738-with-off.json`

The first OBrush-on setup failed before any stroke because its published
projection ratio differed by about `6.7e-6`, above the frozen `5e-6` bound.
The unchanged-harness fresh retry passed camera setup: on X/Y ratios
`0.999997257/0.999997195`, off `1.000000508/1.000000401`;
world-radius differences `1.14e-7` on and `2.11e-8` off. It preserved the
earlier setup failure rather than erasing it.

Within the actually affected ten IDs, mapped OBrush/Source XYZ were exactly
equal at Before, held, After, Undo and Redo for both settings. Active GPU
position and display-normal comparisons were exact; each own GPU position
matched its public Save. All four runs made one undoable edit, restored
Before/After Save bytes by Undo/Redo, preserved topology and attributes,
and had quiet post-release GPU words. The off/on algorithms *did* differ
at affected ID 337: after Smooth off its position was
`[0, 0.023333333432674408, 0.0234883651137352]`, while Relax-only on
yielded `[0.000010583959920040797, 0.02339646779000759, 0.023899711668491364]`
in both apps. That diagnostic difference does not repair the frozen ID 287
gate or close `SGL-BRUSH-SMOOTH`.

A later comparison must independently preregister a gesture/fixture whose
raised and protection controls actually fall inside the published brush
footprint, then repeat the paired normal UI and all frozen geometry/history/
GPU checks under a new ID. The C309 failure remains part of the record.
