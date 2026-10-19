# C261: fail-closed cold Source worker capture

This change adds `SourceWorkerColdCapture` in the shared Rust core. It is a
preflight and one-use reply witness, **not** an ordinary browser Worker brush
and not a new sculpting mode. `capture` requires a nonzero host epoch and
document revision, a closed Source stroke, an admissible Source mesh, and **no
retained Source normal state in any SubTool**. That last refusal is essential:
the public `.obrush` JSON captures geometry and scene, but does not encode the
Source normal field, ordered picking tree, generations, or live history claim.

The admission uses explicit state presence, rather than an allocation-size
estimate, so even an empty-but-semantic retained producer is refused.

An admitted cold capture holds a bounded public project byte string and its
BLAKE3 digest. Its one-use `decode_candidate` checks the epoch, host revision,
active SubTool/level, history depths, closed claim, zero retained state and
complete current public scene before passing a sparse position/**normal**
packet through the existing checked `OBRSDEL1` decoder. It only returns a
validated `Mesh` candidate. It does not install that candidate, rewrite Undo,
update retained normals, update the GPU or bind the ordinary held-pointer
route. No silent fallback or performance claim follows from this seam.

The worker architecture still needs a bounded cross-WASM transfer of the live
retained normal/tree state, the existing Source alpha and pointer inputs, and
an atomic publication of candidate geometry **and** Source state into the
main-thread Document with exact history and renderer validation. A cold-only
worker cannot be used for subsequent warm strokes. Cloning `.obrush` each dab
or merely applying a sparse mesh packet would fork the brush's state. The
one-time project capture can itself be expensive at 1m vertices; no latency
improvement is inferred from the 32 MiB sparse packet cap.

The frozen 100k normal-browser baseline remains the C202 60-second,
24,000-requested-move SGL sphere. Fixture SHA-256 is
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`;
the existing C202 result SHA-256 is
`1143e84c18e9ec3ce642e1ee3961de206445783de52e164650c24f90fd4aa733`,
and independent complete-file/shape audit SHA-256 is
`f0be219200ff37ba37f380562a92709ecec77b36e29f8b91a2a52bc94afa4344`.
It recorded 1,145 applied dabs, 16.7 ms active RAF P95, 10.2 ms app CPU P95
and 15.17 ms release acknowledgment. Rechecking that untouched historical
receipt with the current SP-WORKER-01 verifier gives a **nonzero, incomplete**
result: actual current-path pointer attestation, trusted delivered DOM trace
and VBO position/normal readbacks for every public state are missing. The
failed verifier output is retained in
`.artifacts/worker-cold-capture/c202-verifier.json`. This is a historical
baseline, not a current-build worker/ordinary comparison.

Validation from base `1188948273053e5bdd6bbf246278804bf71992f9`:
`cargo fmt --all -- --check`, `git diff --check`, three of three focused
`source_worker_cold_capture` core tests, strict native `obrush-core`
all-target Clippy and strict WASM core-library Clippy all pass. The current
SP-WORKER-01 verifier output above has SHA-256
`be2801b427cc94f72ba70d9aed67f2b8efd04b083b5e55ce007e46605a49e00f`.
The parent checkout must run the full workspace suite after integration. No
SP-WORKER-01 100k worker run or frozen 1m run is justified until an actual
ordinary held-pointer worker exists.
