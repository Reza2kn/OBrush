# C281 prospective read-only picked-face diagnostic of C280 failure

C279 stopped at a frozen setup gate. C280 used a distinct bounded setup gate
and failed its unchanged `1e-6` second-held/on public XYZ gate by
`3.120319831850793e-6` across nine front RGB IDs. Both failures and every
raw capture remain preserved. C280 static analysis indicates that the second
trusted ray lies on the edge `[221,222]`, and the Source culling-on held
displacement aligns with the normal interpolated from upper face
`[221,222,243]`, whereas Source culling-off and both OBrush runs align with
lower face `[200,222,221]`. This is an inference from displacement and
first-held GPU normal words, not a direct Source picked-face observation.

C281 is a **read-only diagnostic replay**, before any product edit. It keeps
the same pinned Source commit and hosted bundle, independent fixture SHA-256
`80b6430b152aad76fface01e9bf4890fdd525b2d9f11c60cd74e6c1b021d876b`,
four fresh normal-UI documents (Source/OBRush × Culling on/off), visible
controls (Brush, Clay off, Accumulate on, 64px, 25%, Alpha None, no pressure,
negative, symmetry, continuous, lock, or alternate topology), 1024×808
integer viewport and center+40→52→64 horizontal held stroke. It retains
the C280 public Save/Undo/Redo and active GPU observations, `5e-6`
projection-ratio with `1e-6` actual world-ray/radius setup bounds, exact
history/membership/identity/attribute/topology gates, and **unchanged `1e-6`
paired XYZ/GPU component gates**. No pointer rescaling, direct kernel call,
reference patch, or fixture change is permitted. The exact harness, audit,
served build, Source bundle, fixture hashes and source revision must be
recorded in the run.

The sole added observation is a read-only snapshot immediately after each
held GPU readback: the Source public-runtime `getPicking()` face index,
mesh-face vertex indices and RGB IDs, picked normal, intersection point,
eye direction, local radius² and picked-vertex count; OBrush's already
published contact/normal report is retained. These reads must not call
intersection, recompute normals, mutate geometry or change stroke timing by
more than the existing `page.evaluate` observation. On/off and Source/OBRush
faces are identified by RGB IDs, not their different imported face indices.

The diagnostic hypothesis is: after the second held point, Source-on uses
upper face `[221,222,243]`, while Source-off and both OBrush runs use lower
face `[200,222,221]`; direct picked normals should align within `1e-5` of
the directions inferred from second-held GPU position deltas. Capture any
contrary result and leave the gap unresolved. If the hypothesis holds, C281
still fails the frozen numerical parity gate; the result only identifies a
candidate tie-break boundary. A fix requires a later, separately frozen
shared-core trial. The full Brush Culling row stays open.
