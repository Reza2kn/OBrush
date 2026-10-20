# Q06 matched-camera Move close-up

**Frozen before Q06 browser contact.** Q03–Q05 remain failed paired visual
gates. [Q05](NATIVE_MOVE_CLOSEUP_Q05_RESULT.md) showed public Open frames the
initial and deformed Q02 meshes at different camera distances
`3.2746615409851074` and `3.279921770095825`. The Q02 public `.obrush`
files contain only version/name/mesh and cannot carry Q02's Undo history;
loading final then undoing eight strokes is unavailable. Q06 retains their
exact SHA-256 hashes
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`
and `5aa199b064dec5aea4f924e4fac030fd402f5119b6b301b4729914c7f8233632`.

Use owned Chrome at 1600 × 1000 CSS pixels/DPR 1, verify exact served/local
WASM hash, and public Open/Save to prove exact mesh on each state. Each
fresh tab gets one ordinary pointer move to `[800,470]`, one animation frame
and 70 ms, then three wheel-up events with `deltaY=-120` at the same point,
an animation frame and 70 ms after each. Q05 measured resulting distances
`1.5468405485153198` and `1.5493254661560059`, with the same target and
orientation. Q06 sends one additional **after-state only** fractional wheel
event at the same point with fixed `deltaY=-0.770475948625655`, derived
before this run as `480*ln(1.5468405485153198/1.5493254661560059)` from
the measured `-120 → exp(-0.25)` zoom ratio. This is a camera-only browser
input; it does not modify project geometry. If it does not produce the
expected camera match, preserve failure. Do not adjust this value mid-run.

Move the pointer to `[1000,30]`, settle 300 ms, save a full native 1600 ×
1000 PNG, and Save again to prove mesh unchanged. Both resulting cameras
must be finite, distance within `[1.4,1.7]`, and match distance, target and
view-projection components within `1e-5` absolute. Record initial camera
differences, but they are expected from automatic framing and are not a Q06
gate. The fixed ROI is `x=260..800, y=250..650`, deliberately including
the Q05 sharp bump near `[330,340]`. Require at least 100 original pixels
with an RGB-channel difference above 8; record full image dimensions and
hashes. Inspect the final image beside baseline at original pixels for
triangular spikes, detached-looking slivers and stepped faceting. Report
quantitative mesh-quality/fold metrics from Q02 separately; a Q06 result
cannot imply broad Move, SculptGL or ZBrush parity.
