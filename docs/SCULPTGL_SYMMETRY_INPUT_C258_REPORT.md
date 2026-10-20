# C258: pen restart explains the failed held stroke

The preregistered two-arm SculptGL diagnostic confirmed the C257 no-op's
immediate cause. Both arms delivered trusted held PointerEvents at pressure
`0.5`; original picker calls used pressure factor `1`, nominal radius `250`
CSS px and local radius squared about `1.95376`. The same active triangle,
SymOffset `+0.5`, Draw settings and three local points were used in fresh
1600×1000/DPR1 Chrome profiles. Their projected spans differed by only
`1.09e-6` relative. Both had zero WebGL errors and no page exceptions.

| Original-call observation | Pen `force=0.5` | Mouse `force=0.5` |
| --- | ---: | ---: |
| Hammer `onPanUpdateNbPointers` | 1 | 0 |
| `onDeviceDown` | 2 | 1 |
| `sculptStroke` | 5 | 3 |
| `makeStroke` | 0 | 1 |
| `Brush.stroke` | 0 | 2 (primary + mirror) |
| Primary/mirror selected vertices | none | `[1]` / `[1]` |
| Right vertex Δ X/Y/Z | `(0,0,0)` | `(-0.0015718937,+0.0005894899,+0.1373661757)` |

The pen route's first compatibility `mousedown` entered SculptGL's sculpt
action and set the tool's last position to `(859,835)`. At the first held pen
move, Hammer called `onPanUpdateNbPointers(1)`, which called `onDeviceDown`
again via its event proxy at `(886,836)`. That restarted the tool at the
midpoint. Subsequent `sculptStroke` distances were `1.4142`, `25`, and
`25.01999` CSS px, all below the pinned `37.5` px threshold. No dab or vertex
selection ran. The mouse route had no pan restart: its first move was `26` px
and its second was `52` px from the original start, admitted one `makeStroke`
at `(911,835)`, selected vertex `1` on both sides and deformed it.

As descriptive image evidence, a post-run application of C257's radius-90
CSS crop and RGB-channel threshold `>4` to the parked-cursor screenshots
found `34` changed pixels for pen (bbox `(858,873)–(912,876)`) and `3317`
for mouse (bbox `(856,785)–(912,877)`). This is a same-renderer confirmation
of the no-op versus admitted dab, not an OBrush visual-parity score. Each
Chrome arm logged one missing `favicon.ico` network resource; those logs are
preserved and were unrelated to the sculpt route.

Receipt:
`.artifacts/sculptgl-symmetry-input-c258/702eb431-5dff-4e52-a346-3f3c9ed5adec/result.json`,
SHA-256 `ddc72c888b7add587cd6986dce674b8343c7eb4bc58a61aeb73460b3630c2558`.
The report includes all original-call action, spacing, pick and radius traces,
before/after vertices, actual trusted events and screenshot hashes. Pinned
SculptGL source commit is `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
wrapper SHA-256
`a87cc1246a6bc39cebf81b78dcf6d6c5c6201ef72710dab7f177d1fe4acd11f0`,
fixture SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`,
and harness SHA-256
`f2187d4382cbd5b7d58137e88ce7f4df18b2657d9ce33468e27dc8382cde6aa1`.
The root checkout commit recorded at run time was
`fb64db11676b2130b826ba58a781fd260c3d06c2`.

C257 remains a failed paired gate. C258 validates that Chrome's documented
`mouse` pointer route with explicit `force=0.5` can carry matched raw pressure
and admit a reference dab on this fixture. A separate preregistered OBrush
pair can use that route next. This result closes no SculptGL symmetry, Draw,
or browser-parity row.
