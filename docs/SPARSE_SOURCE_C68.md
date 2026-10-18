# Sparse Source candidate buffer (C68)

Every useful Source geometry dab kept a full copy of the original position array for rollback, then made a second full `proposed` position copy even when the brush wrote a small local patch. C68 retains the original array and stores only `(vertex ID, proposed position)` rows for the selected vertices. It computes every candidate against the unchanged original field, commits only after all fallible candidate calculations, then runs the existing fold guard, retained-normal/tree advance, Legacy normal refresh, history and rollback paths. The operation budget remains conservative. This is a shared Rust core change, so native and browser use the same implementation.

The exact old and new cores produced identical 367,655,252-byte public projects after five useful dabs on the independently authored 999,002-vertex fixture, changing 20,470 vertices. Eleven additional native cases produced byte-identical projects: Draw with and without accumulation, Clay, Inflate, Crease, Pinch, two Smooth modes, Flatten, and guarded Draw and Crease. The final source passes 2,350 workspace tests (one ignored), strict native/WASM Clippy, and the official web and signed macOS builds. A scoped owned-browser brush run passes 15 cases with 58 downloads and 58 GPU/save checks. [The C68 evidence index](evidence/sculptgl-sparse-source-c68-0.1.20.json) binds the exact source, builds, outputs, checks and retained preliminary fixture failures.

The frozen million-vertex 60-second browser protocol was replayed on C68 with 24,000 scheduled pointer motions. Public files and whole-project Undo/Redo passed; the independent whole-model audit found finite useful geometry with no degenerate or relatively reversed original faces. **The same four responsiveness gates still fail.**

| Separate 1M/60s run | Frame CPU P95 | Independent RAF P95 / max | Delivered moves | Changed original vertices |
| --- | ---: | ---: | ---: | ---: |
| C67 | 106.9 ms | 107.7 / 451.4 ms | 657 | 40,377 |
| C68 | 108.8 ms | 109.3 / 434.9 ms | 668 | 40,409 |

The runs have different delivered/coalesced inputs, so the P95 difference does not prove a speedup or regression. Removing the full proposed copy reduces per-dab allocation and memory traffic, but validation, state matching, retained cache/history work, normal updates and rendering remain. The current WebGPU point program is still isolated from the shipped Source brush and undo transaction. C68's 120-second and smaller sustained cohorts, matched SculptGL reference responses, native physical input and the full 317-ID-plus-five-control inventory remain open; no SculptGL or ZBrush family is promoted to parity.
