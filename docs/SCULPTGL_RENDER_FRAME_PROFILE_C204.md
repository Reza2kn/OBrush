# C204: locate the million-vertex paint-frame CPU cost

The C203 complete-file protocol was rerun on the same independently authored
999,002-vertex / 1,998,000-triangle SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`).
Its timed 60-second / 400-requested-moves-per-second input protocol remains
SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
The C203 runner only extends the post-timing download-publication wait; its
hashes and deviation are recorded in [C203](SCULPTGL_FILE_PUBLICATION_C203.md).

This change adds monotonic CPU wall timers to the host's complete active-scene
snapshot and to the actual renderer callback. The renderer timer separates
scene admission (resource accounting and selected-normal certification) from
mesh upload preparation/calls. It does not change sculpt, validation, drawing,
input, or file behavior. The first, snapshot-only browser WASM is SHA-256
`eee89cd31fd7f3bd4c2546e7a542c3af9f7ecd916f0b661041e7484ec17739e5`;
the second, renderer-stage browser WASM is SHA-256
`9fb8e028cfeb2d7488701082fa5bfed14097f7bfbaf8b9667f89b5900925e765`.

| Measured 1M/60s stage | Snapshot-only run | Renderer-stage run |
| --- | ---: | ---: |
| Applied dabs | 863 | 838 |
| Complete queued Source dab CPU | 16.83 s, 19.51 ms mean | 17.04 s, 20.34 ms mean |
| Full scene snapshot CPU | 2.44 s, 2.82 ms mean, 865 samples | 2.50 s, 2.98 ms mean, 840 samples |
| Renderer scene admission CPU | Not instrumented | 10.08 s, 8.05 ms mean, 1,252 callbacks |
| Renderer mesh upload CPU | Not instrumented | 16.22 s, 12.95 ms mean, 1,252 callbacks |
| Entire renderer callback CPU | Not instrumented | 27.78 s, 22.19 ms mean, 1,252 callbacks |
| App paint-frame CPU total | 59.96 s | 59.95 s |
| Independent RAF P95 / maximum | Not promoted | 75.6 / 633.2 ms |
| App paint CPU P95 / release acknowledgment | Not promoted | 75.1 / 144.3 ms |

Renderer admission and upload are **nested in** the whole callback; all of
those stages and queued dabs are included in app-frame CPU. These totals must
not be added to infer an independent wall-clock budget. Separate browser runs
delivered different contacts, so the table does not establish a speed ratio
between builds. It does establish that copying the active snapshot takes only
about 2.5 of the approximately 60 measured paint-frame CPU seconds; changing
snapshot ownership alone would not close the responsiveness gap. Admission
rechecks immutable mesh topology/positions against its retained normal state
for each new render pair, while upload scans packed vertex rows for sparse
changes. The next optimization must preserve their exact stale-state and
attribute protections.

The snapshot-only raw result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-35c4daf0-eb42-4e85-85b8-bac3fe4ee1ac/result.json`
(SHA-256 `ab77fe4b45d6b4bb386f395b257d241dad2992713743751cc8e77d2ccea32b00`).
Its independent audit `.artifacts/c204-audit-1m.json` passes all complete
project, channel, shape, Save and exact Undo/Redo checks (SHA-256
`cf5bfebbe70c6e6d9986018da49484fb08fd13e873de3c3ca86a1fbd8c43d258`). The renderer-stage
raw result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-5fb230a1-4773-44b2-b897-4ed0a09e1a7a/result.json`
(SHA-256 `f0a9a6ddda45e644d88df802fdf6fdd97a5416b74a94973b73db5524200d663d`).
Its independent audit `.artifacts/c204-render-audit-1m.json` (SHA-256
`23cfdc1c2e35cdd03ec89870e770c419faeacf20d68ae22a13ce92cf56dc80bf`)
passes the same gates: 40,792 changed vertices, 82,404 affected faces, no new
degenerate or reversed affected faces. Both runs report no application/console
errors and clean owned Chrome exit.

`cargo fmt --all -- --check`, 2,511 workspace tests (zero failures, one
ignored across 83 groups), strict native and `wasm32-unknown-unknown` Clippy,
`scripts/build-web.sh`, and `scripts/package-macos.sh` pass. The macOS app's
signature verifies. The workspace test log is `.artifacts/c204-workspace-tests.log`
(SHA-256 `d1dd1665f9ceea75f49fccb5527346fbcb5c66242130065bfc7cc6848484ed88`).

All four local million-vertex responsiveness gates remain failed: independent
RAF P95 <=33.34 ms, RAF maximum <=250 ms, app CPU P95 <=16.667 ms and release
acknowledgment <=100 ms. No SculptGL or ZBrush parity status is promoted.
