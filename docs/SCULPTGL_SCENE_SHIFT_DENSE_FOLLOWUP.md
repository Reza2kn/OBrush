# Dense Shift-drag Smooth follow-up

The C261 viewport Shift-click path deliberately holds a contact for a
stationary release and resumes temporary Smooth after moving more than four
logical points. The three-box browser fixture could not show whether either
the C260 or C261 Shift-drag path actually changed geometry. This follow-up
uses an independently authored closed UV sphere with a small analytic mound on
its visible side so smoothing has a measurable target. The writer is
`scripts/fixtures/author-dense-shift-smooth.py`; the public SGL v3 fixture is
`scripts/fixtures/independent-dense-shift-smooth.sgl` (236,392 bytes, 4,514
vertices, 4,608 faces, SHA-256
`6d5557d2650d5176c5565415e5d94937f41345f48bdb0f3cdc841b6480dcc9ad`).
Every one of the 9,120 undirected edges has exactly two incident faces, and
its signed volume is +4.2041169493.

`scripts/check-sculptgl-scene-shift-dense.mjs` drives the pinned SculptGL
bundle, the frozen C260 OBrush web build, and the C261 candidate in one owned
normal Chrome session at 1600×1000/DPR 1. The harness records every delivered
Shift-modified pointer event, starts at the projected mound, moves in six
6-pixel increments, and reads Source's live vertex array and OBrush's public
Save project. It then sends the Source-profile Ctrl+Z/Ctrl+Y shortcuts and
re-saves to check Undo/Redo recovery. This is a scoped interaction check; the
brush response shapes and broader selection operation are separate gates.

Browser receipt, observed position deltas, build hashes, validation, and any
failed checks will be recorded after the shared browser/build lane is
available. The Source and ZBrush parity gates remain open.
