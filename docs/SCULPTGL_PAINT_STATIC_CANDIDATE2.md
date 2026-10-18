# OBrush 0.1.16 candidate 2: material Paint and static subdivision browser checks

The normal browser controls pass the bounded Paint channel, retained material, and quad/UV subdivision checks below. The strict immediate Escape GPU equality check fails in two preserved runs. No SculptGL or ZBrush parity status is promoted. The [baseline inventory](SCULPTGL_PARITY.md) and [feature contract](FEATURE_PARITY.md) remain open.

The immutable candidate is `.artifacts/sculptgl-release-0.1.16-candidate2`. Its source manifest SHA256 is `6dfb7a028430577a3a7b0d62e513a50178e1c36c98530e208b132285fbb62f8e`; build record SHA256 is `34b14d67ddfc854d1c4a7040f27534d84d8d87e2748c48518b1654b3072bee90`. These are the same bytes used for the [retained actual OBJ return](SCULPTGL_OBJ_RETURN_CANDIDATE2.md). Previously completed OBJ, shader, export-policy, and malformed-file groups were not repeated.

The [machine evidence](evidence/sculptgl-paint-static-candidate2.json) binds every original run, executed helper copy, fixture, actual download, GPU comparison and retained failure. It contains **66 actual project downloads and 66 independently repacked GPU/project pairs**, across three additive runs. Thirteen successful case observations cover eleven unique case names, including startup. Repeated setup/downloads are not independent workflow types.

## Paint controls

The four ordinary positive strokes select RGB only, roughness only, metallic only, and all three channels through the rendered controls. They use an independently authored closed noisy sphere with nonuniform RGB/material rows, 48 fully protected points, and original triangle/corner UV metadata. World radius is `0.18`, strength `0.35`; roughness target is `1`, metallic target `0`. Each enabled channel changes useful rows, disabled channels remain exact, and positions/normals/masks/indices/original faces/corner IDs remain exact. Fully protected RGB and material rows remain exact. Each stroke is one exact Undo/Redo step and its release probes have zero queued dabs and stable GPU bytes.

An all-off ordinary Paint stroke on an older implicit material project changes no saved project bytes, keeps every material channel absent, keeps RGB display disabled, and preserves an independently prepared geometry Redo. A roughness-only stroke on an older retained Loop project promotes the project from version 2 to version 6 only after a useful edit. Undo restores the older implicit project exactly; Redo restores the edited project exactly. Lowering retains the coarse default material values, and returning to the fine level preserves the complete project exactly.

Material-only locked stamps pass held useful preview, identical repeated absolute endpoint, unchanged geometry/RGB/index stream, and disabled vertex RGB display. Release commits one exact history step. Returning to the press origin restores the complete original implicit project and the earlier geometry Redo. These checks use an independently authored asymmetric PNG and actual pointer input, with no Save command while the stamp is held.

## Retained static subdivision

The normal subdivision combo selects `SculptGL static smooth` and `SculptGL linear edges` separately. Each operates on an independently authored closed welded cube grid with 1,538 vertices, 1,536 original quads, 3,072 render triangles and 1,734 corner UV coordinates. One normal Add Level produces **6,146 vertices, 6,144 original child quads, 12,288 render triangles and 6,534 UV coordinates**. The independent control graph has 3,072 geometric edges and 3,264 chart edges; original UV coordinate prefixes remain exact.

Both modes retain their distinct scheme metadata and disable scheme changes after the first level. Materials/masks remain bounded, all child faces remain quads, Add Undo/Redo is exact, and Lower restores the original mesh including material and surface metadata. Higher restores the entire fine project exactly. A real project download reopened through normal file input also reproduces the entire saved project exactly. These are routing, retention and count checks; matched numerical reference stencils, mixed/boundary/nonmanifold cases, Reverse, and deletion of stack sides require separate evidence.

## Preserved Escape failure

The first executed helper, SHA256 `4781e0d8c838cb5cf2ac8457e89218944e4ea82e1138e0a2396597f39ba9eea4`, dispatches Escape and mouse release without waiting for an application frame. Its unchanged quiet gate compares GPU hashes at delays `0`, `200`, and `800` ms and requires integer `pending_dabs == 0`. The gate fails before subsequent project/Redo assertions. The original function did not retain the probes before asserting; that first failed receipt remains unchanged.

An additive helper, SHA256 `6e7ff4ed604cacb8de5e3678c4b297fb3256af52d0f1d9ede1f88ee3261b8655`, retains all three probes before the same assertion and selects only the failed Escape case. No input delay, numerical gate or tolerance changes. It reproduces the failure: all probes have zero queued dabs and identical geometry/RGB/index hashes. The first asynchronous GPU read still has the held explicit 80,656-byte material preview; the two later probes show the exact original implicit, empty, disabled material channel. The final registered report says `action=cancel`, with `dirty=false` and the original GPU hash.

This establishes a draw-frame transition after input delivery. It does not establish calibrated cancellation latency, growing geometry, or a queued-stroke backlog. The immediate equality gate remains failed; complete saved-project and prior-Redo checks after Escape were not reached in these original runs. The later [separate completion-aware cancellation check](SCULPTGL_CANCEL_COMPLETION_CANDIDATE2.md) passes the exact restored project, prior Redo and original quiet comparisons after registered command/GPU acknowledgement. It preserves these original failures rather than changing their result. Matched reference cancellation timing remains a separate acceptance gate.

## Execution and limits

All three runs used owned isolated headless Chrome processes and localhost servers, with normal UI/file input and real downloads. Nonlocal requests were blocked, all recorded input/file events were trusted, and no browser JavaScript assigned production geometry, camera, tool or document state. The read-only observation forwards original GL calls and restores bindings after readback. It checks the original 40-byte geometry/RGB/mask stream, the separate 8-byte material stream, default material uniform, and actual vertex-color display uniform.

Python independently reconstructs the exact f32/u32 GPU streams from all 66 downloaded projects and verifies every artifact hash. The recorded Chrome processes `13604`, `14360`, and `14577` exited successfully, their servers closed, and process absence was checked. No native GUI, accessibility, VoiceOver, user tab or user profile was controlled.

This batch covers one default positive stroke per channel selection, one retained material fixture, one locked PNG fixture, and one quad level per static mode. Full response/target/sign/pressure/alpha/culling/symmetry matrices, long sessions, physical devices, native GUI, visual equivalence and performance remain unverified. Quiet probes are asynchronous functional observations, not input-to-photon or release latency measurements. The frozen generic helper contains unused inherited contract wording; the selected groups, dedicated `paint_static` contract, and actual recorded cases define this batch's scope.

Original receipts remain under `.artifacts/sculptgl-paint-static-runtime`:

| Run | Original result |
| --- | --- |
| `candidate2-paint-static-90547ee5-908c-4aae-bf1a-2356cbbe515e` | Failed strict Escape equality after nine successful cases; 46 downloads/GPU pairs. |
| `candidate2-paint-static-tail-b387d122-e105-4e1b-b299-c38a29a87d3f` | Failed the same Escape gate with all probes retained; five downloads/GPU pairs. |
| `candidate2-static-only-bfa3a4dc-2b9a-4386-8848-37582ccf2ccd` | Passed startup and both unreached static groups; 15 downloads/GPU pairs. |
