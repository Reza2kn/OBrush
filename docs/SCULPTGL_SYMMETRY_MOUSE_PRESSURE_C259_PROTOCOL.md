# C259: matched mouse-pressure held Source Draw pair

C258 isolated the failure of C257's `pen` host path: SculptGL's Hammer pan
handler restarted the stroke at the midpoint, leaving no dab. Its separate
normal Chrome `mouse` pointer arm with explicit `force: 0.5` delivered trusted
held pressure `0.5`, factor-`1` radius, one admitted `makeStroke`, two brush
passes and a right-vertex Z delta of `+0.1373661757`. C259 preregisters a
**new** normal-browser SculptGL–OBRush pair using only that working mouse
route. It does not reinterpret C257 as a pass or close a full parity row.

Freeze pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
bundle SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
independent 540-byte fixture SHA-256
`99e107226fd5bdfeb687a4d92a61eebcec1abb2e910184e35aa58808ace105fd`,
one visible active triangle with peers preserved, normal GUI Solo, active
SymOffset `+0.5`, ordinary Draw, nominal radius `250`, intensity `0.5`, alpha
None, X symmetry, no continuous mode and local points
`(3.70,-1.99,0) → (3.845,-1.99,0) → (3.99,-1.99,0)`.
OBRush must enter ordinary Source after visible mask conversion, save its
active triangle before/after through the visible project Save action and
keep peer identities unchanged. Use a fresh owned Chrome profile at
1600×1000/DPR1 for the pair; preserve source/build/harness hashes, exact
camera and all failed attempts. Match the projected path span to ≤1%
relative error using OBrush's visible Source projection-zoom field, with
each point inside both sculpt canvases and span >`37.5` CSS px.

Send `Input.dispatchMouseEvent` with `pointerType: "mouse"` and `force: 0.5`
for press and every held move, then force `0` for release on both pages.
Before comparing geometry, require actual trusted held pointerdown and
pointermoves to be `mouse` at raw pressure `0.5` (absolute tolerance `1e-6`).
SculptGL's original primary picker and OBrush's admitted ordinary Source
pass must each report captured pressure `0.5`, pressure factor `1`, nominal
screen radius `250`, actual projected/unprojected X offset `250` CSS px,
and local primary/mirror radius consistency to ≤1% relative or `1e-5` local
units, whichever is larger. Require paired local radii within that same
tolerance after camera matching. Preserve the full camera/project/unproject/
mesh-pose chain. The OBrush pass should identify a mouse input (`pen=false`)
while retaining raw/captured pressure `0.5`; do not call it a pen test.

Before OBrush, require the reference active triangle to change and each of
its nine component deltas to be within `0.005` local units of C258's
`[0,0,0, -0.0015718937,+0.0005894899,+0.1373661757, 0,0,0]`.
Require the reference fixed-cursor screenshot to change at least
`500` pixels in the frozen radius-90 CSS crop about the first path point,
where an RGB channel change `>4` counts. C258's matching mouse arm changed
`3317` pixels by that method; the `500` threshold protects against another
cursor-only/no-dab run without scoring renderer fidelity. If these controls
fail, preserve the receipt and do not interpret an OBrush difference.

Read the three XYZ vertices on each side before and after. Report all nine
component deltas, max absolute paired component error and dominant-component
ratio. Mark a **geometry candidate** only if every paired component differs
by at most `0.005` local units; retain any mismatch. Capture fixed-cursor
before/after screenshots on both sides, count changed pixels with the same
radius-90/threshold-`4` crop, require at least `20` changed OBrush pixels to
exclude a no-op, and report count, bounding box and OBrush/reference ratio.
The count and ratio are descriptive because renderer color and silhouette
remain different; no pixel-ratio threshold establishes visual parity.

Require active-face mirrored picking and a completed OBrush Source pass,
normal project downloads, zero WebGL errors and unchanged peer IDs. Any
failure is a failed scoped attempt, not a reason to relax the frozen gates.
C259 can establish matched-pressure geometry evidence for this fixture only.
Full brush, symmetry and visual parity remain open.

The separate runner is
`scripts/check-sculptgl-symmetry-mouse-pressure-c259.mjs`. Its syntax and
`--self-check` mode may run statically; the full paired browser run waits for
explicit release of the shared Chrome lane.
