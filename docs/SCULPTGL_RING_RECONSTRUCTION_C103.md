# C103: source ring-order reconstruction for coarse edits

C102 found that static subdivision predicts coarse-edited positions closely, but residual detail follows a normal frame whose tangent depends on SculptGL's first fine-vertex neighbor. Using the measured reference neighbor IDs in a diagnostic reproduced the three edited fine meshes; those IDs were an oracle, so this checkpoint reconstructs them from OBrush's independently authored OBJ inputs instead.

The [diagnostic script](../scripts/reconstruct-sculptgl-ring.py) takes each native parent mesh's ordered triangle/quad faces and corner UV IDs. It simulates the pinned source's 32-entry UV-aware post-transform face visit, then uses first-encounter edge and quad-center labels to predict the child-face order and the first neighbor in each fine vertex ring. Reference snapshots and neighbor IDs are read **after** prediction, only to validate it. The three OBJ files, source revision, exact hashes and numerical results are indexed in [C103 evidence](evidence/sculptgl-ring-reconstruction-c103-0.1.20.json).

| Same-OBJ fixture | Source parent face order as host face IDs | Parent/child face rows | First-neighbor mismatches | Predicted-frame max component error |
| --- | --- | --- | ---: | ---: |
| All-seamed tetra | `2,1,0,3` | Exact / exact | 0 / 10 | `1.4901162e-8` |
| All-seamed quad cube | `4,2,0,5,1,3` | Exact / exact | 0 / 26 | `5.9604645e-8` |
| All-seamed mixed pyramid | `4,1,0,3,2` | Exact / exact | 0 / 14 | `2.9340722e-8` |

The final-column diagnostic combines the predicted neighbor IDs with independently computed face-averaged normals and the C102 old/new static predictions. It does **not** read reference neighbor IDs to synthesize positions. It uses Python double-precision operations and matches the observed fine positions to the stated tolerance after mapping vertices by pre-edit physical position. This isolates the ordering dependency but is not a delivered Rust or browser editing path. It has only one subdivision level, one direct coarse-vertex displacement `(0.125, -0.09375, 0.0625)`, and three small fixtures; sustained sculpting, normal brush edits, arbitrary topology, multiple levels, history and ZBrush behavior remain open. The next implementation step is a bounded shared-core ring map plus normal-frame detail transport with old project semantics preserved, followed by normal browser interaction and performance checks.
