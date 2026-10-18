# Block remesh grid alignment (C83)

The C82 Block remesher matched a paired cube but missed an independently
authored sphere by 22 vertices and 44 triangles. This follow-up changes only
Block mode's sampling grid. The grid begins 1.51 voxel widths outside the
source bounds, with the node dimensions implied by that margin. Each mixed
cell's vertex stays in integer grid coordinates until the completed surface
is aligned to the source bounds. Smooth remeshing keeps its prior grid and
extraction path. The edit remains in shared `obrush-core` for native and WASM.

Against the pinned hosted SculptGL bundle (`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`), complete world-space point multisets and oriented triangles now agree exactly for **both** independently authored inputs:

| Input | Resolution | Points | SculptGL quads | Compared triangles | Unmatched points / oriented triangles |
| --- | ---: | ---: | ---: | ---: | ---: |
| Closed triangulated cube | 64 | 24,578 | 24,576 | 49,152 | 0 / 0 |
| Closed triangulated sphere | 32 | 4,746 | 4,744 | 9,488 | 0 / 0 |

The comparison applies SculptGL's recorded mesh transform and splits each
quad into two triangles. It checks complete arrays rather than a sampled
silhouette. The sphere reference OBJ carries corner UVs; the OBrush fixture
preserves its positions and triangle indices but removes UV references to
meet the current topology-edit metadata policy. Source and runtime snapshots,
the failed 2-voxel and intermediate 1.51-voxel comparisons, and the final
scripts/results remain in the [C83 evidence](evidence/sculptgl-block-grid-c83-0.1.20.json).

The rebuilt browser's visible cube and sphere workflows use the same fixture
meshes, toggle Block, select the sphere's resolution 32 through the actual
slider, and check saved geometry, visible GPU buffers, closed topology and
exact Undo/Redo. Its saved outputs are compared to the same pinned reference
arrays. This establishes the two Block fixtures only. Arbitrary curved or
open surfaces, selected multi-mesh fusion, color/material transfer, quad
project exchange, native GUI, long-session performance, the separate smooth
and Marching Cubes modes, and full SculptGL/ZBrush parity remain open.
