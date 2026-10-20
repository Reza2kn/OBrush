# C259: matched-pressure Source Draw geometry candidate

The separately preregistered normal-browser C259 pair passed its scoped
observation gates. Pinned SculptGL and the official OBrush browser build each
received trusted `mouse` pointerdown and two held pointermoves at raw
pressure `0.5`. SculptGL's original picker used factor `1`; OBrush's
ordinary Source pass captured a normalized mouse pressure `0.5` with
`pen=false` and factor `1`. The source reference exactly reproduced C258's
nine-component vertex delta, so the pen/Hammer no-op from failed C257 did
not recur. This is evidence for the **one independent fixture and held Draw
path**, not full symmetry, brush, appearance or SculptGL parity.

The imported active triangle, peers, Solo, SymOffset `+0.5`, ordinary Draw,
radius `250`, intensity `0.5`, alpha None, X symmetry, local three-point path
and 1600×1000/DPR1 Chrome profile followed the frozen protocol. The path
spanned `51.86835917` CSS px in SculptGL and `51.86835983` CSS px in OBrush:
relative difference `1.2607e-8`, below the `1%` limit. OBrush's visible
Source projection zoom was set to `6.49216314509`; the receipt preserves
both camera matrices and wheel/field actions. The active loose-root symmetry
radius was exactly `2.692582607269287` local units per normalized offset
on both sides. The visible guide moved, active-face mirrored picking passed,
and OBrush's admitted pass recorded primary-before-mirror contact. OBrush
history advanced from 4 to 5 undo entries and peer IDs remained `[1,2,3]`;
the normal Save action downloaded before/after `.obrush` documents.

| Original-call measure | SculptGL | OBrush |
| --- | ---: | ---: |
| Raw held pointer pressure | `0.5` | `0.5` |
| Captured brush pressure | `0.5` | `0.5` |
| Project/unproject X offset | `250` CSS px | `250` CSS px |
| Local brush radius | `1.3977691101` | `1.3977692933` |
| Local primary radius² | `1.9537584851` | `1.9537589973` |
| Changed RGB pixels in frozen crop | `3317` | `3297` |

Every component was within the preregistered `0.005` local-unit geometry
candidate tolerance:

| Active vertex delta | SculptGL Δ X/Y/Z | OBrush Δ X/Y/Z |
| --- | --- | --- |
| 0 | `(0,0,0)` | `(0,0,0)` |
| 1 | `(-0.0015718937,+0.0005894899,+0.1373661757)` | `(-0.0015707000,+0.0005890000,+0.1373114100)` |
| 2 | `(0,0,0)` | `(0,0,0)` |

The maximum absolute paired component difference was
`0.00005476565155` local units (vertex 1 Z), about `91.3×` smaller than
the candidate tolerance; the dominant Z displacement ratio was
`0.9996013163`. In the preregistered radius-90 CSS crop, an RGB channel
difference `>4` counted `3317` SculptGL pixels and `3297` OBrush pixels,
ratio `0.9939704552`. Their changed bounding boxes, relative to each crop's
first path point, are similar, but this count comparison is descriptive:
renderer colors, lighting, silhouettes and camera projection are not scored
for visual equivalence here. Both stroke WebGL error checks were zero.

The final receipt is
`.artifacts/sculptgl-symmetry-mouse-pressure-c259/3fe42b25-b081-4269-a13e-9fbe744ca020/result.json`,
SHA-256 `6ac1e87c87942a1f8bc326a53addd4ddd4d5e43740c854a5b791a97ee41d39f5`.
It retains the exact cameras, event and original-call radius chains, masked
mode transition, guide GPU uniforms, Source dab report, saved before/after
documents and screenshot hashes. Pinned SculptGL source commit is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
independent fixture SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`,
C259 harness SHA-256
`ec5b267527cbad75ad840d9639def0856c754aa3425eac854dc75d5b3d346bc1`,
official OBrush portable WASM SHA-256
`f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`,
and root checkout commit at run time
`c8d1153783d372516f1a6954b8bb3015f3e57c36`. No C259 setup failure
occurred; C257 and C258 receipts remain separate and preserved. Full
SculptGL parity rows remain open.
