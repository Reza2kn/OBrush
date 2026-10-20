# OBrush

An open-source sculpting studio written in Rust for desktop and the browser, targeting **ZBrush desktop 2026.2.1**. The **0.1.20** source adds editable SGL scene interchange, explicit source framing and original-face normal policies, environment rendering, and opt-in ordinary Paint controls to the existing sculpting foundations. The [current verification checkpoint](docs/SCULPTGL_0_1_20_CHECKPOINT.md) distinguishes tested candidates, browser checks and remaining limitations. **SculptGL and ZBrush parity remain unfinished**; the [current 317-ID ledger](docs/SCULPTGL_CURRENT_LEDGER.md) tracks scoped evidence and the remaining full gates.

OBRush is being prepared as an **open public alpha** for everyone to try and improve. Read the [alpha announcement, known issues and platform support](docs/PUBLIC_ALPHA.md) before relying on it for a project. Repeated Draw can saturate, worked surfaces can look jagged, small strong Move grabs can leave sharp triangles, and large browser meshes can lag. Save `.obrush` projects frequently: there is no autosave or crash recovery. The [public package checklist](docs/PUBLIC_ALPHA_GATE.md) tracks what still needs verification; a hosted build or downloadable installer is not yet linked here.

## Run

```sh
cargo run --locked -p obrush-app --release
```

On macOS, create a double-clickable local app:

```sh
scripts/package-macos.sh
open .artifacts/OBrush.app
```

The macOS bundle is built for the local host architecture and ad-hoc signed for local use. Windows/Linux targets use the same app source but have not been executed in this milestone. Linux needs its normal graphics/window-system development libraries.

## Browser

The browser uses the same Rust engine compiled to WebAssembly and the same viewport rendered with WebGL2. No backend or external API is required for sculpting.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked --root .tools/wasm-bindgen
scripts/build-web.sh
python3 scripts/serve-web.py
```

Open **http://127.0.0.1:8787**. The server binds only to localhost and serves `web/dist`, not the repository. A modern browser with WebGL2 enabled is required. The glue generator must match the version of `wasm-bindgen` in `Cargo.lock`; update the script/tool together when upgrading dependencies. The build is static and can be hosted as the contents of `web/dist`.

Use **Open** to import a local `.obj`, `.ply`, `.stl` or `.obrush` file; **Save project** and the active-mesh export controls download files. Browser downloads are user-managed saves. Reloading/closing the page loses unsaved work; autosave and recovery are planned.

## Working features

- Separate sculptable parts: add sphere/cube/cylinder/torus, append OBJ/PLY/STL, select, duplicate, rename, reorder, hide, solo, merge chosen parts and delete, with independent levels/layers/targets and scene history.
- GPU studio-shaded viewport; orbit, spherical/plane rotation, pan, bounded raw-wheel zoom, frame scene, orthographic/FOV and axis views; portable wireframe overlay and multisample edge antialiasing.
- Draw, Clay, Inflate, Smooth, Flatten, Crease, Move, Mask, Paint, Morph, Pinch, Twist, Local Scale and Drag brushes.
- World or fixed-screen radius/strength controls, inverted strokes, adjustable hardness/culling and offset X symmetry; Draw Clay/Accumulate, tangent Smooth, connected/normal Move and absolute locked stamps.
- Painted masks, clear/invert mask and per-vertex color painting.
- A 40,962-vertex / 81,920-triangle default sphere; selectable editable density for generated spheres, cubes, cylinders and tori.
- Persistent Loop subdivision levels, coarse-to-fine editing and exact hierarchy undo/redo.
- Geometry sculpt layers with recording, 0–100% intensity, visibility, rename/delete and baking the visible shape; complete layer history and versioned project persistence.
- Voxel remeshing of closed solids (including smooth Surface Nets and stepped Block modes) and constrained triangle decimation, with cancellable background jobs.
- PNG brush alphas with rotation; pen pressure can modulate strength and radius.
- Draw, Clay, Inflate, Flatten and Crease automatically refine stretched/coarse edges near the brush, preserve masks/colors and group the whole refined stroke into one undo step. Persistent subdivision stacks keep fixed topology instead.
- Evenly spaced dabs along strokes, with sampling remainder preserved between input updates and path breaks at canvas/Move transitions.
- Deformation checks that reduce or reject steps that would reverse/collapse faces or create extreme adjacent folds.
- Move captures a fixed material patch, adds bounded local detail before deformation, and applies total displacement on the initial camera plane. Returning to the start restores the original mesh exactly; subdivision stacks retain fixed topology.
- Whole-stroke undo/redo, bounded to 64 operations / 128 MiB of retained mesh history.
- OBJ import/export, polygon triangulation, negative indices, vertex RGB and ZBrush MRGB color import.
- A geometry-only Morph Target stores a shape at its subdivision level, with full restore/swap, local masked Morph brushing, target replacement/delete and undo.
- Version 5 `.obrush` JSON projects preserve ordered SubTools, selection, names, visibility and Solo, with independent mesh/editable state. Single-object Version 4 projects preserve a Morph Target and optional geometry layers alongside mesh geometry, masks, colors and subdivision levels. Layer-only projects remain version 3, plain projects remain version 2, and versions 1–3 remain readable.

Brush names describe OBrush's current operations. Their response curves and parameters have not been matched to ZBrush. Subdivision includes Loop triangles and source-static triangle/quad stencils. New source-static levels transport fine detail in a surface frame on the tested fixtures; Loop and older static levels retain world-space offsets. Fine edits do not reverse-project into coarse levels, and complete multiresolution and HD editing remain gaps. A scene supports up to 32 separate parts; folders, split, persistent transforms/pivots, general multi-selection and booleans remain gaps. Merge uses a separate explicit set of chosen parts. OBJ exports the active part but cannot preserve masks or the full project; use `.obrush` for complete editable state. Face-corner UV and material interchange remain format-specific and only partly verified. History, camera and app settings are session state and are not serialized.

For finer sculpting, select the density for the next **Sphere**, or use **Add subdivision level** on the current mesh. **Lower** and **Higher** select editable levels. Adding a level rounds corners and can shrink the shape or soften existing detail; undo restores the previous hierarchy. **Refine without smoothing** adds editable points while preserving the existing angular surface and replaces any stored levels, with undo available. Small brush marks still require enough geometry; antialiasing improves screen edges without adding sculpt detail.

**Smooth viewport edges** requests 4× offscreen multisampling, with 2× and direct-rendering fallbacks when format, device or memory limits prevent it. The inspector reports the allocated sample count. See [VISUAL_QUALITY.md](docs/VISUAL_QUALITY.md) for the original roughness measurements, changes and verification limits.

Crease now pinches more gently along the local surface, and geometry dabs fade at grazing angles. The engine checks affected triangles and their shared edges before committing a step. These checks do not detect every global self-intersection or repair an already damaged mesh. Strong/repeated edits can reach a limit where the engine reduces or rejects further movement. Geometry dabs touching an edge with more than two incident faces are rejected; masking/painting and edits to unrelated components remain available. See [CREASE_STABILITY.md](docs/CREASE_STABILITY.md) for retained failures, numerical comparisons and limits.

Draw raises a coherent surface patch and blends the directions where mirrored footprints overlap. Crease also blends mirrored directions, follows a coherent depression direction and relaxes small surface irregularities. Draw, Clay, Inflate, Flatten and Crease split long local edges after an accepted dab. Move prepares local detail before its first accepted displacement, then holds that topology and material influence fixed during the drag. Smooth, Mask and Paint retain their current topology. Local refinement preserves the piecewise-linear surface; continuous dabs use at most 1,024 added vertices per call and Move preparation uses at most 2,048. Automatic refinement is split-only. Manual decimation and voxel remeshing are separate operations; both replace any subdivision stack and can be undone. [DRAW_QUALITY.md](docs/DRAW_QUALITY.md) records the historical Draw experiments; [LONG_STROKES.md](docs/LONG_STROKES.md) records the sustained-input and Crease checks.

**Current alpha limitation:** repeating default Draw at the same screen point
eventually stops adding height, even on a fresh sphere. A [256-contact
reproduction](docs/DRAW_STATIONARY_BUILD_GATE.md) first stops changing at
contact 96. The mesh-quality guard prevents a worse triangle, but this is not
the intended brush response; no safe fix has been shipped. Worked-area
jaggedness and repair of previously folded geometry also remain open. See the
[public-alpha gate](docs/PUBLIC_ALPHA_GATE.md) before inviting testers.

Move captures radius, symmetry, alpha, facing and mask protection at the start of each gesture. Strength and pressure scale its absolute target. Shift/Ctrl tool changes or an Alt inversion change start a new undoable gesture on the current surface; they do not overwrite preceding edits. Movement outside the canvas is paused, and re-entry resumes without adding the outside travel. Larger brushes suit broad shape changes; extreme pulls with a tiny brush can be limited to protect the surface. Existing damaged geometry is not automatically repaired.

[MOVE_QUALITY.md](docs/MOVE_QUALITY.md) records the reported settings, retained failures, measured Move correction and remaining range limits.

Use **New layer** in the Object inspector to record geometry edits separately. Fade or hide the layer to change its contribution, or stop recording to edit the base. Recording requires a visible layer at 100%. Intensity commits on slider release as one undo step. Layers keep topology fixed and retain any existing subdivision stack; use **Bake visible** before changing levels or rebuilding the surface. Baking keeps the visible shape and removes all layers, including hidden ones, with undo available. Masks and paint remain ordinary document channels. [SCULPT_LAYERS.md](docs/SCULPT_LAYERS.md) describes the supported geometry workflow and remaining reference gaps.

Under load, the app coalesces older unprocessed brush samples to keep recent movement responsive. It retains at most 32 pending dabs and checks their age before interactive processing. Release commits the visible stroke and at most its latest pending endpoint, then clears pending work. This deliberately does not replay every captured sample when processing falls behind. A single geometry operation can still exceed the redraw budget; large meshes and save/export remain performance work.

Use **SubTools** to add or append parts. Click a row or visible part to select it; each part keeps its levels, layers and target. **Solo active part** filters visibility temporarily; show a hidden active part before sculpting. New primitives are placed beside the visible scene, while duplicated parts and appended OBJ coordinates are preserved. **Save project** saves the whole scene; **Export active OBJ** saves its active mesh. [SUBTOOLS.md](docs/SUBTOOLS.md) describes this foundation and its limits.

Use the Transform inspector to translate, rotate and uniformly scale the active part. Double-click values to type them, then Apply. **T** enables world-axis and camera-plane translation handles; **Escape** cancels a preview. Release commits one undo step. Every retained level, layer and target follows the whole-part change; masks and paint stay attached. [TRANSFORMS.md](docs/TRANSFORMS.md) defines the math, command barriers and remaining gaps.

Paint a mask, then use **EXTRACT MASK → Create part** to make a separate surface or shell. Threshold includes lighter mask weights; zero thickness creates an open patch, positive thickness expands outward, and negative thickness extrudes inward, within −5..5 world units. The original and its mask stay intact, and undo restores the scene. The new part is unmasked and independently editable. [MASK_EXTRACTION.md](docs/MASK_EXTRACTION.md) defines supported topology, numerical rules and remaining extraction controls.

Use **Grow**, **Shrink**, **Blur** and **Sharpen** in the left MASK section to refine the active mask. Passes repeats the operation; Amount reduces its per-pass change. Defaults are one pass at full amount. Each changed result is one undo step; after pending valid edits finish, unchanged results preserve redo. These controls keep current geometry, paint, layers and stored shapes intact. [MASK_FILTERS.md](docs/MASK_FILTERS.md) defines the independent scalar policies and remaining reference gaps.

Choose **Rectangle** or **Lasso** under MASK and drag in the viewport to select a larger region. The outline previews a selection through the active part's depth; release applies a changed selection as one undo step. Hold Alt at the start to erase, or Escape to cancel. **Brush** restores ordinary mask painting; Ctrl temporarily uses the chosen mask mode with other brushes. [MASK_REGIONS.md](docs/MASK_REGIONS.md) defines projection, command cancellation and work limits.

## Controls

| Action | Control |
| --- | --- |
| Sculpt | Left drag on mesh |
| Smooth temporarily | Shift + drag |
| Mask temporarily | Ctrl + drag |
| Invert temporarily | Alt + drag |
| Select brush | 1–9, Morph 0, Pinch P, Twist J, Local Scale L, Drag G |
| Brush radius | `[` / `]` |
| Orbit / pan / zoom | Right drag / middle drag / scroll |
| Frame / X symmetry / wireframe | F / X / W |
| Transform mode / cancel preview | T / Escape |
| Undo / redo | Cmd or Ctrl Z / Cmd or Ctrl Shift Z |
| Save / open | Cmd or Ctrl S / Cmd or Ctrl O |
| Help | H |

Browser pen pressure modulates strength by default; radius modulation is optional. Mouse input retains full nominal pressure. Native force events are consumed when the platform provides them, but physical tablet input, calibration and touch gestures remain unverified. **Import PNG alpha** loads an 8-bit grayscale weight image, up to 1024 × 1024 pixels / 16 MiB; white sculpts, black/transparent pixels protect. Rotation is adjustable. Alpha detail depends on geometry density; brush presets and alpha persistence remain gaps. Small windows expose object/surface controls through the compact toolbar.

Store a shape with **Store target** in Object before sculpting. **Morph** (**0**) brushes back toward it; **Restore shape** recalls all saved points, and **Swap** exchanges the current and stored shapes. Masks protect local Morph strokes, while full Restore keeps paint/masks and restores every point. Targets remain tied to their creation subdivision level; explicit refinement/rebuilding removes them with undo. See [MORPH_TARGETS.md](docs/MORPH_TARGETS.md) for persistence and limits.

## Architecture

```text
crates/obrush-core  — pure Rust mesh, brushes, picking, validation, history and IO
crates/obrush-app   — egui application, shared OpenGL/WebGL renderer and camera
web/index.html     — minimal WASM host, file upload/download and startup errors
scripts/           — native packaging and reproducible static browser build
```

The core is independent of a UI/GPU and can be tested without a display. GPU uploads cache each visible part by stable ID and immutable mesh identity; the browser and native app run the same geometry code. Ordinary Source primary picking prunes triangle tests through retained current-face bounds and uses retained first-triangle rows to visit only candidate original faces. Useful Source dabs store selected-vertex proposals instead of a second full position copy, and the fold guard rescales only those proposed rows. Full mesh validation and state matching still scan the mesh; unretained and mirrored paths remain separate. The declared validation limits are not real-time performance guarantees. See [the C66 sustained-performance report](docs/SOURCE_PICK_ACCELERATION_C66.md), [C67 face-row follow-up](docs/RETAINED_FACE_ROWS_C67.md), [C68 allocation follow-up](docs/SPARSE_SOURCE_C68.md), and [C69 guard follow-up](docs/SPARSE_SOURCE_GUARD_C69.md).

The core refuses more than 1 million vertices, 2 million triangles or a 128 MiB project/input. All parts and subdivision levels share these aggregate limits and are capped at eight levels. Total runtime memory also includes the active mesh, a stroke snapshot, adjacency, GPU buffers and rendering snapshots; the history budget is not a process-memory limit. Remeshing and decimation run on a native thread or a separate browser worker. Input/output serialization and other large operations still run on the UI thread. Decimation accepts at most 250K vertices / 500K triangles; remeshing accepts at most 500K source triangles and resolution 16–128 with cell/work limits. Normals and deformation guards use local face incidence; broader spatial acceleration and partial GPU uploads remain performance work.

## Development and verification

```sh
cargo fmt --all -- --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p obrush-app --target wasm32-unknown-unknown
cargo clippy --locked -p obrush-app --target wasm32-unknown-unknown -- -D warnings
cargo run --locked -p obrush-core --example core_timing --release
```

The frozen **0.1.15 candidate 2** passes **641 tests** (563 core, 78 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. [Candidate 1](docs/SCULPTGL_HOST_CANDIDATE1.md) records individually completed brush/options/camera/scene/file/stamp workflows and preserved failed runs; [candidate 2](docs/SCULPTGL_HOST_CANDIDATE2.md) verifies rapid navigation, bounded wheel input and useful Draw on newly detailed primitives. These local cases do not establish SculptGL parity. The complete [SculptGL checkpoint](docs/SCULPTGL_PARITY.md) retains the frozen gaps and required paired quality/performance gates. Native GUI and physical devices remain unverified for these candidates.

The newer **0.1.16 candidate 2** passes **785 workspace tests** (703 core, 82 app) and the required native/WASM checks and artifact builds. [Initial attribute/browser evidence](docs/SCULPTGL_ATTRIBUTES_CANDIDATE1.md) covers material/UV data, rendering, export-loss controls and malformed-file recovery, including a retained OBJ consumer failure. [The corrected candidate](docs/SCULPTGL_CORE_CANDIDATE2.md) passes that actual OBJ UV/quad return without relaxing its gates. [Paint and static-subdivision controls](docs/SCULPTGL_PAINT_STATIC_CANDIDATE2.md) have scoped browser evidence with a retained Escape frame-transition timing failure; the full reference, native GUI and performance checkpoint is still open.

The current **0.1.17 candidate2** passes **949 workspace tests** (833core,116app) with required native/WASM checks and artifacts. [Its foundations report](docs/SCULPTGL_FOUNDATIONS_0_1_17.md) records normal Fill/picker/subdivision-deletion/UV/rendering/PNG consumer checks,130 exact saved-scene/GPU reconstructions and preserved initial failures. Candidate2 repairs clipped rendering controls; texture image equality uses an observed normal notice timeout and unchanged gates. Per-tool preference browser checks, matched reference response/pixels, native GUI and long-session performance remain open.

The historical **0.1.14** record passes **422 tests** (365 core, 57 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Eight region-mask workflows and 59 behavior gates pass on the frozen browser build, including independently projected mask bits, held previews, cancellation, undo/redo, active-part isolation, schema roundtrips and a region/filter/extraction workflow. All 115 saved snapshots match visible GPU vertex and solid index buffers (127 exact visible buffer pairs). [MASK_REGIONS.md](docs/MASK_REGIONS.md) and its [build receipt](docs/evidence/mask-region-build.json) retain exact parameters, hashes and failed checks. Reference selection behavior, native GUI, physical devices and large/long-session performance remain unverified.

Historical version **0.1.13** passed **361 tests** (320 core, 41 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Eight mask-filter workflows and 45 behavior gates passed on the frozen browser build, including independent scalar masks, exact geometry/layer/target preservation, inactive buffers, undo/redo, schema roundtrips and disabled controls. All 99 saved snapshots matched visible GPU vertex and solid index buffers. [MASK_FILTERS.md](docs/MASK_FILTERS.md) and its [build receipt](docs/evidence/mask-filter-build.json) retain numerical results and failed helper checks. Reference filter behavior, native GUI, physical devices and large/long-session performance remain unverified.

Historical version **0.1.12** passed **323 tests** (285 core, 38 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Eight extraction workflows and 53 behavior gates passed against the frozen browser build, including analytic clipping/volume, closed topology, preserved sources/layers/targets, exact save/history, independent edits, atomic rejection and inspector limits. All 52 saved snapshots matched visible GPU vertex and solid index buffers. [MASK_EXTRACTION.md](docs/MASK_EXTRACTION.md) and its [build receipt](docs/evidence/mask-extract-build.json) retain numerical results and failed fixture/helper checks. Native GUI, physical devices, global intersections, large-scene performance and ZBrush extraction parity remain unverified.

Historical version **0.1.11** passed **282 tests** (247 core, 35 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Eight transform workflows and 43 behavior gates passed on the frozen browser build, including exact save/history, layers and targets, shader-only translation previews, cancellation, command barriers and legacy formats. All 57 saved snapshots matched visible GPU vertex and solid index buffers. [TRANSFORMS.md](docs/TRANSFORMS.md) and its [build receipt](docs/evidence/transforms-build.json) retain exact parameters, numerical errors and the corrected no-op redo and timing-check failures. Native GUI, physical devices, large-scene performance and ZBrush transform equivalence remain unverified.

Historical version **0.1.10** passed **247 tests** (219 core, 28 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Eight scene workflows and 42 behavior gates passed, with 71 saved snapshots matching every visible GPU vertex and solid triangle-index buffer. A separate inspector clipping gate checks 1,390 visible controls across 100 states. Active Draw preserved the inactive part and its upload count exactly, and six short release traces stayed quiet. [SUBTOOLS.md](docs/SUBTOOLS.md) and its receipts preserve the failed intermediate layout and exact release hashes. Native GUI, physical devices, large scenes, long sessions and ZBrush SubTool equivalence remain unverified.

Historical version **0.1.9** passed **208 tests** (188 core, 20 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Eight browser Morph workflows passed, including exact restore/swap/history, protected local restoration, synthetic pressure/radius, v4 and legacy file input, target invalidation, subdivision origin and sculpt-layer coexistence. All 66 saved snapshots matched GPU vertex data; 58 target binding checks and 15 exact layer compositions passed. [MORPH_TARGETS.md](docs/MORPH_TARGETS.md) and its [build receipt](docs/evidence/morph-target-build.json) retain hashes, numerical results and three failed helper attempts. Short release traces stayed quiet; physical devices, native GUI, large-target/long-session performance and ZBrush Morph response remain unverified.

Historical version **0.1.8** passed **179 tests** (161 core, 18 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Seven browser layer workflows passed: separate Draw/Move layers, intensity/visibility, base edits, delete/bake/history, fresh save/load, rejected import and subdivision locking/navigation. All 52 saved snapshots matched the GPU vertex data; 42 layered snapshots matched an independent composition oracle exactly. [SCULPT_LAYERS.md](docs/SCULPT_LAYERS.md) and its [build record](docs/evidence/sculpt-layers-build.json) retain hashes, numerical results and failed attempts. The measured short strokes stayed quiet after release; larger layers, long sessions, native GUI, physical devices and ZBrush layer response remain unverified.

Historical version **0.1.7** passed **156 tests** (138 core, 18 app), formatting, strict native/WASM Clippy, pure-core WASM compilation, browser build and ad-hoc-signed macOS packaging. Fourteen browser interaction scenarios passed against the frozen release, including repeated Move grabs, pressure, masks, release and canvas boundaries; performed undo/redo checks restored exact geometry and displayed vertex data matched saved files. Tiny brushes still limit long pulls, four stronger native movement-range thresholds remain failed, and existing damage is not repaired. [MOVE_QUALITY.md](docs/MOVE_QUALITY.md) and its [build record](docs/evidence/move-quality-build.json) retain numerical comparisons and rejected candidates. Headless timing is scoped to the recorded short traces; native GUI, physical tablet input and ZBrush Move parity remain unverified.

Historical version **0.1.6** passed **142 tests** (126 core, 16 app), formatting, strict native/WASM Clippy and pure-core WASM compilation. Browser and ad-hoc-signed macOS builds pass. [SCULPTING_FOUNDATIONS.md](docs/SCULPTING_FOUNDATIONS.md) describes persistent levels, remeshing/decimation, pressure and alphas with numerical checks and limits. Its browser/runtime and build identities are retained in the linked evidence; native GUI and physical tablet input remain unverified.

Historical version **0.1.5** passed **97 tests** (83 core, 14 app), formatting, strict native/WASM Clippy and core/app WASM compilation. Web and ad-hoc-signed macOS builds pass; the [build record](docs/evidence/long-stroke-build.json) binds the checked source and artifacts. Local face incidence accelerates normals and deformation guards. The input queue processes atomic dabs with an 8 ms budget checked between operations, coalesces older samples under overload, and ends pending work on release. [LONG_STROKES.md](docs/LONG_STROKES.md) records sustained browser tests on the supplied mesh and distinguishes new geometry prevention from existing damage.

Historical version 0.1.4 passed **84 tests** (74 core, 10 app), formatting, strict native/WASM Clippy and pure-core WASM compilation. Its [build record](docs/evidence/browser-performance-build.json) pins that implementation. It processed at most two queued dabs per redraw with an 8 ms budget checked between dabs and flushed pending work before commands. Its short browser tests missed the sustained backlog reproduced in [LONG_STROKES.md](docs/LONG_STROKES.md); these historical results do not establish the changed 0.1.5 behavior. Web compilation continues to use optimization level 3 rather than optimizing for file size.

Historical version 0.1.3 passed **75 tests** (67 core, 8 app). Its [build record](docs/evidence/draw-quality-build.json), [CPU comparison](docs/evidence/draw-overlap-comparison.json), [windowless GPU visuals](docs/evidence/draw-visual-comparison.json) and [isolated browser checks](docs/evidence/draw-browser-runtime.json) identify that earlier implementation. The 0.1.4 same-ray fixtures preserve its output exactly; this does not establish ZBrush parity. Full native desktop interaction, tablet input and global intersections remain unverified.

The historical 0.1.3 CPU timing run measured about 29–33 ms per Draw dab. The paired 0.1.4 native fixtures reduce median sculpting cost from about 33–36 ms to 9 ms, with exact geometry/normal/topology output. These native timings exclude app/GPU/history/UI work and do not establish browser frame time or physical input latency. The browser comparison is recorded separately; full-mesh scans and large meshes remain performance work.

The 0.1.2 [deformation build record](docs/evidence/crease-stability-build.json) and [stability report](docs/CREASE_STABILITY.md) remain historical, comparing 14 fixtures against the frozen 0.1.1 core. They do not identify the changed 0.1.3 Draw implementation.

Historical 0.1.1 had **47 passing tests** (39 core, 8 app). Formatting also passes. Strict native/WASM Clippy and web plus ad-hoc-signed macOS builds passed during this update; the hash-bound [visual-quality build record](docs/evidence/visual-quality-build.json) records final source/build checks and their limits. An earlier 0.1.1 Chrome startup displayed 4× antialiasing active with no reported errors. Runtime checks were not repeated after the final input/state fixes: current native visual quality, resize/toggle behavior and the complete editing workflow remain unverified. A compiled WASM file alone does not count as a browser test.

[VALIDATION.md](docs/VALIDATION.md) and the REA/OBJ evidence remain the historical **0.1.0** results. They do not establish the changed 0.1.1 rendering and subdivision behavior.

## Path to parity

[FEATURE_PARITY.md](docs/FEATURE_PARITY.md) defines feature families, acceptance gates, platform-specific evidence and remaining milestones. The inventory is provisional, so no percentage or completion date is claimed. Major remaining work includes reference-matched DynaMesh, automatic decimation while sculpting, quad retopology, robust multi-resolution detail transport and HD editing, SubTool folders/merge/split, full Gizmo/Transpose workflows, booleans, ZModeler, UVs/map baking, advanced brushes, complete sculpt/paint/mask layer workflows, dynamics, rendering, scripting and native format compatibility.

[REA_WORKFLOW.md](docs/REA_WORKFLOW.md) records the local source build of [morluto/rea](https://github.com/morluto/rea), reference binary identity and the investigation protocol. REA and its analysis providers are local development tools under `.tools`; they are not runtime dependencies or redistributed commercial code. Implementations in this repository are independently written Rust.

## License

OBrush source is licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. Dependencies retain their own licenses. OBrush is an independent project; ZBrush is a Maxon product and trademark.
