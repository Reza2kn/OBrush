# C174: normal-browser Brush Accumulate effect

This extends the [C172 ordinary Brush UI pair](SCULPTGL_UI_BRUSH_PAIR_C172.md) from Accumulate off to a real checkbox-on comparison and a within-app off/on effect audit. Both pages imported the same independently authored, ID-colored 10,114-vertex / 20,224-triangle SGL sphere (SHA-256 `39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`). The pinned SculptGL bundle was `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`; the normal OBrush browser WASM was `772cd008f5bc33d89c8ad4c7a3c09fa3ab8a6f5c72d1254adf80aaa9ffde837a`. No production sculpt code changed in this check.

The test used the ordinary Brush at source 64 CSS px radius and 25% intensity, with Clay, Alpha, symmetry, pressure factors and fold prevention off. OBrush's visible radius was calibrated to the source world radius under the two actual orthographic projection matrices. The normal UIs each received one trusted press, six alternating 12-pixel horizontal moves over the same area and one release. The read-only DOM observer saw the same 18 trusted pointer/mouse events in all four runs. Before geometry matched across all runs. The source Accumulate checkbox initially failed to publish in earlier C172 attempts because the animating sidebar covered the control; this harness waits for a read-only hit test to show that the real checkbox owns the click point, then verifies both its DOM checked state and SculptGL tool state. OBrush's corresponding normal control also published the requested state.

| Mode | Source changed IDs | OBrush changed IDs | Cross-app After differences | Maximum component difference |
| --- | ---: | ---: | ---: | ---: |
| Accumulate off | 98 | Same 98 | 30 vertices / 33 components | `1.1920928955078125e-7` |
| Accumulate on | 98 | Same 98 | 41 vertices / 51 components | `5.960464477539063e-8` |

Turning Accumulate on, compared with off after the same stroke, changed **the same 96 vertex IDs in each app**. Its maximum absolute positional effect was `0.00035369396209716797` world units in both. The paired effect deltas differed at 45 IDs / 59 components by at most `1.1920928955078125e-7` per component, RMS `1.5941115057291115e-9`. The shorter C172-style two-move run also passed the on-state pair: both changed the same 96 IDs, with 16 cross-app differing vertices and `5.960464477539063e-8` maximum component difference. In that shorter on/off comparison the effect membership differed by two IDs at float32 precision; the six-move retrace is the stronger buildup test, not a claim that every event history is bit-identical.

Both normal-browser modes preserved every oriented face, color, material and mask value. Each public Undo file exactly matched its own Before file, and Redo exactly matched its own After file. The independent audits decode all public save files and map vertices by ID color. The current source and OBrush serialized formats differ, so cross-app file hashes are not expected to match.

Frozen local receipts under `.artifacts/sculptgl-ui-brush-pair/` (all SHA-256):

| Artifact | Directory or file | SHA-256 |
| --- | --- | --- |
| Source off result | `source-b6503720-654f-47ae-9ff7-799e32bdcc40/result.json` | `334974ffb97fdc8af6a5797469c07f433b7fdb0d99e1d7e2add94e49a27ad472` |
| Source on result | `source-2f04daf7-79ce-44d1-8747-b09221f665df/result.json` | `b44acd6bc7e137085b14ef3784ebb39505705831268270425695afe454a018c9` |
| OBrush off result | `root-1b9f0241-2fef-468b-aa25-bd8240c20a7e/result.json` | `77ab8c2aa784ffa28a0ce57e082bfcaaab5b4b4bd859405f4115295392d34f8e` |
| OBrush on result | `root-1a9afe7a-c9a1-48e3-8954-89885bc8078a/result.json` | `a9a78793d3e929752fb13611537e2b134b1763ec4c77fac3b5ef2e1a42e7a50f` |
| Off whole-file pair | `source-b6503720-654f-47ae-9ff7-799e32bdcc40/paired-audit-root-1b9f0241-2fef-468b-aa25-bd8240c20a7e.json` | `bd5106f0a875fd652bf59fa61cec9966dfaaebb0456b8d0b122eaf0106f2ab2f` |
| On whole-file pair | `source-2f04daf7-79ce-44d1-8747-b09221f665df/paired-audit-root-1a9afe7a-c9a1-48e3-8954-89885bc8078a.json` | `039c7356a5feba72cae54a570140af263430b90b0d7abbeb7067e3e2f722d580` |
| Toggle effect | `source-2f04daf7-79ce-44d1-8747-b09221f665df/accumulate-toggle-effect-audit.json` | `7d4bc11d5804748b03a064aabda42fa0ffa3025e2946ca01096892dc3d935b88` |

Validation: `node --check` on the two audit scripts and UI harness; four normal-browser retrace runs with public Save/Undo/Redo; the off, on and effect audits; and `git diff --check`. This is one orthographic sphere and one retrace shape. Alpha, other radii/intensities/cameras/topologies, exact dab scheduling, native GUI and the still-failed million-vertex responsiveness gates remain open. The result does not promote the whole Brush family or SculptGL/ZBrush parity.
