# MRP06: resolve Chrome peak-memory uncertainty before an Arc-reuse candidate

MRP05 failed its strict five-pair Chrome RSS gate: the fifth reuse process
sampled 8,208,384 bytes above its baseline pair even though the exact seed
state, all WASM linear-memory high-water readings and the native memory gate
matched or improved. MRP05 remains failed. This separate experiment asks if
the ~8-MiB discrepancy is normal per-process browser variation or a
repeatable increase. It cannot retroactively change MRP05 or prove a product
speedup.

Use the same independently created 999,002-vertex, 1,998,000-triangle public
`root-before.obrush` project (SHA-256
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`),
the feature-gated **unmodified** MRP05 WASM benchmark hook, ordinary browser
file import, and one fresh owned normal Chrome process for each arm. Pin
actual WASM/JS/index and harness hashes before the first run. Run **16 matched
pairs**, alternating baseline/reuse launch order. Each arm performs one
same-state press seed, validation and first-dab action exactly as MRP05. A
process must close and pass the owned-child absence check before the next
starts. Retain every failure, timeout and sample; an incomplete cohort fails.

For each process, sample total RSS of the owned Chrome process tree every
10 ms using macOS `ps`. After the public project import has fully installed,
take 20 samples over 500 ms before the benchmark action. Let `pre` be their
median. The timed action's `peak_delta` is its maximum sampled process-tree
RSS minus `pre`. Preserve raw samples and process IDs. Also record the
actual WASM linear-memory byte length immediately before, after press, and
after dab, plus the complete retained-state fingerprint and
`SourceGeometryNormalState::approximate_bytes`. Do not use `performance.memory`
as the admission oracle; keep it only as a secondary observation. An
identical Chrome build, viewport, visible foreground state and no concurrent
owned browser runs are required. Keep source and output in ignored `.artifacts`.

The candidate **memory** admission requires all of the following, with no
fallback threshold changes after seeing results:

1. All 32 imports, state fingerprints, process closure and sample series pass;
   every fingerprint equals the MRP05 complete-state value
   `2952f295ecaee65a6e68dd9b5e807c87901fc845a5663c7104ffd8f93c667594`.
2. Reuse's WASM linear-memory maximum and exact retained-state bytes are no
   greater than baseline's in every pair. This is the deterministic allocation
   gate.
3. For 16 paired `reuse peak_delta − baseline peak_delta` values, the median
   is ≤0 bytes and the one-sided 95% bootstrap upper bound for that median is
   ≤8,388,608 bytes. Use 20,000 paired bootstrap resamples with a fixed
   `0x4d52503036` seeded xorshift64 generator; use the sorted 95th percentile
   (`ceil(0.95*N)-1` index). No individual positive delta may exceed
   16,777,216 bytes. This resolves only a **bounded 8-MiB material-increase
   question**, not a literal proof of zero browser RSS variation.

If any gate fails or the bootstrap bound is too wide, stop product work on
Arc reuse and report it. If they pass, a separate product candidate may be
built, but it must still pass the previously frozen exact success/no-op/
rejection/mirror/Undo/Redo/GPU-word checks and all four 60-second browser
responsiveness thresholds. Never substitute this microbenchmark for those
normal interaction gates.
