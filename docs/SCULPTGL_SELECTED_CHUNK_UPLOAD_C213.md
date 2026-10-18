# C213: selected-chunk browser upload

The ordinary million-vertex browser brush previously walked every cached
packed vertex row to find changed WebGL chunks after each checked Source dab.
The shared Rust normal state now records a bounded 2,048-row selected-chunk
envelope for a controlled dab and a weak reference to its exact predecessor.
The renderer uses the envelope only when the previous uploaded normal state
is that precise still-owned Arc. Because mask and RGB can change outside a
geometry dab without changing the normal-state Arc, it first checks those
attributes across the whole mesh and adds any changed chunks. It then compares
every row in the union byte-for-byte before a partial upload. Any missing/nonmatching lineage,
nonselected producer, topology change or cache miss keeps the old complete
scan/full-upload path. The witness does not modify geometry or the public file.

Focused core tests reject an unrelated same-shaped predecessor. An app-level
test applies a real controlled dab, checks that the hinted and full scans find
the same ranges, and reconstructs the exact packed browser vertex buffer. A
separate test changes RGB in a different chunk from the geometry witness and
requires that chunk to be uploaded too.
The independent complete browser run used the frozen 999,002-vertex,
1,998,000-triangle SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
under the 60-second, 400 requested moves/s protocol (SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`).
The final served portable WASM SHA-256 was
`92745352664286e0c183f09b6448bbf2b595930e4e2308673040e5f6932b7b12`.

| Observed normal-browser measure | Prior C210 run | C213 run |
| --- | ---: | ---: |
| Applied dabs | 869 | 888 |
| Certified selected-chunk / full-scan uploads | Not instrumented | 887 / 1 |
| Mesh-upload CPU mean per callback | 12.74 ms | 11.86 ms |
| Scene-admission CPU mean per callback | 7.85 ms | 7.80 ms |
| Queued Source dab CPU mean | 19.11 ms | 19.10 ms |
| Independent RAF P95 | 83.3 ms | 66.8 ms |
| App-frame CPU P95 | 71.5 ms | 70.1 ms |
| Mouse-release acknowledgment | 172.8 ms | 165.5 ms |

These are separate runs with different delivered/applied contacts. The
directly counted 887 witness uses establish that the new path is live; the
timings do not establish a causal end-to-end speed ratio. **All four frozen
million-vertex responsiveness gates still fail**: RAF P95 <=33.34 ms, RAF
maximum <=250 ms (observed 583.3 ms), app-frame CPU P95 <=16.667 ms, and
release acknowledgment <=100 ms. Scene admission and the queued brush remain
substantial costs. The normal studio still uses shared Rust/WASM geometry and
WebGL2; this work does not promote WebGPU or SculptGL/ZBrush parity.

The full result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-39497674-f8dc-41d0-9c4e-7468ada9dd49/result.json`
(SHA-256 `398c78a610c70f4de8aaca2608ebf2165c400609db9001a9199732e904372741`).
The independent public-file audit is `.artifacts/c213-audit-1m-attributes.json`
(SHA-256 `3207ce6b68055ba20d49351acd80108aeeaea178b1ab53a8c8d600922e4d237c`):
all checks pass, including unchanged nonposition channels and directed faces,
finite useful deformation, no new degenerate or reversed affected faces,
stable Save and exact Undo/Redo. It records 40,791 changed vertices and
82,404 changed faces. The Chrome owner exited cleanly with no browser or
console errors. The 317-ID SculptGL inventory and further paired quality
checks remain open.

Final verification passes: `cargo fmt --all -- --check`, `git diff --check`,
2,517 workspace tests with zero failures and one intentionally ignored test
across 103 groups, strict native and WASM Clippy, `scripts/build-web.sh`,
`scripts/package-macos.sh` and `codesign --verify`. The test log
`.artifacts/c213-workspace-tests-attributes.log` has SHA-256
`0a8d77dd6a5b66baaf46f3d5725c8023d209611f59ed41566d08a14910e8a0cf`;
the packaged executable SHA-256 is
`070e401c329a5bba2618bed19774525f66cebf635029ae9ad6580c09babc3ce1`.

The first completed C213 browser run used a build before a conservative weak
Arc-control-block budget charge. Its result and passing audit remain at
`SP-LONG00-1m-60s-root-d6a16013-6861-4190-b67b-189f5ec98d67` and
`.artifacts/c213-audit-1m.json`. A later budget-accounted trial at
`SP-LONG00-1m-60s-root-d8ad39a1-5e1c-47ed-ae8a-a4c304b4e22e` was
superseded after review found that unrelated mask/RGB edits needed a whole
attribute check; its faster observation is **not** the final result. Both
trials remain preserved. An in-progress workspace test suite was likewise
interrupted after that budget edit and retained as
`.artifacts/c213-interrupted-tests.log`; the complete budget-accounted but
pre-attribute-guard test log also remains at `.artifacts/c213-workspace-tests.log`.
The first attribute-guard test compile failed because its test buffer was
moved before a second assertion. The test was corrected and the complete
final-source run above passed. Earlier results are retained rather than
counted as final evidence.
