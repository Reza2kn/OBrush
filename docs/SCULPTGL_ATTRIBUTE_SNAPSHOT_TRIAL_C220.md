# C220: rejected selected-attribute snapshot trial

The million-vertex Source Brush still misses every local responsiveness gate.
One possible upload cost was the whole-mesh mask/RGB scan performed before a
certified selected-chunk geometry upload. This trial cached exact raw mask and
RGB words, including the mask convention, so a geometry-only dab could bypass
that scan. Changed or unproved attributes retained the existing exact row
check. The cache was bounded to 64 MiB, and snapshot allocation failure fell
back to the ordinary scan. A focused regression covered geometry-only edits,
out-of-chunk RGB edits, raw mask changes and convention changes.

The frozen 999,002-vertex / 1,998,000-triangle SGL sphere and 60-second,
400-requested-moves/s input protocol were unchanged (fixture SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`,
protocol SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`).
In a normal owned Chrome run, the trial took the shortcut on 788 selected
uploads, but mesh-upload CPU **rose** to 17.73 ms per callback versus 11.68 ms
in the immediately prior C219 final run. It applied 790 dabs versus 902, and
coalesced 53 versus 9. The browser runs delivered different contacts, so this
does not prove a precise causal throughput ratio. It does show that the
intended shortcut was active and failed to improve the measured workload.
Mouse-release acknowledgment was 127.5 ms versus 129.1 ms, still over the
100 ms gate. The trial was therefore **removed**.

The failed-performance trial completed its browser file/history path. Its
result is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-f9bc2f4a-e5c7-4ecc-8405-5e045b523016/result.json`
(SHA-256 `03a4bea71c261d9bd50f7bb0533bda6f8e05a06060f71952b9c27fde9a6c1288`).
The independent full-project audit at `.artifacts/c220-audit.json` (SHA-256
`50c3c46b7d0b0b2a29328250795593f77b24774d20cb2184532bfc33b9296cd1`)
passes unchanged metadata, faces and nonposition channels, finite useful
geometry, no new degenerate or reversed changed face, stable Save and exact
Undo/Redo. Chrome exited cleanly with no app/console errors. The attempted
`--help` invocation of the audit script was invalid because that script takes
only a result path; the explicit result-path run above passed.

The trial source passed 2,525 workspace tests with zero failures and one
intentional ignore across 103 result groups, strict native/WASM Clippy,
`scripts/build-web.sh`, `scripts/package-macos.sh` and macOS signature
verification. The test log is `.artifacts/c220-tests.log` (SHA-256
`e3654f9ba8e522e15c7bb1f17bddbbb95acfdb177bdf4ca0ac1b34958613e252`).
After removal, `cargo fmt --all -- --check` and `git diff --check` pass, and
both rebuilt artifacts are **byte-identical to C219**: portable WASM SHA-256
`e61740096172417fc678766c0be080595b3845dd519abea8ef54ddd83899ce09`,
macOS executable SHA-256
`ef11c95730f7e80efb22d2e2ef102c0259d57cc19dad0210652bf73b76a85e58`.
The next performance work needs to address repeated full validation and
deformation costs, or move the active geometry path to a separately verified
resident implementation. Another whole-mesh attribute copy/comparison does
not close the observed lag. No SculptGL or ZBrush parity status changes.
