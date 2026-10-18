# C182: remove duplicate resident scene validation

The shared Rust `Document::replace_mesh` path validates the candidate mesh and
the active object's posed/source metadata before changing document state. It
then runs the ordinary scene structure, storage and encoded-size checks. The
scene check previously repeated the same mesh and metadata scans. The C182
change passes the already validated fact only within this replacement call;
all other scene-candidate callers still perform the full validation. The
source mesh fingerprint, fold guard, first mesh/pose validation, scene and
aggregate budgets, history and visible Undo/Redo remain in force.

The comparison uses the independently generated `999,002`-vertex SGL sphere
(SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
in the normal browser document and opt-in WebGPU Brush preview. Two consecutive
three-dab mouse strokes used Draw radius `0.1800000072`, strength
`0.3499999940`, X symmetry and accumulation on, hardness zero, world-space
radius and viewport `[225,57,1365,882]`. The first stroke changed 5,171
vertices and the second 5,654 in both builds. The before, first-after,
second-after, visible Undo and visible Redo PNG hashes are identical across
builds; Undo returns to the first-after state and Redo to the second-after
state. The second-after hash is
`698b943975bac6ff0fa7150e16b0617e19cfc2d2d1eded1c4870c2e6e750cbad`.

The pre-change second stroke took 182.1 ms to release: 91.7 ms of mesh
replacement, including 38.1 ms of repeated scene validation. The changed
build took 146.1 ms to release: 53.8 ms of replacement, with the remaining
scene check below the browser timer's displayed precision. These are single
local runs, not a controlled distribution or a SculptGL benchmark. The first
stroke still built derived topology and normals; its release was 370.5 ms
before and 427.3 ms after in these separate runs, so no first-stroke speedup
is claimed. The observed second-stroke release **still fails** the ≤100 ms
responsiveness gate.

Preserved browser results are
`.artifacts/gpu-pointer-c146/c182-second-million-r0/result.json` (pre-change
SHA-256 `719b9820697cc361ee1949f0f4fe9f39ee0c3491b9d30bf99175ac2eab931f7d`)
and `.artifacts/gpu-pointer-c146/c182-second-million-optimized-r0/result.json`
(changed SHA-256 `2901cc8afa3c687cd2ad62cefe927b0776ca86de91dba01a04577e0e3597124e`).
The changed portable and SIMD WASM SHA-256 hashes are respectively
`4dac404cb380e4ffe78c60c629c137d272d359174ae6cf852ff265718b73aa0f`
and `2d4d16560546001b8aaca2b04ba618cc662c38ca15de80500057916c2b6f82e7`.
This scoped optimization does not establish sustained responsiveness, normal
studio WebGPU integration, SculptGL performance equivalence or ZBrush parity.

The final 10-second SIMD browser trace sent 223 pointer moves, received 225
contacts, coalesced 112 and applied 113 GPU dabs. It changed 25,489 vertices;
the different long-trace event timing prevents byte-exact before/after
comparison with C181. The visible Undo/Redo hashes still matched their own
before/after states. RAF P95 was 50.1 ms, maximum 416.7 ms, release 433.3 ms,
and checked commit 341.2 ms. The core stages were 56.4 ms source match,
132.2 ms fold guard and 137.5 ms replacement, including 94.1 ms normals.
All three standing responsiveness limits (RAF P95 ≤33.34 ms, maximum
≤250 ms, release-idle ≤100 ms) **still fail**. The run is preserved at
`.artifacts/gpu-pointer-c146/c182-simd-million-long-r0/result.json`
(SHA-256 `b2e68584912be4515c13ddb78e1c755c19e2ef1435024de702217bc79d897956`).

Final checks passed: `cargo fmt --all -- --check`, 2,494 workspace tests
across 103 groups (zero failed, one ignored), strict native and WASM-target
workspace Clippy for portable and SIMD configurations, `scripts/build-web.sh`,
`scripts/package-macos.sh` with a valid local app signature, and
`git diff --check`. Browser checks used the rebuilt WASM and host files and
closed their owned browser processes cleanly. These are implementation
checks, not reference parity evidence.
