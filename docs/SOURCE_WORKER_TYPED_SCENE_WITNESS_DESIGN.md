# Source Worker typed scene witness: static design and trial preregistration

This document is a design, not an implementation or performance claim. The
read-only split trial at `52b755a` measured 267.9 ms of complete-scene
witness work and 13.4 ms of delta decoding in the frozen 100k browser cohort.
Its raw result SHA-256 is
`936a10a04c21fbd08c59906e3e4588bd9783a5e01fa745c207df4d5dca2b934c`.
The independent JS final callback was 477.2 ms, and the normal begin call
was 407.8 ms. The existing final witness calls
`Document::project_json_digest`: it validates the complete document,
reconciles levels/layers, constructs the same `ProjectRef` used by public
Save, formats every scalar as compact JSON, and hashes that byte stream.
The 100k project is roughly 35 MB. A document revision alone cannot replace
this witness: the active `Mesh` is publicly mutable, and an in-place edit can
retain its pointer and host revision.

## Proposed candidate and proof boundary

Retain the public `to_json()` and its exact byte and size contract. At Worker
capture, after the already required bounded `to_json()` seed, calculate a
second, versioned BLAKE3 fingerprint by serializing the **same constructed
`ProjectRef`** into a streaming, injective Serde event encoding. At final
publication, keep `verify_live_owner` and `validate_complete_state`, build
the same reconciled `ProjectRef`, calculate only the typed fingerprint, and
compare it with the captured value before decoding or adopting packets. The
typed fingerprint must include field names and presence, type/variant tags,
length-prefixed strings and byte arrays, sequence/map boundaries and lengths,
integer width/sign, and exact f32/f64 bits including signed zero. It must
use a domain-separated format version and a fixed maximum byte count; no
project-sized allocation is permitted. Do not hash raw struct padding,
pointers, cache state, or a hand-picked subset of fields.

The invariant is one-way and sufficient for stale rejection: **if public
JSON bytes differ for two valid Documents, their complete Serde event streams
must differ**. An injective event encoding then yields different prehash
bytes, except for a BLAKE3 collision. At capture, public `to_json()` already
enforces its 512 MiB encoded limit; if a final typed fingerprint equals the
capture's, the same Serde events produce the same bounded public JSON. A
changed or oversized final scene must reject. Keep the current
`validate_complete_state`, reconciliation, `scene_v7_project`, epoch/revision,
active mesh pointer, selected part/level, history depths and retained-state
owner checks. Construct `ProjectRef` in one shared helper for both serializers
so added project fields cannot silently bypass the fingerprint. The typed
serializer itself needs a checked byte cap and fallible bounded buffering.
Any inability to prove canonical event framing or complete field traversal
is a stop condition; retain the JSON witness in that case.

This design deliberately leaves `source_worker_delta`, the retained warm
codec, and the Worker transfer untouched. The prior finished Worker message
transferred about 16.34 MB in roughly 198 ms. It uses transferable buffers,
so that delay is not a structured-clone copy of the final bytes; another
workstream owns its encoding and packet-size cost. The checked candidate mesh
copy remains necessary for atomic rejection before public adoption.

## Adversarial and consumer gates

Before any browser comparison, build a cross-version differential corpus of
independently constructed plain, rich v7, multi-SubTool, multiresolution,
layer, morph, UV/material, mask-convention and SGL-provenance projects.
For each valid scene and targeted in-place mutation, assert that a changed
normal Save byte stream changes the typed fingerprint. Mutate positions,
normals, triangles, masks, colors, materials, UV indices and coordinates,
object name/visibility/selection/pose, inactive payload, raw provenance,
layer/morph/level state and signed-zero scalars while preserving the host
revision and active mesh address where possible. Separately test malformed
geometry, nonfinite values, output/storage caps, collision-like token
boundaries and reordered/omitted optional fields. A final stale response must
reject atomically without spending redo. Corrupt, oversized, duplicate,
unsorted, wrong-target and wrong-checksum delta packets and malformed warm
packets must still reject. A successful gesture must keep exact endpoint,
one-undo history, byte-identical Before/Undo and After/Stable/Redo public
Save, exact GPU positions, checked display-normal packet and cancellation.

The browser prereg remains frozen SP-WORKER-01: protocol SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`,
independent 99,602-vertex / 199,200-triangle SGL fixture SHA-256
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`,
60 seconds, 24,000 requested ordinary viewport moves, Source Draw radius
64 pixels, intensity 0.25, no symmetry. Record source/build hashes, raw
result SHA-256, named stage timings, public files, shape, GPU readback,
transport and unchanged frozen-verifier failures. First add read-only
subtimings for validation/reconciliation/serialization; proceed with the
candidate only if JSON serialization dominates (at least 150 ms of the
267.9 ms witness). A meaningful one-run signal requires witness time at
most 200 ms (about 25% less), normal begin call no more than 458 ms (50 ms
over the observed 407.8 ms), and all correctness gates. Different delivered
dabs and machine conditions still preclude a same-stroke speed claim from a
single pair. Full workspace tests, strict native/WASM Clippy, `cargo fmt`,
fresh web build and the same browser audits are required.

The held Worker remains query-gated and off by default. Existing frozen
failures remain explicit: active RAF maximum 466.6 ms, no proven empty queue
by 100 ms, and public mesh-normal GPU comparison. This candidate cannot be
promoted from one 100k run and does not establish SculptGL or ZBrush parity.
