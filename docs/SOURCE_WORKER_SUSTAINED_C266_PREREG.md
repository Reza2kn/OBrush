# C266: sparse Source Worker sustained held-pointer admission

Status: **preregistered; no run or parity claim**. This freezes the measurement
before a browser build or trial. The product change is the opt-in
`?source-worker-held` route at source commit `866defb`. The default studio is
untouched. The independent pinned SculptGL reference is commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; source protocol SHA-256 is
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
The [machine-readable SP-WORKER-01 protocol](SCULPTGL_WORKER_SUSTAINED_PROTOCOL.json)
is the acceptance authority; this document does not change its thresholds.

Run on one host with one owned Chrome and build at a time. For each cohort run
fresh browsers serially in ordinary/worker/worker/ordinary order. Verify the
independently authored SGL3 fixtures before importing through the normal UI:

| Cohort | Vertices / triangles | Exact SGL SHA-256 |
| --- | ---: | --- |
| 100k | 99,602 / 199,200 | `8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093` |
| 1m | 999,002 / 1,998,000 | `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe` |

Use the ordinary Draw/Source Brush UI, 64 CSS px, intensity 0.25, no clay,
accumulate, symmetry, culling or pressure factors. Reset to orthographic,
frame the whole scene, and record actual camera/viewport matrices. Start
one trusted held pointer at center + (80, 0), send 24,000 timestamped CDP
mouse moves at 400/s around the two-second 80×60 px ellipse over 60 seconds,
then issue a real release. No direct Worker API or hand-authored WASM dab may
replace the pointer path. Preserve every requested send, acknowledgment,
failure, delivered trusted DOM event, worker-accepted dab, coalesced move and
released queue tail. The worker switch itself may be activated only by its
rendered trial button before timing. Record worker init/input/preview/final
bytes, transfer versus clone, main serialize/validate/publish CPU and Worker
geometry CPU; any missing stage fails qualification.

The interaction thresholds are RAF P95 ≤33.34 ms and max ≤250 ms, app CPU P95
≤16.667 ms, release acknowledgment ≤100 ms, queue empty by 100 ms, pacing P99
lateness ≤25 ms and max ≤100 ms. File and shape checks require normal public
Save before/after/stable/Undo/Redo, byte-exact history, whole-file independent
audit, no new affected-face degeneracy or orientation reversal, and changed
edge stretch ≤4. Exact GPU position **and normal** f32 words must match all
five public states from the unique active WebGL VBO. Hash actual served
WASM/JS bytes and capture console/GL errors, recorder caps, physical memory
at 1 Hz, and owned-process closure. Save every failed raw receipt.

Run the 100k cohort before attempting 1m. An early 100k failure may stop the
matrix as a diagnostic, but it cannot support a throughput or parity claim.
The prior 40,962-vertex two-stroke trial and earlier million ordinary runs are
unmatched diagnostics. Even if all worker gates pass, comparing sculpt shape
or speed causally requires matched delivered dabs; broader SculptGL parity
still depends on the separate feature inventory.

The benchmark driver is `scripts/run-source-worker-sustained-c266.mjs`. It
uses the previously frozen C58/C209 owned-CDP harness via an explicitly
hashed `--bench-stage` path, and records those helper hashes in each result.
No browser/build run is authorized in this isolated worktree until the root
agent grants exclusive ownership of that lane.
