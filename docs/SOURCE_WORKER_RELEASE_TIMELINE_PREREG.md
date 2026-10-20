# Held Source Worker release timeline: static audit and read-only preregistration

**Status:** static design only; no source instrumentation, build, browser run,
or performance improvement is claimed. Keep the held Worker query-gated and
off by default. This follows the frozen SP-WORKER-01 100k protocol SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`
and independent SGL fixture SHA-256
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The latest read-only Worker finish profile result SHA-256 is
`a9e3558f6dfbf427e1ae590c9f749f37eecc8e7ce0fc40472ab2f36064bb460c`.

## What is already measured

The unchanged finish-phase profile attributed 194.3 ms from the main
`finish` post to its reply, including 164.2 ms retained warm packet encoding,
18.9 ms checked geometry delta encoding, 8.8 ms `transaction.finish`, zero
measured Worker queue wait, and 1.5 ms for both JS takes. It transferred
119,424 geometry-delta bytes and 16,222,744 retained bytes. `postMessage`
call duration itself was not captured; subtraction across realms is not a
calibrated transport measurement. These stages contribute to release-to-commit
latency but run in the Worker; they do not by themselves account for the
main-thread RAF stall.

On the checked main publication path, the repeated 100k stage receipts rank
the complete-scene witness first: 267.9 ms of the prior 283.7 ms scene/delta
stage; 13.4 ms was delta decode. The second largest is retained warm decode:
149.2–171.0 ms across separate runs, with different delivered dab counts.
The final publisher's delta re-encode is approximately 15.5–15.7 ms; checked
preview comparison is about 2 ms, and scene/output/history/budget/adoption
work is under 5 ms combined in the measured run. Checked main publication
was 453.3 ms in the validated-decode rejected receipt, 465.7 ms in another
retained-codec candidate, and 510.2 ms in the rejected typed-witness trial.
Active RAF maxima in those runs were 450.1, 466.7, and 716.6 ms. They are
unpaired observations, not a speed ratio, but the 453.3/450.1 ms proximity
strongly suggests the final synchronous main callback owns the RAF spike.

The frozen queue predicate is stricter than "no queued dabs": it requires
`active === false`, `pending === null`, zero queued samples, and no release
endpoint pending in a probe at most 100 ms after release. In
`web/source-worker-held-input.js`, `active` becomes null only after the
checked main `source_worker_held_commit` call finishes. The frozen runner
currently begins its CDP tail probes after release acknowledgement and all
24,000 pending move acknowledgements, so a late first probe yields an
*unproven* 100 ms gate. The measured 194.3 ms finish round trip plus the
453–510 ms main commit also make completion by 100 ms implausible even if
the first probe were on time. A faster 15 ms delta re-encode, 40 ms packet
codec, or 50 ms retained decoder alone cannot meet both the 100 ms queue
predicate and 250 ms RAF maximum. This is a lower-bound diagnosis, not a
claim that two unpaired durations add exactly in one stroke.

## Read-only next probe

Before an algorithm or protocol change, add a bounded, query-gated scalar
timeline to the existing held-input controller. Record `performance.now()`
at trusted pointer release, release endpoint apply post/reply, finish post,
finish reply, checked main commit entry/exit, and the `active = null`
transition. At release, schedule a 100 ms timer that records its expected and
actual firing time plus the same status predicate, without changing the
queue, input, Worker messages, packet bytes, or publication order. Cap the
timeline at 32 entries; include only event kind, timestamp and four scalar
queue/status fields, never mesh/packet contents. Export it through the
existing query-gated `status()` snapshot. Keep the frozen runner, fixture,
input schedule, UI endpoint, verifier and public IO unchanged. Correlate
same-realm timeline with the existing browser RAF recorder and named Rust
commit stages; do not subtract Worker and window clocks or alter the frozen
verifier to accept a different definition of empty.

Preregister the exact diagnostic patch and served WASM/JS SHA-256 values, run
focused nonpublication tests that compare pre/post Worker message bytes,
transfer lists, endpoint order, public state and one-undo history, then one
unchanged frozen 100k browser cohort with the existing whole-file, shape,
GPU-word and failed-verifier audits. Preserve the raw receipt and every
failure. A usable timing attribution requires the release-to-`active=null`
interval, the timer's actual lateness, and the RAF gap containing commit
entry/exit to be present in the same browser clock. If absent, stop at an
inconclusive profile. If `active` remains true past release +100 ms, report
the frozen queue gate as failed regardless of the first CDP probe's timing.
If the RAF gap contains the ~450 ms commit, a subsequent performance proposal
must remove or split **more than 200 ms** of synchronous main work and also
reduce end-to-end completion to **at most 100 ms**; no single currently
measured narrow substage satisfies both. Do not weaken complete-scene stale
detection, packet validation, atomic rejection, or public Save/Undo/Redo to
meet timing. A larger ownership/validation architecture needs its own proof
and preregistration; neither this probe nor a faster P95 promotes the Worker
or establishes SculptGL/ZBrush parity.
