# Posed-source mask extraction (C79)

OBRush now accepts a validated version 7 source with an editable affine pose
when creating a mask-extracted part. The source remains in its original local
coordinates with its pose and immutable SGL provenance. The extracted part is
plain geometry in world coordinates: selected points are transformed with the
source pose, and the optional source-normal offset and signed thickness are
measured in world units. Reflected poses reverse the copied patch winding so
the output shell can remain consistently oriented. If world-coordinate
rounding collapses a selected face, extraction rejects without replacing the
document.

An independently authored two-object SGL fixture has a translated three-point
active source, a separate reflected/sheared object, and a single masked active
vertex. The same source was saved as version 7 `.obrush`. Pinned SculptGL and
OBRush used threshold 0.5, whole incident faces, its 0.01-unit source offset,
and thickness 0, −0.125 and +0.125. SculptGL ran through its normal SGL file
input and visible Masking controls; OBrush ran through the shared-core
document and browser controls.

| Thickness | Pinned SculptGL | OBrush | Largest reference-to-OBRush nearest-point gap |
| --- | --- | --- | ---: |
| 0 | 3 vertices, 1 triangle, 3 boundary edges | same | 0 |
| −0.125 | 6 vertices, 4 collapsed triangles, 6 open edges | 6 vertices, 8 noncollapsed triangles, closed | 1.185917389 |
| +0.125 | 6 vertices, 4 collapsed triangles, 6 open edges | 6 vertices, 8 noncollapsed triangles, closed | 1.185917390 |

The zero-thickness unordered point positions match exactly on this translated
fixture. The reference's nonzero output is not a usable shell, so OBrush keeps
its checked result. The nearest-point metric does not establish face
correspondence. A separate shared-core test checks world-space placement and
signed thickness for all three settings, preserves source pose/provenance and
history, and checks a reflected source. The owned browser run also checks the
saved scene, visible GPU buffers and undo/redo. Its exact status and hashes are
in the [evidence index](evidence/sculptgl-affine-extract-c79-0.1.20.json).

Curved posed surfaces, nonuniform masks, arbitrary nonuniform transforms,
native GUI behavior, sustained responsiveness, ZBrush extraction and complete
SculptGL parity remain unverified.
