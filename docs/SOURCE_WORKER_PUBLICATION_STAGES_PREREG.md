# Source Worker publication-stage diagnostic (preregistered)

The predecessor is the one-pass candidate at `aa0272c`, whose frozen 100k
normal-browser receipt SHA-256 is
`a2493643d6f51b9226a6f3a38c343307cfebe8fccd55c5878e53c1947164ac33`.
The held Worker remains query-gated and off by default. This diagnostic uses
the same SP-WORKER-01 protocol SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`
and independent SGL fixture SHA-256
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`:
99,602 vertices, 199,200 triangles, 60 seconds, 24,000 requested held-pointer
moves through the ordinary browser viewport, Source Draw radius 64 pixels,
intensity 0.25 and no symmetry. The actual product build hashes and
unmodified frozen verifier must be recorded.

Before changing any geometry or publication algorithm, record named wall
times for complete-scene witness/delta decode, preview match, retained warm
decode, and inside `publish_cold_source_worker_result`: candidate delta
re-encode, retained match/state budget, scene check, output budget, history
snapshot/budget, changed scan and final adoption. The timing probe is
read-only, bounded to one latest record and emitted only from a successful
query-gated Worker publication. Compare its sum with the independently
instrumented outer callback to account for unmeasured overhead. A new
instrumented 100k browser receipt must still preserve public Save/Undo/Redo,
shape, GPU position and display-normal checks, exact endpoint, cancellation
contract and zero errors. Preserve any failed run.

Only after these stage timings identify a dominant duplicate or avoidable
operation may a separate candidate remove it. Such a candidate must keep
complete-scene stale detection, bounded packet/allocation checks, exact
retained-state authentication, atomic rejection and one-undo publication.
Run focused native/WASM tests, full workspace tests, strict Clippy, fresh web
build and the same frozen browser trial. The current 100 ms queue tail cannot
be claimed passed from a faster main commit alone: the Worker finish round
trip was already about 194 ms. No frozen threshold or normal oracle changes
are authorized by this diagnostic.
