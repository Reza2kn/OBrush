# STL import: exact empty facets

The shared Rust importer now skips STL facets with exactly repeated corner
positions, validates every coordinate first, and reports the skipped count.
It does not use approximate welding or alter the remaining surface.
Files containing only empty facets still fail. Skipped rows still count
against the source facet limit. ASCII and binary STL share this policy.

On the supplied Sparkdancer fixture (source SHA-256 in the
[evidence record](evidence/stl-repeat-facets-2026-10-20.json)):

- 738,904 source facets; four exact repeated-position facets skipped.
- 369,407 retained points and 738,900 retained triangles.
- Independent comparison of every retained corner and winding against the
  original binary STL: **zero coordinate difference**.
- Ordinary browser Open, public project download, Undo and Redo passed.
- A Paint stroke changed 1,312 vertex colors, with 848 changing by more than
  0.01 in a channel; geometry remained exact. Stroke Undo/Redo were byte exact.

The user-supplied model is not distributed in this repository. The regression
fixtures are small, independently authored ASCII and binary facets. Workspace
tests, strict native/WASM Clippy, core WASM check, the browser build and the
macOS package passed for source `50f36181b4537ec35d443a073062ed7f5b120a7b`.
The public SIMD and portable browser startup/save/warning checks passed.

## Open limitations

This fixes import rejection, not surface repair. The model retains its original
triangles, including coarse areas that become visible at high zoom. One full
four-way subdivision would require 2,955,600 faces, exceeding the current
two-million-face limit. More local detail remains necessary for that workflow.

A separate actual Draw test at radius 1, strength 35%, symmetry off made no
geometry change. Its ray hit an editable front surface, but existing thin
triangles caused the current quality guard to reject every proposal. This
remains an open sculpting blocker; the import fix does not establish that Draw
or close-up sculpting quality is resolved. Failed attempts remain retained.
