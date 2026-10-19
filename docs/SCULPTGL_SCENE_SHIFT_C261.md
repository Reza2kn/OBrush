# C261 Source viewport Shift selection

OBRush already had real selected-part membership in `obrush-core`, panel
Ctrl/Cmd-click, and selected group operations. This change adds a canvas
Shift-click path in the SculptGL shortcut profile. On pointer-down it captures
the visible scene hit without sculpting; a stationary release toggles that
part through the existing `Document::select_subtools` policy. The active part
stays selected and the final selected member cannot be removed. A move beyond
four egui logical points leaves the pending click path and resumes temporary
Smooth. Selection still creates no Undo entry, as with the existing panel
path. Existing ordinary canvas click continues to choose one active part.

The fixture is an independently authored, closed three-box SGL v3 file at
`scripts/fixtures/independent-three-closed-boxes.sgl` (1,560 bytes,
SHA-256 `939110d73dec934f0af70fb5c364eeb18f3ce18608449fe76804896c50c9c0ae`).
`author-scene-three-closed-boxes.py` and `fixture.json` in the same directory
record all original positions, faces, colors, and materials. The left, center,
and right boxes occupy X `[-2,-1]`, `[0,1]`, and `[2,3]`; each uses Y/Z
`[0,1]`. The right box is initially selected.

The pinned SculptGL bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
at source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`. The fresh OBrush
browser WASM SHA-256 is
`0e8b1515ed9e97c4ee7674468f2b8ab58d95aa57892386652530cc5bc308fd2e`,
built from pre-change base `1188948273053e5bdd6bbf246278804bf71992f9` plus this worktree's
uncommitted diff. `node scripts/check-sculptgl-scene-shift-c261.mjs` drives both
normal browser canvases with real pointer events at 1600×1000, DPR 1, and
checks the public OBrush Save result. The pinned Source sequence is selected
indices `[2] → [2,0] → [2,0,1] → [2,1]`; OBrush gives IDs
`[3] → [3,1] → [3,1,2] → [3,2]`. Every Source box vertex stays exact; every
OBRush saved per-ID mesh stays exact. OBrush's Undo count remains 1 through
all three Shift-clicks. An ordinary click on the left box collapses OBrush
selection to `[1]`. The successful browser receipt is
`.artifacts/sculptgl-scene-shift-c261/b08f4f5c-8b6d-417b-bd7a-1944d2977516/result.json`
(SHA-256 `7b252120910ec730c803cd43da1868c35278d7e28a1b3fd1df8b8d0d0148f41e`).

The same coarse box fixture did **not** prove the Shift-drag Smooth branch:
the attempted 12-point drag near its left corner left geometry and Undo
unchanged. The strict failed guard is preserved at
`.artifacts/sculptgl-scene-shift-c261/1fdfbfbc-08e5-4592-a3d9-b150fcdca710/result.json`;
the later retry at projected point `[310,594]` is
`.artifacts/sculptgl-scene-shift-c261/0bc11209-9e0b-4b15-a531-3facdbcb7316/result.json`.
The click/drag state machine has a focused unit test, but a denser mesh,
pointer-contact trace, and normal-browser deformation check are still needed
before claiming that Shift-drag is preserved. Source itself also uses a
Shift-click brush-start path in its pinned implementation; this comparison
deliberately checks membership and unchanged geometry under its lock-position
setting, not all brush effects. Native GUI, other modifiers/tools, physical
pen and touch, selected-operation scope from this exact gesture, and the full
SculptGL/ZBrush gates remain open.

## C260 baseline drag comparison

The unchanged C260 web bundle (portable WASM SHA-256
`66f80111dd6c871f557050037cdbba9068828220f5da430c4c501d323d15fb90`,
as recorded in `SCULPTGL_COMBINED_C260.md`) was run through the same three-box
browser harness without rebuilding. Its Shift-click membership predictably
collapsed to one object (`[1]`, `[2]`, `[1]`), so that run's overall
membership status is FAIL. Its later near-corner 12-point Shift-drag at
`[310,594]` changed no per-ID saved mesh and left Undo at 1, exactly as the
new build did on this coarse fixture. The baseline receipt is
`.artifacts/sculptgl-scene-shift-c261/7d9357ac-4843-4152-bc48-3d93661e435e/result.json`
(SHA-256 `1dd5cdfa60f3cd1b060fa89fd020b08c3ac1952cd5ffc859adb2f512d0b47cb7`).
Its `build.commit` field names the harness checkout; the WASM hash above
identifies the tested C260 product. This fixture demonstrates no Shift-drag
regression, but it cannot establish that either build actually smoothed.

`cargo fmt --all`, all 2,583 workspace tests, strict native/WASM Clippy,
and `scripts/build-web.sh` passed. Logs are under
`.artifacts/sculptgl-scene-shift-c261/validation/`. The first focused compile
failed because the helper imported `egui` directly instead of
`eframe::egui`; that import was corrected before all checks above passed.
