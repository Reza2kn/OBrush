# Q03 normal-camera close-up of retained Q02 Move states

**Frozen before Q03 browser contact.** Use the exact public `.obrush` files
from [Q02](NATIVE_MOVE_REPEATED_VISUAL_Q02_RESULT.md): initial SHA-256
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`
and final SHA-256
`5aa199b064dec5aea4f924e4fac030fd402f5119b6b301b4729914c7f8233632`.
The first is a fresh sphere and the second is its eight effective small-radius
Move states; no Q03 sculpt event may alter either. Use the current official
root web build in an owned isolated Chrome at CSS 1600 × 1000, DPR 1,
verify local/served WASM equality, and record the browser/build hashes.

For each state, open a fresh normal app tab and load the project through its
public browser file input. A direct CDP chooser file selection is allowed;
geometry mutation APIs are not. Verify a visible single active sphere and
Save a public project immediately after load. Require its mesh to equal the
input mesh byte-for-byte. Starting at the default camera, perform the same
ordinary navigation sequence in each tab: middle-button drag from CSS
`[650,470]` to `[800,470]` in twelve evenly spaced held moves, then five
wheel-up events (`deltaY=-120`) at `[800,470]`, with an animation frame
between wheel events. Move the pointer to `[1000,30]`, wait for camera
settling, and capture a full original-resolution PNG. Save again and require
its mesh to remain byte-identical to the imported public state.

Camera gate: both resulting cameras must have finite pose/target/projection,
distance in `[1.0,1.6]`, and matching distance, target and view-projection
elements within `1e-5` absolute. The fixed visual inspection region is
`x=550..1080, y=260..740` in the 1600 × 1000 PNG, at native one-image-pixel
per CSS pixel; do not rescale screenshots for a pass. Require at least 100
pixels in this region with an RGB channel difference above 8 between matched
before/after frames so that the deformation is actually visible. Preserve
full screenshots and report image width/height, ROI and hashes.

At original pixels, inspect whether the final state exposes sharp triangular
spikes, detached-looking slivers or stepped faceting around the changed
surface; compare directly with the baseline crop at identical camera pose.
An absence of those features in this one zoomed state is a scoped visual
witness only. A failed camera, public-file, image-difference or capture gate
remains a failed Q03 run; do not tune the frozen camera path or threshold
post hoc. Q02's normal-browser numerical checks and M03's native failure
remain separate, and no ZBrush/SculptGL parity status follows.
