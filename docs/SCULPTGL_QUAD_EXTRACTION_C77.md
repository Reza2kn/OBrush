# Mask extraction from original quads and UV-bearing sources (C77)

Mask extraction now accepts a validated source with retained original faces
and corner UVs. Extraction creates a new part and never changes the source,
so it does not require detaching that metadata. In **Whole incident faces**
mode, if one vertex of a source quad reaches the mask threshold, both
canonical triangles of that quad are admitted. The extracted part has fresh
triangle topology without a UV channel; the original retains its faces, UV
indices, color, material and mask through save, undo and redo. Clipped
selection can also extract from a source with retained metadata.

A new independently authored closed eight-vertex, six-quad OBJ masks vertex
1, which lies at a non-leading corner in two quads. That placement makes
per-triangle admission visibly different from original-face admission. The
pinned SculptGL runtime was exercised through normal OBJ import, Masking,
Thickness and **Extract !** controls at threshold 0.5 and thickness 0,
−0.125 and +0.125. It retained the original source exactly and created:

| Thickness | Pinned reference | OBrush | Largest reference-to-OBRush nearest-point distance |
| --- | --- | --- | ---: |
| 0 | 7 vertices, 3 quads / 6 triangles, open patch | 7 vertices, 6 triangles, open patch | 0 |
| −0.125 | 14 vertices, 9 quads / 18 triangles, 10 open edges | 14 vertices, 24 triangles, closed shell | 1.477709409 |
| +0.125 | 14 vertices, 9 quads / 18 triangles, 10 open edges | 14 vertices, 24 triangles, closed shell | 1.369091290 |

The zero-thickness coordinate match is for unordered vertices on this one
fixture. At nonzero thickness, the reference's partial bridge leaves an open
mesh. OBrush keeps its closed noncollapsed shell and does not apply the C76
triangle-border smoothing rule to a quad source. A separate instrumented
reference run reproduced the pinned quad smoothing result exactly through
three boundary-aware Laplacian passes, but that diagnostic does not justify
copying its open result into OBrush. Mixed tri/quad surfaces, curved geometry,
nonuniform masks and UV handling on extracted parts remain unverified.

The shared-core test includes a separately authored quad cube with six UV
seams. A browser version 6 project of that cube exercised all three
thicknesses through visible controls, kept the source's six UV faces exact,
and verified the new part, save, undo/redo and actual GPU buffers. The final
owned-browser run passed 14 cases without JavaScript exceptions. Two earlier
browser checks failed because the new harness first wrote UV metadata into
a version 2 project, then assumed a single-mesh version 6 project had to
contain a scene. Both failures are retained; the corrected fixtures and
schema check passed on the same WASM. A full-suite run also exposed an old
test asserting that UV-bearing extraction must fail; the test was updated to
verify the new source-preserving behavior and the rerun passed.

Formatting, 2,357 workspace tests (one ignored), strict native workspace and
WASM core Clippy, the official browser build and signed macOS package passed.
No complete SculptGL extraction, sustained performance or ZBrush parity
claim follows. The [evidence index](evidence/sculptgl-quad-extract-c77-0.1.20.json)
binds the source and runtime records.
