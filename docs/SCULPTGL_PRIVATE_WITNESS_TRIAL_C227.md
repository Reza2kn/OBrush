# C227: rejected private-state position witness shortcut

The C226 million-vertex browser comparison identified repeated Source
geometry work. A trial changed the privately owned normal-state advance to
rewrite only selected position witness words after a controlled dab. The
uncontrolled refresh retained its full scan. A focused shared/private test
checked exact state words, retained buffers and mesh matching. Its first run
failed because it expected the no-change private path to increment its
generation; the corrected assertion and final focused test passed. The first
failure was visible in the tool output, but was not saved to a file. The final
passing output is `.artifacts/c227-focused-test.log`.

`cargo fmt --all -- --check`, `cargo test --workspace --locked`, strict native
and WASM Clippy, `scripts/build-web.sh` and `scripts/package-macos.sh` passed
for the trial. In the frozen 999,002-vertex, 60-second normal-browser cohort,
the independent public-file, exact Save/Undo/Redo and changed-face shape audit
also passed: 40,789 changed vertices, 82,402 affected faces, zero new
degenerates and zero reversals. The result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-34f0ad76-8404-4916-8757-1e10afd50674/result.json`
(SHA-256 `d108f68d4c144337dc9dc29ac2fdec6a23728cda9c39263321ff06ffbc5c0db7`).
The independent audit is `.artifacts/c227-browser-audit.json` (SHA-256
`e873a4ff55120e890ea7bf9423ae7518907c052f4650c4a74d68201fff7921e0`).

| Observed 1M/60s measure | C225 baseline | C227 trial |
| --- | ---: | ---: |
| Applied dabs | 886 | 831 |
| Source state advance mean | 6.796 ms | 6.950 ms |
| Source retained match mean | 7.372 ms | 8.007 ms |
| Active RAF P95 | 83.3 ms | 83.4 ms |
| Release acknowledgment | 161.2 ms | 168.5 ms |
| Preregistered active-recorder gates passed | 0/4 | 0/4 |

The runs delivered different contacts, so these differences are observations,
not a causal speed ratio. The trial did not improve the user-facing measures
and was removed. The first geometry pass aliases its original Arc and takes
the shared-state path; private-buffer reuse is limited to unaliased
intermediate states. The next performance change must address the measured
retained mesh/state validation, renderer admission, or shared-state advance.
After restoring the source, `scripts/build-web.sh` reproduced the prior
portable WASM byte-for-byte at SHA-256
`5b591faca1e520b37d1b542b1d824b4deb024f1cb1b0d7eb15163fef1dafd91f`.
No SculptGL or ZBrush parity status changes.
