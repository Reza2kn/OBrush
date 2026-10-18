# C218: checked sparse viewport bounds updates

The normal WebGL2 renderer previously rescanned all active vertex positions
after every changed mesh upload to find the center used for translucent draw
ordering. It now reuses the exact prior bounds when the selected-refresh upload
provides checked changed-row ranges. The prior packed GL rows identify whether
any changed range contained an old minimum or maximum coordinate. If so, the
renderer uses its original full vector bounds scan, because that extremum may
have moved inward. Otherwise it expands the old bounds from just the changed
ranges. Missing cache, changed vertex count, a full upload or any rejected
proof also uses the full scan. The bounds and diagnostic counters publish only
after the upload succeeds. This does not change the shared Rust sculpt mesh or
document history.

The independent focused regression moves an interior vertex beyond all three
old maxima, then moves an old X minimum inward. Both paths match a fresh
whole-mesh bounds scan exactly; the second must fall back. On the frozen
999,002-vertex, 1,998,000-triangle SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`),
the final-source 60-second/400-requested-moves-per-second normal Chrome run
used 243 incremental and 660 full bounds updates during 901 applied dabs.
Selected-chunk uploads were used 900 times. The input protocol SHA-256 is
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
The served portable WASM SHA-256 is
`28f33a3dbbe8c778a9d8661a8688270e84b1aa7ccf40e641c1390d46912f0a36`.

| Final-source 1M/60s observation | Value | Local gate |
| --- | ---: | ---: |
| Independent RAF interval P95 | 66.8 ms | ≤33.34 ms |
| Independent RAF interval maximum | 583.4 ms | ≤250 ms |
| App frame CPU P95 | 69.4 ms | ≤16.667 ms |
| Mouse-release acknowledgment | 117.6 ms | ≤100 ms |
| Renderer mesh-upload CPU mean | 11.71 ms | No isolated speed gate |
| Renderer scene-admission CPU mean | 7.81 ms | No isolated speed gate |

All four responsiveness gates still fail. The prior C213 run applied 888
dabs and measured 66.8 ms RAF P95, 70.1 ms app CPU P95, 165.5 ms release
and 11.86 ms upload mean. The final run applied 901 dabs on a different
deformed state. These separate runs establish neither a causal speedup nor a
release-latency improvement; the direct counters establish only that 243
full bounds scans were avoided in the final run. The first C218 version did
extra per-coordinate work even on its fallback path: its 878-dab run measured
83.3 ms RAF P95, 145.4 ms release and 13.00 ms upload mean. That failed
performance observation is retained in `.artifacts/c218-benchmark.log`; the
fallback was changed back to the original vector fold. A later 909-dab run
measured 66.8 ms RAF P95 and 136.0 ms release before the final publication
ordering fix. It remains in `.artifacts/c218-benchmark-final2.log`.

The final browser receipt is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-df68a870-1dca-4f7d-b99a-dbde268a7cfc/result.json`
(SHA-256 `9b7361eda068707c39b7539e798e9d7f5400bf27c2a7c774bbd7aa8ba3451b85`).
Its independent public-file audit is `.artifacts/c218-audit-source.json`
(SHA-256 `f503df6aea11475c3649ef96d22d9dae9769a0c43aaadc4c46050abee9bb8c13`):
all checks pass, including exact stable Save and Undo/Redo, unchanged scene
metadata, faces and non-position channels, finite geometry and no new
degenerate or reversed affected face. It reports 40,795 changed vertices and
82,410 changed faces. Chrome exited cleanly with no app/console error.

Final-source checks pass `cargo fmt --all -- --check`, `git diff --check`,
2,523 workspace tests with zero failures and one intentional ignore across
103 result groups, strict native/WASM Clippy, `scripts/build-web.sh`,
`scripts/package-macos.sh` and macOS signature verification. The test log
SHA-256 is `51e8f93704ff096b2d01bcfa286c14c8830f974f3d90a240fd2e42ba7dea8ba2`;
the packaged executable SHA-256 is
`556daf229dfcb7a901eb73f8fbca8777e9a576ff7d19a4d233787e80f262a589`.
The first strict Clippy attempt failed on a test-only range literal and remains
at `.artifacts/c218-clippy-native.log`; it was corrected before the passing
checks. This renderer-local optimization does not establish SculptGL or ZBrush
parity. The bottlenecks still include per-dab shared Rust geometry, scene
admission and WebGL upload work.
