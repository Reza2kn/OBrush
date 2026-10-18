# Block remeshing comparison (C82)

OBRush's Rebuild / reduce surface panel now exposes **Block remesh**. The
setting selects a distinct shared-Rust extraction path: one vertex per
mixed-sign voxel cell, quads around crossing grid edges, and no smoothing.
The output is recentered and uniformly scaled to the source bounds before
attribute transfer. It remains a bounded, cancellable document replacement
with one Undo/Redo step. The existing smooth remesh path is unchanged.

On an independently authored closed triangulated cube at resolution 64, the
pinned hosted SculptGL Block control and OBrush produce **24,578 vertices and
49,152 triangles** after splitting SculptGL's 24,576 quads. Applying the
reference mesh's actual transform gives exact world-space agreement for the
complete point multiset and every oriented triangle. The source result was
obtained through its normal OBJ import, numeric Resolution field and Block and
Remesh controls. The OBrush browser run used the same OBJ, its visible Block
checkbox and Remesh button, then checked saved data, visible GPU buffers,
closed topology and exact Undo/Redo. Comparing that browser-saved mesh against
the same reference also finds zero unmatched world-space points or oriented
triangles. [The comparison script](../scripts/compare-block-remesh.py)
rechecks complete arrays from the preserved runtime snapshots.

The cube is **not** general remeshing parity. An independently authored sphere
at resolution 32 differs: SculptGL produces 4,746 vertices / 9,488 split
triangles; OBrush produces 4,768 / 9,532. The reference-to-OBRush nearest
vertex distance has maximum `0.0795002631` and RMS `0.0366064421` world
units; the reverse maximum/RMS are `0.0792974406` / `0.0374445058`. The
source and OBrush volume measurements are `4.0673828125` and
`4.1950329816` respectively. The reference sphere carries corner UVs; the
OBRush input is the same positions/triangles with UV references removed,
because OBrush requires explicit metadata detachment before topology changes.
That interchange policy and the different voxel field remain open gaps.

The initial reference check incorrectly required a new mesh ID; SculptGL
actually reused the ID while replacing geometry. The failed receipt is kept,
and the corrected run checks the complete geometry change. Exact cube
agreement is scoped to this fixture, resolution, transform and pinned runtime.
Open/nonmanifold sources, arbitrary shapes, matching resolution range, masks,
selected multi-mesh fusion, native GUI, long-session performance and full
SculptGL/ZBrush parity remain unverified. [C82 evidence](evidence/sculptgl-block-remesh-c82-0.1.20.json)
binds the fixtures, reference, build, browser and numerical results.
