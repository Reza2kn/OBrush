# C253: ordinary Source worker boundary spike

**Result:** No ordinary held-pointer worker was integrated. The frozen
SP-WORKER-01 100k and 1m acceptance cohorts were not run on a worker candidate;
responsiveness and SculptGL parity stay open. This spike measures only the raw
browser transfer boundary and identifies why the earlier opt-in worker trial
cannot replace the ordinary brush path.

The independent SGL sphere fixtures from C42 were 6,773,032 bytes / 99,602
vertices and 67,932,232 bytes / 999,002 vertices. Their SHA-256 values are
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`
and `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`.
The probe loaded each file in a fresh owned headless Chrome, made five
main-thread copies, then round-tripped each copy through a persistent browser
Worker. Clone mode left the sender's buffer attached; transfer mode detached
it. The returned buffer was compared byte-for-byte with the original after
timing. These are same-machine **SGL byte transport** observations, not a
shared-core document capture, brush dab or renderer publication.

| Fixture | Buffer preparation median | Structured-clone round-trip median | Transfer round-trip median |
| --- | ---: | ---: | ---: |
| 100k, 6.77 MB | 0.7 ms | 2.6 ms | 0.1 ms |
| 1m, 67.93 MB | 6.5 ms | 32.3 ms | 0.2 ms |

Five raw values per case, browser closure and the exact result are in
`.artifacts/source-worker-transfer/dd692223-6463-4d9a-860f-73d7be3c656a/result.json`
(SHA-256 `0237ec46c0116ec888d4ffea2542fc1f3cd8743a82a06a8f7d433f18f6ce9429`).
The committed probe `scripts/measure-source-worker-transfer.mjs` has SHA-256
`185e63a8a5775b570b69daaeea6cc6259d40c8f4c1ca6f1fdb95e937030d23ba`.
Two earlier diagnostic runs are retained in separate ignored receipt folders;
the final run pins input hashes and checks all returned bytes. Worker startup,
WASM instantiation, SGL parsing, Source state initialization, geometry,
normal recomputation, sparse publication, GPU upload, history and Save are
outside these times. The small transfer median therefore **does not predict**
an ordinary worker stroke's responsiveness.

The ordinary Source route starts a `SourceGeometrySession` and an open
`Document` history transaction in `source_geometry_app.rs` / `source_geometry_host.rs`.
Every queued Source dab enters `DabQueue::apply_source_geometry` and mutates
that same document inside `OBRushApp::viewport`; the renderer then consumes a
certified source snapshot before the frame returns. Release drains the queue,
finishes the Source session and validates its retained normal state and history.
The Source session also owns per-object binding, captured alpha/mirror tracks,
selection history, interpolation, pressure and changing-mesh hit preparation.
Moving only the deformation call to another WASM instance would leave two
documents with divergent retained normals and open-stroke claims. Public
`.obrush` JSON is unsuitable as a complete live worker snapshot because
`Document::to_json` serializes the project fields but omits transient
`source_geometry_states` and history/claim ownership.

The old isolated worker branch (`267677c`, `115186f`) proved a useful **opt-in**
100k transport: a checked changed-row delta rather than full result lowered
one frozen payload from 18,940,554 to 122,476 bytes, with a small exact
Save/Undo/Redo and GPU audit. It accepts scripted strokes of at most 32 dabs,
rejects meshes above 100k vertices / 200k triangles and depends on removed
worker-snapshot/delta APIs. Cherry-picking it into the current resident studio
does not create a normal held-pointer path or 1m candidate.

An admissible implementation needs a new bounded core-owned stroke snapshot
that carries the retained normal/selection state, a single worker-owned Source
transaction, ordered ordinary pointer/timer input from the existing viewport,
checked changed position **and normal** rows for incremental rendering, and a
checked release commit that preserves the document's exact public Undo/Redo.
The main thread must reject stale worker epochs and must never silently fall
back to a different brush. Only after that work can SP-WORKER-01 compare normal
100k/1m 60-second browser strokes, including trusted DOM events, RAF/CPU,
release/queue, GPU words and all five public Save states. Historical C226/C233
1m ordinary results remain failing (RAF P95 83.3/66.8 ms, CPU P95 71.2/68.6
ms, release 161.2/165.4 ms); this diagnostic changes no admission gate.
