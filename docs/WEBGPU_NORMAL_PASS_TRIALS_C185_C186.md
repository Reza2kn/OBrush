# C185–C186 WebGPU normal-pass trials (rejected)

These trials tested whether reducing the resident brush's per-dab full-mesh
normal work would improve the opt-in WebGPU pointer preview. Both changes were
removed after controlled native and browser checks. The final GPU WASM is
SHA-256 `7400c7230649d4c5c56b134d05ecad57ca4090017da88b1f1ae8cfeeb47c7a81`,
identical to the C184 build; the SIMD app WASM is also unchanged at
`9f9b9e8b58deb8e57c691674b1ff16696ac0f41626f1cce50557b0ac3fb326ed`.

The C185 stage-profile run (`.artifacts/gpu-pointer-c146/c185-long-stage-profile-r0/result.json`,
SHA-256 `1ba445e383d964fbd1be7b4bf2683fd2d3dacae043bb40c25027c27262df8320`)
fenced pick, sculpt and draw stages separately on the 999,002-vertex independent
SGL sphere. Across 39 dabs they averaged about 91.5, 85.8 and 86.7 ms per dab,
respectively. These are *fenced wall/queue times*, not isolated shader times;
they cannot be summed to attribute total GPU cost.

C185 fused the position and face-normal commits into one compute dispatch.
The Metal picked-brush probe's JSON was byte-identical to the saved C180 probe,
and the two-stroke browser run retained the same before/edit/Undo/Redo PNG
hashes. Its long browser run (`c185-million-long-fused-r0/result.json`, SHA-256
`838ea40ee010e0e3c3a4d2a9d28bb9a524fad922b5d637015b7f844ce8db5a6b`)
still measured 50.1 ms RAF P95, about 91 ms GPU wait per dab and 419.9 ms
release. Delivered contacts differed from C184, so this is not an exact
shape/time pair. There was no material responsiveness gain.

C186 instead appended selected face IDs while computing candidate normals and
used a bounded GPU worklist with indirect dispatch to avoid the second
full-face commit scan. The independent native Metal probe used radius 0.5 and
intensity 0.25 on the authored 162-vertex sphere. Its position words, sparse
payload, cancel and Undo/Redo exactly matched the saved C180 result; maximum
GPU/reference component error was `3.4924596548080444e-10`. The only changed
native report field was the extra GPU resident allocation. The new result is
`.artifacts/gpu-pointer-c146/c186-picked-native.json` (SHA-256
`f03c39d97df97f61c6a9fd9e2b79387ae8528680dc63930635d98aa1b24ee2e4`).

The C186 million-vertex two-stroke browser result
(`c186-two-stroke-worklist-r0/result.json`, SHA-256
`4d98671c7d8e3248b04d6bc54b161f39152df9f5bc99b2832f503ecd5debf94e`)
passed WebGPU validation, visible edits and exact Undo/Redo. All six saved
before/preview/edit/second-edit/Undo/Redo PNG hashes matched the C185 restored
baseline. The ten-second result (`c186-million-long-worklist-r0/result.json`,
SHA-256 `dd8d2e5a0a337606d24e67682834dbc03b9666c6d1e9edeb8d769c1b02df71a9`)
sent 221 moves, received 223 contacts, coalesced 111 and applied 112 dabs,
the same counts as C184. RAF P95 stayed 50.1 ms, maximum stayed about 400 ms,
and GPU wait was 91.29 ms/dab versus 91.13 ms/dab in C184. The complete
ten-second output hash differed with event timing, so the short exact-output
check is the correctness evidence; the long run is a pacing diagnostic.
The additional storage, atomic append and indirect-dispatch synchronization
therefore did not justify retaining this implementation.

Both rejected source changes were removed. The retained C184 ten-second run
still fails its RAF P95, frame-maximum and release-idle gates, and the normal
studio still uses WASM geometry with WebGL2 drawing. These trials do not
establish SculptGL responsiveness or ZBrush parity. A useful next experiment
must address whole-dab GPU execution or the per-dab completion wait and must
retain the same output/history and source-scope checks.

After restoration, `cargo fmt --all -- --check`, `cargo test --locked
--workspace`, strict native `cargo clippy --locked --workspace --all-targets
-- -D warnings`, strict WASM `cargo clippy --locked --workspace --target
wasm32-unknown-unknown -- -D warnings`, `scripts/build-web.sh`, and
`git diff --check` all passed. The final browser binaries have the C184
hashes above. No production WebGPU brush path was promoted.
