# SculptGL mask Blur and Sharpen response (C80)

OBRush now has an optional **SculptGL Blur / Sharpen response** control in the
Mask panel. It uses the same display-free Rust core in native and browser
builds. The original OBrush filter policy remains available; Grow and Shrink
still use that policy. The SculptGL option makes Blur and Sharpen one-pass
actions, so the OBrush passes and amount fields are disabled while selected.

The pinned hosted SculptGL bundle was run through its ordinary OBJ import and
visible Masking controls on an independently authored triangulated cube with
eight different `#MAT` mask weights. The same OBJ was imported by OBrush. Both
outputs were compared by source position after converting OBrush protected
weight `P` to SculptGL unprotected weight `U=1-P`; the reference reordered
vertex IDs during import. Sharpen's eight output weights match exactly. Blur's
largest difference is `2.9802322387695312e-8` in `U` (one f32 rounding step).
SculptGL changed only mask material values on this fixture. OBrush's browser
workflow checks saved geometry, visible GPU buffers, useful edit history and
exact Undo/Redo for both actions.

The reference Blur uses one-ring expansion around vertices with `U<1`,
neighbor-only averaging, and a boundary-neighbor rule. Sharpen adds `0.1` to
masked values above `0.5` and clamps values at or below `0.5` to zero. OBrush
implements these rules for original triangle and quad edges with bounded
temporary arrays. A separate authored quad-grid test checks that boundary
averaging excludes the center and quad diagonals. The existing no-op/history
policy stays OBrush's and is not claimed as exact SculptGL behavior.

This is a paired result for one closed triangulated cube, plus an independent
quad boundary test. Dynamic topology, nonmanifold edge flags, repeated actions
on large meshes, exact blur accumulation order on arbitrary valence, physical
input, native GUI behavior, long-session responsiveness, and full SculptGL or
ZBrush parity remain open. [The evidence index](evidence/sculptgl-mask-filters-c80-0.1.20.json)
records reference/build hashes and the browser and native checks.
