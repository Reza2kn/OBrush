# C284 read-only retained warm encoder subphase profile

**No product codec change.** Base commit `2666f67ef5de24710b38746515851c8d60aad86b`;
temporary diagnostic patch `read-only-subphase.patch` SHA-256
`4736322385503d27f1f0cfdc75f302a6751d0de32a3560810aae99cc24393493`.
The unchanged frozen SP-WORKER-01 100k fixture/protocol passed preflight.
The focused core test ran **one** test and proved profiled and ordinary warm
packets byte-identical with equal decoded state fingerprints. A prior command
with `--exact` accidentally matched zero tests; its log is retained separately.
The Node VM nonpublication test verified unchanged Worker success/rejection,
packet bytes, transfer buffers and owner/take order, with additive timing only.
`cargo fmt --check`, JS syntax and `git diff --check` passed. WASM browser
build passed.

Served instrumented SHA-256: WASM
`b35b616f7766f4fdd65e03d799200b8479b23560c296103b83af50890bb46554`,
WASM JS `3cdc9d3bf45d47489d3dedc6e764719738f4debc1fac617a3d8f0b6aadd9beb0`,
Worker JS `f1046cb3125097bcf54328ba5c630b7d662af303639b0f47abc115cd69391a21`.
Raw run: `.artifacts/source-worker-sustained-spw01/100k-worker-b337d37d-9073-47c8-9d7e-d696b65c1177/`,
`result.json` SHA-256
`53c4e1e2065633375843cce9966e5c83dc1e9cef561b10d275fcbc49bbb9cbc4`.
It completed 1,289 dabs with one commit, zero Worker rejections and zero
JS/GL errors. Final transfer was 16,341,256 bytes, of which 16,221,832 were
the retained warm packet. Main-to-main finish interval was 204.8 ms and
release-to-commit was 694.7 ms.

| Encoder stage | Duration, ms |
| --- | ---: |
| Public mesh digest | 32.4 |
| Semantic state fingerprint, total | **99.0** |
| Ordered-tree part of state fingerprint | 55.9 |
| Mesh validation | 1.4 |
| State-to-mesh exact binding | 0.6 |
| Storage budget | 0.0 |
| Scalar array serialization | 9.0 |
| Ordered-tree serialization | 21.6 |
| Final packet checksum | 8.7 |

The bounded writer grew 22 times and observed 25,165,750 prior bytes at
relocations; this counts prior buffer lengths when the pointer changed, not
measured allocation-copy CPU time. The semantic fingerprint threshold of
at least 40 ms is met and its tree portion is substantial. However a fused
walk must still perform all semantic hash updates and emit all packet bytes;
the 99.0 ms is **not** avoidable time. No independent same-state native/WASM
microbenchmark predicts the preregistered 40 ms net win. **Fusion is not
admitted for implementation yet.** No compression or writer candidate was
started. The next decision would require that separate same-state prediction,
not a causal ratio against a different 100k dab delivery.

`shape-audit.json` exited 0: 4,013 changed vertices, no new degenerate or
reversed faces, maximum changed-edge stretch 2.219. `gpu-audit.json` exited
1 at all saved-mesh-normal stages (the known Source display-normal oracle
mismatch). `frozen-verifier.json` exited 1 on active RAF max 483.2 ms,
queue not empty by 100 ms, and GPU position+saved-normal all states. Active
RAF P95 was 16.8 ms, app CPU P95 10.8 ms, release acknowledgment 5.79 ms.
These failures are preserved; the diagnostic is not a qualified cohort.

The temporary probe source was reversed and the normal browser build restored.
Restored WASM SHA-256:
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`;
restored Worker JS SHA-256:
`4b56bb3d1e1b0a594f0d288213ba856c11e1a95f8fb94b3ecb5dde73188caa60`.
Tracked worktree is clean. No source change is promoted.
