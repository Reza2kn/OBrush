# Q05 hover-corrected Move close-up

**Frozen before Q05 browser contact.** [Q04](NATIVE_MOVE_CLOSEUP_Q04_RESULT.md)
failed because three wheel events left camera distance unchanged at
`3.27466154`; unlike Q03, it had sent no pointer movement over the viewport.
Q05 tests only that setup difference. It retains Q04's exact initial/final
Q02 public `.obrush` files and SHA-256 hashes, browser size 1600 × 1000/DPR
1, three wheel-up `deltaY=-120` events at `[800,470]`, original-resolution
screenshots, camera distance `[1.4,1.7]`, paired camera tolerance `1e-5`,
fixed ROI `x=350..800, y=330..650`, and at least 100 pixels with an RGB
channel difference above 8. No mesh or visual threshold changes.

After public Open and immediate Save/mesh equality, send one ordinary mouse
move to CSS `[800,470]`, wait one animation frame and 70 ms, record camera,
then run the three wheel events with an animation frame and 70 ms after each.
Move pointer to `[1000,30]`, settle 300 ms, capture the full PNG, and Save
again to require byte-identical mesh. The harness must verify the initial
state's wheel sequence reaches the camera gate before opening the final
state. Both initial and resulting camera pose/target/projection must match
across states within `1e-5` and be finite. If the ordinary move is still
insufficient, preserve failure and stop; no further tuning in Q05.

Inspect final and initial original pixels in the ROI for triangular spikes,
detached-looking slivers and stepped faceting. A clean result is one scoped
visual witness only; it cannot close the user's repeated Move tearing report
or establish SculptGL/ZBrush parity. Q03 and Q04 remain failed setup runs.
