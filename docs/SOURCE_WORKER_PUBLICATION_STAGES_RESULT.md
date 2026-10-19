# Source Worker final-publication stage result

The read-only diagnostic preregistered in
[SOURCE_WORKER_PUBLICATION_STAGES_PREREG.md](SOURCE_WORKER_PUBLICATION_STAGES_PREREG.md)
is commit `a51d6a7`. It ran the unchanged SP-WORKER-01 100k protocol
(`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`)
and independent SGL fixture
(`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`)
in an owned normal browser. The actual browser WASM SHA-256 was
`352c3dbe88b2fbf2e472a2b406406ee2f4883e2ef4ff8b312532d06c51856b43`,
binding JS `177e56b1bdb5df7e966ac14e78cf8953bfbd390cd24949d79563b552d0cc7986`,
and held-input JS `f804ad97824def3a35443f158b1d9a37e0dccc6f0067e1b62e35b8e4adac1bb3`.
The complete raw result SHA-256 is
`42fb53ec181115b220699a6fe6d3bcef23616529f21055a7372f81d5fe7cce99`.
Raw receipts and failed checks are retained locally under
`.artifacts/source-worker-sustained-spw01/100k-worker-c9736eb2-ec12-4e9b-8b57-f6d9b781a2ce/`.
The read-only instrumentation is retained separately from the normal product
build as `.artifacts/source-worker-sustained-spw01/publication-stage-instrumentation.patch`
(SHA-256 `025cfcaa2e55a3912543943f99e146993892ece52d626954cab091945da2302f`).

| Main-thread final commit stage | Measured time |
| --- | ---: |
| Complete-scene witness and candidate delta decode | 373.6 ms |
| Checked sparse-preview comparison | 2.0 ms |
| Retained warm packet decode | 171.0 ms |
| Candidate delta re-encode inside publisher | 15.5 ms |
| Retained mesh match and state budget | 0.4 ms |
| Scene and output checks | 3.8 ms |
| History snapshot and budget | 0.4 ms |
| Changed scan and adoption | 0.1 ms |
| **Instrumented total** | **566.9 ms** |

The independent JS wrapper measured 569.7 ms for the final callback, a
2.8 ms gap beyond the named Rust stages. This is one 1,293-dab stroke, not a
paired same-stroke speed comparison. The original hypothesis was wrong:
`publish_cold_source_worker_result` itself accounted for about 20.2 ms, so
removing its 15.5 ms delta re-encode cannot resolve the 566.9 ms callback.
The dominant work is the full-scene witness/delta decode and retained warm
decode. No publication algorithm was changed or promoted on this evidence.

The frozen verifier is still **incomplete_or_failed**. Active RAF maximum was
566.7 ms, the queue was not proven empty by 100 ms, and the raw public-mesh
normal GPU oracle failed at Before, After, Stable, Undo and Redo. The Worker
finish round trip alone was about 198.1 ms for a 16,341,088-byte final
transfer. Pacing P99/max passed at 0.64/6.12 ms in this run, as did RAF P95
16.7 ms, app-frame CPU P95 10.8 ms, release acknowledgment 5.82 ms, and
endpoint application. Passing pacing here does not erase the predecessor's
frozen pacing failure.

The independent public Save audit passed exact Before/Undo and
After/Stable/Redo bytes. The shape audit passed with 4,007 changed vertices,
zero new changed-face degeneracy and zero orientation reversals. GPU position
f32 words matched public Save in all five states; the static Source display
normal oracle matched GPU Before and Undo exactly. Edited display normals
remain without an independent canonical oracle, and the raw public-normal
comparison remains a recorded failure. Focused core/app tests, the full
workspace suite, strict native/WASM Clippy, `cargo fmt`, and a fresh browser
build passed. The query-gated Worker remains off by default and unpromoted;
this result does not establish SculptGL or ZBrush parity.
