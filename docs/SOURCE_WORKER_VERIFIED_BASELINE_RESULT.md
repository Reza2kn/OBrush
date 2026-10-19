# Source Worker verified-baseline decision: no decoder candidate

The read-only split timing at commit `52b755a` executed the first stage of
[the preregistered trial](SOURCE_WORKER_VERIFIED_BASELINE_PREREG.md). The
frozen SP-WORKER-01 100k protocol SHA-256 was
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`;
the independent SGL fixture SHA-256 was
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The actual browser WASM SHA-256 was
`bda5b0f3579abe4303dc4cd46dce6bf88c9ee7b2006cf98ab2f6da64346e6a60`,
binding JS `073e05f499c7be0751bafddc4296ea7c675b47c1a72b15e5de09df8dcc746db4`,
and held-input JS `f804ad97824def3a35443f158b1d9a37e0dccc6f0067e1b62e35b8e4adac1bb3`.
The raw result SHA-256 is
`936a10a04c21fbd08c59906e3e4588bd9783a5e01fa745c207df4d5dca2b934c`.
All raw browser, shape, GPU, static-normal and failed-verifier receipts are
retained locally under
`.artifacts/source-worker-sustained-spw01/100k-worker-0c99e5aa-19a4-423d-8b8f-32948cbe5b0b/`.
The read-only split instrumentation is retained locally as
`.artifacts/source-worker-sustained-spw01/verified-baseline-split-timing.patch`
(SHA-256 `984b1f573c796231415055ac0a19388e7b6ed623d8cffcb6456241de5c9ed3b8`);
it was not merged into the product source.

The combined scene-and-delta stage was 281.3 ms, comprising **267.9 ms
complete-scene witness** and **13.4 ms delta decode**. The Rust final total
was 474.7 ms and the independent JS callback 477.2 ms, with 1,293
acknowledged dabs and 689.2 ms release-to-public commit. The split reveals
that delta decoding is below the preregistered 15 ms materiality threshold.
Consequently the proposed cached-baseline digest decoder was **not
implemented**, and no second browser run occurred. The next performance
target in this path is the exact complete-scene witness itself; skipping
packet baseline validation would at most address a small fraction of it.

The frozen verifier remains **incomplete_or_failed**: active RAF maximum
466.6 ms, no proven empty queue by 100 ms, and the public mesh-normal GPU
comparison failed in all five states. Pacing P99/max passed at 1.15/20.65 ms,
RAF P95 was 16.7 ms, app-frame CPU P95 10.8 ms and release acknowledgment
5.29 ms. Public Save passed exact Before/Undo and After/Stable/Redo identities;
shape passed with 4,012 changed vertices and zero new degeneracy or
orientation reversals; GPU positions matched all five public Save stages.
Static display normals matched GPU Before and Undo. Edited display normals
still lack an independent canonical oracle. The exact final endpoint was
applied, one public commit occurred, and browser errors were empty.

Focused and full workspace tests, strict native/WASM Clippy, `cargo fmt` and
a fresh web build passed. The split probe changes only timing fields and
call-site accounting. The held Worker remains query-gated, off by default,
and unpromoted; this diagnostic does not establish parity.
