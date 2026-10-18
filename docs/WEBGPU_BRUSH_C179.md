# C179: explicit-direction resident WebGPU Brush candidate

The isolated resident-GPU path now has a bounded ordinary Brush displacement
pass. Rust core validates a finite unit direction, center, radius, intensity,
and at most 256 passes. Core also supplies a scalar f64/f32 reference. The GPU
uses its resident position and unprotected-influence buffers, proposes a smooth
falloff displacement along the supplied direction, and reuses the existing
failure check, sparse commit, normal refresh, checkpoint, and cancel path.

The independently generated versioned sphere fixture is
`crates/obrush-gpu/tests/fixtures/resident-sphere-162.obrush` (SHA-256
`ab16459719d55e25d49fa8c2cc0fd16809749a991a6d08e960f6a822d34afe0a`).
The exact test used center `[0,0,1]`, unit direction `[0,0,1]`, radius `0.5`,
intensity `0.25`, one pass, accumulation, and no alpha. On native Metal, seven
of 162 vertices changed; the largest per-component difference against the
scalar reference was `0.0` f32 units. The 140-byte sparse result committed to
the versioned `Document`; Undo and Redo recovered exact project strings and
cancel produced no pending sparse result. The native JSON is
`.artifacts/history-limit/c179-resident-brush-native.json` (SHA-256
`862797b0ba4af255cfd9eafb19ea47a3f8b2926b789a396ab06951d7877bd202`).

In an owned Chrome browser WebGPU run, the same fixture and parameters
produced all 648 f32 position words byte-exactly equal to native Metal. Its
sparse result was 140 bytes, cancel emptied it, and no browser console,
exception, GPU failure, or cleanup errors were recorded. The browser JSON is
`.artifacts/gpu-pointer-c146/c179-brush-browser/result.json` (SHA-256
`5fef4afa16e081d9055740751ef797e6fcf796d509de146d7c0d9a696edc2ff6`).
The browser GPU WASM SHA-256 is
`2920e0f309ad4428fb404f9adb7b65b302f86cc7bf3e9bd2e199c26401faf409`.

Final-source checks: `cargo fmt --all -- --check`, all 2,494 passing
workspace tests across 102 groups (zero failed, one ignored), strict native
and WASM-target workspace Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh` passed. The local macOS app signature verified.
The first shell wrappers for the test and Clippy logs attempted to assign
zsh's read-only `status` parameter after those commands finished and exited
with a wrapper error. Separate strict Clippy invocations returned exit zero;
the completed workspace test log was independently parsed for every result
group, with zero test failures. The wrapper error remains visible in the
tool record; the test log is `.artifacts/history-limit/c179-workspace-tests.log`.

This is a narrow geometry primitive, not a Brush workflow or performance
pass. The normal studio still uses Rust/WASM geometry and WebGL2. The
separate GPU path has no live Brush pointer/pick/first-touch policy, no
Source proxy or alpha/symmetry behavior, no multi-subtool transaction, and
no sustained million-vertex responsiveness evidence. The scoped result does
not change the SculptGL or ZBrush parity assessment. The prior 1M/60-second
normal-browser run still fails all four responsiveness gates in
[C178](SCULPTGL_SPARSE_VERTEX_UPLOAD_C178.md).
