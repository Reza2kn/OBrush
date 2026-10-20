# MRP09 cell capacity attribution: early stop

**Decision:** the measured `Vec::try_reserve_exact(1)` calls take too little
time to justify a capacity-growth product change. The preregistered early
stop applied; no product allocator rule, WASM candidate or 60-second browser
gate was attempted. MRP05/MRP06/MRP08 failures and MRP07's incomplete
rejection admission remain unchanged.
The diagnostic source and complete raw logs remain in isolated commit
`86df9b1`; the primary checkout retains this report and the run table only.

The frozen [MRP09 protocol](SCULPTGL_MILLION_CELL_CAPACITY_MRP09_PREREG.md)
has SHA-256 `17723937e718a0edca36bddcb7e5a55f5be0ffffc58a3d66d3e9f9c49fe5a791`.
All ten fresh release processes loaded the exact saved 351-MB public project,
SHA-256 `367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`,
with 999,002 vertices and 1,998,000 triangles. Each complete seed-state
fingerprint, calculated after timing from positions, faces, normals, root
metadata and the ordered-tree snapshot, was
`e7bb7523e66a38d11800f81c735dd947d45c29af861bbe99a6aac5780eb179e5`.
The diagnostic only observes the existing capacity-boundary branch; it does
not change seed allocation, document, rendering or history.

| Fresh-process pair | Control seed (ms) | Probed seed (ms) | Reserve calls | Total measured reserve (ms) | Probe minus control peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 361.25 | 207.26 | 94,280 | 3.531 | +68.40 MB |
| 2 | 194.80 | 213.19 | 94,280 | 3.590 | +68.86 MB |
| 3 | 211.76 | 203.97 | 94,280 | 4.185 | +68.81 MB |
| 4 | 193.94 | 197.77 | 94,280 | 3.447 | +68.88 MB |
| 5 | 199.30 | 196.40 | 94,280 | 3.488 | +68.80 MB |

The reserve-call median was **3.531 ms**, far below the frozen 20-ms
early-stop threshold. The old/new capacity sequence was exactly 1→2,
2→3, ..., 94,280→94,281 for each probe run; no trace buffer overflow
occurred. The total seed times vary more than the reserve cost, including a
361-ms control outlier. This observer also reserves a fixed 131,072-step
trace buffer and serializes a complete tree snapshot after timing; its ~69-MB
RSS increase is a diagnostic effect, not a proposed product memory cost.
The fixed-size trace and the post-timing fingerprint are excluded from the
reserve interval; the full seed time is still subject to normal process
noise and observer overhead.

Complete per-run outputs and `/usr/bin/time -l` memory receipts are retained
under `.artifacts/mrp04-stage/mrp07-wasm/mrp09-native/`. Its run table SHA-256
is `b689c94a6c2b6c33157d885c620d40873a1216024d080623bdbd56122825a744`,
driver SHA-256 `c8e11ea3af3084d428e6b877ce6e2b05a9ebf6decd12b76ae86e8bd065abbe5d`,
and optimized binary SHA-256
`5f4f48822383c95d0ff8fcf3d08130dde24e4634a2c00effe9cde65c68df0107`.
`cargo fmt --all` and strict core all-target Clippy with the feature passed.
The feature is diagnostic-only and remains isolated from the ordinary build.
