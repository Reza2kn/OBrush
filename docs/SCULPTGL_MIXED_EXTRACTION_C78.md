# Mixed triangle/quad mask extraction (C78)

This check uses an independently authored closed cube with five original
quads and one original quad split into two triangles. Only vertex 1 is masked.
It belongs to two quads and one of the split triangles; the other triangle
must not be selected merely because its former quad neighbor is selected.
The same OBJ was imported through pinned SculptGL's normal file and Masking
controls and through OBrush's shared Rust import/extraction path. Threshold
was 0.5 and signed thickness was 0, −0.125, and +0.125 world units.

| Thickness | SculptGL | OBrush | Largest reference-to-OBRush nearest-point gap |
| --- | --- | --- | ---: |
| 0 | 6 vertices, 5 triangles, 5 open edges | same | 0 |
| −0.125 | 12 vertices, 16 triangles, 8 open edges | 12 vertices, 20 triangles, closed | 1.352668886 |
| +0.125 | 12 vertices, 16 triangles, 8 open edges | 12 vertices, 20 triangles, closed | 1.389243058 |

At zero thickness, the unordered vertex positions agree exactly on this
fixture. That corroborates admission of the two selected original quads and only
the incident original triangle. It does not establish face-order, UV, or
arbitrary mixed-topology parity. The reference's nonzero output is an open
partial bridge. OBrush retains its checked closed shell rather than copying
the open result. Nearest-point gaps do not imply face correspondence.

The shared-core regression asserts the source-face count, zero-thickness
reference positions, closed nonzero shells, and exact source preservation.
The owned browser workflow checks the same three thicknesses through visible
controls, source preservation, undo/redo, saved scene, and actual GPU buffers.
The [evidence index](evidence/sculptgl-mixed-extract-c78-0.1.20.json) records
the hashes and full browser status. Curved and transformed mixed surfaces,
UV-bearing mixed inputs, nonuniform masks, native GUI interaction, pen input,
and sustained performance remain unverified. SculptGL and ZBrush parity
remain open.
