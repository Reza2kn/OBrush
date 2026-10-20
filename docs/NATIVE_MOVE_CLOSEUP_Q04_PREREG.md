# Q04 camera-corrected Move close-up

**Frozen before Q04 browser contact.** Q03 failed its fixed camera gate and
missed the edited surface; its [failed result](NATIVE_MOVE_CLOSEUP_Q03_RESULT.md)
remains unchanged. This separate Q04 fixture uses the same exact Q02 public
initial and final projects (SHA-256
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`
and `5aa199b064dec5aea4f924e4fac030fd402f5119b6b301b4729914c7f8233632`).
The previous public initial project opened at camera distance `3.27466154`.
Three visible wheel-up steps produced `1.54684055` in Q03. Q04 therefore
removes the middle drag, which displaced the Q02 edited marks out of view,
and uses exactly three wheel-up events of `deltaY=-120` at CSS `[800,470]`.
This is an observed, geometry-independent camera correction, not a retuned
sculpt or quality threshold.

Use fresh normal app state for each project, owned Chrome at 1600 × 1000 CSS
pixels/DPR 1, and current official OBrush WASM with exact local/served hash.
Load each through the public file input, Save immediately and require exact
mesh equality to the input. With no drag or brush action, apply the frozen
three wheel events with an animation frame between each, move pointer to
`[1000,30]`, settle 300 ms, capture original-resolution PNG, then Save again
and require mesh unchanged. Both resulting cameras must be finite, distance
in `[1.4,1.7]`, and distance/target/view-projection elements paired within
`1e-5` absolute. The initial camera poses must also match each other within
`1e-5`, or comparison is invalid.

The fixed visual region is `x=350..800, y=330..650`. Record full PNG
dimensions, SHA-256 and original-pixel RGB difference count in this ROI.
Require at least 100 pixels where any RGB channel differs by more than 8.
Inspect the final and initial images at original pixels for sharp triangular
spikes, detached-looking slivers or stepped faceting in the Q02 edited area.
No rescaling may support a pass. A clean result is a scoped visual witness,
not closure of the user's repeated Move tearing complaint or SculptGL/ZBrush
parity. Preserve every failed run.
