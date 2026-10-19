# Source Worker final-publication duplicate-work trial

This trial starts at OBrush commit `0bdd547` and uses the frozen
SP-WORKER-01 contract SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`.
The query-gated imported-SGL held-pointer path stays off by default. The
99,602-vertex SGL fixture SHA-256 is
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.

The retained 100k Worker result SHA-256
`caf6b0c07a253a7c18e161c29f7c3f75e9c5b5a224773a67ddd2455a6b256fa4`
reported a 931.5 ms main-thread finish callback and a 916.6 ms RAF gap.
The cold bridge currently calls `decode_candidate` to compare its final
mesh with the checked preview, then `commit_candidate` calls the same
complete-scene JSON witness and delta decoder again. The hypothesis is that
one checked decode, passed directly to the one-use publication, reduces that
main-thread long task without weakening scene, packet, retained-state or
history checks. The 107.66 ms scheduling maximum occurred in the CDP sender:
its 39.9-second requests and acknowledgements stalled while app RAF gaps
remained below 19 ms, so this source change is not expected to fix pacing.

The intervention is restricted to the cold-capture commit API and bridge.
The capture must verify the complete scene once, decode the same bounded
packet once, call a comparison closure before publication, authenticate the
retained warm packet against that decoded candidate, then publish one undoable
change. Any failed comparison or malformed/stale packet must leave the public
document and history untouched. The diagnostic no-preview path keeps its
existing behavior. Unit tests will cover comparison failure, one call,
stale identity and unchanged-public-state failure. Native/WASM strict Clippy,
workspace tests, `cargo fmt`, fresh web build, and the exact frozen normal-UI
100k run are required. The browser receipt must include endpoint, Save/Undo/
Redo, checked display-normal preview/cancel, GPU position/display-normal
words, renderer/JS errors and the full frozen verifier. Preserve every failed
trial and report the RAF maximum, main commit time, release-tail queue probes
and pacing separately. A 100k improvement cannot establish 1m or SculptGL
parity; no frozen threshold will be changed for this trial.
