# MRP06: normalized browser memory remains inconclusive

**Verdict:** the separately preregistered 16-pair memory gate failed. All 32
fresh owned-Chrome imports, complete seed fingerprints, process closures and
sample series passed. The paired median of pre-action-normalized Chrome-tree
peak RSS was −8,192 B, and the fixed 20,000-resample one-sided 95% bootstrap
upper bound was +57,344 B. However two positive pairs exceeded the frozen
16,777,216-B individual cap; the largest was +115,220,480 B. MRP05 remains
failed, and no Arc-reuse product edit is admitted from these microbenchmarks.

The [frozen protocol](SCULPTGL_MILLION_MEMORY_MRP06_PREREG.md) SHA-256 was
`93da9986d2af95b25b7d1c94faaef45d23a1f831af2e2084542520f5762bd094`.
The exact independent public project SHA-256 was
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`,
with 999,002 vertices and 1,998,000 triangles. All 32 runs returned complete
seed fingerprint
`2952f295ecaee65a6e68dd9b5e807c87901fc845a5663c7104ffd8f93c667594`.
In every pair the retained-state accounting was 203,651,892 B for both arms
and the WASM linear-memory byte length was 1,502,150,656 B before, after
press and after dab. The median first-dab microbenchmark difference was
213.6 ms. This is not a full browser frame, input or GPU result.

| Pair | Reuse peak delta minus baseline peak delta |
| ---: | ---: |
| 1 | −622,592 B |
| 2 | 0 B |
| 3 | +163,840 B |
| 4 | −73,728 B |
| 5 | +16,384 B |
| 6 | −32,768 B |
| 7 | −134,504,448 B |
| 8 | +57,344 B |
| 9 | +57,344 B |
| 10 | **+115,220,480 B** |
| 11 | −114,688 B |
| 12 | −100,769,792 B |
| 13 | −16,384 B |
| 14 | +425,984 B |
| 15 | −44,949,504 B |
| 16 | **+79,372,288 B** |

Pairs 7, 10, 12, 15 and 16 include tens or hundreds of megabytes of
process-tree RSS movement during the roughly 0.3-second seed action, despite
constant WASM heap length and retained-state bytes. In pair 10's reuse arm,
the pre-action samples ranged from 3,287,121,920 to 3,323,953,152 B and
action samples from 3,071,000,576 to 3,406,348,288 B. The sampled process
ID set stayed constant. This implicates broader Chrome memory activity or
resident accounting variation, but the data do not isolate its cause; the
pre-registered per-pair cap therefore correctly fails. No retrospective
filtering or threshold change was applied.

The ignored full table `.artifacts/mrp04-stage/mrp06-memory/runs.json` SHA-256
is `5c7eb37f795116f0bbd86e6ad1e8a99a84c734d67aad2c4f7cd3e1830f40a91e`.
Each of its 32 linked `result.json` files retains all 20 pre-action samples,
156–197 action samples, process IDs, import revision/count, WASM heap lengths,
timing, complete state hash, errors, CDP trace and closure evidence. The
ignored harness source SHA-256 is
`23501061cf1ea3e14448c840c3b24781bcb8f61b4f9227c34acd6fca6d621dfd`.
The actual SIMD WASM, JS and index SHA-256 values were respectively
`b2e3f37f8b27969f1e703a2204edae185a6896b3a817f2140f6f1a2e77968c99`,
`beb16f93bb4d655913aecd88125096a0b999b4f7d3fd012fe76ddfd07be9bccb`, and
`4b3564c4c8b6167c55642d5d6cb5ef269c71a4dae049ef4fbdaa5d0e1be44d8f`.
All runs were visible at one viewport, and no console, CDP, external-request,
sampler or closure errors were recorded.
