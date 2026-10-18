# C158: shared immutable Source stroke witness

This follow-up keeps the C156/C157 independently authored 999,002-vertex, 1,998,000-triangle SGL fixture (SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`) and frozen 1M/60-second browser protocol (SHA-256 `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`). It requests 400 mouse moves per second at 64-pixel radius and 0.25 intensity with Source Ordinary Brush, no clay/accumulate/alpha/symmetry/fold prevention. The pinned SculptGL bundle remains `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`; its earlier distinct browser run had 21.5 ms RAF P95 and 2.11 ms mean dab.

An accepted controlled dab previously rebuilt a full mask/color/material/UV word witness while staging the stroke. Those inputs are already checked against the captured witness before staging and cannot change during its exclusive document borrow. The staged stroke now shares these immutable buffers through `Arc` and copies only normal words, which advance after a successful geometry update. Failed dabs retain the original stroke and mesh; accepted dabs retain exact input validation, project format, and Undo/Redo. A focused test verifies shared immutable allocation identity, separate mutable normal storage, and exact witness matching after a staged normal mutation.

| Pinned OBrush browser observation | C157 count shortcut | C158 shared witness |
| --- | ---: | ---: |
| WASM SHA-256 | `de056f1e40def579ffb76f734d9c4b9d2286bef5d38eafd7535b9694b4daf9df` | `2af30e489017f2706ef1893b89be18bd53ccdde2a0bc3f6e0bbf0d8c22ee40db` |
| Delivered moves / applied dabs | 732 / 675 | 746 / 685 |
| Mean queued Source dab | 36.710 ms | 35.316 ms |
| Mean deformation / preparation | 20.963 / 15.741 ms | 19.593 / 15.716 ms |
| RAF P95 | 91.0 ms | 89.7 ms |
| Release acknowledgement | 300.1 ms | 269.5 ms |
| Independent whole-project geometry/history audit | pass | pass |

The runs received different pointer-event sequences and changed different vertex sets. These observed timing differences are **not** a controlled causal speed ratio or matched SculptGL brush-response error. The C158 full 60-second result is `.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-1ba4800a-7bc1-410b-9ef8-ce0ecdde7b12/result.json`; the independent audit has zero failed checks, 746 delivered moves, 89.7 ms RAF P95 and 40,636 changed vertices. The ordinary browser studio still fails all four pinned responsiveness gates and still uses WASM CPU geometry with WebGL2 drawing. It does not reach SculptGL or ZBrush parity.

Validation: `cargo fmt --all -- --check`, `cargo test --workspace --all-targets` (2,479 passed, zero failed, one ignored across 101 groups), strict native and WASM Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`, `git diff --check`, and the browser plus independent output/history audit pass. The first formatting check reported one line-wrap difference; `cargo fmt --all` corrected it before the passing final check. Logs and browser files remain in ignored `.artifacts`. The measured dominant costs still include about 15.7 ms preparation (including 5.4 ms retained match and 10.3 ms contact preparation) and 7.1 ms state advancement per dab, plus rendering and release work. Further WebGPU work needs an integrated resident path and must retain exact shared-core document/undo behavior.
