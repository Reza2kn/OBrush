# Source Worker scene-witness streaming candidate

The candidate preregistered in
[SOURCE_WORKER_SCENE_WITNESS_PREREG.md](SOURCE_WORKER_SCENE_WITNESS_PREREG.md)
is code commit `b300451`. The complete-scene witness now hashes the same
validated, size-limited public JSON serializer into a 64 KiB buffered writer
at final publication. It does not allocate a second 35 MB JSON string or
change public Save output. Exact digest equivalence for plain and rich scenes,
in-place stale rejection, atomic failure, checked delta decoding and one-undo
publication passed focused tests.

The frozen 100k normal-browser protocol SHA-256 was
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`;
the independent SGL fixture SHA-256 was
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The candidate WASM SHA-256 was
`7f4c97d0135ab053c793481ed0bfbbd9c30c9a9d4cfe3cfec40ef10b80741c61`,
binding JS `f139a57aa54d388c2b3542d121a8ebe28f3f9f9da80837bb2e0783e2c9a1fe71`,
and held-input JS `f804ad97824def3a35443f158b1d9a37e0dccc6f0067e1b62e35b8e4adac1bb3`.
The complete raw result SHA-256 is
`41c166f9c00fe3edcd782e94af5ca7fde1ac99ca75d1325e3c59397e754dd429`.
Raw browser, shape, GPU, static-normal and failed frozen-verifier receipts are
preserved locally under
`.artifacts/source-worker-sustained-spw01/100k-worker-3dd5ae82-52d8-48d0-a7c7-1d4514178899/`.

| Measured final stage | Prior read-only diagnostic | Streaming candidate |
| --- | ---: | ---: |
| Acknowledged dabs | 1,293 | 1,282 |
| Complete-scene witness plus delta decode | 373.6 ms | 283.7 ms |
| Retained warm decode | 171.0 ms | 173.8 ms |
| Publisher delta re-encode | 15.5 ms | 15.7 ms |
| Rust final publication total | 566.9 ms | 479.8 ms |
| JS final callback | 569.7 ms | 483.3 ms |
| Release to public commit | 784.8 ms | 698.3 ms |

The measured scene/delta stage fell 89.9 ms (24.1%) and cleared the
preregistered 10% single-run signal. The candidate and baseline delivered
different dabs and ran at different times; these two observations do not
establish a repeatable same-stroke speedup. The 64 KiB streaming allocation
is bounded independently of project size, and the existing maximum encoded
project limit remains enforced.

The frozen verifier remains **incomplete_or_failed**: active RAF maximum
483.3 ms, no proven empty Worker queue by 100 ms, and public mesh-normal GPU
comparison failed in all five states. Pacing P99/max passed at 1.40/70.88 ms,
active RAF P95 was 16.8 ms, app-frame CPU P95 11.2 ms and release
acknowledgment 5.99 ms. Passing pacing here does not erase earlier recorded
frozen failures.

The independent Save audit passed exact Before/Undo and After/Stable/Redo
bytes. Shape audit passed with 4,014 changed vertices, zero new changed-face
degeneracy and zero orientation reversals. GPU position f32 words matched
public Save in all five states; static Source display normals matched GPU
Before and Undo. The final endpoint was applied, one public commit occurred,
and browser/GL/Worker errors were empty. Edited display normals lack an
independent canonical oracle here; the raw public-mesh normal comparison is
still a failed frozen check. Full workspace tests, strict native/WASM Clippy,
`cargo fmt`, and a fresh web build passed. The held Worker remains query-gated,
off by default and unpromoted. This does not establish SculptGL or ZBrush
parity.
