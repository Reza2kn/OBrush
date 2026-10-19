# C229: one-pass retained topology validation

The checked Source-geometry retained state still validates the actual public
mesh at each new dab. Its topology comparison now branches once for
original-face surface versus triangle-only mesh, checks exact array lengths
first, then walks each face and its triangulation together. It rejects the
same changed face/diagonal/triangle words without per-row optional lookups.
The normal-state position-bit comparison and all public entry validation
remain in place. This is a validation scan optimization, not a saved trust
flag or a change to sculpt mathematics.

Focused tests cover altered triangle words, an extra or missing triangle,
changed original-face words and an extra original face. The 100 scoped core
Source-geometry tests pass. The final source passes `cargo fmt --all -- --check`,
2,531 workspace tests with zero failures and one intentional ignore
across 83 groups, strict native/WASM Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh` with a valid ad-hoc signature.

The same frozen 999,002-vertex / 1,998,000-triangle SGL sphere and 60-second,
400-requested-moves/s protocol were run twice in an owned normal browser.
Both independent public-file/shape audits pass exact Save, Undo, Redo,
metadata, non-position channels, original triangles, useful edit and no new
affected-face degeneration or reversal. There were no app, console or
cleanup errors. The final portable WASM SHA-256
`fe8b99f4ec0eadf26c775dd834af19e3a423a1f34ae7e89c570e0d772789cf8d`
matches the artifact used in both browser trials byte-for-byte.

| Observed 1M/60s measure | C228 first | C228 final | C229 first | C229 repeat |
| --- | ---: | ---: | ---: | ---: |
| Applied dabs | 967 | 947 | 962 | 962 |
| Retained mesh/state match mean | 7.185 ms | 7.349 ms | 6.466 ms | 6.568 ms |
| Complete Source geometry mean | 21.757 ms | 22.412 ms | 21.473 ms | 21.519 ms |
| Active RAF P95 | 66.7 ms | 66.7 ms | 66.7 ms | 66.7 ms |
| Release acknowledgment | 127.3 ms | 164.8 ms | 118.2 ms | 124.9 ms |
| Preregistered active-recorder gates passed | 0/4 | 0/4 | 0/4 | 0/4 |

The roughly 0.6–0.9 ms lower retained-match means are consistent across
these separate browser runs, while delivered contacts and scheduling differ.
They do not establish a causal end-to-end speed ratio. Frame P95 did not move,
release timing remains variable, and all four responsiveness gates still
fail. Pinned SculptGL's separate run observed 16.8 ms RAF P95 and 8.0 ms
release. This work does not promote SculptGL or ZBrush parity.

The [first C229 result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-1d5dff6c-0f37-4554-b6e8-5d6f501abf82/result.json)
has SHA-256 `955b4cb29e301333db0b0e0f3885ae3083934b77915984ebb7708968de10b73d`;
the [repeat result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-d7752368-61af-4873-b048-e18701eb751f/result.json)
has SHA-256 `6b59d1c059766968cd03490c42c26145218836c6578576720c0a8c4da9a6a54d`.
Their full metrics are in
`.artifacts/c229-browser-summary.json` and
`.artifacts/c229-repeat-browser-summary.json`. The independent audits are
`.artifacts/c229-browser-audit.json` (SHA-256
`2d31238a17922531c80e9660efe6765b90c4d6dd1159ae53d2185de5dfa00bad`)
and `.artifacts/c229-repeat-browser-audit.json` (SHA-256
`dca176ac6caeabd95604fa627554eea16b329970e15e9990cd7ee9def3aab92e`).
The packaged macOS executable SHA-256 is
`2310da86bd6866655baba362817bf54fbf1c8bff55fe30f49d27652cc2e70a60`.
