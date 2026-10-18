# Open-quad mask filter comparison (C81)

The optional SculptGL Mask Blur and Sharpen response introduced in C80 now has
a second paired reference fixture. The independently authored `3 × 3` open
quad grid carries nine `#MAT` mask weights, including a partly masked corner
and a fully masked center. The pinned hosted SculptGL bundle imported the OBJ
and ran both filters through its visible Masking controls. OBrush imported the
same OBJ through its shared Rust core and through its browser file control.

SculptGL reorders OBJ vertex IDs, so comparisons use exact source XY positions
and convert OBrush protected weight `P` to SculptGL unprotected weight `U=1-P`.
For all nine vertices, Blur's maximum `U` difference is
`2.9802322387695312e-8`; Sharpen matches exactly. Blur leaves the low-valence
corner unchanged, averages its two boundary neighbors along the edge, and
changes the center from `U=0` to `U=1`. The original quad edges matter: using
the triangulated diagonal would produce different values.

The source-only reference run records before/after positions, normals, colors,
materials, faces, UVs and history for both cube and open-quad fixtures. Its
four filter cases changed only mask materials. The OBrush core test fixes the
reference values by source position. The browser test imports both fixtures,
enables the visible response option, runs both actions, checks every output
weight, unchanged shape and paint, saved document and visible GPU buffers,
and exact Undo/Redo. The full mask-filter browser suite was rerun after adding
the quad cases.

This extends the paired result to one open quad grid; it does not prove
arbitrary edge/nonmanifold behavior, dynamic topology, repeated large-mesh
actions, physical input, native GUI behavior, long-session responsiveness or
full SculptGL/ZBrush parity. The evidence index is
[C81 mask-filter evidence](evidence/sculptgl-mask-filters-c81-0.1.20.json).
