# C201: rejected contiguous topology-match trial

An attempted retained-state matcher compared all original face rows in one
contiguous pass, then checked rendered triangles in a second pass. Exact
triangle/quad mutation tests and the full-project browser audit passed, but
the normal browser was slower. The code change was removed; the adversarial
mutation regression remains. This report preserves the failed measurement.

The frozen 999,002-vertex SGL fixture SHA-256 was
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`;
the 60-second/400-requested-moves-per-second protocol SHA-256 was
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
The rejected browser WASM SHA-256 was
`22bf7fe779ae2246149b5cee194e599960cd46f3da8e104dd855f2aca7ac955c`.

| Observation | C200 original matcher | C201 trial |
| --- | ---: | ---: |
| Applied dabs | 830 | 680 |
| Mean retained match | 7.48 ms | 16.18 ms |
| Mean complete queued dab | 20.59 ms | 28.73 ms |
| Independent RAF P95 | 76.4 ms | 93.2 ms |
| App paint CPU P95 | 75.7 ms | 92.4 ms |
| Release acknowledgement | 146.9 ms | 180.8 ms |

The raw failed-performance trial at
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-ea9a1866-f139-4eea-a79d-c1539d778247/result.json`
has SHA-256 `a1b5a6cecbd20ee4a5da2fd21f5d225880bee6aa87fee17768671801d3761be5`.
Its independent audit at `.artifacts/c201-audit.json` has SHA-256
`11c2a99c0c5eb8c9396d33870ef1c5af6d709dc244ea10350aee7ca720f14e8d`;
all public geometry, non-position channels, Save and exact Undo/Redo checks
pass, with no new degenerate or reversed affected faces. These are separate
browser runs with different applied contacts, but the directly instrumented
retained-match stage more than doubled. No responsiveness or parity gate was
promoted.
