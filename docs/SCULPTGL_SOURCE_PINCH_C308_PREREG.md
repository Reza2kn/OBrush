# C308 Source Pinch normal-UI pair: prospective protocol

Frozen before any C308 browser execution or product edit. This is one scoped
comparison for `SGL-BRUSH-PINCH` and its Negative response, not closure of
either inventory row. The Source Pinch kernel already exists in OBrush core;
its literal unit tests are not a reference-browser result. Any first failed
setup, geometry, GPU, or history gate must be retained as a failed receipt.

## Pinned inputs

- Reference: SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`;
  locally hosted `sculptgl.js` SHA-256
  `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
  `Pinch.js` in that checkout has SHA-256
  `faf424cffc1c6a102f49698bc4e7611e1bfa16f73e574eb1ff75a7bd7cd2e6f4`.
- Fixture: independently Python-authored CC0 SGL v3 curved grid
  `tests/fixtures/independent-source-pinch-c308.sgl`, SHA-256
  `f864eb78bf6f280c92fcaa3debcaa406c9839a82e861d213e396233ac6269144`,
  160,324 bytes, 2,401 RGB-ID vertices and 4,608 oriented triangles.
  `scripts/fixtures/author-source-pinch-c308.py` defines every field and the
  independent binary writer (SHA-256
  `b9ddbb547649bbda58185b7b2bc24b508bb2813704d44c8ca00e4d3ab433d984`).
  ID 1201 is fully protected, ID 1202 is half
  unprotected, all other IDs fully unprotected. Independently decoding the
  finished fixture confirmed those counts before browser use.
- Baseline OBrush source commit: `11f01b9` (full build identity, served WASM/JS
  SHA-256, and actual release-mode flags must be recorded at execution).
  The ordinary root user's browser server is outside this protocol. Each
  capture uses owned browsers and processes from this isolated worktree.

## Four frozen normal-UI captures

Run fresh reference Negative off, reference Negative on, OBrush Source Negative
off, OBrush Source Negative on sessions. Each imports the same fixture through
the normal visible file chooser with source import scale-and-center disabled,
then selects Pinch via its visible control. Set orthographic front view and
visible fit/center; wait for settled camera timers and 12 unchanged terminal
frames over at least 250 ms. Align the logical viewport to 1024 × 768 CSS
pixels at DPR 1. Source defaults are 50 px radius and 75% intensity; verify
their actual displayed and tool-owner values. Set OBrush's Source Pinch to the
same published intensity and match the world radius and requested screen
offset using both actual projection matrices and viewport widths. Record
camera matrices, rays, pick center, local radius and the derived calibration;
do not assume matching numbers from merely equal UI labels.

Choose Alpha None, Culling off, X symmetry off, Continuous off, Lock Position
off, pressure-size and pressure-intensity factors zero, and OBrush Prevent
folds off. Keep original imported colors/materials/masks and leave dynamic
topology off. The only case difference is the visible Negative checkbox,
verified against the active tool owner after the click. Neutral mouse force is
one. The single gesture is a trusted press at viewport center plus `(10, 0)`
CSS pixels, one held move to plus `(25, 0)` with one animation frame after
press and move, then release at the held point. Record delivered pointer events
and stroke acknowledgments. A setup or control failure stops before a stroke;
do not silently alter the gesture or fixture.

Capture complete public Before, After, Undo, and Redo project exports per
session. Capture actual active-draw GPU position and display-normal Float32
streams at each stage, using a unique selected OBrush WebGL2 VBO readback and
the reference WebGL1 active-draw upload observer. Saved normals are not a
substitute for active GPU display normals. Observe the quiet After stage and
ensure no extra dab occurs after release.

## Admission gates fixed before measuring

1. The exact reference bundle and independent fixture hashes, vertex/face
   counts, one selected object, viewport/DPR, owner tool index, visible options,
   trusted pointer delivery, and actual served OBrush WASM/JS hashes are
   recorded. Both hosts must agree on horizontal/vertical projection ratios,
   two requested world rays, pick center and world brush radius to `1e-8`
   before geometry is scored. If that alignment fails, preserve the receipt
   and do not score a paired deformation.
2. RGB-ID mapping must be complete and unique. The public Before positions,
   oriented faces, color/material/mask fields and object transform must match
   across hosts exactly as saved Float32 words (allow ordering changes only
   through the explicit RGB-ID mapping). Geometry stays finite, same topology
   and same per-ID attributes through After, Undo, and Redo. A nonzero changed
   set is required for each polarity; ID 1201 must stay bit-exact. Record ID
   1202's signed movement and require it be nonzero in both hosts.
3. For each polarity, require equal changed RGB-ID sets and maximum absolute
   paired final XYZ component difference `<= 1e-6` across all 2,401 IDs.
   Record differing component counts and RMS even on failure. For each host,
   Negative off must move affected vertices toward the measured picked center
   in aggregate and Negative on must move them away; retain per-ID signed
   center-directed displacement, including any exceptions. The four-run
   matrix requires at least one common affected unmasked ID whose signed
   center-directed displacement changes sign and whose magnitude exceeds
   `1e-7` on both sides. Do not claim a perfect inverse after repeated live
   Float32 updates.
4. Each host's public Undo export must match its complete Before bytes and
   Redo its complete After bytes; only one history step may be added by the
   stroke. For Before, After, Undo, and Redo, actual GPU position words must
   match that host's mapped public positions exactly, with one active draw
   buffer, unchanged object identity, stable buffer provenance and GL error
   zero. Compare mapped active GPU display-normal words across hosts with
   maximum absolute XYZ component `<= 1e-6` at all four stages. If either
   observer cannot prove active identity, report normals unqualified and keep
   the overall C308 gate open.

Do not relax thresholds or reuse a failed C308 case under the same name. A
later candidate may be admitted only against the unchanged four-run protocol,
plus formatter, workspace tests, strict native/WASM Clippy, WASM check, web
build and a fresh ordinary-browser check. Even a passing C308 fixture leaves
radius zoom/DPR, intensity matrix, held Alt/N, Culling, Lock, alpha, dynamic
topology, physical pen, sustained performance and all ZBrush parity open.

## Static harness freeze

These scripts were completed and passed `node --check` before browser use;
their SHA-256 values are part of the admission contract. The capture is run
with `--one-move --side=source|root --negative=off|on`; each root run also
requires `--match-source` pointing to its same-polarity Source receipt.

| Script | SHA-256 |
| --- | --- |
| `scripts/check-sculptgl-source-pinch-c308-ui.mjs` | `aeae1ad85ed77bbb4f5c704cd5370ed4dc11862a2f0fffad63411e0fc4c6f691` |
| `scripts/audit-sculptgl-source-pinch-c308-ui.mjs` | `36062902b05af06f9f3b775e088aaefe158989fc8e71ded37083c14ce8d73b4e` |
| `scripts/audit-sculptgl-source-pinch-c308-matrix.mjs` | `8a7d75549830d89f9ee4e6c5b70d065c7dd4bc673a5aef9f682ec73fbd284022` |
