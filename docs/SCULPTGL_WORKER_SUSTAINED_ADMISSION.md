# SP-WORKER-01: ordinary versus worker held-stroke admission

This is a **protocol and verifier**, not a successful worker benchmark. It
retains the C58/C226/C233 100k/1m SGL fixtures and original 60-second,
24,000-requested-move normal-browser route. The exact machine-readable frozen
contract is `SCULPTGL_WORKER_SUSTAINED_PROTOCOL.json` (SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`).
The reference is pinned SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; the source protocol SHA-256
is `234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.

One candidate run must use the ordinary held pointer path, not a direct worker
API or hand-authored WASM dab. The 100k and 1m candidate workloads must both
perform normal File Import, 60 seconds of 400 requested mouse moves per second,
actual release and public Save/Undo/Redo. Record scheduled, acknowledged,
trusted delivered, queued, coalesced and applied dabs separately. The
candidate must report independent RAF P95/maximum, app CPU P95, release
acknowledgment, queue drain and the original file/shape gates. A worker run
additionally needs all init/dab/publication transfer bytes, transfer versus
clone, accepted dab count, main serialization/validation/publication CPU and
worker geometry CPU. GPU readback must identify exactly one active WebGL VBO
after each public Save state and match **both position and normal f32 words**
independently derived from that public file. The observer
`scripts/source-gpu-readback-observer.js` and independent auditor
`scripts/audit-source-gpu-words.py` provide this measurement surface; the
new observer has not yet been exercised in a normal browser at 100k or 1m.

Run fresh owned browsers serially in ordinary/worker/worker/ordinary order
per cohort, on the same host and build. Retain every failed run. The opt-in
persistent worker trial at `dbbaf704` has a 100,000-vertex/200,000-triangle
cap and accepts scripted one-shot requests, so it cannot qualify the 1m
cohort or the held-pointer workflow. A 100k transport microprobe may diagnose
bytes and stage costs, but cannot close these responsiveness gates.

`scripts/verify-source-worker-sustained.py` rechecks bounded fixture and
recording hashes, all requested CDP acknowledgments, timings, normal public
file identities, independent shape audit, GPU receipt, browser closure and
worker transport evidence. It exits nonzero if any required item is absent or
fails. An isolated one-case test changes only a GPU normal-word hash and
confirms the independent auditor rejects it. Syntax, JSON and this test pass;
no Cargo, browser or web build was run for this documentation/harness-only
branch while another agent owned those lanes.

The verifier was run read-only against retained prior ordinary-browser
receipts; each independent whole-file audit passed. These are **historical
diagnostics**, from different builds and delivered dab counts, not matched
candidate comparisons. The normal-browser receipts predate the explicit
trusted-DOM attestation and full GPU position/normal readback, so none is a
qualified SP-WORKER-01 run.

| Historical receipt | Applied dabs | Active RAF P95 / max | App CPU P95 | Release ack | Interaction result |
| --- | ---: | ---: | ---: | ---: | --- |
| [C202 100k](SCULPTGL_RELEASE_MATCH_C202.md) | 1,145 | 16.7 / 66.7 ms | 10.2 ms | 15.2 ms | Recorded gates pass; new DOM/GPU evidence missing |
| [C226 1m](SCULPTGL_MILLION_PAIRED_C226.md) | 886 | 83.3 / 600 ms | 71.2 ms | 161.2 ms | RAF, CPU, release and queue-by-100 ms fail |
| [C233 1m](SCULPTGL_APP_FRAME_PROFILE_C233.md) | 932 | 66.8 / 700 ms | 68.6 ms | 165.4 ms | Pacing, RAF, CPU, release and queue-by-100 ms fail |

The exact retained result/audit pairs are C202
`1143e84c18e9ec3ce642e1ee3961de206445783de52e164650c24f90fd4aa733` /
`f0be219200ff37ba37f380562a92709ecec77b36e29f8b91a2a52bc94afa4344`,
C226 `45bdb6bc641b884040c9356ef603ebc3edae4d4f38a4c6ea76deb61baab19406` /
`64b671be454faaed50c3c08b1733ed2c8125cd2f07997b37331d58b1b54ecd26`,
and C233 `4f6e1ac2df5159a4cfcaabd5bb7a0116afa2737f5b1a5d02d4733cf8c3b1feba` /
`ae718ff77b5f39d44b1f148bf8a618900fb5feb34a8bea83d3d40fe21d6e957a`.
Verifier outputs remain in this isolated worktree's ignored `.artifacts` as
`worker-benchmark-c202-100k.json`, `worker-benchmark-c226-1m.json` and
`worker-benchmark-c233-1m.json`.

Next requirement is a normal-browser worker candidate that can accept the
actual 60-second held pointer stream and safely handle the 999,002-vertex
fixture. Once it exists, run the complete serial matrix and independently
audit all five normal Save states. Different delivered dabs must be reported;
they cannot support an exact brush-response or causal speed claim. Full
SculptGL parity still depends on the separate 317-ID feature inventory.
