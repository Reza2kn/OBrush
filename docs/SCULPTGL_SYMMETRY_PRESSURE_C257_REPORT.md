# C257 matched-pen Source Draw: failed reference stroke

C257 attempted the preregistered held-stroke pair using Chrome's documented
`Input.dispatchMouseEvent` pen route (`pointerType: pen`, `force: 0.5`). It did
**not** produce a paired deformation result. Pinned SculptGL received trusted
`pointerdown` and both held `pointermove` events at pressure `0.5`, and its
original picker used pressure factor `1` and a `250` CSS px radius. Its active
triangle nevertheless remained byte-identical after the held stroke. The
runner stopped at the required reference deformation gate, before navigating
to OBrush. No C257 per-vertex difference, pixel ratio, or parity conclusion
can be reported.

The imported three-vertex active mesh had positions `[1,-2,0, 4,-2,0,
2.5,2,0]` before and after the stroke. The mirrored picker ended on face `0`
of active mesh `2`, at local point `(3.7053562515,-1.9848323566,0)`, with
local radius squared `1.9537585174`, but its final picked-vertex array had
length `0`. The actual original-call pressure-`0.5` picker records had factor
`1`, projected/unprojected X offset `250` CSS px, and local radius squared
about `1.9537585`. The held path projected from `(859.6277,875.9243)` to
`(911.4960,875.9243)`, a `51.8683` CSS px span above the pinned `37.5` px
spacing threshold. The reference ROI changed only `34` pixels, bounding box
`(858,873)–(912,876)`, consistent with the cursor path; it is not evidence of
sculpted geometry. The report records this observation without inferring that
input admission or the brush calculation succeeded.

Two independent failed run directories are retained under
`.artifacts/sculptgl-symmetry-pressure-c257/`:

| Receipt | SHA-256 | Failure |
| --- | --- | --- |
| `1fc8d5f0-058b-4bdf-9913-5ddbf642966a/result.json` | `28654436ca89a477d6b26540e1f399f29e3d23e94d859fb91785b4c347a07e34` | Initial event assertion counted a pre-contact pen hover (`pressure=0`, `buttons=1`) as a held stroke. |
| `6e3c8823-83f5-45f3-af5f-48b5b2a3063c/result.json` | `ffe42fc95ed7ff0a8fd676926147927701d66730598c9211da510b78434c5738` | The corrected post-`pointerdown` event gate passed; SculptGL did not deform the triangle. |

The second run used harness SHA-256
`92f470ed6d6b270f1879a43ff30290cb972bf8c7d226c0c922e4aa11b0934249`,
official OBrush WASM SHA-256
`d61ca3e060dad45fd769afc6fa460f50d1d2ebd432ab121b7a86b09b5558e6d2`,
SculptGL source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and independent `.sgl` fixture SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`.
The skipped OBrush route means its build hash identifies the available
artifact, not a measured C257 result. C256's mouse-pressure finding remains
valid. SculptGL symmetry and Draw/brush parity rows remain open.
