# Source Worker sparse publication seam

The old isolated `codex/worker-authoritative-source` trial (`267677c`,
`115186f`) is **not** part of the current ordinary browser brush. Its
`SOURCE_WORKER_DELTA_TRIAL.md` result came from an opt-in, one-shot API with a
100,000-vertex / 200,000-triangle admission cap. The 99,602-vertex native
microprobe returned 122,476 bytes for 4,371 changed position/normal rows,
compared with an 18,940,554-byte full snapshot. It measured transfer size,
not a normal-browser held-pointer speedup. The 999,002-vertex frozen cohort
was rejected by that cap.

The current shared core has a move-owned `SourceWorkerTransaction` with an
epoch and ordered dabs, but no cross-WASM ownership transfer or ordinary
browser Worker host. Restoring the old resident host would bypass the current
ordinary selection, history, and rendering path. This change therefore adds
only a reusable, checked *mesh candidate* packet to `obrush-core`:

- `encode_source_worker_mesh_delta(before, after, active_id)` first validates
  both complete meshes and rejects topology, masks, colors, materials, UVs, or
  other non-geometry changes. It encodes strictly ascending changed vertex
  rows, each with the ID and exact position and normal `f32` words.
- The internal `OBRSDEL1` version-1 envelope has a vertex count, active
  SubTool ID, BLAKE3 digest of baseline position/normal words, and a BLAKE3
  packet checksum. It is capped at 32 MiB, with checked arithmetic and
  fallible allocation. It is not a public `.obrush` format.
- `decode_source_worker_mesh_delta` checks the envelope, cap, checksum,
  target, exact baseline, row order and uniqueness, then validates the whole
  candidate before returning it. It never edits a `Document`. Empty packets
  return `None`.

The codec can handle one million rows under its byte cap. That is a wire-size
fact only: it creates a complete candidate mesh and revalidates it, so it
does not establish low-latency publication. A future host must add a one-use
main-thread token and document-revision check, a non-active scene/pose and
attribute witness, bounded complete initial Worker ownership, and atomic
history publication. It must also transfer or otherwise reconcile the
transient retained `SourceGeometryNormalState`; a sparse mesh packet alone
would silently alter later brush behavior. Worker failure must preserve the
main-thread document and its Undo/Redo state. The old opt-in path expressly
did not adopt this normal state into the studio.

No normal-browser 100k or 1m held-pointer run has been performed for this
codec. The frozen `SP-WORKER-01` protocol remains unqualified, including
all four responsiveness gates, release/queue tail, exact public-file/GPU
audit, and matched delivered contacts.

## Validation

From the isolated worktree based on `1205811`, `cargo fmt --all -- --check`
and `git diff --check` pass. `cargo test --locked -p obrush-core
source_worker_delta -- --nocapture` passes six of six focused tests, including
one completed `SourceWorkerTransaction` Inflate dab, exact signed-zero words,
stale target/baseline, tampered checksum, nonfinite geometry and resealed
duplicate/unsorted rows. Strict native `obrush-core --all-targets` and
`wasm32-unknown-unknown` core-library Clippy pass with warnings denied. The
parent checkout must run the full workspace suite after integration. No
browser run is appropriate for this core-only, non-publishing codec.
