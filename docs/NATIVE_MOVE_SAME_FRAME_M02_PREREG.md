# M02 same-frame native Move recovery gate

**Frozen before product edit or test execution.** M01 established that egui
retains ordered `PointerButton` positions and `PointerMoved` positions in
`InputState.events`, while its summarized pointer state exposes only the
final interaction position. The production Move host starts on `pressed` and
samples in an `else if down || released` branch, so a complete fast contact
within one frame cannot sample its movement.

The proposed narrow repair applies only when the selected effective tool is
Move, egui reports both a primary press and release in one focused frame,
and the raw event stream contains exactly one ordered primary press, at
least one nonzero movement, and one release. The full path must contain at
most 64 in-viewport finite sample positions, no pointer loss, no competing
pointer button, and no navigation. The raw press position must be used for
the normal hit/pick and Move preparation; ordered motion and release
positions must be offered to the existing Move sampler before the ordinary
release finalizes the stroke. No new geometry implementation or project
format is allowed. Ambiguous/overflowed streams fail closed to existing
behavior. Normal held multi-frame drags must not take this path.

Pass gates on a fresh default sphere at radius `0.037`, strength `0.79`, X
symmetry and backface culling on, with the M01 camera-plane map:

1. A single egui frame containing press at `(450,350)`, eight ordered
   movement positions in `(5,-2)` increments, then release at `(490,334)`
   changes at least eight original vertices on both X sides, validates after
   every sample, keeps all final triangles nondegenerate/outward, creates
   exactly one Undo step, and restores exact public project JSON by Undo/Redo.
2. Eight held frames followed by release retain M01 geometry and one-step
   history. A stationary same-frame click remains a geometry/history no-op.
3. Raw primary press outside viewport, release outside viewport, conflicting
   buttons, pointer loss, more than 64 samples, or malformed event order do
   not trigger recovery or create geometry/history in the control harness.

This is an egui/host regression. It does not prove that the retained native
CUA attempt actually delivered one frame, that picking succeeds in a normal
native window, or that the reported rendered triangles are visually repaired.
Those require a separate normal native multi-frame consumer capture and
changed saved project. Preserve any gate failure and its exact parameters.
