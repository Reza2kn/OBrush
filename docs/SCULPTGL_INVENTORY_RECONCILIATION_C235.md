# C235: scoped Brush, Mask and input inventory links

The [current 317-ID crosswalk](SCULPTGL_CURRENT_LEDGER.md) links ten previously
unreconciled active rows to existing, pinned SculptGL evidence. This is a
documentation audit: no geometry, host behavior, fixture, reference run or
performance result was changed. All ten row acceptance gates remain **Open**.
The links are stored in
[`sculptgl-current-overrides.json`](evidence/sculptgl-current-overrides.json)
so regenerating the crosswalk preserves them. The already present C231/C232
pressure links were also carried into that source JSON; the prior checked-in
crosswalk and its generator had disagreed on those two rows.

| Pinned ID | Existing scoped observation | Still-open acceptance boundary |
| --- | --- | --- |
| `SGL-BRUSH-BRUSH-CLAY` | [C172](SCULPTGL_UI_BRUSH_PAIR_C172.md) and [C173](SCULPTGL_UI_BRUSH_SETTINGS_C173.md) use the ordinary browser Brush with Clay off on an authored sphere. | The default/on Clay plane response and repeated buildup remain unpaired. |
| `SGL-BRUSH-BRUSH-ACCUMULATE` | [C170](SCULPTGL_BRUSH_REPEATED_NORMAL_C170.md) pairs two accumulated direct-method dabs on one triangle; [C172](SCULPTGL_UI_BRUSH_PAIR_C172.md) exercises Accumulate off in a normal browser drag. | The off/on per-stroke ceiling and separate-stroke buildup are not paired through both UIs. C172 records failed source checkbox setup attempts. |
| `SGL-BRUSH-BRUSH-ALPHA` | [C168](SCULPTGL_ALPHA_QUANTIZATION_C168.md) matches one imported PNG's 256 decoded bytes; [C171](SCULPTGL_BRUSH_MOVING_ALPHA_C171.md) pairs a moving alpha-frame direct-method dab and retains a failed strict signed-zero assertion. | None/Square/Skin selection, per-tool state, normal UI footprint, centerline mirror and reset remain open. |
| `SGL-BRUSH-BRUSH-IMPORT-ALPHA` | [C166](SCULPTGL_JPEG_ALPHA_PAIR_C166.md) pairs one imported JPEG's 256 bytes and five primary/mirror samples; [C168](SCULPTGL_ALPHA_QUANTIZATION_C168.md) pairs one low-opacity PNG matrix. | Arbitrary image/profile behavior, extent/orientation, reused selection and same-mesh UI displacement remain open. |
| `SGL-BRUSH-MASKING-CLEAR`, `SGL-BRUSH-MASKING-INVERT` | [C162](SCULPTGL_BACKGROUND_MASK_C162.md) observes the shared-core actions via trusted Ctrl background gestures in both normal browsers, changing all default-sphere masks in the same semantic direction; OBrush Undo/Redo saves recover exactly. | Different startup meshes prevent vertexwise comparison. Mixed masks, mask-free no-op, source history/redo, signed zero and native input remain open. |
| `SGL-BRUSH-MASKING-BLUR`, `SGL-BRUSH-MASKING-SHARPEN` | [C80](SCULPTGL_MASK_FILTERS_C80.md) pairs an eight-vertex cube and [C81](SCULPTGL_MASK_FILTERS_C81.md) pairs a nine-vertex open quad grid through visible controls. Sharpen weights match exactly; Blur differs by at most `2.9802322387695312e-8` after mask-convention conversion. | Irregular/nonmanifold topology, repeated large-mesh actions, long-session behavior and full history/no-op policy remain open. |
| `SGL-INPUT-BACKGROUND-MASK-INVERT`, `SGL-INPUT-BACKGROUND-MASK-CLEAR` | [C162](SCULPTGL_BACKGROUND_MASK_C162.md) pairs Ctrl background click/drag semantic transitions through trusted browser events; OBrush held-press Save cancels delayed release and Undo/Redo recovers exact own-format projects. | Nondefault masks, off-viewport release, pen/touch, native GUI, concurrency and responsiveness remain open. |

Each pinned source behavior and full acceptance criterion remains in
[the frozen inventory](SCULPTGL_PARITY.md). A row-level link means that a
specific check exists; it does not promote any Brush, Mask, input, SculptGL or
ZBrush parity family. The [C226 million-vertex comparison](SCULPTGL_MILLION_PAIRED_C226.md)
and later [C228](SCULPTGL_CERTIFIED_RENDER_HANDOFF_C228.md)/[C229](SCULPTGL_RETAINED_TOPOLOGY_WALK_C229.md)
still leave all four responsiveness gates failed.
