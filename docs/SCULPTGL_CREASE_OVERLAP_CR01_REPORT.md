# CR01 retained repeated-Crease comparison: protocol failure, useful observations

CR01 was frozen in `SCULPTGL_CREASE_OVERLAP_CR01_PROTOCOL.md` before browser use. The independent 18,242-vertex closed sphere was imported through ordinary browser controls in separate clean 1600×1000 DPR1 sessions. Pinned SculptGL was commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`; OBrush was the official root web build with portable WASM SHA-256 `f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`. The fixture SHA-256 was `572bed8539b5fb2606f9362e12e94964aa24b2b232f404197b8ab930346aaa08`.

**CR01 fails its frozen paired gate.** Its preregistration required pressure effects off, but the visible OBrush default Crease published `pressure_strength=true`, `pressure_radius=false`, `adaptive_detail=false`, radius `0.180000007`, strength `0.349999994`, X symmetry on, and continuous off. We kept the default to observe the user path. Do not count the two browser scripts' scoped `passed` status strings as a protocol pass or as Crease parity. A distinct CR02 is needed for the actual default-pressure route. Other preregistered details were not fully measured: the OBrush per-dab primary/mirror admission was inferred from symmetric changed vertices rather than instrumented directly, and the reference trace recorded 24 Crease calls per stroke without sign tags. Exact geometry/pixel parity was never an objective of this gate.

| Observation | Pinned SculptGL | OBrush |
|---|---:|---:|
| Visible Crease radius/intensity | 57 CSS px, 35%; projected `0.180` was `56.8521` px | local radius `0.180`, strength 35%; camera path span relative error `2.27e-8` |
| Per-stroke admitted Crease calls | 24 each | not instrumented; 225 changed vertices on each X side by stroke 8 |
| Changed vertices at stroke 8 | 450 | 450 |
| New tiny/fold faces, strokes 1–8 | 0 / 0 | 0 / 0 |
| Maximum corresponding edge stretch by stroke 8 | 2.356× | 2.545×, below frozen 4× alarm |
| ROI RGB pixels changed at stroke 8, threshold >4 | 17,378 | 24,967 |
| Eight Undo and eight Redo stages | exact ordered XYZ | exact saved XYZ |

OBRush retained exactly 18,242 finite vertices and 36,480 triangles through all eight strokes. Each stroke produced a changed surface (426–450 vertices relative to baseline); no post-release changes were observed in Save comparisons at 200 and 800 ms. The serialized hidden peer object, including all fields in the saved scene object, retained the same SHA-256 at each stroke. Those are conservative mesh-safety checks on one smooth, moderate-scale sphere, not proof against the reported slivers in denser, more deformed or very close-up cases.

The fixed 361×360 CSS-pixel ROI, enlarged 2× by nearest-neighbor only for inspection, shows no visible triangle slivers on this fixture. OBrush's first stroke changed 426 vertices but zero ROI pixels exceeded the >4-channel threshold; this could be subthreshold color movement or a frame-timing issue, and warrants a separate display-timing check. At stroke 8, the OBrush ROI had maximum channel change 45 and RGB RMS 3.77; the reference had 109 and 5.94. Both had zero background-colored pixels within the frozen radius-150 interior disk at baseline and final. Different shading, lighting and pressure behavior mean those pixel values cannot be treated as fidelity scores. SculptGL retained its red brush cursor in the final crop even after a pointer move outside the canvas, so its changed-pixel statistics include UI overlay pixels. At the retained crop scale the grooves look continuous; a closer camera-framed visual comparison is still needed.

The retained source receipt is `.artifacts/sculptgl-crease-overlap-cr01/3e4f00cd-f15a-423e-8f24-86026623dbc4/result.json` (SHA-256 `46145565c5fb114d1b65555305c83bb6486b8a046f04c36d5e14d7451737bb74`). The OBrush receipt is `.artifacts/sculptgl-crease-overlap-cr01/f25b9e87-eef8-4c64-8315-2863e87963c3/result.json` (SHA-256 `0614e904307744f02aa34751ff13da39eb85e53503b858c3973fb2db0fa01aad`). Both directories contain baseline, every-stroke, and derived fixed-ROI images, plus all normal Save snapshots on the OBrush side. Failed setup receipts remain under the same ignored artifact directory: the first Source attempt had a harness path-shadowing error, and five OBrush attempts exposed import/selection/event-slicing/triangle-schema harness assumptions; none was substituted for the final observations.

Full SculptGL Crease, pressure, and ZBrush parity rows remain open.
