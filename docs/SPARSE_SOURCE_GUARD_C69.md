# Sparse Source fold guard (C69)

An ordinary Source dab already has an unchanged full-position rollback copy and a bounded list of proposed `(vertex ID, position)` rows. With fold prevention enabled, the old guard then cloned the entire candidate position array, scanned all vertices to rediscover changed IDs, and scaled every position on each rejected candidate step. C69 passes the existing proposal rows to a Source-specific guard. It sorts the changed IDs into the same order as the old full scan, evaluates the same face/fold guard and 12-step halving policy, updates only proposed rows during rescaling, and restores unchanged component words exactly. The generic guard used by other tools is unchanged. This remains CPU geometry shared by native and WASM; it is not WebGPU integration.

The C68 frozen 999,002-vertex / 1,998,000-triangle fixture produced a **byte-identical 367,655,252-byte `.obrush` project** after five useful dabs. All 11 frozen tool/control projects, including fold-guarded Draw and Crease, also matched byte for byte. A focused test compares the new guard with the preserved old implementation on 24 local candidates, including cases that require guard rescaling, and compares every final position word and scale word. The complete workspace test suite, strict native/WASM Clippy, formatting, official browser and signed macOS builds pass. The owned browser brush suite passes 15 cases, 58 completed downloads and 58 visible GPU/save checks.

The unchanged frozen 60-second browser cohort scheduled 24,000 pointer motions and delivered 677 moves on C69, versus 668 on C68. The independent whole-project/geometry audit passed finite useful output, original topology, zero degenerate or reversed original faces, edge-stretch bound, full saved data and Undo/Redo. Responsiveness **still fails four gates**:

| Build | CPU frame P95 | Independent RAF P95 / max | Queue empty by 100 ms |
| --- | ---: | ---: | --- |
| C68 | 108.8 ms | 109.3 / 434.9 ms | No |
| C69 | 103.2 ms | 103.8 / 428.8 ms | No |

Different delivered/coalesced contacts mean this single run cannot establish a controlled speedup. The new guard removes known allocation and arithmetic work; full mesh validation/state matching, retained cache/history work, normal updates and rendering remain. The separate WebGPU point program does not execute product brushes or their undo transaction. A GPU-resident integrated path and the remaining 120-second/smaller cohorts, matched SculptGL responses and full 317-ID-plus-five-control inventory remain open. [The C69 evidence index](evidence/sculptgl-sparse-source-guard-c69-0.1.20.json) binds exact inputs, outputs, builds and checks.
