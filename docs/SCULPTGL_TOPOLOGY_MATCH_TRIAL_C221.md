# C221: rejected contiguous topology-match trial

The retained Source geometry match still costs about 7 ms per applied dab on
the frozen million-vertex sphere. A trial replaced its face walk with contiguous
face-array equality and a direct triangle comparison. A focused equivalence
check and the full workspace suite passed, as did strict native/WASM Clippy,
the browser build and macOS package.

The frozen input was the 999,002-vertex / 1,998,000-triangle SGL sphere
(SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
and the 60-second/400-requested-moves-per-second protocol (SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`).
The normal Chrome trial applied 705 dabs versus 902 in C219. Directly measured
retained-match time was 16.48 ms versus 7.22 ms in the separate C219 run;
scene admission was 14.15 ms versus 7.80 ms, queued dab work 28.44 ms versus
19.35 ms, and release acknowledgment 158.0 ms versus 129.1 ms. Different
delivered contacts prevent an exact throughput ratio, but the intended match
path was active and clearly failed to improve this workload.

The [browser result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-e3f1c767-71cd-4c9e-8d0c-61fc747e235b/result.json)
has SHA-256 `80e95701332199a0ad4ec385d3626ab917390957b69f609127b89d959765b2ac`.
The independent [whole-project audit](../.artifacts/c221-audit.json) (SHA-256
`00d6fe17479b1153af09b29d6fba95ff5f71772490e0377a3c2f39fae4e916f4`)
passed exact Save, Undo/Redo, metadata, channels and face checks with no new
folds. The trial was removed. Rebuilt portable WASM and macOS executable
match C219 byte-for-byte at SHA-256
`e61740096172417fc678766c0be080595b3845dd519abea8ef54ddd83899ce09`
and `ef11c95730f7e80efb22d2e2ef102c0259d57cc19dad0210652bf73b76a85e58`.
No responsiveness or parity gate changed.
