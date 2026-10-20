# C300, C301 and C303 Source Drag symmetry setup failures

These are retained failed normal-browser runs on the integrated root build.
They are **setup** failures, not geometry parity results. The independently
authored fixture, reference, path, settings and numerical gates were fixed by
the corresponding preregistrations. Raw JSON and browser artifacts remain in
the ignored `.artifacts/sculptgl-source-drag-c300-ui`, `c301-ui` and `c303-ui`
directories of this isolated worktree and must be copied unchanged before the
worktree is archived. SHA-256 identities below refer to raw `result.json`.

| Run | OBrush symmetry-on raw JSON SHA-256 | Failure before pointer stroke |
| --- | --- | --- |
| C300 | `a09715bedc15d44e312830e7d64fd344bbd15769de08b003e538f093eb026c67` | `Normal control offscreen settings.symmetry` |
| C301 | `553e5d024fd65430a6063dbec5a0b947f38da9201563c36aec199afa6e40ae7a` | `Normal control offscreen source_geometry.mirror` |
| C303 | `0901c21d4feff9347601a119ec16a2621bb551e6b1490247b8091324480f015e` | `C303 ordinary Source mirror control absent` |

C300's symmetry-off Source/OBRush pair passed the frozen XYZ, active GPU
display-normal, changed-ID, mask, public history and GPU-word scorer. Both
symmetry-on Source captures completed, but no OBrush on-case reached the
stroke. C301 changed only the normal-UI selector; C303 additionally tried a
bounded inspector wheel scroll. Neither fixed the missing control.

Static source inspection identified the cause after C303: the Source geometry
panel only calls `source_geometry_alpha_panel` when the tool is **not** Drag.
That function alone rendered and tracked `source_geometry.mirror`. Thus the
inspector rectangle is absent for Drag; C300/C301's shared normal-control
helper used a generic “offscreen” error for both absent and offscreen cases.
C303's explicit absence check resolved the ambiguity. No geometry tolerance
or failed receipt is reinterpreted as a pass. The distinct C304 preregistration
records the visible-control candidate and unchanged numerical gate.
