# Source Worker release endpoint trial (preregistered, unpromoted)

This is a preregistration for the opt-in `?source-worker-held` route. The
baseline in [the sparse held-pointer report](SOURCE_WORKER_SPARSE_HELD_TRIAL.md)
dropped one queued mouse-up endpoint from each of two 40,962-vertex strokes.
The trial remains off by default. No SculptGL or ZBrush parity claim follows
from this protocol check.

## Frozen acceptance before implementation

- A trusted viewport pointer-up on an eligible Source Draw gesture is the last
  position used by the Worker. A pen release carries its positive measured
  pressure, or the last in-contact pen pressure if the release event reports
  zero; mouse pressure remains 0.5. A duplicate endpoint already applied is
  not dabbed twice.
- Release may wait for at most the one already outstanding Worker apply, then
  send at most one endpoint apply, then one finish. No pending path batch is
  drained after release; status separately counts skipped queued path samples
  and a skipped raw intermediate move. There are no dabs after the final
  publication.
- A cancelled, invalid, stale or changed-owner gesture publishes nothing and
  restores preview. A successful release produces one public history commit;
  zero-effect release preserves redo. A second warm stroke still works.
- Browser evidence must use owned Chrome, an independently created mesh
  fixture, trusted viewport pointer events, public Save/Undo/Redo, and exact
  GPU position words after release. Record input coordinates/pressure, hashes,
  sequence counts, release-to-commit and pointerdown-to-preview timings, and
  retained-state identity. A long held stroke must report the release endpoint
  and no post-release input backlog.
- A 100k and a million-vertex sustained-pointer responsiveness gate remain
  open until separately run. The protocol may add one Worker apply latency to
  release; this trial cannot be described as a general speed improvement.

## Results

The implementation serializes one final checked endpoint input at pointer-up,
clears queued intermediate path samples, and allows at most one outstanding
apply plus one endpoint apply before finish. The endpoint bypasses ordinary
spacing, so a sub-spacing release is still represented. The browser reports
skipped queued dabs and a skipped pending raw move separately. A pen release
with positive pressure uses that value; a zero-pressure release uses the last
in-contact pressure. Cancellation still terminates the Worker and cancels the
provisional preview. The final checked publication remains the sole history
transaction.

`node --experimental-default-type=module --test web/source-worker-held-input.test.mjs`
passed 3/3 protocol tests, covering queued release, one outstanding dab,
zero-pressure pen fallback, duplicate endpoint, and cancellation. `cargo fmt
--all --check`, `cargo test --workspace --locked --quiet`, native workspace
all-targets strict Clippy, and strict WASM core/app/gpu Clippy passed. The
first `scripts/build-web.sh` invocation failed because this isolated worktree
used a shared `CARGO_TARGET_DIR` but the script reads `target/` relatively. An
ignored `target` symlink to that cache allowed the unchanged official script
to pass. No source files were copied from a commercial product.

The final owned Chrome 155 check used the normal import control and trusted
viewport mouse events on an independently generated version-7 raw-Source UV
sphere: 40,642 vertices, 81,280 triangles, 128 latitudes, 320 longitudes,
fixture SHA-256 `7f4fc59231332658c6fbb92fa89ec2bafe811722b797a2bd68ea0b2a1b33d98f`.
The ordinary controls selected SourceOrdinary Draw, disabled X symmetry, and
selected Studio Clay shading. Two strokes had eight rapid held moves each,
with mouse-up at `(850,460)` then `(815,445)`, pressure 0.5. Both final
trusted positions appeared exactly in the trial status, both endpoint applies
were acknowledged, and both strokes produced one public commit and zero
rejections. One and two intermediate inputs respectively were skipped at
release; no queued dabs remained. The second stroke used the warm path.
The seed reported cold then warm explicitly. Release-to-commit was
258.7/266.8 ms; pointerdown-to-first-preview was 198.7/295.3 ms.
The 500 ms quiet tail sent no additional dab and a second
public Save was byte-identical. Public Save SHA-256 was
`63d3c27472d5cbacb1e28b05a33fe0df397b71470b8851f25e8a108175b3ff0e`
before strokes,
`1367c1f4f6db6ee3e7169a549ee507b93784c4a49be87a73316c6dbad656f5dd`
after stroke one, and
`87a9bb13a5559689a91cbd97ab1487269ea86ae759e6283f11a624b8bb877dc7`
after stroke two. Undo matched stroke one and Redo matched stroke two exactly.
The final receipt is the ignored
`.artifacts/source-worker-release-endpoint/c6129f31-0cad-46e7-96ef-76811185f190/result.json`
with SHA-256 `e0fd7ecb97401887de1c727558e16a3267b68f9664190b847e9c733082f693ba`.

Failed checks are retained under the same ignored artifact root: the first
run did not admit a gesture because the startup sphere was in Legacy mode
with symmetry and Matcap; the next runs exposed the need for an explicit
raw-Source import and the Studio Clay control name. A later public Save
attempt timed out because the diagnostic returned the entire multi-megabyte
project through CDP on every poll; its corrected version hashes the actual
normal Save callback in the page. These are harness/admission failures, not
evidence of an endpoint regression.

The browser check does not include a physical pen, exact VBO word readback,
cancel after a changed document, 100k/million sustained-pointer cohorts, or
a matched SculptGL gesture. Those preregistered gates remain open. This route
stays query-gated and off by default.
