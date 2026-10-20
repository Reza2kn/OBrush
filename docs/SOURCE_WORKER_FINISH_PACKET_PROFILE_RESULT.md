# Source Worker finish-phase attribution, 100k frozen cohort

Probe only. No packet codec or transport candidate was promoted. The frozen
SP-WORKER-01 input, fixture, runner and verifier were unchanged. Product base:
`de7c6f6230b86acdbc58e6126a3a1480f289a4c8`. Temporary diagnostic patch:
`.artifacts/source-worker-finish-phase-probe/phase-timer.patch`, SHA-256
`ac554e2ddcd46fe77112b1f008c204e144f2e0e38cb60c04d0e71e48ecec5295`.
Node nonpublication test SHA-256
`1a20dbf17c76a57f033deb4b07411954a9caf3c2b983ebef993a7b74331101a2`.
The test compared the original and probe Worker finish messages, bytes, transfer
buffers and ownership/take order. It passed. JS syntax, rustfmt and diff checks
passed. The first build invocation compiled but packaging failed because the
script expected a worktree-local `target/`; the ignored path was symlinked to
the shared cache and the unchanged build script then passed.

Served build SHA-256:

| File | SHA-256 |
| --- | --- |
| `web/dist/pkg/obrush_app_bg.wasm` | `f8a5f23f06df13087b2aa778bfd7698aca7d154b925c989f3ff3b7f2cbb9e798` |
| `web/dist/pkg/obrush_app.js` | `b408b6bddb463476ec881af92bd6f2f74843f7556ad381b1ab0d506a12871970` |
| `web/dist/source-geometry-worker.js` | `e23504f63399173456ffd8f8d2353c066e3738d68eeba2b59a17c57277f2e25e` |
| `scripts/run-source-worker-spw01.mjs` | `1a261ae48cb119a0ac1508d174ce7321dfcc70c8fdb584370a445b8b57414cee` |

Raw run: `.artifacts/source-worker-sustained-spw01/100k-worker-40b653b8-bc32-437d-b574-dc542b999d3f/`.
`result.json` SHA-256
`a9e3558f6dfbf427e1ae590c9f749f37eecc8e7ce0fc40472ab2f36064bb460c`.
The fixture SHA-256 was
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`,
protocol SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`.
There were 1,289 applied dabs, one commit, zero rejections and zero JS/GL
errors. The result's final normal Worker response transferred a 119,424-byte
delta and 16,222,744-byte retained packet, 16,342,168 bytes total.

| Phase | Duration, ms |
| --- | ---: |
| Main-to-main finish message interval | 194.3 |
| Worker queue wait | 0.0 |
| Rust `transaction.finish` | 8.8 |
| Rust checked delta encode | 18.9 |
| Rust retained warm encode, including checksum | **164.2** |
| JS `owner.finish` call | 192.6 |
| JS `take_delta` and `take_retained` | 0.1 + 1.4 |
| Worker receipt to just before `postMessage` | 194.1 |

The diagnostic emitted a best-effort Worker `console.debug` after postMessage
returned, but the frozen runner did not capture that Worker console call.
Thus postMessage call duration is **unobserved**. The independent main-to-main
and Worker pre-post intervals are close; subtraction is not a calibrated
per-stage transfer measurement. This run identifies retained encoding as the
dominant measured construction stage, not packet size or transfer as the
cause. The preregistered 40 ms construction-stage threshold is met, but no
same-packet replacement microbenchmark or net speed claim exists. The
separate main-thread held commit was about 688 ms from release in this run;
finish encoding alone cannot clear the browser frame/queue gates.

`shape-audit.json` exited 0: 4,013 changed vertices, no new degenerate or
reversed faces, maximum changed-edge stretch 2.222. `gpu-audit.json` exited 1
at all five saved-mesh-normal stages (the pre-existing Source display-normal
oracle mismatch). `frozen-verifier.json` exited 1: active RAF max 466.6 ms,
queue not empty by 100 ms, and GPU position+saved-normal all-states check.
Active RAF P95 was 16.8 ms, app CPU P95 10.7 ms, release acknowledgment 4.24
ms. Preserve these failed checks; this diagnostic is not a qualified cohort.

After recording this receipt, the source probe is reversed and the normal
browser build is restored. The restored WASM SHA-256 is
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`;
the restored Worker script SHA-256 is
`4b56bb3d1e1b0a594f0d288213ba856c11e1a95f8fb94b3ecb5dde73188caa60`.
The tracked worktree is clean. No temporary source change is promoted.
