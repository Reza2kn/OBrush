# C271 preregistration: local normal-witness publication

This isolated trial starts at `f6559c7`. The unchanged frozen C264 million-vertex ordinary Source Brush baseline used the 999,002-vertex, 1,998,000-triangle fixture SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe` and the 60-second, 400-requested-moves/s protocol SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`. It recorded 957 dabs, 66.7 ms active RAF P95, 68.0 ms app RAF CPU P95, 159.0 ms release acknowledgment, 22.943 ms mean Source geometry and 0/4 responsiveness gates. C268 paged face bounds were rejected after worsening browser responsiveness. These are separate observational runs, not paired timings.

`SourceGeometryStroke::apply_access_math` currently copies every public mesh normal into the stroke's `DomainWords` witness after each successful local normal refresh. `Mesh::recompute_normals_near_vertices_cached` already knows the exact dirty vertices plus all corners of their incident triangles. This trial returns that exact affected-vertex list and updates only those normal-witness rows. Complete stale-state checking at each public dab entry remains unchanged. Fallbacks that recompute all normals must refresh every witness row. Signed-zero and all normal f32 words, rejection rollback, import boundaries, undo/redo, and native/WASM behavior must remain exact. No format or render-path change is intended.

Before browser admission, compare baseline and candidate public `.obrush` before/after/stable/Undo/Redo files byte-for-byte for a deterministic sequence including repeated local dabs, changed-face neighbors, a zero-change dab and rejection. Check that adversarial normal edits between dabs still fail with the same error. Run focused tests, formatting, workspace tests, strict native/WASM Clippy and `scripts/build-web.sh` on the candidate. A local microbenchmark should report complete normal-witness copy versus selected-row update for the million-vertex scale, including affected-row count; it is only a substage diagnostic. Reject if any bitwise or stale-state oracle fails.

If the focused oracle passes, use a fresh owned Chrome browser and the frozen fixture/protocol hashes above. Preserve raw receipt and independent Save/Undo/Redo shape audit, collect app/console errors, capture actual WebGL position and normal row words against the correct retained Source-normal oracle, and record clean closure. Report release acknowledgment, active RAF interval P95/max, app RAF CPU P95, dabs/coalescing, Source geometry and publisher phase mean, renderer timings, and all four existing responsiveness gates. Different delivered contacts prohibit causal speedup claims. Do not promote merely on a microbenchmark; reject this candidate if the published witness stage does not improve or whole-brush responsiveness materially worsens. Even a successful narrow optimization leaves SculptGL parity open unless frozen gates pass.

## Focused result, not browser admission

The new exact-witness-row/full-rebuild unit oracle passed. The full `obrush-core` test suite passed, including the existing adversarial public normal-edit stale checks. The initial formatting check failed only on a line wrap in the new test; `cargo fmt --all` repaired that failure. No browser result or end-to-end latency result exists yet.

The ignored `million_normal_witness_copy_vs_local_rows` diagnostic used 999,002 rows, 100 rounds, and scattered affected IDs with a 977-row stride in a debug build. Complete normal-witness copy took 518.614/500.875/501.324 ms for the three cases; local updates took 1.558/10.935/48.669 ms for 1,000/10,000/40,000 rows respectively. This deliberately measures only the witness write, and its rows are synthetic rather than the actual selected Source one-ring. The snapshot comparison and browser responsiveness gates remain open.

## Frozen million-vertex browser result: rejected

The candidate was rebased onto `590effe` and built with `scripts/build-web.sh` after formatting and strict native workspace/all-target and WASM workspace Clippy passed. The candidate portable WASM SHA-256 was `64f6f5df49f406bcf000de64bc2258b88c7f9806f8b6b420c3f85fba393beeb8`; JS SHA-256 was `a41508543d9192409dceaf43d79ba4c8917f75074579892c00df8e988672b08c`. The fresh owned Chrome was `155.0.8059.80`. The exact frozen 1m fixture and protocol retained SHA-256 values `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe` and `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`. The harness changed only product binding and output scope; its SHA-256 was `eb34c87c10c70faf676d39b3f95056b6b28a4a0f9fec05580082d3eb4d76b214`.

| Active minute | C264 ordinary baseline | C271 candidate |
| --- | ---: | ---: |
| Applied dabs | 957 | 977 |
| Normal-witness publish mean | 0.647 ms | 0.014 ms |
| Whole Source geometry mean | 22.943 ms | 22.507 ms |
| Retained match / state advance mean | 6.843 / 7.116 ms | 6.790 / 7.008 ms |
| Renderer callback / mesh-upload mean | 15.879 / 10.328 ms | 15.365 / 10.012 ms |
| Active RAF interval P95 / maximum | 66.7 / 666.6 ms | 66.7 / 633.4 ms |
| App RAF callback CPU P95 | 68.0 ms | 65.9 ms |
| Release acknowledgment | 159.0 ms | 151.0 ms |
| Preregistered responsiveness gates | 0/4 | **0/4** |

The focused publisher stage improved, but all four interaction gates still fail. Delivered dab counts differed, so these browser totals are observational and do not establish a causal end-to-end improvement. The candidate is **rejected for product promotion**. The tracked source files were restored to the `590effe` base; the rejected source patch is preserved in ignored `.artifacts/c271-bench/c271-rejected-source.patch` (SHA-256 `ec6404cef8b03afafd15786375281e29f6641400efbc7b047060bcefc4d50a0a`). Do not infer that the ordinary million-vertex brush is responsive or at SculptGL parity.

The ignored raw browser result is `.artifacts/c271-bench/bench/actual/SP-LONG00-1m-60s-root-ffb6aa35-b70a-43ed-83d4-2186c01440a5/result.json` (SHA-256 `9c316474c17d936accd7dcd1ccd40f0ecd6eb63ea465d488727794630935fe80`); scalar recording SHA-256 `cdea193f1aa04e7e78595a310f2c1a553645f11a671d7323da9df208505b094d`. The independent `scripts/audit-source-sustained.py` audit passed all 16 checks, including exact public Before/Undo `367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36` and After/Stable/Redo `fe726f4eec2f813ab01c01d0bf8a8418cb4ee0e7e0304b04c1269441ea1111e2` identities, unchanged faces/non-position channels, finite words, useful edit, no new affected-face degeneracy and no orientation reversal. Its output SHA-256 is `fd3a0f72ca8cf398209a7b844dd410369db8888145f56476e404bd545a2fd0a9`; 40,759 vertices and 82,339 faces changed. The 24,000 motion requests completed, memory sampling passed, there were no app/console/cleanup errors, and owned Chrome exited 0.

This frozen run did not capture actual GPU buffer words. The existing public-normal GPU auditor is invalid for Source mode because the renderer packs retained Source derivative normals; a focused readback would still need that correct oracle. With all responsiveness gates failed and only 3 GiB free after retaining the complete 1.7 GiB browser receipt, no second GPU run was undertaken. Displayed-output equivalence remains unproven, another independent reason not to admit this candidate.
