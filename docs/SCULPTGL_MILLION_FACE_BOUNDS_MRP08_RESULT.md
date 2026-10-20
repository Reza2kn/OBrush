# MRP08 face-bounds candidate: exact but too small to pursue

**Decision:** the early bounded native screen did not justify the frozen
≥20-ms ordered-tree gate. The candidate is **not admitted**, remains only
in this isolated worktree, and no browser/held-pointer parity claim follows.
MRP05/MRP06 failures and MRP07's incomplete seed-rejection admission remain
unchanged.

The frozen [MRP08 protocol](SCULPTGL_MILLION_FACE_BOUNDS_MRP08_PREREG.md)
has SHA-256 `c28d2451dd4737534c5aaf902c5c52c937479a8ce9dfac381f91a95e072bfb3a`.
The independently retained legacy formula and direct Float32 extrema loop
returned the same error and all nine output words for signed-zero,
subnormal, finite-extreme, duplicate-corner, invalid-index, NaN and infinity
cases, plus 100,000 deterministic randomized rows. On the exact saved
351-MB public project (SHA-256
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`),
all 1,998,000 original face rows matched bit-for-bit; both streaming hashes
were `dcc4f79d980baa53022f61c166615810c005521b60a077f33cd2bde7f425473b`.
The all-face log SHA-256 is
`9c0c097fea7069b2a8235366c7151cb9c65a06e6720ef97431a30751c110686d`.
An initial all-face invocation failed because the fixture environment path
was relative to Cargo's crate working directory; that failed log is retained
with SHA-256
`6b0ba765748f019e10fd476375e06b3f73944817a7f6032ec978a6d7e2afa474`.
The retry used the absolute path and passed.

An exploratory **bounds-only** release test used one optimized binary
(SHA-256 `cf9b2ddb667fe5d7f383318597ff887c1e9aa6e19de8c74a1f700a5205209e7e`),
fresh processes and alternating order. It timed the two formulas over every
public original face without timing project import or hashing:

| Pair | Legacy (ms) | Candidate (ms) | Saved (ms) |
| --- | ---: | ---: | ---: |
| 1 | 29.638 | 29.247 | 0.392 |
| 2 | 29.595 | 30.720 | −1.126 |
| 3 | 29.644 | 29.350 | 0.295 |
| 4 | 45.817 | 29.716 | 16.101 |
| 5 | 29.851 | 29.305 | 0.547 |

Median saving was **0.392 ms**; the outlying fourth legacy process does not
establish a repeatable gain. Complete rows:
`.artifacts/mrp04-stage/mrp07-wasm/mrp08-bounds-screen.json` SHA-256
`d8accededa571632a9f62172f0f50dacad2ce4658b0a56595cbeb57ed8d348f3`;
driver SHA-256 `395f4ed5e733862aff746cb82cafc82a2a4e2d033092051e97ee1d6d247b4d6a`.

The face-bounds loop is the only changed production calculation in this
candidate. It runs once per original face during tree build. A repeatable
sub-millisecond isolated saving cannot plausibly close the preregistered
20-ms full-tree threshold or the much larger first-press and held-pointer
frame gaps, so the expensive full-seed WASM and 60-second browser gates were
not run. The candidate remains isolated for audit, not integration.
