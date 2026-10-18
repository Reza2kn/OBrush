# C173: normal-browser Brush size and intensity pairs

[C172](SCULPTGL_UI_BRUSH_PAIR_C172.md) compared one ordinary Brush setting. This check keeps its independently authored, ID-colored 10,114-vertex/20,224-triangle SGL sphere (SHA-256 `39ddfd1559fde94aa5f6d2602beffc6cf9f77fb7f934989ee9f8677739f47c1b`), actual browser file inputs, trusted pointer events, public Save/Undo/Redo and whole-file vertex-ID audit. The pinned SculptGL bundle is SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`; OBrush browser WASM is SHA-256 `772cd008f5bc33d89c8ad4c7a3c09fa3ab8a6f5c72d1254adf80aaa9ffde837a`. No production geometry or renderer code changed for this comparison.

The harness requests additional settings through the visible source slider and records the *published* value before sculpting. Its pixel granularity published 31 rather than requested 32 px and 95 rather than requested 96 px; OBrush used those published values, converted by the measured projection ratio for the different viewport. Each page separately imported the same sphere bytes, selected ordinary Brush with Clay, Accumulate, Alpha and symmetry off, and received a short horizontal drag through its ordinary viewport. Source intensity was 50% and 10%; OBrush's normal numeric control published 0.5 and 0.1. Both browsers' mapped original Float32 positions and oriented faces were identical before either stroke. The requested initial world rays and world radius agree to floating-point arithmetic precision after calibration.

| Source published setting; stroke | Changed vertex IDs in each app | Cross-app After differences | Maximum absolute position component |
| --- | ---: | ---: | ---: |
| 31 px, 50%; one move | Same 23 | 6 vertices / 6 components | `2.9802322387695312e-8` |
| 95 px, 10%; two moves | Same 194 | 29 vertices / 30 components | `5.960464477539063e-8` |

The independent audit compares every mapped position, oriented face, vertex color, material and mask value in the public save files. Attributes and faces match across apps and remain unchanged by each stroke. Each app's public Undo and Redo files reproduce its own Before and After bytes exactly in both runs. Both pages received the same trusted DOM event pattern per pair: eight stroke events for one move, ten for two moves. This does not establish equal internal dab counts.

Receipts and audit SHA-256 values:

| Setting | SculptGL `result.json` | OBrush `result.json` | Whole-file audit |
| --- | --- | --- | --- |
| 31 px / 50% | `source-01c9f360-3ee0-48e4-a5ee-4dfe47d11f89` · `fe7736b2c0a7d8ddf5585f620e3002e9b9757fcec2f26c7bdf274b1647c61560` | `root-57bd2feb-ff35-498f-a0e0-6a5a8a0e48e5` · `5e46213bb1be1b379e1805c8d96105782c70dfb029236b5f59127d00e662852f` | `297667d1d07f26a0631642d8e7c99d4f06280b08c21524725177dded88a9953d` |
| 95 px / 10% | `source-c022a995-5ade-42d5-b337-71e1f0f8d0d8` · `37a7772bf970f84101304677a1ddec0d9b591972961cc5ec57dff98b5106fb9e` | `root-2f79cc10-86d0-48af-bd0c-052de864c559` · `eb2fca6f62c6560d84ccc37170e229dcc9bb0ad48557e15f0cc5a7b98309aa07` | `ace1039627bfe33e78db0684466f385970d37873fcb8ac0a4c927398024af534` |

The audit files are `paired-audit-<root directory>.json` inside the respective `.artifacts/sculptgl-ui-brush-pair/<source directory>/` folder. Failed attempts to edit source number fields or click an animating slider are retained in sibling `source-*` receipts. The source slider was initially covered by its sidebar during menu animation; waiting until a read-only hit test confirmed the slider owned the point made the drag effective. The useful runs do not claim exact response over all strengths, radii, camera modes, alpha/pressure/symmetry, topology, sustained performance, native GUI, or ZBrush behavior. The million-vertex responsiveness failures in [C156](SCULPTGL_MILLION_PAIRED_C156.md) remain open.

Validation: `node --check` on both comparison scripts, `git diff --check`, two normal-browser source/root pairs, and two independent whole-file audits. A third root run against the earlier C172 default one-move source receipt reproduced its 87/87 changed IDs, 10 differing vertices and `2.9802322387695312e-8` maximum difference after the harness change. No Rust source changed, so this check does not supersede the earlier native/WASM build and test records.
