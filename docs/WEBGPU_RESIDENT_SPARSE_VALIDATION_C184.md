# C184: checked sparse candidate validation

The experimental WebGPU Brush preview still commits a sparse position result
through the shared Rust document. At capture, the source mesh and posed
metadata are fully validated and its exact serialized fields fingerprinted.
At release, the document must still match that fingerprint and pose; the
records have sorted in-range IDs, finite bounded positions and protected-mask
checks; and the deformation must pass the surface fold guard. The candidate
is cloned from that validated source and changes only those checked position
rows. The resident replacement now rechecks allocation/array limits and the
changed positions in world space while continuing to validate current render
and raw-origin metadata, scene structure/budgets, and Undo/Redo. Ordinary
`replace_mesh` callers retain full validation. The resident fast path is
private to this proven capture/commit sequence.

The independently generated `999,002`-vertex SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
was edited through two consecutive three-dab Draw mouse strokes at radius
`0.1800000072`, strength `0.3499999940`, X symmetry and accumulation on,
hardness zero, world-space radius and viewport `[225,57,1365,882]`.
Before and after this change the first stroke changed 5,171 vertices and the
second 5,654. The complete before, first-after, second-after, visible Undo
and visible Redo viewport PNG hashes matched byte-for-byte across the two
builds. Undo returned to the first-after image; Redo to the second-after
image. The second-after hash is
`698b943975bac6ff0fa7150e16b0617e19cfc2d2d1eded1c4870c2e6e750cbad`.

In these individual local browser runs, the second stroke's replacement
stage measured 54.3 ms before and 35.1 ms after, with the candidate-validation
substage 13.6 ms before and below displayed browser timer precision after.
Release measured 146.8 ms before and 128.0 ms after. First-stroke release
measured 329.3 ms before and 313.3 ms after; both still pay for first-time
topology and normal work. These observations do not establish a stable speed
distribution or reference performance. The pre-change result is
`.artifacts/gpu-pointer-c146/c183-two-start-blake3-r0/result.json`
(SHA-256 `65a19133c9c9fe282a5a5bfa7903061e5549fedc4b496a9436b409a0c0b7de92`);
the changed result is
`.artifacts/gpu-pointer-c146/c184-two-stroke-fast-validate-r0/result.json`
(SHA-256 `f6baf27a76203c478101ed11b26fa4de3d60502dd2c12b93178691e463c2393c`).

An independently authored posed SGL tetra test proposes a rigid translation
whose local coordinates remain bounded but whose transformed world points
exceed the limit. The sparse commit rejects it without changing the saved
document. Earlier test attempts failed at the local coordinate and protected
mask checks before exercising world bounds; their logs are preserved as
`c184-world-bound-test.log`, `-r1.log`, `-r2.log` and `-r3.log` under
`.artifacts/gpu-pointer-c146/`. The corrected `-r4.log` passes. Existing
resident tests cover stale fields, invalid records, folds and exact history.

The final 10-second SIMD browser trace started preview in 591.5 ms, sent
221 pointer moves, received 223 contacts, coalesced 111 and applied 112 GPU
dabs. It changed 25,486 vertices; the delivered event count differs from the
prior C183 trace, so their complete shapes are not a paired comparison.
RAF P95 was 50.1 ms, maximum 400 ms, and release 414.7 ms, including a
322.7 ms checked commit. Visible Undo/Redo restored the run's own exact
before/after PNG hashes. The standing RAF P95 ≤33.34 ms, maximum ≤250 ms
and release-idle ≤100 ms gates **still fail**. The result is
`.artifacts/gpu-pointer-c146/c184-simd-million-long-r0/result.json`
(SHA-256 `6e0136ad61e08dda3e65cfd5c21be181353c3e3f136bf763f750daa70a4165ac`).
The portable app, SIMD app and GPU WASM SHA-256 hashes are respectively
`d563795f36a152e82bd21a7f752e93cab24d54a443cc4a4ed3459d5d72b75244`,
`9f9b9e8b58deb8e57c691674b1ff16696ac0f41626f1cce50557b0ac3fb326ed`
and `7400c7230649d4c5c56b134d05ecad57ca4090017da88b1f1ae8cfeeb47c7a81`.
This remains an opt-in WebGPU preview, not a normal-studio WebGPU or SculptGL
parity result.

Final checks passed: `cargo fmt --all -- --check`, 2,497 workspace tests
across 103 groups (zero failed, one ignored), strict native and WASM-target
workspace Clippy for portable and SIMD configurations,
`scripts/build-web.sh`, `scripts/package-macos.sh` with a valid local app
signature, and `git diff --check`. Browser checks used the rebuilt packages
and closed their owned browser processes cleanly. These implementation
checks do not establish reference parity.
