# Warm Source retained-state transfer seam

This is an independently written, versioned private transport for a warmed
`SourceGeometryNormalState`. It is a prerequisite for ordinary Source geometry
work in a browser Worker, not a Worker host or a responsiveness result. The
public `.obrush` format and ordinary Save/Open policy remain unchanged.

The `OBRSNWV1` packet carries a nonzero subtool ID, the exact public mesh
geometry digest, a behavior-state fingerprint, a bounded payload length and a
BLAKE3 checksum. The payload keeps generation, original-face flag,
interaction center, exact position words, selected-refresh face and vertex
normals, render selection bits, and the complete private ordered tree: live
and free cells in slot order, serials, parent/children/depth, split and loose
bounds, ordered leaf face rows, face boxes, leaf/position maps and incident
rings. It has a 256 MiB packet ceiling and retains the existing 128 MiB tree
and 256 MiB state limits. Decoding first validates the public mesh and rebuilds
canonical immutable face rows, then checks every private count, allocation,
tree link, face membership, face box, ring, checksum and fingerprint before
returning an uninstalled producer. A caller must still check its owning
Document revision and preserve history before publication.

`render_predecessor` is a local `Weak` pointer and cannot be transported. The
decoder retains the selected chunk bits but clears that pointer, so it cannot
authorize a sparse renderer handoff from a different realm. The next normal
paint/render operation must use its ordinary checked path. Active strokes,
their proxy/first-touch words, full Document state and undo/redo stacks do not
cross this codec. That transfer and browser Worker publication remain open.

## Validation

The independent 13×13 OBJ grid contains 169 vertices and 144 original quads,
forcing a branched ordered tree. One Inflate dab warms the producer; public
project Save/Open creates a separate receiving mesh allocation. The packet is
decoded against that mesh and two further dabs are applied independently to
the original and receiver. The test compares exact mesh JSON bytes, exact
behavior-state fingerprints and semantic retained normal reports after each
dab. Heap capacities inside the report can differ after deserialization. A
separate tree test checks exact cell/free/serial/bounds/ring words and rejects
duplicate faces, omitted free slots, cycles, stale bounds and rings. Packet
tests reject stale target/mesh, checksum corruption and resealed invalid
counts, parent links and child cycles.

The codec test intentionally does not assert Document Undo/Redo for a
transported state: it never installs that state into a Document, and the
captured Document history is neither serialized nor transferred by this API.
The ordinary Worker host needs a separate checked, atomic publication test
including Undo/Redo before it can accept this packet for sculpting.

The fixture parameters are Inflate, first center `(0,0,0)`, later centers
`(0.2,0,0)` and `(-0.2,0.2,0)`, squared radius `0.45`, original face `65`,
pressure `0.8` then `0.65`, eye direction `(0,0,-1)`, nonnegative default
intensity and one new stroke at the realm boundary. Each dab changed 37
vertices. The first warm mesh JSON has BLAKE3
`57311fcc4ba5403464bf329cdea7547a703d5a8b2fe9dce2c07f9940ed11108f`;
the 15,276-byte packet has BLAKE3
`bb6490237fc6a232f295301c9b02db27db3087a3a90184d95836d6a324541b79`.
The two later mesh-byte and state-fingerprint differences are exactly zero.

`cargo test -p obrush-core --lib --locked source_worker_retained_warm` and
`source_ordered_tree::warm` both pass (four focused tests). Strict native
`obrush-core --all-targets` and strict WASM `obrush-core --lib` Clippy pass.
A broader `obrush-core` suite had no failures in the groups observed, but was
interrupted after the focused gates to release the shared Cargo lane for the
Worker host integration. The root checkout must run the full workspace gates
after merge. No SculptGL parity or browser speed gate is closed here.

For a 999,002-vertex, 1,998,000-triangle retained state with one original
face per triangle, compulsory packet arrays alone are roughly 140 MB decimal
before leaf-face rows and cells. The former 128 MiB candidate cap would have
excluded that shape, so the bounded ceiling is 256 MiB. An actual million-row
packet and Worker transfer have not yet been measured.
