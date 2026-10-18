# C151 mask threshold check and rejected GPU-pick trials

The pinned SculptGL Masking `sharpen()` branch adds 0.1 to unprotected
weights above 0.5, clamps all other masked weights to zero, and leaves
unmasked weights at 1. The previously delivered optional OBrush source
response and paired cube/open-quad browser comparisons are in C80/C81. This
follow-up adds an independent eight-vertex threshold vector with unprotected
weights `0, 0.1, 0.49, 0.5, 0.51, 0.9, 1, 0.75`. It checks two consecutive
actions under both OBrush mask conventions against the pinned scalar formula,
the saved project's two-step Undo/Redo recovery, and a mask-free no-op that
creates no history. The native shared-core test passes with maximum asserted
unprotected-weight error `f32::EPSILON` (conversion to/from protected weights
can round). This is not a new hosted-reference or browser run; C80/C81 remain
the browser and paired reference evidence. The pinned `Masking.js` file used
for the formula has SHA-256
`601966e5fa08285e3b8b744db1e653df9f23138986f67821a4b29aff54f4ddd3`;
the independent cube OBJ is
`c9038e8adcbe5649273871feba73699753fa9abee0984575e14f41c9d610dde7`.

The million-vertex WebGPU pointer investigation used the independent
`independent-id-sphere-1m.sgl` fixture (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`),
999,002 vertices / 1,998,000 triangles. The C150 full-face-pick 10-second
observer run awaited 108 eight-byte maps, median 94.25 ms, P95 101.9 ms,
sum 10,254.1 ms. A GPU block-bound/loop trial awaited 112 maps, median
90.0 ms, P95 93.8 ms, sum 10,268.0 ms. A CPU block-selection plus compact
GPU candidate-ID dispatch trial awaited 112 maps, median 89.95 ms, P95
93.8 ms, sum 10,233.3 ms. All three preserved actual visible stroke and
Undo/Redo, but delivered contacts differed (214, 223, 222), so the runs do
not support a controlled speed ratio. The CPU-block trial still had a
50.1 ms frame P95 and 548.1 ms release. The two block trials were removed.
The restored short browser run passed with the same before/after viewport
SHA-256 values as C150:
`5398141e2042c211dcd70ad09f7796b918381a54c29269fe89fc43a3ddf7903b`
and `ae5446f55d34a3ac542c68812759df27ea12e933734e488ebb5265369e3b7ce7`.

The observer's `mapAsync` duration includes earlier work on the ordered GPU
queue; it does not isolate shader execution. Owned copy-only and no-op
compute probes showed sub-2 ms warmed readback/dispatch on their own, which
rules out a fixed eight-byte map cost. The full pick trace still needs a
queue/compute/render separation before attributing the roughly 90 ms wait
to a particular shader stage. A local fixture-diagnostic test initially
failed because it used a crate-relative path, then passed when anchored at
`CARGO_MANIFEST_DIR`; this ignored fixture test was removed from source.

Preserved result JSON SHA-256 values are `5ffe16c456bbe8d1afa6be0901caee03a65211af0d01ec1cd2bcb4e989aa0c5a`
for `.artifacts/gpu-pointer-c146/c151-pick-profile-r0/result.json`,
`634335151538bce9a64d4a07e5a0846749b8bea84603196eef94cd2b72ffd3eb`
for `c151-block-loop-long-r0/result.json`,
`059e121bd83982ba77a9416724ee12acd6b8cc6e2e59829ae648b663f036ad86`
for `c151-cpu-block-long-r0/result.json`, and
`193ab615b877aa495d5b9373a77d1a0bb6c2630c0fc6983dfbcc3134acffac82`
for `c151-restored-short-r0/result.json`.

The restored official browser build SHA-256 values are
`6d231576f06b2b60cf0621375886695d6ff41633da1479fe88b96a46a7d5c733`
for `obrush_app_bg.wasm`,
`83a5bb09399a49ecf01f47454f9dae71e001d2aec18da819e51b79dcbb0b170c`
for `resident_bg.wasm`, and
`052b19db036d7fc32f377e683ad7c6b0a9c31ca77d9be2c344bb26c9ec03941d`
for `resident-pointer.js`. The resident binary is not byte-identical to C150;
the scoped short-stroke output and visible history, not binary equality, are
the restoration checks.

Final source has only the added threshold regression test, SHA-256
`14e6e8f66825024b2855deff914376d13137fd9415b9367a3cde5b4d8837a114`.
`cargo fmt --all -- --check`, 2,477 workspace tests across 100 reported
suites (zero failures, one preexisting ignored test), strict native all-target
Clippy, and strict core/app/GPU WASM Clippy pass. The restored picker also
passed its six focused tests, the official web build, and the experimental
WebGPU browser short-stroke/historical visible-hash check. The normal browser
studio still uses WASM CPU sculpting and WebGL2; its experimental WebGPU path still fails
the sustained responsiveness gates. No SculptGL or ZBrush parity status is
promoted by these results.
