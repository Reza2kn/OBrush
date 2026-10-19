# Source Worker retained seed witness

The shared core now has a fixed 112-byte `OBRSN0V1` witness for a retained
Source normal/tree producer that is **exactly reconstructible** from its mesh.
Encoding checks the current producer against a separately built seed: exact
positions/topology, both normal fields, interaction center, complete ordered
tree words, zero generation, no render predecessor and no selected render
chunks. Decoding checks version, target SubTool ID, checksum and exact mesh
position/topology words before building a fresh validated seed under the
existing Source allocation limits. A second digest checks all reconstructed
normal and tree words against the sender, so a runtime/algorithm mismatch
fails closed. It returns a producer only; it does not
replace a `Document`, history or live stroke.

This is a narrow cross-realm building block. A warmed producer has historical
selected-refresh normals and mutable ordered-tree cell/free/serial/bounds
state. Rebuilding it from its current mesh would change later contact and dab
behavior. Such a producer is refused, including on an otherwise valid public
`.obrush` scene. A complete warm-state transfer needs an independently
validated structural tree codec, full document/history ownership, epoch and
revision handoff, and atomic renderer publication. The existing
`SourceWorkerColdCapture` still refuses *all* retained producers; this packet
is not wired into the ordinary browser brush or a Worker host. No speed or
SculptGL parity conclusion follows.

The independently authored single-face fixture sends its mesh through public
`.obrush` JSON, then compares two consecutive Inflate dabs from the original
seed and decoded seed: exact public mesh bytes, changed row counts, next
retained generation/tree snapshot and normal report. Separate negative checks
reject a warmed producer, stale signed-zero mesh position, different topology,
wrong SubTool ID, damaged checksum, and resealed stale mesh/state digests. This is a
core-only fixture; a normal browser worker validation remains open.

Validation from the isolated worktree based on `e679380`:
`cargo fmt --all -- --check`, both focused
`source_worker_retained_seed` tests, strict native `obrush-core --all-targets`
Clippy and strict `wasm32-unknown-unknown` core-library Clippy pass with
warnings denied; `git diff --check` passes. The parent checkout must run the
full workspace suite after integration. No browser run applies to this
non-publishing core codec.

The first fingerprint revision failed the focused round-trip check because
the decoder compared the mesh digest against both digest fields. The slice
boundary was corrected to the named mesh/state offsets, and the unchanged
fixture then passed on rerun. This failed check is retained here.
