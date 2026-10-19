# C232: pressure changes during one held browser stroke

The ordinary Draw brush now has a four-case normal-browser pressure-ramp
check in `scripts/check-brush-inputs.mjs --ramp-only`. An isolated Chrome
instance loads the existing `web/dist` build (WASM SHA-256
`9742032c6abf9c56068093a08f7dd158e24292f2acbec33d8abe8ad9ea7e486e`),
sets the visible SculptGL-style factor controls, and sends trusted synthetic
pen contacts. The complete [C232 result](../.artifacts/brush-inputs-0.1.6/c232-ramp-final-60ca2615-ac98-436a-9ca7-3481b008d3df/result.json)
includes browser event traces, GPU readbacks, public `.obrush` saves, and
browser cleanup. All four cases start from the same 40,962-vertex sphere.

Each stroke presses at pressure 0.25, then moves 100 screen pixels while held
at either 0.25 or 0.75 before release. The browser and host both record the
moving pressure. Each case processes one press dab and eight further moving
dabs, then has zero queued dabs. With intensity factor 1 and radius factor 0,
the initial press GPU buffers match byte-for-byte between the two cases.
After the move, the final meshes differ at 1,215 original vertex IDs, with a
maximum position-component difference of `0.02642306` world units; the 0.75
case also adds 38 vertices through local detail. With both factors 0, varying
pressure yields exactly the same final mesh as constant pressure. The 0.75
stroke is one released history step, and public Undo/Redo recovers exact
initial/final mesh hashes. Every final saved mesh matches the actual GPU
buffer readback.

The previous [four-case stationary-factor check](SCULPTGL_PRESSURE_FACTOR_BROWSER_C231.md)
and [nine-case alpha/pressure browser matrix](../.artifacts/brush-inputs-0.1.6/c232-full-regression-d406fffe-33db-461b-9118-ef7300880c1e/result.json)
pass again against the same WASM build. This proves per-dab factor sampling in
these synthetic OBrush browser strokes, not physical-tablet behavior or a
matched SculptGL UI stroke. Reference-paired geometry, other tools and meshes,
long strokes and release responsiveness remain open. Neither pressure inventory
ID is promoted to parity.

The final harness passed `node --check`, `git diff --check`, Rust formatting,
strict native workspace and WASM app Clippy, and
`cargo test --workspace --locked` (2,533 passed, zero failed, one intentional
ignore across 83 groups). The test transcript is retained at
`.artifacts/c232-cargo-test.log`. No Rust application source changed in C232;
the browser ran the same C230 build.
