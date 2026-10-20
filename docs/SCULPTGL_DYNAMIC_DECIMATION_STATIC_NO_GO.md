# Local dynamic decimation: existing-QEM reuse no-go

Pinned SculptGL source `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
`src/mesh/dynamic/Decimation.js` SHA-256
`830931f9100779407595262fd8027c6505c78c098dbaa2bfbea637533c705db6`.
The current OBrush `crates/obrush-core/src/decimate.rs` is an independently
written global quadric-error-metric operation. This is a static source-level
comparison, not a paired reference-runtime result.

The pinned dynamic operation traverses the picked face set and chooses each
face's shortest edge, with a squared-length threshold from local radius,
falloff, decimation factor and the mean of its two unprotected mask weights.
It can flip a crowded edge; otherwise it averages survivor RGB/material
channels and moves the surviving position to a smoothed one-ring point
projected onto its tangent plane. Boundary and opposite-vertex ring checks
reject some candidates. Face/vertex arrays are compacted by swapping the
last row, which affects the subsequent visited order.

OBRush's current QEM ranks a global edge heap by quadric error, pins boundary,
masked and optional sharp vertices, applies its own candidate guards and
attribute interpolation, then compacts once. Adding a center/radius pin to
that QEM could be useful as an independent local simplifier, but it does not
reproduce the reference's candidate ordering, edge flips, survivor position
or material response. No QEM-region candidate was committed or tested as
SculptGL parity. A future candidate needs an independently authored dense
dynamic patch, exact source-runtime collapse and channel evidence, bounded
memory/time, exterior invariants, and one-step Save/Undo/Redo before it can
make even a scoped parity claim.
