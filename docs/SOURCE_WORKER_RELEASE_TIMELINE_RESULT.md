# Held Source Worker release timeline: 100k read-only result

The bounded read-only probe preregistered in
`SOURCE_WORKER_RELEASE_TIMELINE_PREREG.md` ran on current root product base
`510101a8283f814ab0e8319c49532f7ae054efdc` with diagnostic commit
`a3a831d0c47dcb90a8ce5f3fed94684653d7180f`. It changed only the
query-gated held-input controller's 32-entry scalar timeline and a 100 ms
timer. The original and instrumented controllers sent byte-identical Worker
messages with the same transfer-list lengths and endpoint order in a focused
nonpublication scenario. That test script SHA-256 was
`81b01f0e244949e93875a987ed5aa2bf0149c7d583f2b8ab30d974638b2b0ad9`;
the diagnostic patch SHA-256 was
`129b0c2eecdc19baa1d8d5c68a75bcc52b0381374d377ccd50643a15283dae85`.
An initial Node harness failed because its data URL could not resolve the
relative Worker URL; it was corrected to import test copies from files before
the successful nonpublication check. No product behavior was changed by
that test correction.

The frozen SP-WORKER-01 protocol SHA-256 was
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`;
the independent 99,602-vertex SGL fixture SHA-256 was
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The served portable WASM SHA-256 was
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`,
binding JS `03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`,
and diagnostic held-input JS
`831f4390291facb2ff4d1fbfdf957ea65a7671e9035294a468936151e19c5b21`.
Raw result SHA-256 was
`0424c256883651b79c08ae9f3cc1b5c5ce323bfefcab6d8131ff26a237e93857`;
all raw files and failed audits are retained at
`.artifacts/source-worker-sustained-spw01/100k-worker-588e764b-eb00-47df-89aa-2d2f0a3ef260/`.

## Same-page release and frame sequence

This run accepted 1,291 dabs, applied the exact release endpoint, and made
one public commit with no Worker, JS or GL errors. These timestamps all use
the **same page `performance.now()` clock**, relative to trusted pointer-up:

| Event | Time after release |
| --- | ---: |
| Endpoint apply posted | 0.0 ms |
| Endpoint reply received | 15.2 ms |
| Finish posted | 15.3 ms |
| 100 ms timer fired | 102.4 ms (2.5 ms late) |
| Finish reply received | 211.8 ms |
| Checked main commit entered | 211.9 ms |
| Checked main commit exited | 687.2 ms |
| Worker trial became inactive | 687.3 ms |

At the 100 ms timer, the Worker trial was **active**, its pending request was
`finish`, and its input queue held zero samples. Its release endpoint was
settled, but the frozen verifier defines queue-empty as `active=false` *and*
`pending=null` as well as zero queued samples. This run therefore disproves
the frozen 100 ms gate under the actual predicate; it is not merely an
unobserved queue. The independent CDP tail sampled at Node release +8.84 ms
and +60.46 ms and found the trial still active (pending apply, then finish).
The verifier's canned detail says a late first probe cannot prove the gate,
but the retained raw samples and page timer show the stronger result here.
Node and page clock origins were not subtracted to produce these timings.

The main checked commit occupied **475.3 ms** without yielding. The app RAF
callback timestamps show a **481.5 ms** gap from page time 68633.8 to
69115.3 ms, enclosing the complete commit interval 68639.6–69114.9 ms.
The frozen RAF timestamp metric was **466.6 ms**, exceeding its 250 ms
maximum. This same-clock overlap establishes that synchronous publication
accounts for this long frame; it does not attribute the individual Rust
substage costs in this probe. Earlier named-stage receipts put complete-scene
witness near 268 ms and retained decode at 149–171 ms, with the publisher's
delta re-encode near 16 ms. The measured 196.5 ms finish-post-to-reply
interval happened before the main commit and contributes to the release
tail, not the main RAF gap.

The unchanged whole-file shape audit passed: 4,012 vertices and 8,272 faces
changed, with no new changed-face degeneracy or orientation reversal. Public
Before/Undo Save SHA-256 was
`1100e81f00726ea03ad699f16356b6bddea8861b795262f51b0b56b8af3c7e46`;
After/Stable/Redo was
`c2c4ec65e1a7b83cab6b6adcaa782a8c1f812c344d4695f7922a6d3cdcdb1c40`.
GPU position words matched public Save in every state, and independently
reconstructed static Source display-normal words matched GPU Before and Undo.
The frozen verifier remains **incomplete_or_failed** for active RAF maximum,
queue empty by 100 ms, and its raw public-mesh-normal GPU comparison in all
five states. Its RAF P95 was 16.7 ms, app CPU P95 10.7 ms, release
acknowledgment 6.81 ms, and pacing P99/max 0.98/8.96 ms. Preserve the
saved-normal failure; Source display normals can differ from public mesh
normals, and this trial did not collect an independent edited-state oracle.

The timeline establishes that neither a narrow packet reduction nor a
15–50 ms main substage improvement can pass both frozen gates. Meeting the
current **100 ms completion predicate** requires a larger ownership and
publication redesign; meeting the **250 ms RAF maximum** requires splitting
or removing more than 225 ms of the measured synchronous commit. Any such
design must independently preserve complete-scene stale rejection, checked
retained packet and exact public Save/Undo/Redo before a performance claim.
The held Worker remains query-gated, off by default and unpromoted; this run
does not establish SculptGL or ZBrush parity.
