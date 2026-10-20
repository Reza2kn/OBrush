# MRP08: exact face-bounds arithmetic in the full Source seed

MRP04 measured 154.1–222.7 ms in `OrderedTree::build` for each cold
million-vertex seed. MRP07's sparse press preparation was fast but did not
prove the full seed's spatially dependent rejection contract. MRP05/MRP06
Arc retention failed their frozen memory admissions. This is a new candidate
that preserves **both** complete seed calls and all tree allocation and
failure decisions: replace only the per-face bounds calculation inside
`OrderedTree::build` with an exactly equivalent arithmetic loop. The earlier
failed gates remain failed.

The current loop widens each finite Float32 corner to Float64, expands a
six-component Float64 box through generic `expand`, computes the center in
Float64 and narrows the six endpoints to Float32. Because every endpoint is
already an exact Float32 input coordinate, the candidate may compare and
retain the original Float32 endpoints directly, preserving the first value
on equal comparisons (including signed zero), then compute the center with
the **same** Float64 addition/multiplication and Float32 cast. It must keep
the same corner order, bounds-checking, finite rejection and error variant.
No cached state, extra face array, relaxed cap, alternate topology, altered
normal, history, document, GPU or host path is allowed.

Before editing the product function, retain a test-only copy of the old
formula as an independent oracle and freeze these checks:

1. Exact return/error and all nine Float32 output words for triangles and
   quads spanning positive/negative zero, subnormals, finite extremes,
   duplicate corners, invalid index, NaN and infinity. A fixed-seed 100,000
   row randomized valid-coordinate corpus must compare both formulas.
2. On the saved 351-MB public `root-before.obrush` with SHA-256
   `367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`,
   compare **all** 1,998,000 original face rows (or the stored original-face
   count if different), not a sample; record independent rolling hashes for
   legacy and candidate words and any first differing row.
3. Compare complete `SourceGeometryNormalState` public normals, ordered-tree
   snapshot, bounds, candidate/selected rows, rejection classes and a
   controlled sculpt/Undo/Redo result on independent small triangle, quad,
   degenerate, invalid and mirrored fixtures. For the huge public project,
   compare all face-bound words and the complete seed-state hash, with
   a bounded streaming hasher rather than allocating a duplicate JSON blob.
4. Time five alternating fresh-process native and five owned-Chrome WASM
   pairs of full `SourceGeometryNormalState::seed` on that same public file,
   using a diagnostic-only old-formula route in the same optimized binary.
   Record wall-clock seed and `OrderedTree::build` time, peak WASM linear
   memory and sampled Chrome-tree RSS, exact build/harness/file hashes,
   errors and closure. The candidate is admitted only if all exact checks
   pass, paired median tree-build saving is at least **20 ms** in both
   runtimes, total seed time does not regress in any pair by more than 5 ms,
   and deterministic WASM high-water pages are no higher. Chrome RSS is
   reported separately and cannot erase a contradictory memory signal.

After these gates, run the unchanged normal-browser 60-second million-vertex
held-pointer protocol with its **four** pre-existing responsiveness
thresholds and exact public file, geometry, GPU and Undo/Redo checks. A
microbenchmark or successful build never closes that gate or SculptGL parity.
Preserve every failed run and do not loosen a threshold after measurement.
