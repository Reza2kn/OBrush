# Source Worker final-publication trial: scoped result

This is the result of [the preregistered one-pass candidate](SOURCE_WORKER_TAIL_PREREG.md)
at code commit `e3dfbc1`. The frozen SP-WORKER-01 protocol SHA-256 was
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`;
the independent 99,602-vertex / 199,200-triangle SGL fixture SHA-256 was
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The normal browser imported that file, ran Source Draw with 64-pixel radius,
0.25 intensity, no symmetry, and used a trusted held pointer for 60 seconds
at 400 requested CDP moves/s. The query-gated Worker trial was switched on;
it remains off by default and **unpromoted**.

The candidate's browser WASM SHA-256 was
`9b36ad0931a2e48669ffd5ea7d8f8308764da80f4dcc2855390b2cd819a99697`,
binding JS `f26f81325d9eae6c953cf449100ba8ac1d918320589a625f5c924996752f9562`,
and held-input JS `1efc7dabe5e02b2c0ae3d881589c64ffbb657cb8e75f12c839f5c9867df41ac4`.
The complete raw result SHA-256 is
`a2493643d6f51b9226a6f3a38c343307cfebe8fccd55c5878e53c1947164ac33`;
failed verifier, GPU, shape, static-normal and scalar receipts are retained
under `.artifacts/source-worker-sustained-spw01/100k-worker-2b4983d7-01ed-447f-be96-5b86b444e482/`.

The bridge now decodes and witnesses the final candidate once, comparing it
with checked sparse previews before retained-state authentication and the
one-undo publication. A focused test proves that comparison failure, stale
revision and malformed delta leave public scene/history unchanged. Full
workspace tests, strict native/WASM Clippy, `cargo fmt`, and a fresh browser
build passed.

| Measured stage | Retained prior run | One-pass candidate |
| --- | ---: | ---: |
| Acknowledged Worker dabs | 1,287 | 1,274 |
| Main final commit callback | 931.5 ms | 565.9 ms |
| Active RAF maximum | 916.6 ms | 550.1 ms |
| Worker finish round trip | 195.3 ms | 194.4 ms |
| Release to public commit | 1,143.7 ms | 776.8 ms |

The candidate's main commit callback was 365.6 ms (39.25%) shorter, but
delivered dab counts differ and this is one run per build, not an exact
same-stroke speed comparison. Active RAF P95 was 16.7 ms, app-frame CPU P95
11.5 ms, and release-event acknowledgment 5.84 ms. The frozen verifier
remains **incomplete_or_failed**: pacing P99/max 26.24/148.26 ms, active
RAF max 550.1 ms, no empty Worker queue by 100 ms, and all five public-mesh
normal GPU comparisons failed. The Worker was still finishing at tail probes
+8.3, +58.8, +108.4, +157.9 and +209.3 ms; its ~194 ms finish round trip
alone prevents the 100 ms queue gate. The pacing outlier was observed in the
external CDP sender while app RAF remained regular; it stays a recorded
frozen failure, not an app-speed pass.

Independent public Save audit passed exact Before/Undo and
After/Stable/Redo byte identities. Shape audit passed: 4,009 changed vertices,
no new changed-face degeneracy or orientation reversal, maximum affected
edge stretch 2.2193. GPU position f32 words matched the public Save at all
five stages with one active VBO, zero GL/JS/Worker errors and a checked final
endpoint. An independent static Source display-normal oracle matched GPU
Before and Undo, both with hash `02a0c38496e4147fc63cce3da0967a2c08975598ed1f25f38f83b6a6fc9f7461`.
Edited-state display normals are distinct from public mesh normals and lack
an independent canonical oracle here. Same-stroke GPU cancellation, the 1m
cohort and the full ordinary/Worker serial matrix remain open. Neither this
improvement nor the passing shape/history checks establish SculptGL or
ZBrush parity.
