# Sparse main publication: frozen timing blocker before implementation

**Decision:** do not implement a sparse clone/decode-only main-publication
candidate under the current SP-WORKER-01 gate. This is a static feasibility
audit, not a product change or a new browser result. The held Worker stays
query-gated and off by default. The frozen 100k protocol SHA-256 is
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`;
the independent 99,602-vertex fixture SHA-256 is
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`.
The release-timeline raw result SHA-256 is
`0424c256883651b79c08ae9f3cc1b5c5ce323bfefcab6d8131ff26a237e93857`,
retained under `.artifacts/source-worker-sustained-spw01/100k-worker-588e764b-eb00-47df-89aa-2d2f0a3ef260/`.

## Counterfactual bound

The same-page timeline in `SOURCE_WORKER_RELEASE_TIMELINE_RESULT.md`
observed the checked endpoint reply at release +15.2 ms, final Worker reply
at +211.8 ms, main commit entry at +211.9 ms, and public completion at
+687.3 ms. The frozen queue predicate requires the trial to be inactive,
with no pending Worker request, within 100 ms. In the current controller,
the trial becomes inactive only after main publication. **If main publication
were made instantaneous, the earliest observed completion in this run would
still be +211.8 ms**. Sparse main publication cannot change the 196.5 ms
finish-post-to-reply interval and therefore cannot pass the 100 ms gate on
this measured path. This is a within-run counterfactual, not an extrapolated
throughput comparison.

The checked main commit occupied 475.3 ms inside a 481.5 ms actual app-RAF
callback gap; the frozen RAF metric was 466.6 ms against its 250 ms maximum.
Earlier named-stage 100k receipts attributed approximately 267.9 ms to the
complete-scene JSON witness, 149.2–171.0 ms to retained warm decode,
13.4 ms to checked geometry delta decode (which includes the candidate
mesh snapshot), 15.5–15.7 ms to publisher delta re-encode, and under 5 ms
to the remaining checks/history/adoption. These observations were made on
different 100k strokes, so their sums are diagnostic, not a paired causal
speed ratio. Even treating **all** delta decode and re-encode time as
avoidable, the measured 475.3 ms commit would remain about 446 ms: far
above the 250 ms RAF gate. Removing the entire 149.2–171.0 ms retained
decode as well would still leave a scene witness near 268 ms before other
work. A clone/decode-only sparse optimization is thus a token change relative
to both frozen limits.

## Correctness boundary for any later architecture

`decode_source_worker_mesh_delta` currently checks packet version, target,
length, checksum, ordering and exact baseline binding, then creates a
candidate mesh snapshot before applying changed rows. The retained warm
decoder checks the candidate and historical tree before
`publish_cold_source_worker_result` adopts it with one undo event.
`SourceWorkerColdCapture` separately hashes the entire validated public
project at capture and final publication; host revision and active-mesh
pointer alone are insufficient because a direct in-place edit can preserve
both. Applying sparse rows to the public mesh before the retained and scene
checks would allow a late rejection to leave partial public geometry or
history changes. A safe sparse overlay would have to remain private until
every packet, scene, retained-state and budget check succeeds, then prove
byte-exact public Save/Undo/Redo and GPU positions/normals at adoption. The
existing retained decoder accepts `&Mesh`, not a virtual sparse overlay, so
this would be a decoder interface and ownership redesign rather than a
small copy removal.

Before any such code, preregister a separate end-to-end architecture that
plausibly places **both** full validated finish and main publication within
100 ms, or changes the frozen gate only through an independently reviewed
protocol revision; simply relabeling a pending finish as an empty queue is
not equivalent. Keep the current frozen verifier unchanged for a candidate
trial. Exact 100k targets remain inactive/empty queue by release +100 ms,
active RAF maximum ≤250 ms and P95 ≤33.34 ms, app CPU P95 ≤16.667 ms,
release acknowledgment ≤100 ms, pacing P99 lateness ≤25 ms and max ≤100 ms,
with one exact endpoint and one atomic history commit. Before browser
admission, adversarial focused tests must reseal malformed, duplicate,
unsorted, truncated, oversized, wrong-target and wrong-checksum deltas;
malformed retained trees/centers/normals; stale host revision, selected
SubTool, active mesh and in-place name/geometry/UV/material edits; and
late failures after candidate construction. Every rejection must leave
public Save, undo/redo depth, renderer buffer and retained state unchanged.
Successful Before/Undo and After/Stable/Redo public files must match exactly,
including f32 words; checked displayed-normal packets need a qualified
producer-to-VBO oracle in addition to GPU positions. Preserve all raw
failures, including the current frozen public-mesh-normal oracle mismatch.

No new code or Cargo/browser run was attempted for this blocked candidate.
The measured release lower bound and dominant scene/retained stages require
an ownership/validation architecture change before a sparse-publication
experiment could credibly address user-visible latency. This does not
establish SculptGL or ZBrush parity.
