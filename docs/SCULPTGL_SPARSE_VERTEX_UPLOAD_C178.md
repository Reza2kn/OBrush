# C178: bounded exact-row WebGL updates for Source strokes

The normal browser studio still runs Source brush geometry in WASM and draws
with WebGL2. This change keeps one active object's last packed 40-byte vertex
rows and optional 8-byte material rows in a **48 MiB capped CPU cache**. A
subsequent fixed-topology selected-normal state is compared byte-exactly with
those published rows. At most 128 changed 2,048-row chunks are uploaded with
`bufferSubData`; a larger edit, different topology, absent certificate, or
cache refusal uses the existing complete upload. The exact material cache
avoids reuploading an unchanged material stream. Inactive objects discard the
cache. The shared Rust geometry, project format, and history are unchanged.

The frozen workload uses the independently authored 999,002-vertex,
1,998,000-triangle fixture (SGL SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
and the 1M/60-second protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
It requests 400 pointer moves per second with Source Ordinary Brush at 64 px
and intensity 0.25, without alpha, symmetry, clay, accumulation, or fold guard.
The final browser WASM SHA-256 is
`09a32902e903e43ab79deb7a7b3c7cba731f8b2be77eb0199c5cab42d7d2fcf4`.

| Normal browser observation | Pre-change | Final C178 |
| --- | ---: | ---: |
| Delivered moves / accepted dabs | 1,760 / 803 | 1,756 / 807 |
| Complete 39,960,080-byte vertex uploads | 804 | 1 |
| Complete 7,992,016-byte material uploads | 804 | 1 |
| Partial vertex uploads | 0 | 806 |
| Recorded mesh-sized upload bytes | 41.79 GB | 6.37 GB |
| Mean CPU time inside Source geometry dab | 24.58 ms | 24.55 ms |
| App paint callback CPU P95 | 76.3 ms | 76.4 ms |
| Browser RAF interval P95 | 76.9 ms | 76.9 ms |
| Release acknowledgement | 269.6 ms | 264.6 ms |

The transfer reduction is directly observed, but the browser remains slow.
The final run fails all four existing responsiveness gates: RAF P95 ≤33.34 ms,
RAF maximum ≤250 ms, app CPU P95 ≤16.667 ms, and release idle by 100 ms.
The final RAF maximum is 624.4 ms. Delivered moves and changed vertices differ
between runs, so this is not a controlled causal speed ratio. The 48 MiB cache
is a deliberate memory cost, admitted only for one active object under the cap.
Full-range row scans and WASM deformation still consume main-thread time;
GPU-resident picking, deformation and presentation remain the larger work.

The final browser run is
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-11beed93-3a89-414d-a17d-a748036d6a72/`.
Its public `.obrush` before/after/stable/Undo/Redo saves recover exact file
hashes. The independent `audit-c178.json` reads the complete public before and
after meshes: all 999,002 vertices and 1,998,000 triangles remain declared,
triangles and nongeometry attributes match, 40,806 vertices change, and 82,432
affected faces remain nondegenerate and without orientation reversal. Maximum
changed-edge stretch is 2.1873. All ten audit checks pass. The browser recorded
no application, console, or cleanup errors. Pure row tests reconstruct the
entire packed target buffer from the sparse ranges, including signed-zero and
color edits. A complete frame-pixel oracle and matched SculptGL stroke response
are still open.

Two exploratory browser runs are preserved. A weak reference to the previous
mesh never upgraded at the repaint boundary, so it made no partial uploads.
The first 64-chunk cache version made 678 partial uploads, reduced transfers,
but raised RAF P95 to 80.2 ms. The final bounded 128-chunk path removes those
fallback uploads while keeping the frame result near the original baseline.
Neither exploratory run is counted as a performance pass.

The pinned SculptGL source bundle and its earlier separate 1M trace remain the
reference context in [C161](SCULPTGL_TOPOLOGY_TOKEN_C161.md). This renderer-only
change does not establish SculptGL or ZBrush parity, nor a browser speedup.

Validation on the final source: `cargo fmt --all -- --check`, 2,493 passing
workspace tests (zero failed, one ignored), strict native and WASM-target
Clippy, `bash scripts/build-web.sh`, `bash scripts/package-macos.sh`, the final
normal-browser workload and independent saved-mesh audit all pass. The
macOS app package passed its local signature check. The exploratory failed
speed/activation results above remain part of this record.
