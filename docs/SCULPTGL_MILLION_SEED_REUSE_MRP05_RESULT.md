# MRP05: same-state seed reuse is fast, but memory admission failed

**Verdict:** the exact seed-state check and the ≥5-ms native/WASM first-dab
microbenchmark gates passed. The preregistered *no larger peak memory in every
matched pair* gate did **not** pass in owned Chrome: reuse was 8,208,384 bytes
higher in pair 5. This is a failed candidate admission, not a product speedup
or SculptGL parity result. No production Arc-reuse edit was made.

Protocol: [MRP05 preregistration](SCULPTGL_MILLION_SEED_REUSE_MRP05_PREREG.md)
SHA-256 `7e9b5a5ff3e7e22015671fc620e94bfff43b9d97bdf080873c81382bb7b96fef`.
The saved independent `root-before.obrush` project SHA-256 was
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`;
the fixture has 999,002 vertices and 1,998,000 triangles. Both arms decoded
and imported this same public file. All ten native and ten WASM runs produced
the identical complete faces/positions/normals/tree fingerprint
`2952f295ecaee65a6e68dd9b5e807c87901fc845a5663c7104ffd8f93c667594`.
The baseline builds a press seed, checks the mesh, drops it, then builds a
first-dab seed. Reuse keeps and clones the checked press Arc. No mutation or
document publication occurs in this test.

| Paired fresh processes | Native first-dab net gain | Native RSS reuse minus baseline | WASM first-dab net gain | Sampled Chrome-tree RSS reuse minus baseline |
| --- | ---: | ---: | ---: | ---: |
| 1 | 187.395 ms | −21,479,424 B | 254.2 ms | −11,911,168 B |
| 2 | 186.139 ms | −21,463,040 B | 242.7 ms | −5,718,016 B |
| 3 | 186.292 ms | −21,463,040 B | 241.0 ms | −2,260,992 B |
| 4 | 187.857 ms | −21,479,424 B | 255.6 ms | −114,688 B |
| 5 | 195.687 ms | −21,463,040 B | 258.5 ms | **+8,208,384 B** |

The medians were 187.395 ms native and 254.2 ms WASM. Native
`SourceGeometryNormalState::approximate_bytes` was 208,932,004 B; WASM was
203,651,892 B. This is a target-layout/accounting difference, not an arm
difference. The WASM linear-memory high-water value was 1,502,150,656 B at
before, after-press and after-dab in all ten runs. That coarse allocation
measure cannot override the contradictory sampled process RSS. Chrome-tree
RSS sampled every 10 ms was noisy across fresh browser processes; pair 5
prevents a definite no-increase conclusion under the frozen rule. All ten
owned Chrome children exited 0, their local servers closed, and the independent
owned-process absence checks passed.

The native release non-test benchmark binary SHA-256 was
`e3db6bc2a207912316efea651ac59a8e6f249e34d5b1249726672ca45b3a0e61`.
The owned Chrome SIMD WASM/JS/index SHA-256 values were respectively
`b2e3f37f8b27969f1e703a2204edae185a6896b3a817f2140f6f1a2e77968c99`,
`beb16f93bb4d655913aecd88125096a0b999b4f7d3fd012fe76ddfd07be9bccb`, and
`4b3564c4c8b6167c55642d5d6cb5ef269c71a4dae049ef4fbdaa5d0e1be44d8f`.
The ignored full native table
`.artifacts/mrp04-stage/mrp05-native/valid-native-runs.json` has SHA-256
`f3b0d5c9bc71a4392277ce156c96f5e7d54efcad59b718bef1548eaf86853b0f`;
the full WASM table `.artifacts/mrp04-stage/mrp05-wasm/runs.json` has SHA-256
`af62597daac9ac51955dba5166c5c0ad620bbcdde27d2c0a3a5fbd05373c0885`.
Its ten individual `result.json` receipts retain import revision/count,
timings, memory samples, exact module hashes, CDP commands, errors and cleanup.
The ignored harness source SHA-256 was
`ff4db9ba0d58f05e6155a0e5a570125c03f8c736e191ac6cb7850556b07b9509`.

An initial opt-in release **test** baseline measured about 10 seconds per
seed, but `#[cfg(test)]` enables an `OrderedTree::check_limit` scan absent
from the production path. It is invalid performance evidence. Its original
log was accidentally overwritten by a parser retry; the same binary (SHA-256
`559ce54c07c30274ed857feb5c3663de94498ccd094711ddaf443152b92c5337`)
was rerun and retained as
`.artifacts/mrp04-stage/mrp05-native/invalid-test-only-baseline-reproduction.log`.
The initial 8.6-second value and reproduced 10.4-second value were excluded
from all tables. A first valid-run parser attempt also failed after writing
one log because a duplicate JSON `mode` key caused Python `TypeError`; the
complete five-pair cohort used a corrected parser and distinct log names.

Verification for the diagnostic implementation: `cargo fmt --all`, the full
locked workspace test suite, strict native workspace and feature-core Clippy,
strict feature WASM Clippy, and the official `scripts/build-web.sh` all exited
0. The feature-gated SIMD package was then built and identified by hash for
the owned-Chrome cohort. With the feature off, the benchmark export is absent.
