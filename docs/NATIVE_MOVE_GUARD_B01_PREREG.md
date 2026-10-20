# B01 normal-browser small Move guard candidate

**Frozen before B01 browser contact.** The candidate changes only the
`MoveStroke::apply` adjacent-fold limit from 90° to 30°. The independent
core analogue at radius `0.037`, strength `0.79`, X symmetry changes 1,016
vertices, adds 988 vertices, and moves an original vertex at most 0.010749
world units at 29.993° maximum fold. B01 tests the normal browser route
against the previously retained Q02 repeated-contact sequence. It does not
reinterpret Q03–Q06 failed camera gates.

Build the candidate with `scripts/build-web.sh` in this isolated worktree,
record exact portable/SIMD WASM and harness SHA-256, and serve it in owned
Chrome 1600 × 1000/DPR 1 with local/served WASM equality. Use
`scripts/check-move.mjs` with only `sphere-twelve-repeated-q02` selected and
`OBRUSH_MOVE_GUARD_CLOSEUP=1`; do not change the Q02 visible Move selection,
radius `0.037±0.00011`, strength `0.79±0.0011`, X symmetry or backface
culling. Replay all twelve ordinary browser mouse contacts, twelve held
samples/contact at 30 ms, including the four final overlapping contacts.
Save after every contact and prove no queued post-release geometry by
the idle Save. Require at least four effective contacts, at least eight
original vertices changed, maximum original-vertex displacement `>=0.005`,
finite closed consistently wound mesh, minimum triangle quality `>=0.09`,
and maximum adjacent fold `<=30.01°` after every contact. Require exact
GPU/saved geometry, unchanged mask/color, and one exact Undo per effective
contact followed by exact Redo to final.

Before Undo, use one ordinary pointer move to viewport `[800,470]`, then
three wheel-up `deltaY=-120` events at that point with an animation frame
between. Move pointer to `[1000,30]`, settle 300 ms, save final close-up
PNG and public project. Require wheel distance decreases and zoom Save mesh
equals pre-zoom final mesh. Use the already-required public Undo chain in
the **same tab** to reach the exact initial mesh without re-import, then
capture initial close-up PNG and camera metrics before Redo. Require
distance, target and view-projection match within `1e-5` absolute; Redo
must restore the exact final mesh. Inspect original 1600 × 1000 pixels in
fixed ROI `x=260..800,y=250..650` for sharp triangular spikes, detached
slivers and stepped faceting. Require at least 100 pixels with any RGB
channel difference above 8 to call the image comparison informative.
Preserve full images, mesh saves and failures. This candidate witness cannot
close the user's broader Move complaint or establish SculptGL/ZBrush parity.
