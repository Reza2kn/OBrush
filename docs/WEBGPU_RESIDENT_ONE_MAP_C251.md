# C251: one-map local sparse release for the opt-in WebGPU preview

The current `web/resident-pointer.js` preview already keeps the held-stroke
mesh in WebGPU memory. Its dabs transfer no mesh rows to the CPU. On release,
`ResidentViewport::sparse_positions` counts changed vertices, compacts them
to sorted 20-byte ID/RGBA records, and submits the result to the shared-core
document as one validated Undo step. This branch changes only the readback for
a nonempty payload of at most 1 MiB: one staging buffer receives both packed
rows and the post-collection four-byte GPU count. It maps once instead of
mapping the rows and count separately. The initial four-byte count map and
the strict count equality check remain. Above 1 MiB the existing chunked
path remains intact, including its bounds checks. The wire record, brush
shader, document validation, history, and ordinary CPU/WASM brush route are
unchanged.

For the small-payload path, the release performs two GPU maps rather than
three, and transfers the same `4 + payload + 4` bytes. This is a bounded
synchronization reduction, not a measured speedup claim. The one-map count
is exposed as `sparse_one_map_readbacks` in the GPU diagnostic report. The
opt-in pointer status records failure readback, sparse readback, shared-core
commit, total commit time and sparse bytes separately. The browser probe
`scripts/check-resident-sparse-one-map.mjs` exercises the real overlay and
retains a build-hashed result.

## Validation

The isolated candidate passes `cargo fmt --all`, all workspace tests, strict
native and `wasm32-unknown-unknown` workspace Clippy, `scripts/build-web.sh`,
`node --check` for both browser files and `git diff --check`. The focused
native Metal probe changes 7 vertices, returns 140 sparse bytes, reports
exactly one combined map and zero full-position readbacks, and checks exact
project before/accepted/Undo/Redo plus empty no-op/cancel behavior. Its one
observed sparse stage took 69.236 ms; there is no matched native baseline.
The native result is
`.artifacts/resident-sparse-one-map/native/result.json` (SHA-256
`bb05de2eb40a7c85fb8d76c8a892b9b696568af869347d4e6dff1d09f52a5aa5`);
the sparse bytes are SHA-256
`676cf71a17d0dd637053e8b69ee0c4265f46a30481ed7f731c95c79f4401117b`.

The final owned headless Chrome run drove the opt-in overlay with 7 pointer
dabs, 7 received samples and no coalescing. The browser produced 7,140
sparse bytes for 357 changed vertices, one combined map, zero full-position
readbacks and a successful shared-core commit. Its observed failure-word,
sparse-readback, core-commit and release stages were 0.7, 1.2, 14.0 and
18.0 ms. These are one short-stroke observations, not a speedup or 60-second
responsiveness result. The final browser result is
`.artifacts/resident-sparse-one-map/becb8656-68f7-4427-93b2-0a0a33b0ded4/result.json`
(SHA-256 `97b77fa0e6acb4c3790bcb281398281f00e7fa89c0e3ab8462485d3b0ee327c4`),
with GPU WASM SHA-256
`ea7e42c5f47b5ea0c58957c7bea4a1a594f0672617458d4411cbf90348d85975`
and app WASM SHA-256
`88c1be7ca7d4d6a9dc45b59025bedcdc41b174c4e16d212cb9e9f87b439df13f`.
The final browser checker is SHA-256
`e210359b8db9df27f0e12f9f63cc5d286679cbb36e96fea16a755680441a9bb1`.

Two prior browser attempts with the stricter GPU-report assertion canceled
before release, with no edit or reported error; receipts are preserved at
`.artifacts/resident-sparse-one-map/624ab901-6d2a-4782-b572-0464b9d9ed01/result.json`
(SHA-256 `84b5a70cf8c34fa90a26a5bfc2f6665a95da873b438300361da5813ad68de50d`)
and `.artifacts/resident-sparse-one-map/7a55e35a-bc26-4e8e-af0c-d3fbb4fe62d8/result.json`
(SHA-256 `5662d8701439bf584c97835046630c895bfde0f7b8edf9166fe85b00e78b4244`).
The checker now waits 600 ms after startup before capturing the camera; the
specific cancellation signal in those failed runs was not recorded. An
earlier, less strict browser run did commit successfully but did not assert
the one-map report; its receipt SHA-256 is
`b28fc37d0f3833714b7e8c0f5d81c66f9e6526cdd97d37025f16ce7635d41bed`.
None of these receipts are rewritten as passes.

The [C233 ordinary million-vertex profile](SCULPTGL_APP_FRAME_PROFILE_C233.md)
measured 22.407 ms mean Source geometry per dab and 17.702 ms mean renderer
callback, with 66.8 ms active RAF P95, 68.6 ms app CPU P95 and 165.4 ms
release acknowledgment. The opt-in WebGPU preview's release map cannot close
those ordinary held-pointer RAF/CPU gates. The next measurable architecture
step is an ordinary-route worker candidate that owns shared-core geometry
across the actual held-pointer stream and publishes bounded changed position
and normal rows to the main WebGL viewport. It needs the unchanged
[SP-WORKER-01](SCULPTGL_WORKER_SUSTAINED_ADMISSION.md) serial 100k/1m matrix,
including public Save/Undo/Redo and independent exact GPU position/normal
word checks. No worker candidate is implemented by C251.
