# Smooth Surface Nets comparison (C84)

OBRush now offers **Smooth Surface Nets** beside its existing triangle voxel
remesher and stepped Block mode. The new option uses one vertex in each
mixed-sign voxel cell, averaging its edge intersections, and connects those
vertices around crossing edges. It uses the shared Rust core for native and
WASM; choosing it leaves the original triangle remesher available. Block
and Smooth Surface Nets are exclusive visible choices.

The pinned hosted SculptGL bundle is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
On the two independently authored closed meshes already paired for Block,
the complete Smooth Surface Nets cell sets and oriented triangles agree
after splitting the reference quads. The reference mesh's recorded output
transform is applied before measuring world positions:

| Fixture | Resolution | Vertices | Compared triangles | Core max / RMS world error | Browser-save max / RMS world error |
| --- | ---: | ---: | ---: | ---: | ---: |
| Triangulated cube | 64 | 24,578 | 49,152 | `1.296e-7` / `5.161e-8` | `1.446e-7` / `4.927e-8` |
| Triangulated sphere | 32 | 4,746 | 9,488 | `1.036e-7` / `6.046e-8` | `1.236e-7` / `6.254e-8` |

The `2e-7` world-unit comparison tolerance was selected after the initial
core diagnostic and before running the browser cases. Every paired cell has
a unique vertex, with zero missing cells and zero unmatched oriented
triangle IDs. These numerical errors measure output-coordinate conversion,
not an exact bitwise match. The browser cases use the actual Smooth Surface
Nets checkbox, resolution control, Remesh action, saved project, visible GPU
buffers and exact Undo/Redo. A Block sphere run also passed after adding
the new mode control. A separate native test checks constant mask, RGB and
material channels across the new topology, and rejects budget exhaustion
without changing the source mesh.

An initial core attempt retained OBrush's `0.02`-voxel distance floor. The
sphere then had a `0.0029733` world-unit maximum vertex error despite matching
cell topology. Removing that floor only for Smooth Surface Nets brought its
maximum below `1.1e-7` in the native core. The failed output is retained in
the [C84 evidence](evidence/sculptgl-smooth-surface-nets-c84-0.1.20.json).

The current public OBrush mesh splits each generated quad into triangles;
it does not store a persistent quad remesh result. The sphere reference
contains corner UVs, while the OBrush input omits those references under its
current topology-edit metadata policy. Reference voxel RGB/material
transfer, open and arbitrary sources, selected multi-mesh fusion, resolution
8–400, the separate Marching Cubes command, native GUI usability and
sustained browser responsiveness remain unverified. These two paired
fixtures do not establish the complete SculptGL remeshing family or parity.
