# Source Worker verified-baseline decode: proposed next trial

This is a static preregistration. No implementation or new browser result is
claimed here. The accepted streamed-witness candidate at `b300451` retained
complete-scene hashing and measured 283.7 ms for the combined final
scene-witness/delta-decode stage in the frozen 100k trial. Its result SHA-256
is `41c166f9c00fe3edcd782e94af5ca7fde1ac99ca75d1325e3c59397e754dd429`.
The JS callback was 483.3 ms, Worker finish round trip about 198 ms, and
the final Worker-to-main transfer was 16,341,340 bytes. That transfer uses
`postMessage` transfer, not structured cloning. The separate retained warm
codec work owns the large final payload and is outside this candidate.

First split the existing `scene_and_delta_decode_ms` timer into complete-scene
witness and delta-decode sub-stages without changing behavior. Preserve a raw
normal-browser receipt and run the unchanged frozen verifier. Implement the
candidate below only if delta decode is a material part of the 283.7 ms stage
(at least 15 ms). This avoids treating a 198 ms Worker packet/encode delay or
the remaining JSON witness serialization as a delta-decoder problem.

The scoped candidate is a crate-private, verified-baseline delta decoder.
At capture, store the exact existing `geometry_digest` of the active mesh.
At final commit, `verify_scene` must run first: it verifies epoch/revision,
retained producer and pointer ownership, validates the complete Document via
the same bounded JSON serializer, and compares both encoded byte length and
BLAKE3 digest with the captured project. With that proof and an immutable
borrow of the Document, the crate-private decoder may compare the packet's
baseline digest with the captured geometry digest without repeating
`before.validate()` and a full geometry hash. The public standalone decoder
keeps its current validation and digest checks. Packet magic/version,
32 MiB byte cap, exact length arithmetic, checksum, target/count, sorted
unique in-range rows, changed-row check, finite candidate validation and
bounded atomic candidate copy all remain. The candidate must not edit the
retained warm codec or Worker transfer path.

Use the unchanged SP-WORKER-01 protocol SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`
and independent 99,602-vertex / 199,200-triangle SGL fixture SHA-256
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`:
60 seconds, 24,000 requested ordinary viewport moves, Source Draw radius
64 pixels, intensity 0.25, no symmetry. Record actual source/build hashes,
numeric stage differences and every failed receipt. Full workspace tests,
strict native/WASM Clippy, `cargo fmt`, fresh web build and the frozen
browser audit are required before integration.

Correctness gates: exact stale scene/selection/metadata/in-place geometry
rejection, malformed/oversized/wrong-target/unsorted/duplicate packet
rejection before publication, no redo loss on rejection, one successful
undo step, exact endpoint, byte-exact public Save/Undo/Redo, shape and GPU
position checks, checked display-normal packets and cancel behavior. For a
meaningful single-run performance signal, require at least 10 ms less in
the split delta-decode stage with no increase in the complete-scene witness;
different delivered dabs and run conditions prohibit a same-stroke speed
claim from one pair. The existing frozen failures remain: RAF max 483.3 ms,
no proven empty queue by 100 ms, and the public mesh-normal GPU comparison.
No 100k result alone promotes the query-gated Worker or establishes parity.
