# Imported Source sparse preview admission (preregistered)

This candidate remains query-gated and off by default. It targets a single
active imported SGL object whose editable affine is bitwise identity, whose
encoded render shader is 0 (PBR), with matcap 0, full opacity, no wireframe,
no flat shading, no UV override, zero symmetry offset and no transform drag.
Other visible parts may retain metadata, but only the active target may receive
preview rows. A posed active object or different material remains rejected.

The existing packet transports public Mesh position and normal words; final
publication compares those words against the checked complete mesh. Imported
SGL rendering instead uses fresh static or warm selected Source normal words.
The candidate will add a versioned, checked display-normal pair per sparse row,
keeping the public-normal comparison unchanged. First-row display words must
match a captured baseline; later packets must chain exactly. GPU gap fill and
cancel must use the active Source baseline, not Mesh.normals. Missing Source
baseline state, a changed style/pose/owner, or a packet mismatch must fail
closed without changing the public Document or history.

Admission checks: focused native/WASM unit tests for cold/warm display-word
chaining, changed-display-only rows, invalid pose/style and cancel restoration;
workspace fmt/tests/strict Clippy; fresh browser build; one normal UI SGL
import and trusted held-pointer stroke with GPU position plus display-normal
f32 readback, public Save/Undo/Redo, cancellation and no JS/GL errors. The
frozen SPW01 100k/1m 60-second gate remains separate and unchanged. A small
passing stroke is not a responsiveness or SculptGL parity result. Preserve
failed runs and exact build/fixture hashes.
