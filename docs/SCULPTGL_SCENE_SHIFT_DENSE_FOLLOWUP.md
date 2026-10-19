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

The frozen C260 portable WASM is
`66f80111dd6c871f557050037cdbba9068828220f5da430c4c501d323d15fb90`;
the C261 candidate is
`0e8b1515ed9e97c4ee7674468f2b8ab58d95aa57892386652530cc5bc308fd2e`.
The pinned Source bundle is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`
at `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`. All three ran the same
fixture and six-step physical Shift-modified drag, with 80 ms down hold,
6/12/18/24/30/36-pixel moves separated by 70 ms, and a 350 ms release
pause. Source's projected page contact was `[697,477]`; both OBrush builds
used `[845,472]` for their own camera on the same world mound. The complete
45-event pointer/controller/shortcut trace is in the receipt.

| Browser | Changed position components | Largest component delta | History | Undo/Redo |
| --- | ---: | ---: | --- | --- |
| Pinned SculptGL | 15 | 0.00190204 | 1 → 2 | Exact geometry through the GUI Undo/Redo controller callbacks |
| OBrush C260 | 59 | 0.0355874 | 1 → 2 | Byte-exact public project Save after Ctrl+Z/Ctrl+Y |
| OBrush C261 | 54 | 0.0314101 | 1 → 2 | Byte-exact public project Save after Ctrl+Z/Ctrl+Y |

The final normal-browser receipt is
`.artifacts/sculptgl-scene-shift-dense/33116e22-e8b0-438e-838f-c910917b3a4e/result.json`
(SHA-256 `d214c36b7f094d5367100bf4fc77c19e36ea1030403f9516e16668beb76c2d18`).
The C261 Shift-drag route **does** reach Smooth and changes the dense mesh;
the prior coarse-box no-op did not reveal a routing regression. Its deformation
is not the same as C260 or SculptGL, and this test does not close brush-response
parity. The first harness run failed its C260 import wait because its diagnostic
snapshot omitted `subtools`; the pinned Source portion had already changed 15
components with exact Undo but its keyboard Redo probe was inconclusive. That
failed receipt is preserved at
`.artifacts/sculptgl-scene-shift-dense/117e2546-b7f2-4333-9de6-81eebbfb300c/result.json`
(SHA-256 `cc981b66317603e4336d38561916a8604509ea1b1b8c7e03b02b7e03de310cdd`).
The harness then used the same Source GUI Undo/Redo controller path as the
earlier C248 scene audit and corrected the missing diagnostic field. The Source
and ZBrush parity gates remain open.
