# Source Worker scene-witness candidate (preregistered)

The read-only 100k stage receipt at `bf1b694` measured 373.6 ms for
complete-scene witness plus delta decode, 171.0 ms for retained warm decode,
and 566.9 ms for the whole Rust publication. Raw result SHA-256:
`42fb53ec181115b220699a6fe6d3bcef23616529f21055a7372f81d5fe7cce99`.
The candidate under test will preserve the exact v7 JSON scene witness and
its bounded serialization, but compute its BLAKE3 digest without first
materializing a second complete JSON string at final publication. The public
save serializer and bytes must remain unchanged. This is a hypothesis about
allocation/copy cost, not a predicted speedup.

The frozen SP-WORKER-01 protocol SHA-256 is
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`.
Use the same independent 99,602-vertex / 199,200-triangle SGL fixture
SHA-256 `8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`,
the same ordinary browser viewport, 60 seconds, 24,000 requested pointer
moves, Source Draw radius 64 pixels, intensity 0.25, no symmetry, and the
unchanged frozen verifier. Record actual product/build hashes and retain all
failed attempts.

Correctness gates are: the streamed digest equals BLAKE3 of the exact
`to_json()` bytes for legacy and rich scenes; in-place geometry, metadata,
selection and history/retained-state staleness reject atomically; malformed,
oversized, duplicate, unsorted and wrong-target delta packets still reject;
successful publication gives the exact endpoint, one undo step, byte-exact
Before/Undo and After/Stable/Redo normal Save files, shape acceptance, exact
GPU positions, checked display-normal packets and cancellation. No allocation
may exceed the existing project/delta budgets. Run focused and full workspace
tests, native/WASM strict Clippy, `cargo fmt`, fresh web build, and the frozen
browser trial.

For a meaningful single-run signal, look for at least a 10% reduction in the
373.6 ms combined scene/delta stage while all correctness checks stay green.
The run-to-run stroke and machine conditions can differ, so this alone is not
a speed or parity claim. The baseline frozen failures remain explicit: active
RAF maximum 566.7 ms, no proven empty queue by 100 ms, and the public
mesh-normal GPU oracle. The Worker stays query-gated and off by default; do
not promote it from this scoped result, regardless of the candidate timing.
