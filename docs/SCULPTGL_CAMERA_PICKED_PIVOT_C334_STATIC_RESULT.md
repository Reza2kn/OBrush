# C334 static preflight receipt — candidate not promoted

Run on 2026-10-20 in the isolated `codex/camera-picked-pivot` worktree, before
Cargo or browser validation:

`OBRUSH_SHARED_ROOT=/Users/Ajab/AI/OBrush node scripts/audit-sculptgl-camera-pivot-c334-static.mjs`

Exit 0. The script confirmed pinned `Camera.js` SHA-256
`da34fd15e0d5ef78db754508719d1e9fdefc0a78ec55fd4bec7232ccff0092fe`
and authored OBJ SHA-256
`a4688247d60eed963730ff6d99a9768c08bcdd07c2c5a40e6d714a81db8e291b`.
The static checker itself has SHA-256
`39aee7d4bc178bff519875b9013a4e1cb3a34a5aac3fa4d2a19f65de8603b4a3`.
It evaluated two successive picked points with identity and 36° Y rotation,
for perspective and orthographic views. Maximum independent camera-space
component difference was `8.881784197001252e-16` in perspective and
`3.3306690738754696e-16` for orthographic X/Y. The static test deliberately
does not assert orthographic depth because the pinned reference clears its
depth offset on a picked pivot.

`git diff --check` also passed. Neither check compiles or runs the candidate
product code, proves source Float32 rounding, or exercises the ordinary
browser. The acceptance gates in the preregistration remain pending.
