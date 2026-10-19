# SPW01 100k diagnostic: Source Worker admission blocked

**Result: no sparse Worker speed or sustained parity claim.** The ordinary
99,602-vertex held-pointer control completed, but the opt-in sparse Worker
route rejected the same imported SGL before sending a Worker message. The
frozen 100k gate is unmet. The 999,002-vertex cohort was not attempted: the
same metadata eligibility guard is independent of vertex count. No fixture,
threshold, default UI, or product path was changed during this audit.

The frozen protocol is [SPW01](SOURCE_WORKER_SPW01_PREREG.md), SHA-256
`800641d09091e55df4618c5e76b97186f74e96ecf34872dd335e316e01cd7443`.
The independent SGL fixture SHA-256 is
`8f8d76f0d71e420a81358a0a44d3a4ffa2a3ac76e9221165e1359c8e720a7093`;
the source protocol and manifest hashes are respectively
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`
and `f83022cb2766e44a147fb490c169e40a0a35e7a4e89c48d644566af16d0aea4a`.
The served WASM, JS, Worker JS, and Worker input JS SHA-256 values were
`6bd8e1b585e4cb92c465dbb62e6d9243b7349b708f370d846aee203bcf498403`,
`d88da99a09a9af1198258a23e2dc16c577f85012275f4ad235699458e13ac79a`,
`4b56bb3d1e1b0a594f0d288213ba856c11e1a95f8fb94b3ecb5dde73188caa60`,
and `1efc7dabe5e02b2c0ae3d881589c64ffbb657cb8e75f12c839f5c9867df41ac4`.

The completed ordinary control's raw result is
`.artifacts/source-worker-sustained-spw01/100k-ordinary-563fc0c1-81b8-49e6-a957-97f7585d1e5f/result.json`,
SHA-256 `3cd3be3780e5b00dc75a43531ba2d81d2c73006252470a2e6c15396986c92348`.
It requested 24,000 CDP moves over 60 seconds; 16,771 trusted DOM moves
were delivered and 1,142 dabs applied. RAF P95/max were 16.8/50.1 ms,
app CPU P95 9.3 ms, release acknowledgment 15.253 ms, and pacing P99/max
1.019/2.546 ms. Public Save before/after/stable/Undo/Redo worked. Before
and Undo files matched byte-for-byte; after, stable, and Redo files matched.
The independent whole-file shape audit passed 17/17 checks: 4,064 changed
vertices, 8,377 changed faces, 2.2182 maximum edge stretch, 0.22658 maximum
vertex displacement, and no new affected-face degeneracy or orientation
reversal. The unique active VBO positions matched saved f32 words at all five
states. GPU **normals** differed from saved normal words in all five states,
including the ordinary baseline. As preregistered, saved normals are not a
validated displayed-normal oracle, so this is an oracle mismatch of unclear
cause, not evidence of a Worker regression. The frozen strict GPU gate fails.

The first two Worker setup failures were harness issues, both preserved:

| Receipt SHA-256 | Observed failure |
| --- | --- |
| `86f8844a5fb8077c2aed3e5d67037f60b073da51fc3e77bfeec7bc2785a9f00a` | The old UI press acknowledgment did not recognize the rendered trial button. |
| `67e47cfd241e4d320f794fb9ba5a0a0f19431f52aaff125cd3690e82f68502e9` | The trial click removed canvas keyboard focus, preventing the Save shortcut. |

After repairing only the driver, a full 60-second Worker attempt was
preserved at `100k-worker-052847b0-41cd-4f03-b983-3167303ace84/result.json`
(SHA-256 `258a9ee84abae6a2679cb19a14da2e0b04038cc81e53aca88485150cdc2aabfe`):
the route began no Worker job, accepted no dabs, and committed no geometry.
The diagnostic rerun failed fast at
`100k-worker-923b231d-1619-4630-a404-9d0644c3a2a7/result.json`
(SHA-256 `f82bf42042447708fdb076fb12c825752f64ff0580275b2b849a4c8901208426`)
with the precise `source_worker_held_begin` error **“Worker sparse preview
requires plain Studio rendering”** in 0.3 ms. That rerun included a normal
trusted SGL import, trial-button click, canvas focus, and held-pointer start.
No Worker message, geometry CPU, preview, release comparison, or matched
Worker dab count exists. It is invalid to divide the ordinary time by this
failure or infer throughput from it.

The eligibility check is in `crates/obrush-app/src/source_worker_held.rs`:
both the active object's metadata and *all* rendered objects' metadata must
be absent, in addition to Studio shading, no wireframe/wire-only/flat/UV
rendering, and no transform drag. The imported fixture has Source scene
metadata despite identity `local_to_world`, shader 0, matcap 0, no wireframe,
and no flat shading. The diagnostic therefore points to metadata presence as
the blocking condition; the error string calls this a render-style problem.

Relaxing that check for identity affine + shader 0 alone is **not a safe
fix**. `renderer.rs::source_normal_policy` selects `StaticOriginalFaces`
when metadata is present even in Studio shading. The sparse-preview draw path
also explicitly rejects metadata, selected normals, or Source normals before
writing rows to the active GPU buffer. A follow-up needs a defined Source
normal-row/GPU mapping, the correct transform and face-order behavior, and
an independently validated displayed-normal oracle across Save/Undo/Redo.
Only then should the same frozen gate be rerun, with all failures retained.
