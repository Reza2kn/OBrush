# C257: matched-pressure held Source Draw diagnostic

C256 established that its fourfold radius mismatch arose at browser input:
the owned Chrome mouse route emitted trusted `PointerEvent.pressure = 0` on both
pages; pinned SculptGL used that value while OBrush normalized mouse contact
to `0.5`. C257 keeps C256 and all failed receipts intact, and tests a separate
**normal pen event route** at pressure `0.5`. This is a scoped diagnostic, not
a full brush, symmetry or render parity claim.

Use Chrome DevTools Protocol `Input.dispatchMouseEvent` with
`pointerType: "pen"` and `force: 0.5` for `mousePressed` and every held
`mouseMoved`, then `force: 0` for `mouseReleased`. The protocol documents
`pointerType` values `mouse`/`pen` and normalized `force` in `[0,1]`:
<https://chromedevtools.github.io/devtools-protocol/tot/Input/#method-dispatchMouseEvent>.
Record the actual trusted PointerEvent `pointerType`, `pressure`, buttons and
CSS coordinates on **both** pages. Do not synthesize DOM events or assign
`Tablet.pressure`/OBRush settings directly. If Chrome does not deliver the
documented pen path, fail and preserve the receipt; do not substitute a
different host route within this gate.

Freeze the same pinned SculptGL source commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, independent 540-byte
`independent-affine-two-mesh.sgl` SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`,
active translated triangle and peers, normalized SymOffset `+0.5`, Solo,
OBRush mask conversion, ordinary Source/Draw, nominal radius `250`, intensity
`0.5`, alpha None, X symmetry and no continuous stroke. Use the same held
local points `(3.70,-1.99,0) → (3.845,-1.99,0) → (3.99,-1.99,0)` with a fresh
owned 1600×1000/DPR1 Chrome profile and the normal visible app controls.
Keep the reference default camera and match OBrush's path span using its
visible Source projection-zoom numeric field. Require ≤1% relative span
error, every point inside the sculpt canvas and path span >`37.5` CSS px
before the stroke. Preserve exact camera, source/build/harness hashes and
every failed attempt.

Before comparing deformations, require each side's trusted held
`pointerdown` and `pointermove` to be `pen` at pressure `0.5` (absolute
tolerance `1e-6`). Require SculptGL's actual original-call primary picker and
OBRush's actual admitted Source pass to report pressure factor `1`, nominal
screen radius `250`, X offset `250` CSS px and primary/mirror radius
self-consistency within 1% or `1e-5` local units, whichever is larger. Record
the complete original-call radius chain as in C256, including projected hit,
unprojected edge, camera/viewport and mesh pose. If any pressure/radius gate
fails, classify it as input/setup or geometry-preparation failure, and do not
interpret pixel or vertex differences as brush parity evidence.
At the C256 matched-camera scale, a 250 CSS px radius predicts approximately
`1.39778` local units and radius² `1.9538` on either side; the **actual**
original-call and admitted-pass values, rather than those rounded predictions,
must agree to the stated tolerance.

Save OBrush's active three vertices before/after using the visible Save
project action and owned download directory. Read the corresponding SculptGL
active vertices before/after from its pinned mesh instance without mutation.
Report each vertex's X/Y/Z delta, maximum absolute paired component error,
the largest displacement and the ratio for its nonzero component. Mark a
**geometry candidate** only when every component error is at most `0.005`
local units; this threshold is a diagnostic screen, not row closure. Capture
fixed-cursor before/after screenshots on both sides, count pixels exceeding
RGB channel difference `4` in the frozen radius-90 CSS crop about the first
stroke point, and report the count and ratio. The crop counts are descriptive:
different renderer color and silhouette still confound visual equivalence.

An acceptable C257 scoped result proves pressure/radius match and complete
vertex/pixel observations, even if geometry or appearance mismatch. Preserve
the mismatches and advance to a new preregistered correction gate. If either
side has no admitted dab, no active-face mirror, incomplete downloads or
nonzero GPU error, mark this attempt failed rather than weakening the
conditions. C256's mouse-pressure finding remains separately valid.

The separate runner is `scripts/check-sculptgl-symmetry-pressure-c257.mjs`.
`node --check` and its `--self-check` mode are static and do not open Chrome;
the full paired run must wait for explicit release of the shared browser lane.
