# C202: reuse the release-time retained match

`Document::end_source_geometry_stroke` already checks its active mesh against
the retained Source state before preparing history and balancing the derived
tree. `balanced_state` repeated that million-vertex match later in the same
exclusive document call, with no mesh or retained-state write between the two
checks. The private continuation now retains the generation, domain and
budget checks but reuses the earlier exact mesh-state admission. An ordinary
warm release regression observes one full retained match, exact Undo/Redo and
rejection/rollback of a direct stale edit. Public dab and stroke entries still
perform their own checks on each call.

The frozen 60-second/400-requested-moves-per-second protocol SHA-256 is
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
The served C202 WASM SHA-256 is
`94919c81ee4e2c848611ec5c4c156e98d604142253466e51d3e77c18e72d6968`.

| Normal-browser observation | 100K C202 complete | 1M C200 complete baseline | 1M C202 diagnostic only |
| --- | ---: | ---: | ---: |
| Applied dabs | 1,145 | 830 | 860 |
| Mean queued dab | 2.33 ms | 20.59 ms | 19.71 ms |
| Mean retained preparation match | 0.90 ms | 7.48 ms | 7.24 ms |
| Independent RAF P95 / maximum | 22.3 / 69.4 ms | 76.4 / 645.8 ms | 72.6 ms / unqualified |
| App paint CPU P95 | 10.2 ms | 75.7 ms | 72.1 ms |
| Release acknowledgement | 15.2 ms | 146.9 ms | 144.8 ms |

The 100K browser run completed Save, stable Save and exact Undo/Redo; the
independent full-project audit passes all checks, including 99,602 vertices,
199,200 triangles, all non-position channels and no new degenerate or reversed
affected faces. Its frozen fixture SHA-256 is
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The raw browser result at
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-100k-60s-root-1fc290e4-f213-4440-9f3d-9e5f7c0bbb84/result.json`
has SHA-256 `1143e84c18e9ec3ce642e1ee3961de206445783de52e164650c24f90fd4aa733`;
its audit `.artifacts/c202-audit-100k.json` has SHA-256
`f0be219200ff37ba37f380562a92709ecec77b36e29f8b91a2a52bc94afa4344`.
The audit script now checks the frozen fixture hash and exact count table for
10K, 100K and 1M instead of hardcoding 1M; it also passes the prior complete
C200 1M result.

Two C202 million-vertex attempts failed the frozen file-adapter publication
window. The first failed before timing although Chrome later reported the
initial 351 MB download complete; its raw result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-53f3c700-491b-4061-b53b-1a81fdd2a942/result.json`
(SHA-256 `cd44bcf228ce2412175c7d2891d40e566875b3e98addb4058a9ff1692c3c2e68`).
The second completed the timed stroke and
scalar recording, then failed while polling the post-stroke download, which
Chrome also reported complete. Its read-only timing above is retained at
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-d141b9a5-21c1-42ed-878f-1d94aafdb236/result.json`
(SHA-256 `37150a634f733ea4468c2d747d79de86a8949620fb427a173c6c32981e04648d`).
That attempt lacks stable Save and Undo/Redo, so it cannot promote a 1M
whole-project gate. Its 2.1 ms lower release acknowledgement is within
cross-run noise and does not establish a speedup. The existing C200 complete
1M run still fails every responsiveness gate. SculptGL and ZBrush parity remain
open.

The subsequent [C203 publication-only companion run](SCULPTGL_FILE_PUBLICATION_C203.md)
completes a full 1M file/shape audit for this same WASM build using a longer
post-timing download wait; it still fails all four response gates.

The first workspace test run failed one old validation-probe assertion that
required the two identical retained matches being removed. Its log is retained
at `.artifacts/c202-workspace-tests.log` (SHA-256
`fcdb948db26af060bf4af8b75b9157e638a635a68a29919b3fc82a6a2befe53a`).
The assertion now requires one retained match while preserving its mesh and
domain counts; the targeted rerun passes. The final workspace run has 2,511
passing tests, zero failures and one ignored across 83 groups. Its log is
`.artifacts/c202-workspace-tests-final.log` (SHA-256
`524df47f7676e51e7e06b9578f6c21402397d5c5330da8803f9daa2a62316cca`).
`cargo fmt --all -- --check`, strict native and
`wasm32-unknown-unknown` Clippy, `scripts/build-web.sh` and
`scripts/package-macos.sh` pass. The signed macOS executable SHA-256 is
`e2c3c46e5652de823c5c6a6a9d5a36a441c144a7affab0386f20cb39f673e4d5`.
