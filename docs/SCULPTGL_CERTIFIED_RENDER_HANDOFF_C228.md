# C228: checked Source dab to renderer snapshot handoff

A successful controlled Source-geometry dab now captures its selected render
mesh and retained normal state inside the same exclusive core operation that
updates them. The core-issued proof has a private constructor. The browser
uses that immutable `Arc<Mesh>` snapshot for its current active object, and
the renderer recognizes only that exact mesh/state pair. A different pair,
an ordinary edit, or a later paint dab still takes the complete stale-state
validation path. This removes a redundant validation of the just-published
pair; it does not weaken the public `SourceGeometryNormalState::matches_mesh`
validator or establish reference brush mathematics.

The shared Rust core test exercises nine geometry tools with mirror, alpha and
fold-guard variations, comparing exact state, output and history with the
existing split path. The app renderer test uses a real checked core dab: the
exact proof avoids a second match, while a modified mesh fails validation.
The queue discards a proof if paint or an ordinary dab follows in the same
drain, and the app refuses adoption after an error. On the final source,
`cargo fmt --all -- --check`, 2,531 workspace tests (zero failed, one
intentional ignore across 83 groups), strict native/WASM Clippy,
`scripts/build-web.sh` and `scripts/package-macos.sh` passed. The macOS
package's ad-hoc signature is valid.

The frozen 999,002-vertex / 1,998,000-triangle, 60-second normal-browser
workload (fixture SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`,
protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`)
completed twice with the new handoff. Its public-file/shape audits pass both
runs, including exact Save/Undo/Redo, unchanged non-position channels, no new
affected-face degenerates or reversals, and zero app/console/cleanup errors.
The final run captured 947 certified handoffs for 947 applied dabs. The
first run captured 967 for 967. The handoff counter is an app observation;
the renderer test establishes the exact-pair fast path and changed-pair
rejection.

| Observed 1M/60s measure | C225 baseline | C228 first | C228 final |
| --- | ---: | ---: | ---: |
| Applied dabs | 886 | 967 | 947 |
| Renderer admission mean | 7.984 ms | 4.146 ms | 4.120 ms |
| Renderer callback mean | 21.155 ms | 16.978 ms | 17.040 ms |
| Source geometry mean, including proof snapshot | 19.797 ms | 21.757 ms | 22.412 ms |
| Active-recording RAF P95 | 83.3 ms | 66.7 ms | 66.7 ms |
| Active-recording RAF maximum | 600.0 ms | 616.7 ms | 583.3 ms |
| App frame CPU P95 | 71.2 ms | 65.2 ms | 65.2 ms |
| Release acknowledgment | 161.2 ms | 127.3 ms | 164.8 ms |
| Preregistered active-recorder gates passed | 0/4 | 0/4 | 0/4 |

These are separate runs with different delivered dabs and scheduling. The
renderer-stage reduction is consistent with the active certificate path, but
the end-to-end differences are not a causal speed ratio. Release latency is
variable and is **not** claimed improved. The retained mesh/state match still
costs about 7.3 ms per dab, and normal Source geometry now includes the
snapshot copy. Pinned SculptGL's separate 1M/60s run recorded 16.8 ms RAF
P95 and 8.0 ms release. This implementation remains well short of its
responsiveness gates and does not promote SculptGL or ZBrush parity.

The final [browser result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-1b16f034-fd41-4c6b-acfb-eff03284c281/result.json)
has SHA-256 `ddda3c8284607dc6024159775ea273c470887cd839cf26f1f6ad5c77d02d1a7d`.
The independent audit `.artifacts/c228-final-browser-audit.json` has SHA-256
`16067539bcaa386a461cf46a1f575e3c7ccd3d46a665395d81691e656094ed61`.
The first run is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-d1011511-ed97-4b4e-9839-4aa6a9ecc5d2/result.json`
(SHA-256 `c4b2cbfaf1aedd76e8fc4eaf62a4b0f29b019eb4012f84fb8100235e6db2d11d`).
The final portable WASM SHA-256 is
`0bd610fb0b294e65b6114e1fb54db0e637d08c7f30a7ba031e3ea7d5c4c8bb8f`;
the packaged macOS executable SHA-256 is
`219bcc18c7042b199b56eb72e17ad7ac68da94ca70a465b748e12f622a01b8ea`.
