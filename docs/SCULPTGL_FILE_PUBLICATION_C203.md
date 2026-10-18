# C203: complete million-vertex file audit with extended publication wait

Two C202 million-vertex attempts finished 351 MB browser downloads after the
original adapter's 120 RAF polling attempts. A separately named C203 runner
changes only that post-recording publication wait to 1,800 RAF attempts. It
keeps the frozen fixture, 60-second/400-requested-moves-per-second input,
recorder caps, complete-file size/hash checks, stable Save, Undo/Redo and
owned-browser cleanup. The original failures remain in
[C202](SCULPTGL_RELEASE_MATCH_C202.md); this run is identified as a companion
protocol, not silently substituted for the original preregistration.

Runner SHA-256 hashes are
`e7450568f4aee3a491c9751eac1e20017ef0d92d041c67d7861dde5e6c15106b`
for `large-perf-sustained-c203.mjs` and
`d7f389249e4168bfbcda6e0d3d9bb447bf560e3e3a52cae44469358357279516`
for `large-perf-driver-c203.mjs`. The unchanged timed-motion protocol SHA-256
is `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`;
the independently authored 999,002-vertex/1,998,000-triangle SGL input
SHA-256 is `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`.
The served WASM SHA-256 is
`94919c81ee4e2c848611ec5c4c156e98d604142253466e51d3e77c18e72d6968`.

| Normal-browser observation | C200 original file wait | C203 extended file wait |
| --- | ---: | ---: |
| Applied dabs | 830 | 687 |
| Mean complete queued dab | 20.59 ms | 24.35 ms |
| Mean retained preparation match | 7.48 ms | 9.39 ms |
| Independent RAF P95 / maximum | 76.4 / 645.8 ms | 96.2 / 761.4 ms |
| App paint CPU P95 | 75.7 ms | 95.5 ms |
| Release acknowledgement | 146.9 ms | 202.0 ms |

All four million-vertex responsiveness gates still fail: RAF P95 ≤33.34 ms,
maximum ≤250 ms, paint CPU P95 ≤16.667 ms and release ≤100 ms. The runs have
different applied contacts and possible host-load differences; this table
does not establish a causal slowdown from the C202 release change. The C202
diagnostic run under the original wait recorded 860 dabs and 144.8 ms release
but had no stable Save or history check. A single release-time scan removal is
not sufficient to make the million-vertex studio responsive.

The complete C203 raw result at
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-7377aa14-9368-42fb-8ed2-df28d092bded/result.json`
has SHA-256 `16dbac7529cabe607e378bf548fc68ed9586697894ad1c7ca6055e964125a3f7`.
It records no application/console errors and a clean owned Chrome exit. The
independent audit at `.artifacts/c203-audit-1m.json` has SHA-256
`0a4f8f83d36cd26e52241f6a5590c998ea7d3058c235a4ba77497f841fe5fa10`.
It passes every public geometry, non-position channel, exact Save and
Undo/Redo check: 40,592 changed vertices, 82,004 affected faces, no new
degenerate or reversed affected faces, maximum changed-edge stretch 2.1665.
This closes the C202 build's missing complete 1M file/shape audit under the
declared publication-only protocol variant; SculptGL and ZBrush parity remain
open.
