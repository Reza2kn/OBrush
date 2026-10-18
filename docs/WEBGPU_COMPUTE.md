# WebGPU compute investigation

The later [C72 static-buffer trial](STATIC_UPLOAD_TRIAL_C72.md) tested whether avoiding unchanged WebGL2 index/material transfers was enough to help the delivered million-vertex browser workload. It removed those transfers but did not improve observed responsiveness, so its code was removed and the validated C71 WASM restored. This narrows the likely benefit of a renderer-only swap: full CPU geometry, publication, packing and queue work still need an integrated ownership design. The separate WebGPU experiments below remain prototypes, not the production sculpting path.

OBrush's delivered C58 build still uses Rust/WASM CPU geometry and a WebGL2 viewport. The following experiment establishes a possible compute direction; it does not deliver a GPU sculpting backend or close any SculptGL feature or performance gate.

## Actual experiment

An isolated Chrome 155 browser acquired its default Apple `metal-3` adapter, reporting a hardware adapter. No unsafe GPU flags, user browser tab or native GUI was used. The input was the complete independently authored 999,002-vertex / 1,998,000-triangle SGL fixture already used for sustained performance tests, SHA-256 `dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`.

A standalone Rust library, compiled both natively and to WASM, loaded that fixture using immutable C58 core code. It captured the fresh Source raw normal field, then evaluated an Inflate-like **pointwise arithmetic kernel** for 32 passes: center `(0,0,1)`, radius `0.5`, intensity `0.25`, raw U `1`. WGSL evaluated the corresponding f32 expression. Both routes retained f32 positions between passes and kept the captured normals fixed.

The experiment deliberately excludes contact preparation, spatial selection, front-facing selection, normal refresh, alpha, mirror, fold guards, topology changes, undo and rendering. Production CPU sculpting already gathers a local patch; the diagnostic CPU loop visits the entire array. Its ratio therefore cannot be interpreted as an application or production brush speedup.

Eight pairs alternated CPU/GPU order, with restores outside measured kernel intervals. Measured GPU intervals include encoding, submission and actual queue completion, rather than isolated hardware timestamps.

| Observation | Result |
| --- | ---: |
| Rust/WASM f64 arithmetic, median 32-pass interval | 84.65 ms |
| WebGPU f32 arithmetic, median 32-pass interval | 9.00 ms |
| Ratio for this standalone kernel | 9.41x |
| Initial upload, 27,972,056 bytes | 21.50 ms |
| Complete final-position readback, 11,988,024 bytes | 7.50 ms |
| Pipeline/setup compilation | 191.10 ms |
| Useful changed vertices, both routes | 41,065 |
| Maximum GPU-vs-WASM coordinate error after 32 passes | 2.384185791015625e-7 |
| RMS coordinate error | 4.763311596862317e-9 |
| Vertices with at least one different f32 word | 17,044 |

Every final coordinate was compared. All 957,897 vertices outside the original footprint remained bit-identical in both routes. Native Rust and WASM outputs matched every word, as did a separate NumPy f64 replay with f32 storage after every pass. A second independent audit decoded the entire original SGL, verified every input position/U word, and reconstructed all 1,998,000 face crosses and all 999,002 incident-face means; captured normal words matched exactly. This is fresh static-field and pointwise-kernel evidence, not reference brush parity or validation of refreshed normals.

The native reference calculation finished before all timed browser pairs. Native and browser initialization overlapped; native elapsed time is not used for a performance comparison. Owned browser processes exited with code zero and their servers closed.

Raw sources, WGSL, frozen parameters, binary inputs/outputs, timing samples, failed initial diagnostic compilation, complete numerical audits and process closure records are retained under `.artifacts/sculptgl-webgpu-kernel-feasibility-stage/`. The public evidence index is [the feasibility record](evidence/sculptgl-webgpu-kernel-feasibility-0.1.20.json).

## Architecture required for a useful product change

1. **Retain Rust/WASM and the GPU-independent core.** File IO, validation contracts, history and the scalar reference must continue compiling without a GPU. A compute backend should consume explicitly validated operation data and preserve a CPU fallback; an arbitrary public Mesh pointer or cached validation flag is insufficient authority across edits.
2. **Use one GPU API for compute and drawing.** The current WebGL2 renderer cannot directly consume a browser WebGPU storage buffer. Keeping positions on the GPU therefore also requires a WebGPU rendering path, rather than a compute shader followed by whole-mesh readback and WebGL upload every dab. A shared Rust graphics layer can support browser WebGPU and native GPU targets while keeping core behavior separate.
3. **Move the dependent pipeline, not just the arithmetic.** Picking, spatial selection, retained normal updates, bounds and selected-row tracking need an ownership/synchronization design. GPU f32 ray tests cannot silently replace current f64 predicates, original-face/quad precedence or tie ordering. Conservative candidate generation and the exact scalar fallback need explicit evidence, including pathological inputs. Dense validation and CPU copies left in the old per-dab path would retain the present lag.
4. **Make document and history ownership explicit.** Import/replacement must validate before installation. Direct CPU mutation, topology changes, load, undo/redo, export, device loss and backend switching must establish a coherent authoritative state. Readbacks, GPU snapshots and selected-row history need bounded allocation accounting and rollback; marking a queue empty before actual work is finished cannot satisfy the release gate.
5. **Measure numerical behavior before accepting a kernel.** This experiment demonstrates that ordinary GPU f32 arithmetic differs from the scalar f64 path. No new numerical tolerance has been accepted. Full tool/control fixtures, updated normals, stability/shape checks and pinned-reference comparisons must determine whether each implementation is acceptable on native and browser backends.
6. **Retest the original consumer workload.** Required evidence includes the unchanged normal consumer and history/interchange suites, full-size 60/120-second interaction matrix, memory, actual frame/display observations, finite/shape checks, complete saved outputs, undo/redo and device recovery. The existing million-vertex failures remain open: CPU-frame P95, independent RAF P95/max, and queue empty by 100 ms after release. Official builds and required native/WASM checks are still prerequisites for delivering a backend.

The small fixed-corner validation experiment was rejected separately: it preserved malformed-input results but produced no observable browser improvement. C58 remains the delivered build, and the original 317 SculptGL IDs plus five hosted controls retain their existing individual gates. ZBrush 2026.2.1 remains the longer-term objective.

## Shared Rust residency and drawing checkpoint (C60 prototype)

The next prototype uses a GPU-independent, immutable point-program input in staged `obrush-core` and one shared Rust `wgpu 27.0.1` implementation for native Metal and browser WebGPU. It uploads complete positions, fresh raw normals/U, RGB and canonical indices. Candidate computation, atomic acceptance, GPU checkpoint/cancellation and indexed drawing retain private buffer ownership. The current app's renderer has not been replaced.

On the same complete 999,002-vertex / 1,998,000-triangle fixture, 32 point passes ran and the indexed mesh was drawn **before any position readback**. A separate observer recorded the actual browser GPU API calls: candidate, commit and vertex drawing bound the same canonical position buffer; only a 48-byte uniform and a 4-byte failure word were uploaded between the start of computation and the completed draw. All 1,998,000 triangles were submitted. GPU checkpoint/cancel copies covered every position word.

Whole-output audits found:

- Native Metal and browser WebGPU final position buffers were bit-identical for this fixture. All four complete 512×512 images—before, after, cancel and recreated device—also matched pixel-for-pixel across backends.
- Cancellation restored all 15,984,032 position bytes and every original pixel. Explicit device destruction rejected subsequent draws; a newly created device reproduced the original image. This is not physical device-loss fault injection.
- A deliberately invalid candidate fixture covered 768 vertices across three workgroups. One invalid candidate blocked the complete commit across three passes, retaining every original position. Invalid CPU parameters were rejected before enqueueing point work.
- The current Rust f64 reference matched a separate complete NumPy replay exactly. GPU f32 arithmetic differed at 17,121 vertices, with maximum coordinate error `2.384185791015625e-7` and RMS error `4.742990071152209e-9`. No general numerical tolerance or brush parity was accepted.
- Full input positions, RGB, raw U and directed triangle indices matched an independent decode of the original SGL. Every captured raw normal matched the prior independently reconstructed complete static field.

The prototype retains 105,993,364 fixed GPU bytes on this fixture. Its per-viewport 256 MiB admission bound also accounts for one complete position readback. This is not an aggregate application/process memory qualification. GPU geometry never passes through a CPU coordinate array between compute and drawing; complete pixel readbacks are used for the diagnostic image audit.

The staged source passed 2,350 runtime tests, two compile-fail doctests, formatting, strict native/WASM all-target Clippy and the WASM target check. The standalone GPU crate also passed native/WASM all-target Clippy and formatting. Both official default-app scripts built successfully, the macOS artifact's ad-hoc signature verified, and an isolated browser smoke check rendered the default 40,962-vertex / 81,920-triangle sphere without runtime errors. Those default-app checks do not exercise a production GPU sculpting backend.

Failed preliminary compilation and browser artifact-transfer checks remain recorded. The first browser run completed its GPU phases but failed to publish an eleventh diagnostic download; a separate run with artifact download triggers spaced 1.1 seconds apart produced all eleven complete files. No geometry, shader, mesh size, input workload or original performance gate was changed by that transfer repair.

Sources, binaries, whole outputs, GPU API traces, failed checks, visual reviews and final check receipts are retained under `.artifacts/sculptgl-webgpu-resident-render-stage/`. [The C60 evidence record](evidence/sculptgl-webgpu-residency-c60-0.1.20.json) binds the exact artifacts.

This remains an internal prototype with fixed fresh normals and a basic diffuse offscreen renderer. Frontend surface presentation, full renderer features, current-surface picking, selected/ordered normal updates, the complete brush/control set and document/history synchronization still require implementation and comparison. C58 remains the delivered preview. No original SculptGL inventory or performance gate has been promoted from this work.

## Retained selected normal updates (C61 prototype)

The GPU point program now updates normals after every deformation pass. It follows the existing Rust cache update policy: recompute incident original faces, then update only selected vertex means in descending original-face order. Other cached vertex rows retain their previous words. Original quads keep one raw face normal and the explicit 0–2 rendering diagonal; compact face headers and descending incident rings preserve this distinction.

The GPU runs six dispatches per pass: coordinate candidates, affected face candidates, selected-mean validation, coordinate commit, face-cache commit and selected-vertex commit. Face candidates cross a physical f32 storage boundary before mean evaluation. An invalid candidate blocks all committed field writes. Checkpoints now cover positions, raw normal/U rows, original-face cache rows and cumulative selected IDs.

The scalar reference remains in staged GPU-independent `obrush-core`. Partial, empty and duplicate selections and a nonplanar mixed quad/triangle fixture match the existing private Rust normal updater exactly. This proves that reference contract on the named tests, rather than complete Source brush behavior. The GPU still uses a radial point program without ordinary Source first-touch proxy, contact, culling, alpha, mirror or fold-guard semantics.

On the complete original 999,002-vertex / 1,998,000-triangle fixture, 32 passes and subsequent drawing completed before any CPU readback of positions or retained caches. Actual browser API tracing verifies all 192 dispatches, their six-stage order, exact shader sources, the shared drawing buffers, and checkpoint/restoration copies for all four retained fields. The only CPU uploads inside that interval were the parameters and failure word.

All 24 shared complete native Metal/browser WebGPU files—inputs, reference data, edited/cancelled fields and three PNGs—were byte-identical. Cancellation restored every retained field and pixel. The four additional GPU cases cover fractional masks, negative intensity, a nonplanar mixed quad/triangle patch and fully protected vertices. A 768-vertex invalid-candidate case spans three workgroups; complete before/after word arrays independently confirm that positions and both normal caches remain unchanged and visited IDs stay clear.

An independent decoder verified every original SGL position, RGB/U word and directed face. The audit checked every incident-ring membership and strict descending order, reconstructed the initial raw face/vertex normals, and replayed all 32 f64 reference passes. Every reference position, vertex-normal, face-normal and visited word matched that replay.

The numerical gap remains explicit:

| GPU f32 versus the current f64 reference, after 32 passes | Maximum difference |
| --- | ---: |
| Position coordinate | 8.471310138702393e-6 |
| Raw vertex-normal component | 1.0369694791734219e-7 |
| Raw original-face-normal component | 1.5015393728390336e-7 |
| Defined normal direction | 0.003741311422097671 degrees |

The position-coordinate RMS difference was `1.3667943865679816e-7`; cumulative selected IDs matched exactly. No numerical tolerance or brush parity is accepted by reporting these differences. In this fixture, 957,897 unvisited vertex rows and 1,914,944 untouched face-cache rows retained every original word. The f64 retained vertex field differed from a fresh whole-mesh mean at 808 rows, demonstrating why a whole-mesh normal rebuild would change the intended cache policy. All 1,998,000 deformed triangles remained nonzero in area and retained their orientation relative to the original; this is not global self-intersection or ordinary-brush quality qualification.

The explicit resident resource size is 241,857,432 bytes; adding the largest complete readback reaches 265,833,432 bytes, within the unchanged 256 MiB per-viewport cap. The CPU upload snapshot also remains under its unchanged 128 MiB cap. These are application buffer/texture accounting limits, not driver memory, RSS or aggregate scene qualification. Further production buffers will require reuse or a different layout within the admission policy.

The staged workspace passed 2,352 runtime tests, two compile-fail doctests, formatting, strict native/WASM all-target Clippy and the WASM check. The GPU crate's strict checks pass. Both official default-app build scripts and macOS signature verification pass; an isolated browser smoke test confirms default-app startup and rendering. A test-only constant-assertion repair did not change either runtime binary; exact hashes bind the current source checks to the tested native/WASM binaries. Initial test-fixture, diagnostic macro and Clippy failures remain preserved.

Sources, full outputs, current audits, actual API traces and receipts are under `.artifacts/sculptgl-webgpu-selected-normals-c61-stage/`; [C61 evidence](evidence/sculptgl-webgpu-selected-normals-c61-0.1.20.json) indexes their hashes. C58 remains the delivered app. Actual viewport presentation, complete renderer capabilities, picking, full brush/control semantics, document/history integration, numerical policy and the original interaction/performance matrix remain required.

## Direct browser WebGPU canvas (C62 prototype)

The staged GPU crate now draws its canonical resident positions and updated selected normals into a real browser WebGPU canvas. Rust/WASM still owns fixture validation, scalar reference and operation setup; WebGPU owns the point-program dispatches and scene draw. The shipped C58 OBrush UI still uses the WebGL2 renderer and CPU geometry.

On the same complete 999,002-vertex / 1,998,000-triangle original fixture, an owned Chrome browser presented the updated 32-pass mesh at a 1920×1080 canvas backing size. The actual GPU trace ties the surface draw group to the canonical position, normal and color buffers and records a 5,994,000-index draw after the six-stage-per-pass compute path. No GPU buffer mapping occurred between update and presentation. A 512×512 screenshot crop of that canvas matched the current native Metal after image **pixel for pixel**; after an explicit logical device destruction and reconstruction from the immutable original fixture, its 512×512 crop matched the native before image exactly. This pixel comparison covers the named simple scene renderer, not the production studio renderer.

The canvas rejects a 2400×1350 request before changing its dimensions, suspends on zero size and resumes at 960×540. In the browser loss trial, destroying the owned test `GPUDevice` fired the Rust device-loss callback and subsequent presentation was rejected. Reconstruction reloaded the original fixture; it does not recover uncommitted GPU sculpt edits. The 24 complete native output files were unchanged from C61. The independent C62 audit compares the frozen 526 product-source files, complete native outputs, browser screenshots, actual GPU events and protected root hashes. A failed preliminary audit that accidentally included generated parent build outputs remains preserved alongside the corrected audit.

The prototype's explicit allocation accounting at 1080p is 241,857,448 resident bytes + 24,883,216 requested canvas/depth/uniform bytes + a reusable 1,048,576-byte diagnostic readback buffer = 267,789,240 bytes, under the unchanged 268,435,456-byte cap. Whole diagnostic outputs use repeated bounded GPU copies; presentation does not use that readback. This accounting does not measure driver/compositor memory or process RSS. GPU crate formatting, both unit tests, strict native/WASM all-target Clippy and the WASM release build passed. [C62 evidence](evidence/sculptgl-webgpu-surface-c62-0.1.20.json) binds the audit and generated artifacts.

This is not yet a shipped WebGPU OBrush backend. Full studio UI, picking, all brush inputs and semantics, document/history transactions, native surface GUI, consumer frame/latency measurements, interchange and reference comparisons remain open. The original SculptGL 60/120-second performance failures are unchanged.

## Current resident surface picking (C63 prototype)

The isolated shared Rust GPU crate can now query its **current deformed surface** without downloading the mesh. Two compute passes intersect a ray against the resident triangle and position buffers: the first selects the nearest positive f32 distance, and the second chooses the lowest original face ID at that exact distance. The diagnostic browser camera turns a canvas pixel into a validated ray. Each query maps an 8-byte distance/face result. This is a first bridge from browser pointer coordinates to a GPU-resident brush center; the production camera, multi-part pick policy, stroke input and undo are not connected.

The full original 999,002-vertex fixture was picked before deformation, after 32 GPU point passes, and after another point dab centered on a picked point of the **already deformed ridge**. Native Metal and browser WebGPU matched every face, distance and world-position word on six named samples. An independent f64 ray search across all 1,998,000 triangles found the same face IDs; the largest f32-vs-f64 hit-distance difference was `2.104789362800119e-7`. The final native/browser full position buffers and actual 512×512 browser/native RGBA pixels matched exactly. The ridge dab changed 927 vertices, with no zero-area or reversed triangles in this fixture. Background and out-of-bounds picks were rejected; destroying the owned browser GPU device caused the Rust loss callback to reject another pick.

The actual browser API trace shows four 8-byte copies/maps for four after-32 picks and no whole-mesh or image readback before the edited frame was presented. A complete diagnostic position export occurred afterward. The original C62 24 native output files remain byte-identical. At 1080p, explicit resident/canvas/scratch requests total 267,789,296 bytes under the unchanged 256 MiB admission cap; this excludes driver/compositor allocations and RSS. A named browser sample measured approximately 91 ms for the first post-compute pick and 1–1.4 ms for three subsequent picks. Those timings mix queue completion and isolated test conditions, so they do not qualify the original 60/120-second consumer latency gate.

Formatting, three GPU crate unit tests, strict native/WASM all-target Clippy, native release and WASM release passed. [C63 evidence](evidence/sculptgl-webgpu-picking-c63-0.1.20.json) indexes the full buffers, screenshots, shader, API trace and independent numerical audit. The delivered app and every SculptGL parity/performance gate remain unchanged. The next integration must use OBrush's actual camera and validated document/stroke transaction, preserve CPU fallback and undo, and compare full brush semantics rather than treating this point program as a production Inflate brush.

## Actual studio camera bridge (C64 prototype)

The isolated WebGPU presenter now accepts the production `SourceCameraFrame` local-to-GL-clip matrix and its f64 local screen ray. It validates the 4×4 matrix and exact draw-target dimensions before changing either GPU camera uniform, then converts only GL clip depth from `[-1,1]` to WebGPU `[0,1]`. Perspective and orthographic cases use the actual staged OBrush camera source, not an approximate camera in the test driver. Resizing invalidates the old camera; presentation and picking reject until the caller supplies the new frame and matching dimensions. Bad matrices and mismatched dimensions leave the prior frame intact.

On the unchanged 999,002-vertex / 1,998,000-triangle fixture after 32 GPU point passes, native Metal and browser WebGPU agreed on all four visible face IDs and every f32 distance/position word, while two background rays missed. An independent f64 search over **all** triangles found the same faces. The largest absolute f32 distance difference was `0.0002400117357410636`, on the orthographic ray whose source origin is about 2,000 scene units away; maximum reprojected screen-point error was `0.003297279501396133` pixel. Perspective errors were below `6e-8` in distance. The 512×512 orthographic native/browser RGBA images matched exactly; perspective differed in two byte channels by one level. A resized 960×540 perspective frame also produced the f64 oracle face and projected within `0.01` pixel.

The owned browser trace recorded four indexed draws and seven 8-byte pick copies/maps; no full mesh or image was mapped before presentation/picking. It showed two 96-byte camera uniform writes for each camera mode. A rejected invalid/mismatched request preserved the previous 512×512 browser screenshot exactly. The 24 complete C63 diagnostic native files remain byte-identical, and all 526 frozen product-source files remain unchanged. At 1080p, explicit resident/canvas/scratch requests total 267,789,456 bytes under the unchanged 256 MiB cap. This does not measure driver/compositor memory or process RSS.

The separate GPU crate passes formatting, 16 all-target native tests (including 12 tests from the included production camera module), strict native and WASM Clippy, native release and WASM release. One earlier owned-browser rerun failed because its existing isolated profile directory could not be reused; that failed receipt is preserved, and a fresh owned profile passed. [C64 evidence](evidence/sculptgl-webgpu-camera-c64-0.1.20.json) indexes the independent audit, binaries, source hashes, screenshots and API trace. This still is not the shipped OBrush studio path. The actual document/stroke transaction, full brush semantics, CPU fallback, undo, scene transforms, long-session responsiveness and SculptGL parity gates remain open. The orthographic f32 ray-origin error also needs a bounded precision strategy before using this picker for fine sculpting.

## Bounds-aware current-surface ray rebasing (C65 prototype)

The shared Rust GPU picker now keeps a conservative local bound of the **current** point-program mesh. It starts with every validated imported vertex, and each accepted point operation expands the bound to include its full influence sphere plus its maximum possible candidate displacement and a rounding allowance. This means an edited ridge outside the original mesh bounds is still covered without downloading positions. Before a local camera ray reaches the GPU, a f64 slab intersection finds that bound; a small outward margin permits the ray origin to move close to the surface. The GPU still performs its two f32 full-triangle passes and returns only eight bytes. Rust adds the f64 offset back to preserve distance from the original camera ray and reconstructs the local hit position before the final f32 position conversion. This bound rule is proved only for the staged point program; future brush kernels need their own conservative update rule.

On the same complete 999,002-vertex / 1,998,000-triangle fixture after 32 passes, the original maximum `z=1.0` became `z=1.3219239711761475`; the operation bound reached at least `z=1.5500105`. Two orthographic ray origins previously about 2,000 units away were rebased by about 1,995.6 units. An independent f64 search over **all** triangles still found the same faces for the four visible 512×512 samples and the resized 960×540 sample. The largest distance error fell from C64's `0.0002400117357410636` to `1.5535215425188653e-7`—about 1,545× smaller on these samples. Maximum position error was `1.6122385204653256e-7`; maximum reprojection error was `4.362458355444687e-6` pixel.

Native Metal and browser WebGPU returned the same face and **f64 distance words** and the same f32 position words for all four shared named hits; two background rays missed. Orthographic 512×512 RGBA matched exactly, while perspective differed in two byte channels by one level. The browser trace again shows four indexed draws and seven 8-byte pick copies/maps, with no whole-mesh or image readback before picking. Resize and bad-camera rejection remain effective; the rejected request preserved the previous browser screenshot exactly. All 24 C64 diagnostic native output files and all 526 frozen product-source files remain byte-identical. GPU crate formatting, 17 native tests, strict native/WASM Clippy, and native/WASM release builds passed. [C65 evidence](evidence/sculptgl-webgpu-rebased-pick-c65-0.1.20.json) binds these checks and artifacts.

This is still a separate point-program prototype. It does not yet run the shipped studio's brushes, subtools, transformed scene picks, history/undo, CPU fallback or file transactions. The original SculptGL interaction/performance matrix remains failed or unverified; the C65 numerical samples are not a general fine-stroke tolerance or SculptGL/ZBrush reference-parity result.

## C67 product bottleneck check

The shipped C67 CPU picker removes the verified retained primary path's original-face walk, but the frozen 999,002-vertex/60-second browser cohort still fails the same four responsiveness gates as C66: 106.9 ms summed CPU frame P95, 107.7 ms independent RAF interval P95, 451.4 ms RAF maximum, and no cleared release queue by 100 ms. A separate native M2 release probe measured useful dab preparation at 15.672–24.720 ms and application at 40.458–58.157 ms after a 353.509 ms first application. Those native timings are not a browser breakdown and the two browser runs delivered different coalesced contacts. They motivate integrating the *whole* validated deformation, normal update, selection, presentation and history pipeline with GPU-resident state; moving only the ray test or arithmetic kernel cannot establish the original real-time gate. [C67 report](RETAINED_FACE_ROWS_C67.md) and [evidence](evidence/sculptgl-retained-face-rows-c67-0.1.20.json) retain the exact observations and failures.

C68 removes a second full-position proposal copy from each useful Source dab while retaining exact old/new complete projects on eleven tool/guard cases and a five-dab million-vertex fixture. The same frozen browser cohort still fails four responsiveness gates at 108.8 ms CPU frame P95. Different delivered contact counts prevent a controlled C67/C68 ratio. This is additional evidence that local CPU allocation cleanup alone does not qualify dense real-time sculpting; the integrated GPU/state transaction remains the next architectural gate. [C68 report](SPARSE_SOURCE_C68.md) and [evidence](evidence/sculptgl-sparse-source-c68-0.1.20.json) preserve the measurements.

C69 also removes the separate full-position candidate clone and full-array rescale inside the CPU fold guard. Frozen whole project outputs remain exact, while its 1M/60s browser run still fails the same four responsiveness gates at 103.2 ms CPU frame P95. The delivered motion count changed, so the numerical difference is not a controlled improvement. This does not connect the isolated WebGPU point program to real Source deformation, normal update, document or undo. [C69 report](SPARSE_SOURCE_GUARD_C69.md) and [evidence](evidence/sculptgl-sparse-source-guard-c69-0.1.20.json) retain the checks.

## Current-workspace GPU bridge (C112)

The former C65 prototype source has been promoted into the current workspace as `obrush-gpu`, with GPU-independent validated point/normal input in `obrush-core`. A newly built root-workspace native Metal/browser WebGPU pair reproduces exact six-ray outcomes and pixel-identical perspective/orthographic images on the original million-vertex fixture. The browser trace retains positions through compute and drawing with only bounded 8-byte pick maps. The [C112 report](WEBGPU_WORKSPACE_C112.md) and [evidence](evidence/sculptgl-webgpu-workspace-c112.json) bind source, binaries, numerical outputs, actual GPU calls and checks.

The delivered studio does not call this GPU crate yet. Full brush semantics, document/history ownership, fallback, recovery and the unchanged sustained performance matrix remain the required product integration.

## Checked document capture (C113)

`ResidentDocumentInput::capture` now validates and owns one visible active subtool from the ordinary OBrush `Document`, including its ID and exact local-to-world pose. The source document remains authoritative. Fresh protected masks are converted to transient semantic influence with an explicit count of f32 words that would not roundtrip; the raw source masks remain unchanged. [The C113 report](WEBGPU_DOCUMENT_CAPTURE_C113.md) records the scoped tests and open integration work. The normal studio still uses CPU/WASM geometry and WebGL2.

## Versioned-project browser WebGPU probe (C114)

The isolated GPU owner now accepts a checked `.obrush` snapshot, composes its saved object pose with a world camera in f64, and converts world pick rays to local GPU rays in shared Rust. On one five-point reflected/sheared project, seven browser/native GPU picks match and both complete 512×512 RGBA images match exactly; the core CPU picker finds the same seven faces. [The C114 report](WEBGPU_DOCUMENT_C114.md) records the fixture, numeric tolerance and limitations. This does not switch the normal studio from WASM CPU geometry/WebGL2 or close its sustained responsiveness gate.

## Reflected/sheared object lighting in the GPU probe (C115)

The isolated draw shader now uses the saved pose's inverse-transpose 3×3 normal transform. Its 160-byte view uniform keeps the GPU camera and normal transforms separate; world translation affects positions and picks through the camera, but cannot skew normal directions. On an independently modified nonplanar five-point project, the transformed result changes 23,120 pixels versus the old local-normal baseline. Browser WebGPU and native Metal still agree on complete 512×512 RGBA output and all seven picks. The unchanged million-vertex local-camera fixture retains its earlier perspective and orthographic image hashes. [The C115 report](WEBGPU_AFFINE_NORMALS_C115.md) gives the inputs, limits and validation. The normal studio remains WASM CPU geometry plus WebGL2.

## Checked GPU-result document handoff (C116)

`obrush-core` can now accept an explicit 16-byte-per-vertex resident position readback into the matching live document as one undoable edit. Admission streams a SHA-256 fingerprint of the full source mesh at capture and at commit, checks active subtool/visibility/pose, validates every f32 component and fourth word, retains protected vertices, and applies the existing surface fold guard before changing the document. Open strokes, retained levels/layers/morph targets and a redo branch reject; an exact no-op preserves redo. A native Metal point pass on an independently constructed sphere changed seven vertices, matched the scalar reference exactly in its three position components, and completed exact JSON Undo/Redo. [The C116 report](WEBGPU_DOCUMENT_COMMIT_C116.md) records the scope. The normal browser still does not call this bridge, and a whole-position readback per stroke is not a scalable production GPU editing path.

## Browser WebGPU to shared WASM document handoff (C117)

The isolated browser GPU crate now also exposes a separate `ResidentDocumentSession` backed by the ordinary shared-core project decoder and history. Its project input is independent of the canvas owner's copy; the browser obtains actual WebGPU positions, passes those bytes to the checked core commit, and compares the result with a native Metal run on the same versioned sphere fixture. All 2,592 GPU position bytes and the complete accepted `.obrush` JSON match exactly across native/browser. Invalid and repeated stale results reject; Undo/Redo restores the exact source/accepted projects. With a front-facing camera, both the actual canvas and synchronized offscreen draw change 20,146 pixels. [The C117 report](WEBGPU_BROWSER_COMMIT_C117.md) retains the preceding failed camera checks and exact limits. This still does not integrate WebGPU into the normal studio or close the sustained SculptGL responsiveness gate.

## Sparse GPU-result document boundary (C118)

The shared core and browser WASM session now accept sorted, unique vertex-ID/RGBA32F records as one validated undoable edit. On the C117 sphere fixture, seven changed vertices occupy 140 sparse payload bytes and produce the same complete accepted project as the 2,592-byte full result, with invalid/stale rejection and exact Undo/Redo. The browser currently builds those records **after reading back the full GPU buffer**, so no GPU-transfer or latency gain is claimed. [The C118 report](WEBGPU_SPARSE_COMMIT_C118.md) records the exact source/build hashes, browser result and remaining GPU compaction/normal-studio work.

## GPU-owned sparse readback (C119)

The isolated viewport now counts and compacts changed vertex IDs/positions against its GPU checkpoint, then maps only the 4-byte count, 140-byte result and 4-byte post-count for the seven-vertex fixture. Browser WebGPU and native Metal return identical sparse bytes and accepted projects; browser tracing shows zero full-position readbacks. [The C119 report](WEBGPU_COMPACT_READBACK_C119.md) records the initial checkpoint-usage validation failure, corrected build, exact trace and limits. This moves the transfer boundary, but the normal studio still uses CPU/WASM plus WebGL2 and no end-to-end brush speedup has been measured.

## Million-vertex sparse capacity (C120)

On the unchanged 999,002-vertex SGL fixture, one diagnostic point pass changes 41,065 vertices. Native Metal's 821,300-byte compact output matches a complete 15,984,032-byte GPU position readback row for row; browser WebGPU produces the same sparse hash while mapping only `[4,4,821300,4]` bytes and zero full positions. [The C120 report](WEBGPU_MILLION_SPARSE_C120.md) records exact inputs, measured transfer sizes and the one-shot timing limitations. This still does not close normal-studio sustained responsiveness or SculptGL brush parity.

## Normal studio document handoff (C121)

The browser app now captures its actual shared-core document under a one-use token and accepts a WebGPU-produced sparse result through the same checked document/history path. On the normal 40,962-vertex startup sphere, an offscreen GPU pass changed 2,511 vertices; the app rejected corrupt and stale inputs, updated its saved project, redrew the visible WebGL viewport, and ordinary keyboard Undo/Redo restored the sphere's before/after pixels exactly. The official web build packages the separate GPU module lazily. [The C121 report](WEBGPU_STUDIO_HANDOFF_C121.md) binds the frozen build and browser checks. Input and visible rendering still use CPU/WASM and WebGL2, so studio speed and SculptGL parity remain open.
