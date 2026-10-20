# MRP09: ordered-tree cell capacity cost and admission feasibility

MRP04 measured 154.1–222.7 ms for a cold `OrderedTree::build` on the saved
million-vertex project. MRP08 proved exact face bounds but its five-pair
bounds-only median saving was 0.392 ms, far below its frozen 20-ms gate.
This is a **read-only, feature-gated** probe of the remaining cell allocator,
not permission to alter product memory accounting. MRP05/MRP06 and MRP08
failed results remain failed; MRP07 remains unadmitted for rejection parity.

`OrderedTree::allocate_cell` calls `try_reserve_exact(1)` whenever the cell
vector reaches capacity. Instrument only that branch to record the number of
calls, old/new capacities, total elapsed time around the actual reserve, and
total cold tree/seed time. No sleeps, trace serialization or per-face events
may occur inside the timed path. Run five fresh release-process measurements
on the exact 351-MB public `root-before.obrush` (SHA-256
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`,
999,002 vertices, 1,998,000 triangles). Retain the exact binary and source
hash, per-run result/error and peak native RSS. Compare five matched
uninstrumented runs from the same optimized source. A fixture checksum,
complete seed-state fingerprint and identical admission/error must be checked
outside the timed interval; no single run proves a speedup.

If reserve elapsed is under **20 ms median**, stop before a product edit:
eliminating this work cannot plausibly meet the pre-existing 20-ms
ordered-tree candidate gate. If it exceeds 20 ms, document the actual
capacity sequence and a proposed bounded growth rule **before** editing.
The rule must preserve exact successful/error outcomes and allocation caps:
the current cap charges actual `Vec` capacity and progressive leaf-vector
growth, so eager cell capacity can cause a new `StorageLimit` on meshes the
old route accepts. Any proposal must either prove identical admission for
all allowed counts and allocator outcomes or use the old exact-capacity path
when that proof is unavailable. Test successful, near-cap, rejected, empty,
degenerate, quad, and imported public meshes and compare full tree/normal
state, candidate row order, GPU words and Undo/Redo; preserve failures.

Only after that static proof may a separate product candidate run matched
five-pair native and owned-Chrome WASM full-seed timing/memory gates and the
unchanged normal-browser 60-second held-pointer test with all four frozen
responsiveness thresholds. Do not infer parity from an allocator counter,
microbenchmark or build.
