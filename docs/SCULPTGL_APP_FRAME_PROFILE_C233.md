# C233: million-vertex app-frame profile

The normal browser's million-vertex Source stroke remains visibly behind the
pinned SculptGL responsiveness target. This change adds cumulative CPU timing
to the existing sculpt metrics for `eframe::App::update`: total time, the work
before `viewport`, and `viewport` itself. The scope excludes host paint,
browser RAF overhead and work outside the app update callback. It changes no
sculpt geometry, file format or history policy.

The final source passes `cargo fmt --all -- --check`, the full workspace test
suite, strict native and `wasm32-unknown-unknown` Clippy, `scripts/build-web.sh`,
and `scripts/package-macos.sh`. The package has a valid ad-hoc signature; its
executable SHA-256 is
`33e8680d44d282bf0daa1746aba2b98170d744e4624bb6836cabfd2fefce8986`.
The normal-browser factor smoke also passes on the instrumented WASM build.

The same frozen 999,002-vertex / 1,998,000-triangle SGL sphere and 60-second,
24,000-requested-move protocol were run once on the current C230 browser build
and once with the timing instrumentation. Both complete public-file/history/
shape audits pass: exact Save/Undo/Redo and unchanged non-position channels
and triangles, useful edits, no new affected-face degeneracy or orientation
reversal. The owned browsers closed cleanly. Distinct delivered dabs and
scheduling make these independent runs unsuitable for a speedup claim.

| Observed 1M/60s measure | Current C230 build | C233 timing build |
| --- | ---: | ---: |
| Applied dabs | 963 | 932 |
| Active RAF P95 | 66.7 ms | 66.8 ms |
| App frame CPU P95 | 65.0 ms | 68.6 ms |
| Release acknowledgment | 148.9 ms | 165.4 ms |
| Source geometry mean | 21.585 ms | 22.407 ms |
| Renderer callback mean | 16.942 ms | 17.702 ms |
| Preregistered responsiveness gates passed | 0/4 | 0/4 |

For the instrumented run, subtracting the pre-stroke metrics snapshot from
the last timed queue snapshot gives 951 app updates: 48,730.5 ms total
(51.24 ms/update), including 1,040.4 ms before `viewport` (1.09 ms/update)
and 47,670.4 ms inside `viewport` (50.13 ms/update). These are aggregate
wall-clock costs, not exclusive per-stage profiles: `viewport` includes
input, Source dabs and render snapshot preparation. The separately measured
renderer callback is outside this app-update timing. Source geometry alone
exceeds a 16.7 ms frame on average, so a small panel-layout optimization
cannot close the active RAF gate. Moving geometry off the browser main thread
is a candidate architecture, not a validated improvement; worker handoff and
buffer transfer costs still need a measured prototype.

The current result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-00d80220-bdec-4829-81c9-a917f7d0dd55/result.json`
(SHA-256 `995b81e5524e78e8da2a163bc0d4c2a50c784e2d4445ebdc5fe4a8fca24af115`);
its independent audit is `.artifacts/c233-current-browser-audit.json`
(SHA-256 `3229377a0ed495a68203f8a394b17bae2db6262be6027c1b78525331651a5fb9`).
The timing-build result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-1aa0a1d1-63c8-4ab6-be94-51ac99d3e27a/result.json`
(SHA-256 `4f6e1ac2df5159a4cfcaabd5bb7a0116afa2737f5b1a5d02d4733cf8c3b1feba`);
its independent audit is `.artifacts/c233-instrumented-audit.json`
(SHA-256 `ae718ff77b5f39d44b1f148bf8a618900fb5feb34a8bea83d3d40fe21d6e957a`).
The protocol SHA-256 is
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
The delivered WASM SHA-256 values are
`9742032c6abf9c56068093a08f7dd158e24292f2acbec33d8abe8ad9ea7e486e`
and `ac7b284dd33eab4c1202b19ddc91565327cdf2df62d7f7a6495a64b664394c39`,
respectively. Full summaries are in `.artifacts/c233-current-browser-summary.json`
and `.artifacts/c233-instrumented-summary.json`. All four responsiveness gates
and SculptGL/ZBrush parity remain open.
