# MRP05: same-state press-seed reuse admission protocol

This is a bounded **microbenchmark and exact-state check**, not a product
speedup or SculptGL parity claim. MRP04 found one 294.8-ms press-alpha seed
and one 221.8-ms first controlled-dab seed on the independently created
999,002-vertex SGL fixture. The source of this test is the actual saved
`root-before.obrush` public project with SHA-256
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`.
The fixture and MRP04 trace/report hashes must be checked before timing.

For each of five fresh processes per arm, decode the same public project and
hold exactly one immutable mesh. In the **baseline** arm, construct an
`Arc<SourceGeometryNormalState>` for press, call the same-state
`matches_mesh`, drop it, then construct a second state for the first dab.
In the **reuse** arm, construct the press state, keep the same `Arc`, call
the same-state `matches_mesh`, and clone the Arc for the first dab. No mesh
mutation or document publication occurs in this microbenchmark. Require all
five runs to produce one identical complete seed-state fingerprint between
arms and inside each arm; fingerprint includes faces, position words, normals
and ordered-tree words. Report per-run first-step and dab-step elapsed time,
warmup status, `SourceGeometryNormalState::approximate_bytes`, and peak resident
bytes (`/usr/bin/time -l` on macOS). Run baseline/reuse in alternating order
to expose order effects, without changing the saved project or tool build.

The native admission criterion is a median first-dab net gain of at least
5 ms after same-state validation and Arc operations, with no increased peak
resident bytes or conservative concurrent state-payload estimate in any
matched pair. If RSS noise prevents a definite no-increase conclusion, mark
peak memory unresolved. The separate WASM benchmark must use the same file
and arms in owned normal Chrome, run only after the shared browser lane is
released, and meet the same ≥5-ms median net and no-larger-peak conditions.
Use read-only feature-gated hooks and extract a bounded result once after
each run, with actual loaded WASM hashes. A native result alone cannot admit
the product change.

Measurement implementation correction, frozen before the valid native cohort:
the first opt-in `#[cfg(test)]` release-test baseline triggered a test-only
`OrderedTree::check_limit` scan and took 8.6 seconds for its first dab. That
run is retained as an invalid measurement and cannot enter either cohort.
The valid native run uses a separate optimized `mrp05-seed-bench` binary with
the `mrp05-bench` feature, so the core follows its ordinary non-test path.
The file, two-arm logic, five fresh-process pairs, state fingerprint, memory
check and 5-ms admission threshold above are unchanged.

If both benchmarks pass, a separate candidate may retain only the press-built
Arc inside its unique stroke claim and consume it on the first accepted
controlled dab. It must check the same mesh and binding, account for peak
state/history storage, and leave no persistent state after a no-hit,
protected, canceled, stale or failed contact. Compare exact public files,
complete retained-state fingerprints, GPU position/display-normal words,
one-step Undo/Redo, mirror, alpha, no-op and rejection fixtures before a
frozen full 60-second browser performance cohort. All four responsiveness
gates remain active. A microbenchmark or one scoped pass never closes the
million-vertex gap by itself.
