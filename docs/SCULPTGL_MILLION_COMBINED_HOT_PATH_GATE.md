# MCH01: combined million-vertex held-pointer hot path — preregistration

This is an investigation gate, not an implemented performance claim. Pin the
next owned normal-browser control to integrated product commit `30a7e7a`, the
independent 999,002-vertex SGL fixture SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`,
and the existing 60-second/400-requested-moves/s protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
Hash the rebuilt portable WASM, binding JS, renderer/Worker JS, runner and
browser version at execution. Keep the original four active responsiveness
gates unchanged: RAF interval P95 ≤33.34 ms, RAF maximum ≤250 ms,
`App::update` callback CPU P95 ≤16.667 ms, and Chrome mouse-release command
acknowledgment ≤100 ms. The pinned SculptGL comparator is commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`.

## Measured constraint, not an additive estimate

The historical C292 ordinary path failed 0/4 with 66.7/600.0 ms RAF
P95/maximum, 63.2 ms app CPU P95 and 134.7 ms release acknowledgement.
Its Source geometry averaged 21.80 ms per applied dab and its renderer callback
15.11 ms on a **different sample population**. The later MRP02 matched
normal-browser control failed 0/4; a temporary certified hover-pick candidate
reduced RAF P95 from 66.7 to 33.4 ms and app CPU P95 from 62.4 to 35.1 ms,
but still failed all four gates. The candidate averaged 22.19 ms Source
geometry per dab and 7.91 ms renderer callback. Hover picking was directly
measured at 22.896 ms/useful callback (45.99% of callback CPU), and a same-state
browser pair measured 28.6505 ms public pick versus 0.0682 ms certified pick
across 921 calls with zero exact hit mismatches. It remains rejected: no
actual GPU-word oracle was captured and the full responsiveness contract
failed. These overlapping means cannot be added into a frame budget, and
different delivered dab counts cannot establish a causal end-to-end ratio.

The separate query-gated 100k Worker route had a 475.3 ms synchronous final
commit overlapping a 481.5 ms RAF gap; its finish reply arrived 211.8 ms
after release. Its queue was still active at +100 ms. Moving geometry into
that existing Worker unchanged cannot meet the frozen release and maximum-RAF
gates. A million-vertex Worker result has not been measured. The old V2
display-trace fixture failed an exact-two-dab gate because immediate release
discarded a queued sample. These failures are inputs to the design, never
rescored as passes.

## Causal split before a candidate

Run the already preregistered read-only MRP03 probe on an unchanged
`30a7e7a` control and, separately, on a temporary hover-free diagnostic only
after proving its hit and file equivalence on that same revision. Record
same-frame Source geometry, hover pick, scene refresh, renderer upload and
`App::update` CPU with frame IDs; distinguish initial press, early stroke,
steady stroke and release. Preserve raw first-press 600 ms and early 400 ms
intervals even if they occur outside callback CPU. Distinguish release request,
CDP acknowledgment, egui delivery, endpoint apply, queue-empty and history
publication on correctly aligned clocks. A missing attribution or overflow
stops product admission. Report conditional P50/P95/max per phase; do not sum
nested means. The MRP03 static probe was prepared on an older branch but has
**never been run**; porting it requires a source-diff review, full checks and
its own read-only nonpublication test.

Admit a combined candidate only if the control shows (1) an exact-state
hover query that consumes ≥10 ms or ≥25% of pressed callback CPU and (2) a
separately measured geometry/render/publication critical path that remains
above the 16.667 ms callback limit after removing that query. Before
implementation, freeze an identical-state native/WASM microbenchmark with
exact geometry/hit/display-normal words and a predicted ≥5 ms reduction in
the named second stage without greater peak memory. A renderer-only or
hover-only improvement is not sufficient evidence. The proposed ownership
path must explicitly preserve bounded shared-core geometry, input order and
the exact release endpoint while rendering from checked changed rows without
a long synchronous complete-scene publication. If these preconditions fail,
record the rejection and make no product change.

## Browser acceptance and stop rules

Use an owned visible Chrome profile and unchanged runner events. Compare the
candidate with an uninstrumented **same-revision** control, retaining applied
dab count and coalescing so different delivered streams are labeled
observational. Require the independent complete 17-check public-project
Before/After/Stable/Undo/Redo and affected-face auditor, byte-identical
Before/Undo and After/Stable/Redo, one-step history, finite geometry,
unchanged non-position channels and directed faces, and no new affected-face
degeneration or reversal. Capture actual GPU VBO position **and Source
display-normal** f32 words in all five states, independently derive the
edited-state normal oracle, verify endpoint, bounded queue/memory, no
app/GL/Worker errors and clean process/server exit. Public mesh normals are
not an interchangeable oracle for Source display normals.

Candidate admission also requires no regression against the matched control
in any of the four metrics, plus a ≥5 ms reduction in the preregistered
same-state second stage. Promote *responsiveness* only if all four original
global gates pass in the complete 60-second cohort, with no missing release
or first-press observation. A scoped speedup does not establish SculptGL
brush-response, topology, display or ZBrush parity. Preserve every failed
build, audit, browser run and reverted source patch.
