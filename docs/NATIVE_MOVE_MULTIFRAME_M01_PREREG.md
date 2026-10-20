# M01 native Move input and fresh-sphere regression

**Frozen before Rust test execution.** The retained normal native CUA
attempt `.artifacts/native-move-cua-attempt.json` reports identical public
project SHA-256 before and after its first apparent drag,
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`.
It did not record egui frame boundaries, a pick hit, or press/move/release
delivery, so it cannot by itself decide whether Move is broken.

The M01 test uses real `egui::Context::run` and `RawInput` mouse events with
the production Move branch order (`pressed` begins, later `down/released`
samples, `released` ends) and production `MoveDrag` plus shared-core
`MoveStroke`/`Document`. It uses a fresh `Mesh::sphere(6)`, world radius
`0.037`, strength `0.79`, X symmetry and backface culling on, pressure one,
and default adaptive detail. The center is the sphere vertex nearest the
independently stated surface direction `(0.25, 0, 0.9682458)`; camera-plane
right/up are +X/+Y with a fixed `0.0004` model units per logical pixel.
Start at `(450,350)` in a 900×700 egui screen, then deliver eight separate
held move frames in increments of `(5,-2)` logical pixels, followed by a
release at `(490,334)`. A separate control delivers press, move and release
in **one** egui frame.

Pass requires at least one accepted held Move, ≥8 changed original vertices
including both X sides, a valid mesh after every held frame, all final
triangles nondegenerate and outward-facing, one Undo step, exact public
before/after JSON restored by Undo/Redo, and no history or geometry change
in the compressed one-frame control. Record any failed assertion rather
than loosening it. The control is a host-event hypothesis test; it does not
prove that the retained CUA attempt actually coalesced events. This is a
model/egui test, not a native rendered gesture, source-reference match, or
visual-tearing closure. A native multi-frame consumer gesture with a saved
changed project remains a separate gate.
