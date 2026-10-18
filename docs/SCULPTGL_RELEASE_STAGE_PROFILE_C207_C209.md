# C207–C209: rejected release shortcuts and measured release stages

Two additional attempts reused the initial retained-state match later in a
Source release. C207 passed the targeted core checks and complete million-vertex
browser shape/file/history audit, but mouse-release acknowledgment was 144.2 ms
versus 144.3 ms in C204's separate run. C208 also passed those correctness
checks but acknowledged release in 149.0 ms. Neither establishes a useful
improvement; both shortcuts were removed. Their served WASM hashes were
`59a8ad02e20e147169d6659ebaebb5a4e0b1bcb264a6fff98c1f404a5123fb92`
and `3100869f3ba052f0c00b541f106e23a4d4396cb9918aa88adf7d92cc99da7f3b`.
The retained audit files are `.artifacts/c207-audit-1m.json` and
`.artifacts/c208-audit-1m.json`; both report no new degenerate or reversed
affected faces. These are separate runs with different delivered contacts, not
paired causal speed measurements.
Their raw results are
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-7ff77507-81e3-4039-8423-cc2aa95b4914/result.json`
(SHA-256 `5f03f025d09420f7411fbb776d43fd133daca0601d76b4041b930eb766056851`)
and `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-a76f1617-ad12-4641-9de2-f9cd7a026785/result.json`
(SHA-256 `3283d9a30936348c68dbfd119bd720f1f34c06e2eb668c7254424c802585f893`).

C209 adds monotonic diagnostics around the existing successful Source release
stages. It does not change the acceptance checks or geometry operation. The
normal browser ran the frozen independently authored 999,002-vertex,
1,998,000-triangle SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
for 60 seconds at 400 requested motion events/second under protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
Its served WASM SHA-256 is
`9c9255a13d47ea09b76f0f40f648eb09d92fe4bf79988f2449edcad7ea2bef06`.
The C209 runner changes only post-release observation, recording the app's
release diagnostic after the timed tail; its SHA-256 is
`e7e004ac71f9d6c365796b8979b952309354bebee7e304e54c2be838e44747dd`.

| One C209 release stage | CPU wall time |
| --- | ---: |
| Capture, binding and retained-state validation | 23.5 ms |
| Source history preparation and budget | 1.2 ms |
| Retained-state balancing | 10.8 ms |
| Generic stroke finalization, scene/history admission | 45.5 ms |
| Whole shared-core release call | 81.0 ms |
| Mouse-release acknowledgment, including other host work | 167.2 ms |

These are one-stroke observations, not a stage distribution or a controlled
speed comparison. The core stage totals are nested within the 81.0 ms call.
The 86.2 ms difference between that call and browser acknowledgment includes
other host work, event/queue scheduling and possible pending frame work; this
run does not attribute it further. The completed 1M trace applied 864 dabs
with 19.49 ms mean queued Source dab. The file/shape audit finds 40,809 changed
vertices and 82,440 affected faces, no new degenerate or reversed affected
faces, and exact stable Save and Undo/Redo files. Raw result:
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-f1b1ef7c-f4e7-4425-857a-e75d4d717f52/result.json`
(SHA-256 `cca1b52ac65ad22969998d7d970e690f93fa8b1974fcb9061d5d4d1dcff746de`).
Independent audit: `.artifacts/c209-audit-1m.json` (SHA-256
`8928a9d2c4d72c7d6761bea1bfd175ddbf82ed273640f1748626d9549d463801`).
No SculptGL or ZBrush parity status is promoted; the million-vertex
responsiveness gates remain open.

Final C209 checks pass: `cargo fmt --all -- --check`, 2,511 workspace tests
with zero failures and one ignored across 83 groups, strict native and WASM
Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, and the macOS app
signature check. The workspace test log SHA-256 is
`d4ffb91c3352d3163a112173767e6c3ddc60f26abbd503218c1f73b6ba5c2c7f`.
